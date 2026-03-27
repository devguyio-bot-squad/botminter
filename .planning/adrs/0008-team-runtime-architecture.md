---
status: proposed
date: 2026-03-24
decision-makers: operator (ahmed), claude
---

# Team runtime architecture — Formation as internal deployment strategy

## Problem

A team's members need to run somewhere — as local processes, inside a Lima VM, as Kubernetes pods, or in Docker containers. Each deployment target has different mechanics for launching members, storing credentials, delivering tokens, checking health, and preparing the environment. Without a unifying abstraction, every command that touches member lifecycle must know about every deployment target.

Additionally, the operator should not need to understand the deployment internals. The Team is the natural API boundary — operators start, stop, and manage teams and members. How members actually run is an internal concern.

## Constraints

* The operator's CLI workflow (`bm start`, `bm stop`, `bm status`) MUST be the same regardless of deployment target
* The Team is the API boundary — operators interact with teams and members, never with formations, daemons, or deployment internals
* Credentials MUST be stored using the deployment target's native secret management (system keyring for local, Kubernetes Secrets for k8s, etc.) — not in config files
* Credential delivery (how tokens reach members at runtime) MUST be abstracted by the formation — different targets deliver tokens differently
* A team can only use one formation at a time — members don't split across deployment targets
* The formation MUST support multiple credential domains (bridge credentials AND GitHub App credentials) through a single generalized key-value interface
* The environment where agents run may differ from where the operator runs `bm` — the formation abstracts this boundary
* The team repo MUST be in a GitHub organization (personal accounts are not supported — required for `organization_projects` permission on GitHub Apps)

## Decision

### Concept hierarchy

| Concept | What it is | Who sees it |
|---------|-----------|-------------|
| **Team** | The user-facing entity. Has members, projects, a repo. The API boundary. | Operator |
| **Formation** | The deployment strategy. Manages everything below. Internal to the team. | Nobody (implementation detail) |

The formation manages these capabilities:

| Capability | What it does |
|-----------|-------------|
| **Environment** | Prepares and manages the target environment (verify local machine, create VM, configure K8s namespace) |
| **Credentials** | Stores and retrieves secrets (keyring, K8s Secrets) |
| **Credential delivery** | Gets tokens to members at runtime (`hosts.yml`, mounted volumes) |
| **Member lifecycle** | Launches, stops, and monitors member processes. The daemon is an implementation detail here. |

### How it works per formation type

| Capability | Local formation | Lima formation | K8s formation (future) |
|-----------|----------------|---------------|----------------------|
| Environment | Operator's machine (verify only) | Lima VM (create/manage via limactl) | K8s cluster (connect/configure namespace) |
| Credentials | System keyring (gnome-keyring / D-Bus) | Keyring inside VM | K8s Secrets |
| Credential delivery | `hosts.yml` via `GH_CONFIG_DIR` | Same (inside VM) | Mounted secret volume |
| Member lifecycle | Local processes, supervised by daemon | Processes inside VM, supervised by daemon | Pods, daemon per pod |

### Team as API boundary

Commands go through the Team. The Team holds its formation (resolved from config) and delegates:

```rust
// What bm start actually does:
let team = resolve_team(flag)?;
team.start(member_filter)?;     // internally: formation.start_members()
```

Operator-facing commands:
```
bm start [member] [-t team]      # team.start()
bm stop [member] [-t team]       # team.stop()
bm status [-t team]              # team.status()
bm hire <role> [-t team]         # team.hire()
bm fire <member> [-t team]       # team.fire()
bm chat <member> [-t team]       # team.chat()
bm attach [-t team]              # team.attach() → environment shell
bm env create [-t team]          # team.setup_env() → formation.setup()
bm env delete [-t team]          # team.teardown_env()
```

No `bm formation` or `bm daemon` commands. The formation and daemon are never exposed to the operator.

### The `Formation` trait

```rust
pub trait Formation {
    fn name(&self) -> &str;

    // ── Environment ──────────────────────────────────────────────

    /// Prepares the environment for running members.
    /// Local: verifies prerequisites. Lima: creates VM. K8s: configures namespace.
    fn setup(&self, params: &SetupParams) -> Result<()>;

    /// Checks if the environment is ready.
    fn check_environment(&self) -> Result<EnvironmentStatus>;

    /// Checks hard prerequisites. Fails fast with actionable errors.
    fn check_prerequisites(&self) -> Result<()>;

    // ── Credentials ──────────────────────────────────────────────

    /// Returns a key-value credential store for the given domain.
    /// The store interface is simple: store(key, value) / retrieve(key).
    /// Each credential domain composes its own key conventions.
    fn credential_store(&self, domain: CredentialDomain) -> Result<Box<dyn CredentialStore>>;

    /// One-time setup for token delivery to a member.
    /// Creates GH_CONFIG_DIR, writes initial config, configures git
    /// credential helper in workspace .git/config (not global .gitconfig).
    fn setup_token_delivery(&self, member: &str, workspace: &Path, bot_user: &str) -> Result<()>;

    /// Delivers a refreshed token to a member.
    /// Local: atomically writes hosts.yml. K8s: updates Secret.
    /// Called by the daemon on every token refresh cycle (every 50 min).
    fn refresh_token(&self, member: &str, workspace: &Path, token: &str) -> Result<()>;

    // ── Member lifecycle ─────────────────────────────────────────
    // The daemon is an implementation detail here — the formation
    // manages it internally as part of supervising members.

    /// Starts members. Internally ensures daemon is running, generates
    /// tokens, delivers credentials, launches member processes.
    fn start_members(&self, params: &StartParams) -> Result<StartResult>;

    /// Stops members. Daemon keeps running unless all members stopped.
    fn stop_members(&self, params: &StopParams) -> Result<StopResult>;

    /// Returns status of all members including token health.
    fn member_status(&self) -> Result<Vec<MemberStatus>>;

    // ── Interactive access ───────────────────────────────────────

    /// Execute a command in the formation's environment.
    /// Local: exec directly. Lima: SSH into VM then exec.
    fn exec_in(&self, workspace: &Path, cmd: &[&str]) -> Result<()>;

    /// Open an interactive shell in the formation's environment.
    fn shell(&self) -> Result<()>;

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

### CredentialStore trait (key-value)

The `CredentialStore` trait is a simple key-value secret store. It is formation-neutral — formations provide implementations.

```rust
pub trait CredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<()>;
    fn retrieve(&self, key: &str) -> Result<Option<String>>;
    fn remove(&self, key: &str) -> Result<()>;
    fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
}
```

Each credential domain composes its own key conventions:

| Domain | Key pattern | Example |
|--------|------------|---------|
| Bridge | `{member}` | `superman` → bridge token |
| GitHub App | `{member}/github-app-id` | `superman/github-app-id` → `"123456"` |
| GitHub App | `{member}/github-app-client-id` | `superman/github-app-client-id` → `"Iv1.abc123"` |
| GitHub App | `{member}/github-app-private-key` | `superman/github-app-private-key` → PEM string |
| GitHub App | `{member}/github-installation-id` | `superman/github-installation-id` → `"789012"` |

### Credential domains

```rust
pub enum CredentialDomain {
    /// Bridge credentials (e.g., Matrix/Telegram tokens).
    Bridge {
        team_name: String,
        bridge_name: String,
        state_path: PathBuf,
    },
    /// GitHub App credentials (App ID, Client ID, private key, installation ID).
    GitHubApp {
        team_name: String,
        member_name: String,
    },
}
```

The `CredentialDomain` determines which credential store implementation is returned and with what configuration (keyring service name, K8s namespace, etc.). The callers use the key-value interface uniformly.

### Supporting types

```rust
pub struct SetupParams {
    pub coding_agent: String,
    pub coding_agent_api_key: Option<String>,
}

pub struct EnvironmentStatus {
    pub ready: bool,
    pub checks: Vec<EnvironmentCheck>,
}

pub struct EnvironmentCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub struct StartParams<'a> {
    pub team: &'a TeamEntry,
    pub config: &'a BotminterConfig,
    pub team_repo: &'a Path,
    pub member_filter: Option<&'a str>,
}

pub enum MemberHandle {
    Local { pid: u32, workspace: PathBuf },
    // Future: K8s { namespace, pod, context }, Lima { vm, pid, workspace }
}
```

Note: `no_bridge` is NOT in `StartParams`. Bridge auto-start is daemon-level orchestration, not a formation concern. The `--no-bridge` CLI flag is handled by the command layer before calling `team.start()`.

### The daemon — an implementation detail

The daemon is a supervisor process that manages member lifecycle on behalf of the formation. It is NEVER exposed to the operator.

The daemon handles:
- Member process launch, health monitoring, and restart
- Token refresh loops (JWT signing → installation token exchange → `refresh_token()`)
- Webhook/poll event loop
- Web console
- Bridge auto-start (before launching members, based on team config)
- State management — daemon owns `state.json`, CLI reads only

**CLI↔Daemon communication:** The daemon exposes an HTTP API on `127.0.0.1:{port}` (using the existing axum server). The daemon writes PID + port to a state file. CLI commands check daemon health and send requests (start member, stop member, status query). All state mutations go through the daemon — no race conditions on `state.json`.

How the formation uses the daemon:
- **Local formation**: starts one daemon process, all members are children
- **Lima formation**: starts daemon inside the VM (via SSH), all members inside VM
- **K8s formation** (future): each pod contains a daemon + one member

The daemon caches App credentials in memory at startup (read from credential store). It refreshes installation tokens at the 50-minute mark. On crash recovery, it re-adopts orphaned members from `state.json` and immediately refreshes tokens.

### Formation resolution

The formation is resolved from the team's config. The Team struct holds the formation:

```rust
impl Team {
    fn formation(&self) -> Result<Box<dyn Formation>> {
        formation::create(&self.team_repo, &self.formation_name)
    }
}
```

### Module structure

```
formation/
  mod.rs              # Formation trait, CredentialDomain, supporting types,
                      #   resolve, create, load, list
  config.rs           # FormationConfig parsing
  local/
    mod.rs            # Platform detection, delegates to linux/ or macos/
    process.rs        # Shared POSIX process lifecycle (launch, stop, is_alive)
    topology.rs       # Shared local topology writing
    daemon.rs         # Daemon management (start, stop, adopt orphans)
    linux/
      mod.rs          # LinuxLocalFormation — Formation trait impl
      credential.rs   # LocalCredentialStore, GitHubAppCredentialStore
      setup.rs        # Prerequisite verification, keyring setup
    macos/
      mod.rs          # MacosLocalFormation — stub ("not yet supported")
  lima/
    mod.rs            # LimaFormation — Formation trait impl
    vm.rs             # VM lifecycle (create, start, stop, delete)
```

### What a formation owns

| Responsibility | Description |
|---|---|
| **Environment management** | Preparing and managing the target environment |
| **Member lifecycle** | Launching, stopping, health-checking members (daemon is internal) |
| **Credential storage** | Platform-specific secret storage (keyring, K8s Secrets) |
| **Credential delivery** | Getting tokens to members at runtime |
| **Prerequisites** | What must be installed/configured for this formation |
| **Topology writing** | Recording where members are running |
| **Interactive access** | How to exec commands or shell into the environment |

### What a formation does NOT own

| Concern | Why it's not a formation concern |
|---|---|
| Bridge lifecycle (start/stop/health) | Bridges are team infrastructure managed via bridge manifests |
| Bridge identity provisioning | Bridge domain — creating bot users on a chat platform |
| `CredentialStore` trait definition | Formation-neutral interface — formations provide implementations |
| Workspace provisioning (`bm teams sync`) | Workspace layout is formation-independent |
| GitHub App creation (manifest flow) | Happens during `bm hire`, uses operator's auth, not formation-specific |
| Team repo management | Git operations on the team repo |

### How commands interact

Commands are formation-agnostic. They go through the Team:

1. **Resolve** the team (from `-t` flag or default)
2. **Call** team methods (`team.start()`, `team.stop()`, etc.)
3. Team **delegates** to its formation internally
4. Commands **never** instantiate formation-specific types directly

## Rejected Alternatives

### No formation concept — hardcode local behavior

Rejected because: every command that touches member lifecycle would need to branch on deployment type. Adding K8s or Lima support would require changing every command.

### Formation exposed to operator (`bm formation start`)

Rejected because: the operator should think in terms of teams and members, not deployment internals. The formation is how the team runs, not what the operator manages.

### Daemon as a first-class operator concept

Rejected because: the daemon is how the formation supervises members. Exposing it to operators leaks abstraction and creates confusion about whether to use `bm start` or `bm daemon start`.

### Separate `bm runtime` commands independent of formation

Rejected because: the environment is a formation capability. Separating `bm runtime create` from formation creates two parallel concepts for the same thing.

### `credential_store()` with bridge-specific parameters

Rejected because: GitHub App credentials are also a formation concern. The credential store must support multiple domains through a `CredentialDomain` enum, not hardcoded bridge parameters.

## Consequences

* The Team is the only API surface operators interact with for runtime operations
* Adding a new formation type means implementing the `Formation` trait — the compiler enforces completeness
* Commands are formation-agnostic — they call team methods which delegate to the formation
* Credential storage and delivery are formation-driven — the local formation uses the system keyring + `hosts.yml`, a K8s formation would use Secrets + mounted volumes
* The daemon exists but is invisible to operators — it's managed by the formation
* `bm runtime create/delete` are replaced by `bm env create/delete` which delegate to `formation.setup()`
* The local formation can evolve from simple mode (current user) to isolated mode (dedicated user) without changing any commands

## Anti-patterns

* **Do NOT** expose the formation to operators — commands go through the Team, never directly to the formation
* **Do NOT** expose the daemon to operators — it's an implementation detail of member lifecycle
* **Do NOT** hardcode platform-specific types in commands — always go through Team → Formation
* **Do NOT** use bridge-specific parameters on `credential_store()` — use the `CredentialDomain` enum
* **Do NOT** make credential delivery formation-independent — different formations deliver tokens differently (`hosts.yml` vs mounted volumes)
* **Do NOT** put bridge lifecycle in the formation — bridges are team infrastructure, not a deployment concern
* **Do NOT** inspect `MemberHandle` variants in commands — pass them opaquely back to the formation
* **Do NOT** call `formation.start_members()` from commands — call `team.start()` which delegates internally
