# Formation Abstraction Audit

**Date:** 2026-03-15
**Branch:** local-formation
**Purpose:** Inventory the current formation concept, trace its usage, identify gaps, and distinguish between code that bypasses existing capabilities vs code that needs new formation capabilities.

---

## 0. Documented Design Intent

Before looking at the code, formation has documented expectations across several sources. These show what formation was **intended** to be, not just what it is today.

### Bridges concept page (`docs/content/concepts/bridges.md`)

The bridges documentation explicitly describes formation-aware credential storage as a current design feature:

> **Formation-aware credential storage**
>
> Credential storage is formation-aware through the CredentialStore trait:
>
> | Formation | Storage backend | Status |
> |-----------|----------------|--------|
> | **Local** | System keyring (via `keyring` crate) | Implemented |
> | **Kubernetes** | K8s Secrets | Planned |
>
> The CredentialStore trait provides `store`, `retrieve`, `remove`, and `list` operations. **The active formation determines which backend is used.** This means the same bridge code works across formations -- only the credential storage changes.

The docs claim "the active formation determines which backend is used" — but in the code, no formation routing exists. Commands hardcode `LocalCredentialStore` directly.

The credential flow is also documented:

> 1. **Collection** — During `bm bridge identity add` or `bm teams sync --bridge`
> 2. **Config exchange** — Bridge recipes write credentials to `$BRIDGE_CONFIG_DIR/config.json`
> 3. **Storage** — BotMinter stores credentials in the system keyring **(local formations)** via the CredentialStore trait
> 4. **Injection** — At `bm start`, credentials are resolved from the keyring and injected as environment variables

Step 3 explicitly scopes keyring to "local formations," and step 4 describes credential delivery as a separate concern from storage.

### Phase 09 context (`09-CONTEXT.md`)

The design decision that introduced credential management was explicit about formation ownership:

> **Formation-aware secret storage:** design the abstraction in Phase 9, implement the local keyring backend. K8s secret backend comes with K8s formation. This means **credentials route through the formation layer for storage** rather than living only in bridge-state.json.

And:

> The formation-aware secret storage abstraction is forward-looking: design the trait/interface now so when K8s formation lands, the storage backend plugs in without restructuring.

The intent was clear: credentials route through the formation. The trait was designed, the local backend was implemented, but the routing through formation was never wired.

### Requirements (`REQUIREMENTS.md`)

> **BRDG-09**: Bridge credentials resolved in priority order: env var `BM_BRIDGE_TOKEN_{USERNAME}` → system keyring **(formation credential store)**.

The requirement calls the keyring the "formation credential store" — it is not just a bridge concern, it's a formation concern.

### Roadmap (`docs/content/roadmap.md`)

The v0.07 milestone summary:

> **Credential storage** — system keyring with env var fallback, **formation-aware via CredentialStore trait**
>
> **Proving**: Bridge plugin model is extensible. Local bridges can self-provision. **Credential management works across formation types.**

The milestone was supposed to prove that credential management works across formation types.

### Codebase concerns (`CONCERNS.md`)

> **Formation credential resolution is incomplete:**
> Issue: Two TODO comments in `commands/start.rs` note that bridge env var naming is hardcoded and that the formation manager should resolve per-member credentials via CredentialStore.

The codebase analysis itself identified this as an incomplete concern.

### Configuration reference (`docs/content/reference/configuration.md`)

Formations are documented as a first-class config section:

> **Formation config — `formations/{name}/formation.yml`**
>
> Profiles support formations — deployment targets for team members.

### Summary of intent vs reality

| Documented claim | Reality |
|---|---|
| "The active formation determines which backend is used" | Commands hardcode `LocalCredentialStore` — no formation routing |
| "Credentials route through the formation layer for storage" | Credentials go directly from commands to `LocalCredentialStore` |
| "Credential management works across formation types" | Only the local type is implemented, and it's not routed through formation |
| "Formation config is a first-class config section" | Formation config is loaded but ignored for local; only used as a routing branch for non-local |
| K8s backend "plugs in without restructuring" | The trait is ready, but without formation routing, a new backend would require changing all command call sites |

---

## 1. What Formation Currently Provides

The formation concept spans two areas: **config/resolution** and **credential management**. These live in different modules but both belong to the formation.

### 1a. Config Loading and Name Resolution (`formation.rs`)

The `formation.rs` module is a config loader and name resolver:

| Function / Type | What it does |
|---|---|
| `FormationConfig` | Data struct: `name`, `description`, `formation_type`, optional `K8sConfig`, optional `ManagerConfig` |
| `FormationConfig::is_local()` | Returns `true` if `formation_type == "local"` |
| `load(team_repo, name)` | Reads `formations/<name>/formation.yml` and deserializes it |
| `list_formations(team_repo)` | Lists subdirectory names under `formations/` |
| `resolve_formation(team_repo, flag)` | Resolves which formation to use: explicit flag -> validate exists; no flag + formations dir -> `"local"`; no formations dir -> `None` (legacy) |
| `formations_dir(team_repo)` | Returns `<team_repo>/formations/` path |

### 1b. Credential Management — the Local Formation's Behavioral Surface (`bridge.rs`)

The `CredentialStore` trait and `LocalCredentialStore` implementation are **local formation behavior** that currently lives in `bridge.rs`. The code itself declares this:

```rust
/// Different formation backends implement this trait:
/// - `LocalCredentialStore` uses the system keyring (local formation)
/// - `InMemoryCredentialStore` for testing
/// - Future: K8s Secrets backend for K8s formation
pub trait CredentialStore {
    fn store(&self, member_name: &str, token: &str) -> Result<()>;
    fn retrieve(&self, member_name: &str) -> Result<Option<String>>;
    fn remove(&self, member_name: &str) -> Result<()>;
    fn list(&self) -> Result<Vec<String>>;
}
```

`LocalCredentialStore` in `bridge.rs` wraps the system keyring (gnome-keyring via D-Bus Secret Service). It provides:

| Capability | Implementation |
|---|---|
| Store a per-member credential | `keyring::Entry::new(&service, member).set_password(token)` |
| Retrieve a per-member credential | `keyring::Entry::new(&service, member).get_password()` |
| Remove a per-member credential | `keyring::Entry::new(&service, member).delete_credential()` |
| List stored members | Reads member names from `bridge-state.json` identities (keyring doesn't support enumeration) |
| D-Bus session routing | `with_keyring_dbus()` — temporarily swaps `DBUS_SESSION_BUS_ADDRESS` so keyring ops can target an isolated bus while the process-wide bus stays on the real system bus |
| Collection targeting | `with_collection()` — uses `dbus-secret-service` directly to target a named collection instead of the default `login` collection |

Supporting local-formation infrastructure in `bridge.rs`:

| Function | What it does |
|---|---|
| `check_keyring_unlocked()` | Verifies the Secret Service default collection is unlocked |
| `check_keyring_unlocked_for(collection)` | Verifies a named collection is unlocked |
| `ensure_collection_exists(name)` | Creates a Secret Service collection if it doesn't exist |

And in `config.rs`:

| Field | What it does |
|---|---|
| `keyring_collection: Option<String>` | Stored in `~/.botminter/config.yml` — configures which keyring collection the local formation uses |

This is real behavioral code — store, retrieve, remove, check prerequisites, manage collections. It is the local formation's credential backend. A k8s formation would replace this entire surface with Kubernetes Secrets API calls.

### 1c. Profile-Embedded Formation Files

Both `scrum` and `scrum-compact` profiles ship two formations:

**`formations/local/formation.yml`:**
```yaml
name: local
description: "Run members as local processes"
type: local
```

**`formations/k8s/formation.yml`:**
```yaml
name: k8s
description: "Deploy members as pods to a local Kubernetes cluster"
type: k8s
k8s:
  context: kind-botminter
  image: ghcr.io/owner/ralph:latest
  namespace_prefix: botminter
manager:
  ralph_yml: ralph.yml
  prompt: PROMPT.md
  hats_dir: hats/
```

The k8s formation also ships a `PROMPT.md`, `ralph.yml`, and hat directories for its formation manager (a one-shot Ralph session that deploys pods).

The local formation ships only the bare YAML. It has no manager, no hats, no PROMPT.md — because all local behavior is hardcoded in the commands.

### 1d. Supporting Modules

**`topology.rs`** — Records formation output (where members are running):

```rust
pub struct Topology {
    pub formation: String,         // e.g., "local" or "k8s"
    pub created_at: String,
    pub members: HashMap<String, MemberTopology>,
}

pub enum Endpoint {
    Local { pid: u32, workspace: PathBuf },
    K8s { namespace: String, pod: String, container: String, context: String },
}
```

**`state.rs`** — Tracks local process runtime state:

```rust
pub struct RuntimeState {
    pub members: HashMap<String, MemberRuntime>,
}

pub struct MemberRuntime {
    pub pid: u32,
    pub started_at: String,
    pub workspace: PathBuf,
}
```

This module is entirely local-formation-specific (PIDs, local workspaces, `libc::kill` for liveness).

---

## 2. Where Formation IS Being Used

Every call site of `formation::*` functions:

### a) `commands/start.rs` — `resolve_formation()`

```rust
let resolved_formation = formation::resolve_formation(&team_repo, formation_flag)?;
```

Used correctly for name resolution. Resolves which formation name to use from `--formation` flag or defaults to `"local"`.

### b) `commands/start.rs` — `resolve_formation()` result used to skip the formation

```rust
if let Some(ref fname) = resolved_formation {
    if fname != "local" {
        let formation_cfg = formation::load(&team_repo, fname)?;
        if !formation_cfg.is_local() {
            return run_formation_manager(team, &team_repo, &formation_cfg, &cfg.workzone);
        }
    }
}
```

For the **non-local** path: `FormationConfig` is loaded and genuinely used — `run_formation_manager()` reads `ManagerConfig` to find the PROMPT.md and ralph.yml for the formation manager session. The formation concept works here.

For the **local** path: the formation is consulted for its name and then **dismissed**. The code checks `fname != "local"` as a string comparison and doesn't even load `FormationConfig`. The `local/formation.yml` file (with its `name`, `description`, and `type` fields) is never read by any runtime code. The local behavior that follows has no connection to the formation module — it hardcodes everything: how to launch ralph, how to resolve credentials, how to write topology, how to manage bridge lifecycle.

### c) `commands/start.rs` — `run_formation_manager()`

```rust
let formation_dir = formation::formations_dir(team_repo).join(&formation_cfg.name);
let prompt_path = formation_dir.join(&mgr.prompt);
let ralph_yml_path = formation_dir.join(&mgr.ralph_yml);
```

Used correctly. The non-local path reads `ManagerConfig` from `FormationConfig` to locate the formation manager's PROMPT.md and ralph.yml, launches a one-shot Ralph session, and verifies the topology file was written.

### d) `completions.rs` — `list_formations()`

```rust
.and_then(|repo| formation::list_formations(repo).ok())
```

Used correctly. Provides shell completion candidates for the `--formation` flag.

### e) `tests/integration.rs` — Unit tests

Tests `list_formations`, `load`, and `resolve_formation` against extracted profile output. Used correctly.

---

## 3. Where Formation SHOULD Have Been Used (Capability Exists, Code Bypasses It)

### a) Topology formation name — hardcoded instead of passed through

In `write_local_topology()`:

```rust
let topo = Topology {
    formation: "local".to_string(),  // hardcoded
    ...
};
```

The `resolved_formation` variable holds the actual formation name but is never passed into `write_local_topology()`. Instead, `"local"` is hardcoded. If a future formation were `is_local() == true` but named something else (e.g., `"docker-compose"`), the topology would incorrectly say `"local"`.

**Severity:** Minor — the resolved name should be threaded through.

### b) Credential management exists as formation behavior but commands bypass the formation to use it

The local formation has a full credential management implementation (see section 1b): `CredentialStore` trait, `LocalCredentialStore` with system keyring integration, prerequisite checks, collection management. This is real formation behavior that works today.

But every command that touches credentials directly instantiates `LocalCredentialStore` — bypassing the formation entirely:

| Command | Operation |
|---|---|
| `bm bridge identity add` | Store token in keyring |
| `bm bridge identity rotate` | Store rotated token in keyring |
| `bm bridge identity remove` | Remove credential from keyring |
| `bm bridge identity show` | Retrieve token from keyring |
| `bm hire` | Store bootstrap token at hire time |
| `bm start` | Retrieve per-member tokens at launch |
| `bm teams sync` | Access credentials during workspace reconciliation |
| `bm teams show` | Access credentials for display |

Every call site repeats the same pattern:

```rust
let store = bridge::LocalCredentialStore::new(
    &team.name, &bridge_name, state_path,
).with_collection(cfg.keyring_collection.clone());
```

The formation should own credential store selection. Commands should ask the formation for "give me a credential store" rather than hardcoding the local implementation. The trait interface and the local implementation both exist and work — but the routing through formation is missing, so commands go directly to the local backend.

**Severity:** Significant — this is the formation's only existing behavioral surface, and it's entirely bypassed.

---

## 4. Where Code Does Things That Should Be Formation's Responsibility (No Support Yet)

These are places where commands contain inline logic that **conceptually belongs to a formation** but the formation module has no mechanism to support. Adding these capabilities would require extending the formation abstraction.

### 4a. Member Launch Strategy

In `commands/start.rs`, the `launch_ralph()` function:

```rust
fn launch_ralph(workspace: &Path, gh_token: &str, ...) -> Result<u32> {
    let mut cmd = Command::new("ralph");
    cmd.args(["run", "-p", "PROMPT.md"])
        .current_dir(workspace)
        .env("GH_TOKEN", gh_token)
        .env_remove("CLAUDECODE");
    // ... env var injection for bridge type ...
    let child = cmd.spawn()?;
    Ok(child.id())
}
```

The entire launch strategy — what binary to run, what arguments, how to inject credentials as env vars, how to detach the process — is hardcoded to local process spawning. A k8s formation would `kubectl apply` a pod spec. A Docker formation would `docker run`.

### 4b. Member Stop Strategy

In `commands/stop.rs`, `graceful_stop()` and `force_stop()`:

```rust
fn graceful_stop(workspace: &Path, pid: u32) -> Result<()> {
    Command::new("ralph").args(["loops", "stop"]).current_dir(workspace).output()?;
    // Poll for PID exit...
}

fn force_stop(pid: u32) {
    libc::kill(pid as i32, libc::SIGTERM);
}
```

Graceful stop (run `ralph loops stop` in workspace, poll PID) and force stop (`SIGTERM`) are local-only mechanics. A k8s formation would `kubectl delete pod`. **`bm stop` has zero formation awareness** — it doesn't load or check the formation at all.

### 4c. Member Health / Liveness Check

In `state.rs`, the `is_alive()` function:

```rust
pub fn is_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
```

Used throughout `start.rs` (skip already-running, verify alive after launch), `stop.rs` (check if already exited, poll for exit after graceful stop), and `status.rs` (running vs crashed display).

All PID-based. A k8s formation would check pod phase via kubectl/API.

### 4d. Bridge Infrastructure Lifecycle

In `commands/start.rs` (bridge start) and `commands/stop.rs` (bridge stop), bridge lifecycle is managed inline using the `Bridge` struct:

Bridge start logic:
- Discover bridge, construct `Bridge` object
- Check bridge type (`is_local()` vs `is_external()`)
- Call `bridge.start()`, save state
- Controlled by `bridge_lifecycle.start_on_up` config

Bridge stop logic:
- Discover bridge, construct `Bridge` object
- Call `bridge.stop()`, save state
- Controlled by `bridge_lifecycle.stop_on_down` config or `--bridge` flag

This is team infrastructure lifecycle that the formation should manage as part of "bring up the environment" and "tear down the environment." The bridge is logically part of the formation's infrastructure, not a command concern.

### 4e. Runtime State Model

In `state.rs`, `RuntimeState` and `MemberRuntime` are local-only data structures:

```rust
pub struct MemberRuntime {
    pub pid: u32,           // local PID — meaningless for k8s pods
    pub started_at: String,
    pub workspace: PathBuf, // local filesystem path — meaningless for remote
}
```

The state module assumes local processes. `cleanup_stale()` uses PID liveness to detect dead members. A formation-aware state model would be polymorphic (local state vs k8s state vs Docker state).

### 4f. Prerequisite Checking

In `commands/start.rs`:

```rust
if which::which("ralph").is_err() {
    bail!("'ralph' not found in PATH. Install ralph-orchestrator first.");
}
```

What needs to be installed depends on the formation. Local needs `ralph`. K8s needs `kubectl` (and optionally `kind`). Docker would need `docker`. The formation should declare its prerequisites.

### 4g. Verbose Status Introspection

In `commands/status.rs`, verbose mode runs `ralph` CLI subcommands directly:

```rust
for (label, args) in &[
    ("Hats", vec!["hats"]),
    ("Loops", vec!["loops", "list"]),
    ("Events", vec!["events"]),
    ("Bot", vec!["bot", "status"]),
] {
    run_ralph_cmd(&rt.workspace, args)?;
}
```

Running `ralph` CLI subcommands directly against a local workspace's filesystem. Only works for local processes where the workspace is accessible on the same machine.

### 4h. Workspace Access Pattern

In `commands/chat.rs`:

```rust
let ws_path = team.path.join(member);
if !ws_path.join(".botminter.workspace").exists() {
    bail!("No workspace found for member '{}'...");
}
// ... later:
std::process::Command::new(&coding_agent.binary)
    .current_dir(&ws_path)
    .exec();
```

`bm chat` assumes the workspace is a local directory it can `exec()` into. This only works for the local formation.

### 4i. Credential Delivery to Members

In `commands/start.rs`, credentials are injected as env vars on the spawned process:

```rust
match bridge_type {
    Some("rocketchat") => { cmd.env("RALPH_ROCKETCHAT_AUTH_TOKEN", token); }
    Some("tuwunel")    => { cmd.env("RALPH_MATRIX_ACCESS_TOKEN", token); }
    _                  => { cmd.env("RALPH_TELEGRAM_BOT_TOKEN", token); }
}
```

How credentials reach members (env vars on the spawned process) is local-specific. A k8s formation would mount Kubernetes Secrets as env vars in the pod spec. The formation should own credential delivery, not just credential storage.

---

## 5. Summary

### By category

| Category | Count | Items |
|---|---|---|
| **Formation provides it, code uses it correctly** | 5 call sites | `resolve_formation`, `load`, `is_local`, `list_formations`, `formations_dir` (all config/resolution) |
| **Formation provides it, code bypasses it** | 2 items | Hardcoded `"local"` topology string; credential management — `CredentialStore` trait and `LocalCredentialStore` exist as working formation behavior but commands hardcode `LocalCredentialStore` directly instead of going through the formation |
| **Code does it, formation has no support yet** | 9 concerns | Launch strategy, stop strategy, health/liveness check, bridge infrastructure lifecycle, runtime state model, prerequisite checking, verbose status introspection, workspace access, credential delivery |

### Architectural observation

The formation currently has two layers:

1. **Config/resolution** (`formation.rs`) — resolves which formation to use and loads its YAML config. This works correctly everywhere it's used.

2. **Credential management** (`bridge.rs`) — `CredentialStore` trait with `LocalCredentialStore` implementation backed by the system keyring. This is the formation's only behavioral surface. It works correctly as code, but commands bypass the formation to use it directly — they instantiate `LocalCredentialStore` without going through any formation abstraction.

The non-local (k8s) path correctly delegates behavior to a formation manager Ralph session. The local path has real behavioral code (credential management) but it's scattered in `bridge.rs` and wired directly by commands, plus all other behavioral concerns (launch, stop, health, bridge lifecycle) are hardcoded inline in command modules.

### Asymmetry between local and non-local

| Concern | Non-local (k8s) | Local | Routed through formation? |
|---|---|---|---|
| How members start | Formation manager (Ralph session) | Hardcoded in `start.rs` | k8s: yes; local: no |
| How members stop | (Not implemented) | Hardcoded in `stop.rs` | Neither |
| Health checking | (Not implemented) | `libc::kill(pid, 0)` in `state.rs` | Neither |
| Credential storage | (Not implemented, TODO in code) | `LocalCredentialStore` (system keyring) — **works but bypassed** | Neither — commands hardcode `LocalCredentialStore` |
| Credential delivery | (Not implemented) | Env vars on spawned process | Neither |
| Topology output | Formation manager writes it | `write_local_topology()` in `start.rs` | k8s: yes; local: no |
| Bridge lifecycle | (Not implemented) | Inline in `start.rs` + `stop.rs` via `Bridge` struct | Neither |
| Prerequisites | (Not checked) | `which("ralph")` | Neither |
| Runtime state | (Not tracked in state.json) | `state.rs` with PIDs | Neither |

The k8s formation delegates its `start` behavior properly but has no `stop`, `status`, or credential support. The local formation has full behavior — including working credential management via system keyring — but none of it is routed through the formation abstraction. Commands go directly to the implementations.
