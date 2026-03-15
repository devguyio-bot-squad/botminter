---
status: proposed
date: 2026-03-15
decision-makers: operator (ahmed), claude
---

# Local Formation — Deployment Strategy for Running Members Locally

## Problem

A team's members need to run somewhere — as local processes on the operator's machine, as pods in a Kubernetes cluster, as Docker containers, or on remote machines. Each deployment target has different mechanics for launching members, stopping them, checking their health, storing credentials, recording where they are running, and preparing the environment in the first place. Without a unifying concept, every command that touches member lifecycle must know about every deployment target.

This ADR defines the formation concept and specifies the local formation. Non-local formations (k8s, Docker, etc.) will be covered in a future ADR.

## Constraints

* Different deployment targets (local machine, Kubernetes, Docker, remote SSH) have fundamentally different mechanisms for process lifecycle, credential storage, and health checking
* Credentials must be stored securely using the deployment target's native secret management (system keyring for local, Kubernetes Secrets for k8s, etc.) — not in config files or state files
* The operator's CLI workflow (`bm start`, `bm stop`, `bm status`) must be the same regardless of deployment target — the deployment strategy is an infrastructure concern, not a user-facing workflow change
* A team can only use one formation at a time — members don't split across deployment targets within a single team
* The environment where agents run may differ from where the operator runs `bm` — a dedicated user account, a container, a remote machine. The formation must abstract this boundary so commands don't need to know where or how things execute

## Decision

### What is a formation?

A **formation** is a deployment strategy that defines how team members are run. It is an **environment abstraction** — it encapsulates where things happen and how the operator interacts with that environment. It answers:

- **How is the environment prepared?** User account creation, tool installation, coding agent configuration
- **How are members launched?**
- **How are members stopped?**
- **How is member health checked?**
- **Where are credentials stored?**
- **How are credentials delivered to members?**
- **What prerequisites are required?**
- **How is the topology recorded?**

Each formation is a named directory in the profile under `formations/`, containing at minimum a `formation.yml`.

### The `Formation` trait

The formation concept is expressed as a trait. Commands receive a `Box<dyn Formation>` and call its methods without knowing which deployment target is behind it or how it crosses environment boundaries.

```rust
/// A deployment strategy for running team members.
///
/// Each formation type (local, k8s, Docker, etc.) implements this trait.
/// Commands receive a `Box<dyn Formation>` and are deployment-agnostic.
///
/// The formation is an environment abstraction. Its methods may execute
/// in the current process, in a different user account, in a container,
/// or on a remote machine — commands don't know and don't care.
pub trait Formation {
    /// The formation name (e.g., "local", "k8s").
    fn name(&self) -> &str;

    // ── Environment ──────────────────────────────────────────────

    /// Prepares the environment for running members.
    ///
    /// Local: installs ralph, configures coding agent, sets up keyring.
    /// Local (isolated): also creates dedicated user account.
    /// K8s: configures namespace, creates service accounts, pulls images.
    fn setup(&self, params: &SetupParams) -> Result<()>;

    /// Checks if the environment is ready for running members.
    ///
    /// Returns a structured status indicating what is ready, what is
    /// missing, and what needs attention. Unlike check_prerequisites()
    /// which is a hard pass/fail gate, this is a diagnostic.
    fn check_environment(&self) -> Result<EnvironmentStatus>;

    /// Checks that the hard prerequisites for this formation are met.
    ///
    /// Local: `ralph` in PATH, keyring accessible.
    /// K8s: `kubectl` configured, cluster reachable.
    ///
    /// Called before launch. Fails fast with actionable error messages.
    fn check_prerequisites(&self) -> Result<()>;

    // ── Credentials ──────────────────────────────────────────────

    /// Returns a credential store for this formation's secret backend.
    ///
    /// Local: system keyring (gnome-keyring / D-Bus Secret Service).
    /// K8s: Kubernetes Secrets.
    fn credential_store(
        &self,
        team_name: &str,
        bridge_name: &str,
        state_path: PathBuf,
    ) -> Result<Box<dyn CredentialStore>>;

    // ── Member lifecycle ─────────────────────────────────────────

    /// Launches a member. Returns a member handle.
    fn launch_member(&self, params: &LaunchParams) -> Result<MemberHandle>;

    /// Stops a member gracefully, or forcefully if `force` is true.
    fn stop_member(&self, handle: &MemberHandle, force: bool) -> Result<()>;

    /// Checks if a member is alive.
    fn is_member_alive(&self, handle: &MemberHandle) -> bool;

    // ── Topology ─────────────────────────────────────────────────

    /// Writes a topology file recording where members are running.
    fn write_topology(
        &self,
        workzone: &Path,
        team_name: &str,
        members: &[(String, MemberHandle)],
    ) -> Result<()>;
}
```

### Supporting types

```rust
/// Parameters for environment setup.
pub struct SetupParams {
    pub coding_agent: String,       // e.g., "claude-code", "gemini-cli"
    pub coding_agent_api_key: Option<String>,
}

/// Result of checking the environment.
pub struct EnvironmentStatus {
    pub ready: bool,
    pub checks: Vec<EnvironmentCheck>,
}

pub struct EnvironmentCheck {
    pub name: String,       // e.g., "ralph installed", "keyring accessible"
    pub passed: bool,
    pub detail: String,     // human-readable status or fix instructions
}

/// Parameters for launching a member.
pub struct LaunchParams<'a> {
    pub workspace: &'a Path,
    pub gh_token: &'a str,
    pub member_token: Option<&'a str>,
    pub bridge_type: Option<&'a str>,
    pub service_url: Option<&'a str>,
}

/// Opaque handle to a running member.
///
/// The concrete data depends on the formation:
/// - Local: PID + workspace path
/// - K8s: namespace + pod name + context
///
/// Commands pass handles back to the formation for stop/health operations.
/// They do not inspect the contents.
pub enum MemberHandle {
    Local { pid: u32, workspace: PathBuf },
    // Future: K8s { namespace, pod, context }, Docker { container_id }, etc.
}
```

### Formation resolution

```rust
/// Resolves which formation to use.
/// --formation flag > default "local" > None (legacy team without formations/).
pub fn resolve(team_repo: &Path, flag: Option<&str>) -> Result<Option<String>>;

/// Constructs a Formation implementation from config.
/// Returns a `Box<dyn Formation>` — commands don't know the concrete type.
pub fn create(team_repo: &Path, name: &str) -> Result<Box<dyn Formation>>;

/// Loads a formation config by name.
pub fn load(team_repo: &Path, name: &str) -> Result<FormationConfig>;

/// Lists available formation names.
pub fn list(team_repo: &Path) -> Result<Vec<String>>;
```

### The local formation

The `local` formation runs members as local processes on the operator's machine. It implements the `Formation` trait. The runtime detects the platform (Linux, macOS) and uses the appropriate platform-specific implementation.

#### Configuration

```yaml
name: local
description: "Run members as local processes"
type: local
```

#### Platform architecture

The local formation's responsibilities split into a **shared POSIX core** and **platform-specific pieces**:

| Piece | Linux | macOS | Shared? |
|---|---|---|---|
| **Process lifecycle** (launch, stop, health) | `fork`/`exec`, `kill(pid, 0)`, `SIGTERM` | Same — POSIX | Shared |
| **Topology** | `Endpoint::Local { pid, workspace }` | Same | Shared |
| **Coding agent config** | `~/.claude/` paths, API key setup | Same | Shared |
| **Secret storage** | gnome-keyring via D-Bus Secret Service | macOS Keychain via Security framework | Platform-specific |
| **Secret infrastructure** | D-Bus session bus, collection management, `dbus-secret-service` crate | No D-Bus. Keychain groups, XPC | Platform-specific |
| **User account isolation** | `useradd`, `su -`, `/etc/sudoers` | `sysadminctl`/`dscl`, different sudoers model | Platform-specific |
| **Tool installation** (setup) | `dnf`/`apt`/`pacman`, or `cargo install` | `brew`, or `cargo install` | Platform-specific |
| **Service/daemon management** | systemd user services | launchd agents | Platform-specific |

This gives four platform-specific pieces:

1. **Secret backend** — the entire credential storage infrastructure (storage, collection/group management, session routing, prerequisite checks)
2. **Account management** — how to create and configure the dedicated user account for isolated mode
3. **Package management** — how to install ralph and the coding agent
4. **Service management** — how to run agents as persistent services

And one shared POSIX core:

- **Process lifecycle** — launch (`Command::new("ralph")`), stop (`SIGTERM` / `ralph loops stop`), health (`kill(pid, 0)`), PID tracking

The `formation::create()` factory detects the current platform and returns the right implementation. Both `LinuxLocalFormation` and `MacosLocalFormation` implement the `Formation` trait and share the POSIX process management code.

#### Unsupported platforms

On platforms where the local formation has no implementation (e.g., Windows), `formation::create()` returns an error with a clear message:

> Local formation is not supported on this platform. Supported: Linux, macOS.

The `check_environment()` method also reports the platform as the first check, so `bm status` can show whether the formation is compatible before the operator tries to launch anything.

#### Behavior

| Trait method | How `LocalFormation` implements it |
|---|---|
| `setup()` | Installs ralph, configures coding agent, sets up platform-specific secret backend. In simple mode, operates on the current user. In isolated mode (future), creates a dedicated user account first |
| `check_environment()` | Reports platform compatibility, status of ralph, coding agent, secret backend, user account |
| `check_prerequisites()` | Verifies `ralph` in PATH, secret backend accessible |
| `credential_store()` | Returns a platform-specific `CredentialStore` — `LocalCredentialStore` (Linux keyring) or `KeychainCredentialStore` (macOS Keychain) |
| `launch_member()` | Spawns `ralph run -p PROMPT.md` as a background process (shared POSIX), injects credentials as env vars, returns `MemberHandle::Local { pid, workspace }` |
| `stop_member()` | Graceful: runs `ralph loops stop` in workspace, polls for PID exit (up to 60s). Forced: sends `SIGTERM` (shared POSIX) |
| `is_member_alive()` | `kill(pid, 0)` — checks if the process is alive (shared POSIX) |
| `write_topology()` | Writes `Endpoint::Local { pid, workspace }` entries to topology file (shared) |

In simple mode (initial implementation), every method operates directly as the current user — no boundary crossing. The operator is expected to be in the right user account (via `bm connect` or directly). If the local formation later supports transparent boundary crossing (e.g., sudo to a dedicated user), only the `LocalFormation` implementation changes — commands and the trait are unaffected.

#### Linux secret infrastructure

On Linux, the local formation owns all keyring-related behavior:

- **`LocalCredentialStore`** — implements `CredentialStore` trait using `keyring::Entry` for store/retrieve/remove, reads `bridge-state.json` identities for list (keyring doesn't support enumeration)
- **D-Bus session routing** — `with_keyring_dbus()` temporarily swaps `DBUS_SESSION_BUS_ADDRESS` so keyring operations can target an isolated D-Bus session while the process-wide bus stays on the real system bus
- **Collection management** — `with_collection()` uses `dbus-secret-service` directly to target a named collection instead of the default `login` collection
- **Prerequisite checks** — `check_keyring_unlocked()` verifies the Secret Service default collection is accessible; `ensure_collection_exists()` creates a named collection if it doesn't exist
- **Configuration** — `keyring_collection: Option<String>` in `config.yml` specifies which keyring collection to use

#### macOS secret infrastructure

On macOS, the local formation uses the macOS Keychain (future — not yet implemented):

- **`KeychainCredentialStore`** — implements `CredentialStore` trait using the Security framework
- **No D-Bus** — macOS has no D-Bus; keychain access is direct via the Security framework
- **Keychain groups** — equivalent to Linux collections, but with different semantics

### What a formation owns

| Responsibility | Description |
|---|---|
| **Environment setup** | Preparing the target environment — installing tools, configuring agents, creating accounts |
| **Environment status** | Reporting what is ready, what is missing, what needs attention |
| **Member launch** | How to start a member — binary, arguments, env vars, detach strategy |
| **Member stop** | How to stop a member — signal, polling, timeout |
| **Member health** | How to check if a member is alive |
| **Credential storage** | Implementation of the `CredentialStore` trait — where secrets are persisted |
| **Credential delivery** | How credentials reach members at runtime |
| **Prerequisites** | What must be installed and configured for this formation to work |
| **Topology writing** | Recording where members are running using the appropriate `Endpoint` variant |
| **Formation-specific infrastructure** | Anything specific to the deployment target — keyring management for local, namespace management for k8s |

### What a formation does NOT own

| Concern | Why it's not a formation concern |
|---|---|
| Bridge lifecycle (start/stop/health) | Bridges are team infrastructure managed by commands via bridge manifests and recipes |
| Bridge identity provisioning | Bridge domain — creating bot users and tokens on a chat platform |
| `CredentialStore` trait definition | The trait is the formation-neutral interface — formations provide implementations of it |
| `RuntimeState` / `Topology` data formats | Shared data formats — formations write them, commands read them for display |
| Workspace provisioning (`bm teams sync`) | Workspace layout is formation-independent — submodule setup, file surfacing |
| Interactive sessions (`bm chat`) | Chat always execs into a local workspace regardless of formation |
| GitHub token management | Team credential, not a deployment concern |

### How commands interact with the formation

Commands are formation-agnostic. They:

1. **Resolve** the active formation name (from `--formation` flag or default `"local"`)
2. **Create** a `Box<dyn Formation>` via `formation::create()`
3. **Call** trait methods on the formation object
4. **Never** instantiate formation-specific types directly (no `LocalCredentialStore::new()`, no `LocalFormation` in commands)

### Formation module structure

Per ADR-0006, the formation module is a directory module:

```
formation/
  mod.rs              # Formation trait, resolve, create, load, list,
                      #   LaunchParams, MemberHandle, SetupParams,
                      #   EnvironmentStatus, FormationConfig
  config.rs           # FormationConfig parsing, K8sConfig, ManagerConfig
  local/
    mod.rs            # Platform detection, delegates to linux/ or macos/
    process.rs        # Shared POSIX process lifecycle (launch, stop, is_alive)
    topology.rs       # Shared local topology writing
    linux/
      mod.rs          # LinuxLocalFormation — Formation trait impl
      credential.rs   # LocalCredentialStore, gnome-keyring, D-Bus, collection management
      setup.rs        # dnf/apt, useradd, systemd, keyring setup
    macos/
      mod.rs          # MacosLocalFormation — Formation trait impl
      credential.rs   # KeychainCredentialStore, macOS Keychain
      setup.rs        # brew, sysadminctl, launchd, keychain setup
```

### Non-local formations

Non-local formations (k8s, Docker, etc.) are out of scope for this ADR. They will be defined in a future ADR. The `Formation` trait is designed to accommodate them — a `K8sFormation` or `DockerFormation` would implement the same trait with different behavior behind each method.

## Rejected Alternatives

### No formation concept — hardcode local behavior, add k8s later

Rejected because: every command that touches member lifecycle would need to branch on deployment type.

- Adding a third type (Docker, SSH) would require changing every command again
- The formation concept centralizes deployment-specific logic in one place per deployment target

### Free functions with string-based dispatch instead of a trait

Rejected because: passing `formation_name: &str` to every function and dispatching internally is a manual vtable — the trait gives the same dispatch automatically with compile-time safety.

- A trait makes the interface explicit and self-documenting
- New formations implement the trait — the compiler tells you what methods are missing
- `Box<dyn Formation>` is the standard Rust pattern for this (equivalent to Go interfaces)

### Formation manages bridge lifecycle

Rejected because: bridges are team infrastructure with their own manifests, recipes, and lifecycle — not a deployment concern.

- A bridge can exist without a formation (external bridges like Telegram)
- Conflating bridge lifecycle with formation lifecycle creates a false dependency

### Commands cross user boundaries directly (sudo/SSH per operation)

Rejected because: wrapping every formation operation with user-crossing logic in commands defeats the purpose of the abstraction.

- If boundary crossing is needed, the formation implementation handles it internally
- Commands always call trait methods the same way — the formation decides how to execute them
- This keeps the door open for both simple (current user) and isolated (dedicated user) modes without any command changes

## Consequences

* The formation module is the single place to understand how a deployment target works — including environment setup
* Adding a new formation type means implementing the `Formation` trait — the compiler enforces completeness
* Commands become formation-agnostic — they call trait methods on `Box<dyn Formation>`
* Credential storage is formation-driven — the local formation uses the system keyring, a future k8s formation would use Kubernetes Secrets, commands don't need to know which
* The `CredentialStore` trait remains in the bridge module as the formation-neutral interface
* Environment setup (`bm setup`) is formation-driven — the local formation installs ralph and configures the coding agent, a k8s formation would configure namespaces and service accounts
* The local formation can evolve from simple mode (current user) to isolated mode (dedicated user) without changing any commands or the trait interface

## Anti-patterns

* **Do NOT** put bridge-neutral code in the formation module — the `CredentialStore` trait and `resolve_credential_from_store()` work with any formation
* **Do NOT** make shared data formats (`Topology`, `RuntimeState`) formation-private — commands need to read them
* **Do NOT** hardcode formation-specific types in commands — always go through `Box<dyn Formation>` and the formation module's public API
* **Do NOT** add formation awareness to `bm chat` — interactive sessions exec into a local workspace regardless of formation
* **Do NOT** inspect `MemberHandle` variants in commands — pass them opaquely back to the formation for stop/health operations
* **Do NOT** put user-boundary-crossing logic in commands — if the formation needs to cross a user boundary, it does so internally in its trait implementation
* **Do NOT** use platform-specific APIs (D-Bus, keyring, systemd) in shared code — platform-specific behavior belongs in `linux/` or `macos/` sub-modules, not in the shared POSIX layer
