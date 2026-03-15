# Phase 8: Bridge Abstraction & CLI - Research

**Researched:** 2026-03-08
**Domain:** Rust CLI module design, bridge state management, subprocess invocation
**Confidence:** HIGH

## Summary

Phase 8 builds the Rust-side bridge module and all `bm bridge` CLI commands, validated against the stub bridge from Phase 7. The codebase has well-established patterns for every required capability: nested CLI subcommand enums (DaemonCommand), JSON state persistence (state.rs, topology.rs), subprocess invocation (start.rs), comfy-table output (status.rs), and team resolution (config.rs). This phase is primarily a "follow the existing patterns" implementation.

The bridge spec (Phase 7) defines the external contract for bridge implementations. Phase 8 implements the BotMinter side: parsing `bridge.yml`, managing `bridge-state.json`, invoking Justfile recipes via `std::process::Command`, and presenting results through CLI subcommands. Room commands (CLI-10, CLI-11) are required but not yet in the bridge spec -- the stub bridge needs room recipes added, and room operations follow the same invocation pattern as identity commands.

**Primary recommendation:** Follow existing codebase patterns exactly -- DaemonCommand for CLI structure, topology.rs for state persistence, start.rs for subprocess invocation. No new dependencies needed.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Bridge state persisted at `{workzone}/{team}/bridge-state.json` (same directory level as `topology.json`)
- State tracks: bridge name, type (local/external), service URL, container IDs, health status, last health check timestamp, registered identities (username -> user_id + token mapping)
- State file created on first `bm bridge start` or `bm bridge identity add`; absent means no bridge active
- State uses `serde_json` for serialization (consistent with `state.json` and `topology.json`)
- File permissions: 0600 (contains credentials -- same pattern as `config.yml`)
- Top-level: `bm bridge` with nested subcommands following `DaemonCommand` pattern
- Subcommand groups: `bm bridge start/stop/status` at top level, `bm bridge identity {add|rotate|remove|list}` and `bm bridge room {create|list}` as nested subgroups
- All bridge commands accept `-t/--team` flag (standard pattern)
- Output uses `comfy-table` for tabular data (`status`, `identity list`, `room list`), `println!` for single-value responses
- Error messages follow existing pattern: include what failed + what to do next
- Bridge implementation lives as a directory in the team repo: `bridges/{bridge-name}/` containing `bridge.yml`, `schema.json`, `Justfile`
- Active bridge configured in team's `botminter.yml` manifest under a `bridge` key (name reference to `bridges/` directory)
- Bridge loading: parse `bridge.yml` with `serde_yml`, validate `schema.json` with `serde_json`, verify Justfile exists
- No bridge = no `bridge` key in manifest -- all bridge commands return a clean "no bridge configured" message
- Bridge commands invoked via `just --justfile {bridge_dir}/Justfile {recipe} {args}`
- Environment variables set before invocation: `BRIDGE_CONFIG_DIR` (temp dir per invocation), `BM_TEAM_NAME`
- Config exchange: after command completes, read `$BRIDGE_CONFIG_DIR/config.json`, parse as JSON, merge into bridge state
- Command execution uses `std::process::Command` (same as Ralph launch in `start.rs`)
- New core module: `crates/bm/src/bridge.rs` -- bridge manifest parsing, state management, command invocation
- New command module: `crates/bm/src/commands/bridge.rs` -- CLI handlers for all `bm bridge` subcommands
- Structs: `BridgeManifest` (parsed `bridge.yml`), `BridgeState` (persisted state), `BridgeConfig` (validated schema values), `BridgeIdentity` (per-user credentials)
- Bridge module is self-contained; other modules don't depend on it in this phase (integration with `start`/`status` is Phase 9)
- No bridge configured: all `bm bridge` commands print "No bridge configured for team '{name}'" and exit cleanly (not an error, exit 0)
- `bm status` and `bm start` are unaffected when no bridge exists (bridge integration is Phase 9)
- Credential priority: env var -> bridge state file -> (future: system keyring)
- For Phase 8: env var and state file only -- keyring is future work

### Claude's Discretion
- Internal module organization within `bridge.rs` (helper functions, private types)
- Exact `comfy-table` column layout for `bridge status` and `identity list`
- Temp directory strategy for `BRIDGE_CONFIG_DIR` (system temp vs workspace-local)
- Test organization within `integration.rs` vs separate `bridge_tests.rs`
- Whether to use a `BridgeRunner` trait or plain functions for command invocation

### Deferred Ideas (OUT OF SCOPE)
- Bridge integration with `bm start/stop/status` -- Phase 9
- Profile-level bridge configuration and `bm init` wizard -- Phase 10
- Real bridge implementations (Rocket.Chat) -- Phase 11
- System keyring credential storage -- future milestone
- Encrypted credentials at rest -- future milestone
- Health check polling/auto-recovery -- future milestone (BRDG-F01)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BRDG-05 | Bridge config model with bridge type resolution, state tracking, and per-user credentials | `BridgeManifest` + `BridgeState` structs following topology.rs pattern; serde_yml for manifest, serde_json for state |
| BRDG-06 | Bridge state persisted across sessions tracking service URLs, container IDs, and registered user credentials | `bridge-state.json` at `{workzone}/{team}/` using atomic write pattern from topology.rs with 0600 permissions |
| BRDG-08 | Bridge is optional -- a team can operate without any bridge. All bridge-dependent features degrade gracefully | No `bridge` key in `botminter.yml` = no bridge; clean exit 0 messages; existing commands unaffected |
| BRDG-09 | Bridge credentials resolved in priority order: env var -> config file -> system keyring | Phase 8 implements env var + state file tiers; keyring deferred |
| CLI-01 | `bm bridge start` starts the bridge service | Invokes lifecycle.start recipe via just, reads config exchange, persists state |
| CLI-02 | `bm bridge stop` stops the bridge service | Invokes lifecycle.stop recipe via just, updates state to mark stopped |
| CLI-03 | `bm bridge status` shows bridge service health, URL, uptime, registered identities | Reads bridge-state.json, optionally runs health recipe, displays via comfy-table |
| CLI-04 | `bm bridge identity add <username>` creates a user on the bridge | Invokes identity.onboard recipe via just, reads credentials from config exchange, stores in state |
| CLI-05 | `bm bridge identity rotate <username>` rotates credentials for a bridge user | Invokes identity.rotate-credentials recipe via just, updates stored credentials |
| CLI-06 | `bm bridge identity list` lists all users registered on the bridge | Reads identities from bridge-state.json, displays via comfy-table |
| CLI-07 | `bm bridge identity remove <username>` removes a user from the bridge | Invokes identity.remove recipe, removes identity from state |
| CLI-10 | `bm bridge room create <name>` creates a room/channel on the bridge | Needs room.create recipe added to bridge contract and stub; invocation follows same pattern as identity |
| CLI-11 | `bm bridge room list` lists rooms on the bridge | Needs room.list recipe added to bridge contract and stub; reads room state from bridge-state.json |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4 | CLI argument parsing with `#[derive(Subcommand)]` | Already in Cargo.toml, powers all existing commands |
| serde + serde_json | 1 | Bridge state serialization (`bridge-state.json`) | Already in Cargo.toml, used by state.rs, topology.rs |
| serde_yml | 0.0.12 | Bridge manifest parsing (`bridge.yml`) | Already in Cargo.toml, used by config.rs, profile.rs |
| anyhow | 1 | Error handling with `bail!()` and `.context()` | Already in Cargo.toml, used everywhere |
| comfy-table | 7 | Tabular output for status, identity list, room list | Already in Cargo.toml, used by status.rs |
| chrono | 0.4 | Timestamps for state (started_at, last_health_check) | Already in Cargo.toml, used by start.rs |
| tempfile | 3 | Temp directories for `BRIDGE_CONFIG_DIR` | Already in Cargo.toml (both deps and dev-deps) |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::process::Command | stdlib | Invoking `just` recipes | All bridge command execution |
| std::os::unix::fs::PermissionsExt | stdlib | Setting 0600 on bridge-state.json | State file write |
| std::fs | stdlib | File I/O for state, manifest, schema | Throughout bridge module |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Plain functions for invocation | `BridgeRunner` trait | Trait adds testability via mocking but adds complexity; plain functions are simpler and match existing patterns (start.rs has no traits). Claude's discretion per CONTEXT.md. |
| System temp for BRIDGE_CONFIG_DIR | Workspace-local temp | System temp (`tempfile::tempdir()`) is cleaner and auto-cleans; workspace-local leaves artifacts. Recommend system temp. |

**Installation:**
No new dependencies needed. All libraries are already in `Cargo.toml`.

## Architecture Patterns

### Recommended Project Structure
```
crates/bm/src/
  bridge.rs              # Core: BridgeManifest, BridgeState, BridgeIdentity,
                         #        load/save/invoke functions
  commands/
    bridge.rs            # CLI handlers: start, stop, status, identity, room
    mod.rs               # Add `pub mod bridge;`
  cli.rs                 # Add BridgeCommand, BridgeIdentityCommand, BridgeRoomCommand enums
  main.rs                # Add dispatch arm for Command::Bridge
  lib.rs                 # Add `pub mod bridge;`
```

### Pattern 1: Nested Subcommand Enum (DaemonCommand precedent)
**What:** Three levels of clap enums -- `Command::Bridge { BridgeCommand }` -> `BridgeCommand::Identity { BridgeIdentityCommand }` -> `BridgeIdentityCommand::Add { username }`
**When to use:** All bridge CLI commands
**Example:**
```rust
// Source: crates/bm/src/cli.rs (DaemonCommand pattern)
#[derive(Subcommand)]
pub enum BridgeCommand {
    /// Start the bridge service
    Start {
        #[arg(short, long)]
        team: Option<String>,
    },
    /// Stop the bridge service
    Stop {
        #[arg(short, long)]
        team: Option<String>,
    },
    /// Show bridge status
    Status {
        #[arg(short, long)]
        team: Option<String>,
    },
    /// Manage bridge identities
    Identity {
        #[command(subcommand)]
        command: BridgeIdentityCommand,
    },
    /// Manage bridge rooms
    Room {
        #[command(subcommand)]
        command: BridgeRoomCommand,
    },
}

#[derive(Subcommand)]
pub enum BridgeIdentityCommand {
    /// Add a user to the bridge
    Add {
        username: String,
        #[arg(short, long)]
        team: Option<String>,
    },
    /// Rotate credentials for a bridge user
    Rotate {
        username: String,
        #[arg(short, long)]
        team: Option<String>,
    },
    /// Remove a user from the bridge
    Remove {
        username: String,
        #[arg(short, long)]
        team: Option<String>,
    },
    /// List bridge users
    List {
        #[arg(short, long)]
        team: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum BridgeRoomCommand {
    /// Create a room on the bridge
    Create {
        name: String,
        #[arg(short, long)]
        team: Option<String>,
    },
    /// List rooms on the bridge
    List {
        #[arg(short, long)]
        team: Option<String>,
    },
}
```

### Pattern 2: JSON State Persistence (topology.rs precedent)
**What:** Atomic write with temp file + rename, 0600 permissions, load returns default when missing
**When to use:** Bridge state file at `{workzone}/{team}/bridge-state.json`
**Example:**
```rust
// Source: crates/bm/src/topology.rs (save function pattern)
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BridgeState {
    pub bridge_name: Option<String>,
    pub bridge_type: Option<String>,  // "local" or "external"
    pub service_url: Option<String>,
    pub container_ids: Vec<String>,
    pub status: String,               // "running", "stopped", "unknown"
    pub started_at: Option<String>,   // ISO 8601
    pub last_health_check: Option<String>,
    pub identities: HashMap<String, BridgeIdentity>,
    pub rooms: Vec<BridgeRoom>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeIdentity {
    pub username: String,
    pub user_id: String,
    pub token: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeRoom {
    pub name: String,
    pub room_id: Option<String>,
    pub created_at: String,
}

pub fn bridge_state_path(workzone: &Path, team_name: &str) -> PathBuf {
    workzone.join(team_name).join("bridge-state.json")
}

pub fn load_state(path: &Path) -> Result<BridgeState> {
    if !path.exists() {
        return Ok(BridgeState::default());
    }
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

pub fn save_state(path: &Path, state: &BridgeState) -> Result<()> {
    // Atomic write + 0600 permissions (same as topology.rs)
}
```

### Pattern 3: Bridge Manifest Parsing
**What:** Parse `bridge.yml` using typed serde structs matching the Knative-style spec format
**When to use:** Loading bridge configuration from team repo
**Example:**
```rust
// Source: .planning/specs/bridge/bridge-spec.md structure
#[derive(Debug, Deserialize)]
pub struct BridgeManifest {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: BridgeMetadata,
    pub spec: BridgeSpec,
}

#[derive(Debug, Deserialize)]
pub struct BridgeMetadata {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BridgeSpec {
    #[serde(rename = "type")]
    pub bridge_type: String,
    #[serde(rename = "configSchema")]
    pub config_schema: String,
    pub lifecycle: Option<BridgeLifecycle>,
    pub identity: BridgeIdentitySpec,
    #[serde(rename = "configDir")]
    pub config_dir: String,
    pub room: Option<BridgeRoomSpec>,  // New: for room commands
}

#[derive(Debug, Deserialize)]
pub struct BridgeLifecycle {
    pub start: String,
    pub stop: String,
    pub health: String,
}

#[derive(Debug, Deserialize)]
pub struct BridgeIdentitySpec {
    pub onboard: String,
    #[serde(rename = "rotate-credentials")]
    pub rotate_credentials: String,
    pub remove: String,
}

#[derive(Debug, Deserialize)]
pub struct BridgeRoomSpec {
    pub create: String,
    pub list: String,
}
```

### Pattern 4: Subprocess Invocation with Config Exchange (start.rs precedent)
**What:** Create temp dir, set env vars, run `just --justfile`, read output file
**When to use:** All bridge command invocations
**Example:**
```rust
// Source: crates/bm/src/commands/start.rs (launch_ralph pattern)
fn invoke_bridge_recipe(
    bridge_dir: &Path,
    recipe: &str,
    args: &[&str],
    team_name: &str,
) -> Result<Option<serde_json::Value>> {
    let config_dir = tempfile::tempdir()?;
    let config_dir_path = config_dir.path();

    let mut cmd = std::process::Command::new("just");
    cmd.arg("--justfile")
        .arg(bridge_dir.join("Justfile"))
        .arg(recipe)
        .args(args)
        .env("BRIDGE_CONFIG_DIR", config_dir_path)
        .env("BM_TEAM_NAME", team_name);

    let output = cmd.output()
        .with_context(|| format!("Failed to run bridge recipe '{}'", recipe))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Bridge recipe '{}' failed: {}", recipe, stderr.trim());
    }

    // Read config exchange file if it exists
    let config_file = config_dir_path.join("config.json");
    if config_file.exists() {
        let contents = fs::read_to_string(&config_file)?;
        let value: serde_json::Value = serde_json::from_str(&contents)
            .context("Failed to parse bridge config output")?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}
```

### Pattern 5: Bridge Discovery from Team Repo
**What:** Find bridge directory from `botminter.yml` manifest's `bridge` key
**When to use:** Every `bm bridge` command needs to locate the bridge
**Example:**
```rust
fn discover_bridge(team_repo: &Path, team_name: &str) -> Result<Option<PathBuf>> {
    let manifest_path = team_repo.join("botminter.yml");
    let contents = fs::read_to_string(&manifest_path)
        .context("Failed to read team botminter.yml")?;
    let manifest: serde_yml::Value = serde_yml::from_str(&contents)?;

    match manifest["bridge"].as_str() {
        Some(bridge_name) => {
            let bridge_dir = team_repo.join("bridges").join(bridge_name);
            if !bridge_dir.exists() {
                bail!(
                    "Bridge '{}' referenced in botminter.yml not found at {}",
                    bridge_name, bridge_dir.display()
                );
            }
            Ok(Some(bridge_dir))
        }
        None => Ok(None),  // No bridge configured -- graceful degradation
    }
}
```

### Anti-Patterns to Avoid
- **Mixing bridge logic into existing commands:** Bridge is self-contained in Phase 8. Do NOT touch start.rs, stop.rs, or status.rs -- that is Phase 9.
- **Hardcoding recipe names:** Always use the recipe names from `bridge.yml`, never hardcode "start", "onboard", etc. The bridge contract allows any recipe name.
- **Writing credentials to stdout:** All credential handling goes through bridge-state.json with 0600 permissions. Never print tokens to stdout.
- **Treating no-bridge as an error:** When no bridge is configured, exit 0 with a helpful message, not exit 1.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directories for config exchange | Manual mkdtemp + cleanup | `tempfile::tempdir()` | Auto-cleanup on drop, handles race conditions |
| Atomic file writes | Direct `fs::write` | Temp file + rename pattern (topology.rs) | Prevents corrupt state on crash |
| CLI argument parsing | Manual arg parsing | clap derive macros | Consistent with rest of codebase, auto-generates help |
| YAML manifest parsing | Manual field extraction from serde_yml::Value | Typed serde structs with `#[derive(Deserialize)]` | Compile-time field validation, clear error messages |
| Permission-restricted files | Forgetting permissions | `PermissionsExt::from_mode(0o600)` after write | Credentials in bridge-state.json need protection |

**Key insight:** Every building block needed for Phase 8 already exists in the codebase. The only new thing is the bridge domain logic connecting them together.

## Common Pitfalls

### Pitfall 1: Room Commands Not in Bridge Spec
**What goes wrong:** CLI-10 and CLI-11 require `bm bridge room create/list` but the bridge spec (bridge-spec.md) has no room commands. The stub bridge's Justfile has no room recipes.
**Why it happens:** The bridge spec was written focusing on lifecycle and identity. Room management was added to requirements later.
**How to avoid:** Extend the bridge spec with an optional `spec.room` section. Add `room-create` and `room-list` recipes to the stub Justfile. Room commands follow the exact same invocation pattern as identity commands.
**Warning signs:** Tests fail because stub bridge has no room recipes.

### Pitfall 2: External Bridge Gets Lifecycle Commands
**What goes wrong:** Running `bm bridge start` on an external bridge (type=external) tries to invoke a non-existent lifecycle recipe.
**Why it happens:** The CLI handler does not check bridge type before dispatching.
**How to avoid:** Check `bridge_manifest.spec.bridge_type` before lifecycle commands. External bridges return "Bridge '{name}' is external -- lifecycle commands are not available. The service is managed externally." with exit 0.
**Warning signs:** Error messages about missing Justfile recipes when running lifecycle commands on external bridges.

### Pitfall 3: Config Exchange File Not Written
**What goes wrong:** Some bridge recipes (stop, health, remove) do not write `config.json`. Code assumes all recipes produce config output and panics on missing file.
**Why it happens:** Per the spec, only `start`, `onboard`, and `rotate-credentials` produce config output. Other commands do not.
**How to avoid:** The `invoke_bridge_recipe` function must check if config.json exists before reading. Return `None` when no file is written (see Pattern 4 above).
**Warning signs:** Panics or errors on `bm bridge stop` or `bm bridge identity remove`.

### Pitfall 4: State File Race with Multiple bm Processes
**What goes wrong:** Two `bm bridge` commands run simultaneously, both read state, both write -- last one wins, losing the first's changes.
**Why it happens:** No file locking on bridge-state.json.
**How to avoid:** For Phase 8, this is an acceptable limitation (same as topology.json). The atomic write pattern prevents corruption but not lost updates. Document this limitation. Future: consider advisory file locking.
**Warning signs:** Identity added by one command disappears after another command runs concurrently.

### Pitfall 5: just Not Installed
**What goes wrong:** `bm bridge start` fails with an unhelpful error because `just` is not in PATH.
**Why it happens:** `just` is not a standard system tool.
**How to avoid:** Check for `just` in PATH before attempting any bridge command (same pattern as `which::which("ralph")` in start.rs). Provide a clear error: "Bridge commands require 'just'. Install it: https://just.systems/"
**Warning signs:** Confusing "No such file or directory" errors.

### Pitfall 6: Missing --justfile Flag
**What goes wrong:** Running `just {recipe}` without `--justfile` causes just to search for a Justfile in the current directory, not the bridge directory.
**Why it happens:** `just` uses CWD-based Justfile discovery by default.
**How to avoid:** Always pass `--justfile {bridge_dir}/Justfile` explicitly. The working directory for the just process should also be set to the bridge directory for relative path resolution.
**Warning signs:** "No justfile found" errors or wrong Justfile being used.

## Code Examples

### Complete Command Handler Pattern
```rust
// Source: crates/bm/src/commands/bridge.rs (following commands/daemon.rs pattern)
use crate::bridge;
use crate::config;

pub fn start(team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Check just is available
    if which::which("just").is_err() {
        bail!("Bridge commands require 'just'. Install it: https://just.systems/");
    }

    // Discover bridge
    let bridge_dir = match bridge::discover(&team_repo, &team.name)? {
        Some(dir) => dir,
        None => {
            println!("No bridge configured for team '{}'.", team.name);
            return Ok(());
        }
    };

    // Load manifest
    let manifest = bridge::load_manifest(&bridge_dir)?;

    // Check bridge type
    if manifest.spec.bridge_type == "external" {
        println!(
            "Bridge '{}' is external -- lifecycle commands are not available.",
            manifest.metadata.name
        );
        return Ok(());
    }

    let lifecycle = manifest.spec.lifecycle.as_ref()
        .context("Local bridge missing lifecycle commands")?;

    // Invoke start recipe
    let config_output = bridge::invoke_recipe(
        &bridge_dir, &lifecycle.start, &[], &team.name
    )?;

    // Run health check
    bridge::invoke_recipe(
        &bridge_dir, &lifecycle.health, &[], &team.name
    )?;

    // Persist state
    let state_path = bridge::state_path(&cfg.workzone, &team.name);
    let mut state = bridge::load_state(&state_path)?;
    state.bridge_name = Some(manifest.metadata.name.clone());
    state.bridge_type = Some(manifest.spec.bridge_type.clone());
    state.status = "running".to_string();
    state.started_at = Some(chrono::Utc::now().to_rfc3339());

    if let Some(config) = config_output {
        if let Some(url) = config["url"].as_str() {
            state.service_url = Some(url.to_string());
        }
    }

    bridge::save_state(&state_path, &state)?;

    println!("Bridge '{}' started.", manifest.metadata.name);
    Ok(())
}
```

### Bridge Status Display
```rust
// Source: crates/bm/src/commands/status.rs (comfy-table pattern)
pub fn status(team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;

    let state_path = bridge::state_path(&cfg.workzone, &team.name);
    let state = bridge::load_state(&state_path)?;

    if state.bridge_name.is_none() {
        println!("No bridge configured for team '{}'.", team.name);
        return Ok(());
    }

    println!("Bridge: {}", state.bridge_name.as_deref().unwrap_or("unknown"));
    println!("Type: {}", state.bridge_type.as_deref().unwrap_or("unknown"));
    println!("Status: {}", state.status);
    if let Some(url) = &state.service_url {
        println!("URL: {}", url);
    }
    if let Some(started) = &state.started_at {
        println!("Started: {}", format_timestamp(started));
    }
    println!();

    if state.identities.is_empty() {
        println!("No identities registered.");
    } else {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec!["Username", "User ID", "Created"]);
        for (_, identity) in &state.identities {
            table.add_row(vec![
                &identity.username,
                &identity.user_id,
                &format_timestamp(&identity.created_at),
            ]);
        }
        println!("{table}");
    }

    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hardcoded Telegram in config | Bridge abstraction with pluggable backends | v0.07 (Phase 7 spec) | All communication backends use same contract |
| stdout-based config exchange | File-based config exchange ($BRIDGE_CONFIG_DIR) | v0.07 ADR-0002 | Eliminates stdout corruption from diagnostic output |
| Single bridge per profile | Bridge directory in team repo | v0.07 design | Bridges are independently versioned and swappable |

## Open Questions

1. **Room commands in bridge spec**
   - What we know: CLI-10 and CLI-11 require room create/list commands. The bridge spec does not define room commands.
   - What's unclear: Should the bridge spec be extended, or should room commands be a BotMinter-side abstraction?
   - Recommendation: Extend bridge spec with optional `spec.room` section. Add `room-create` and `room-list` recipes to stub Justfile. Room create returns `{"name": "...", "room_id": "..."}` via config exchange. Room list returns `{"rooms": [{"name": "...", "room_id": "..."}]}`. This is consistent with the existing identity command pattern.

2. **Config exchange for room list**
   - What we know: Room list needs to return multiple rooms. Identity commands return single-item JSON.
   - What's unclear: Does room list write a JSON array or does bm track rooms in state?
   - Recommendation: Room list recipe writes `{"rooms": [...]}` to config exchange. bm reads and displays. State tracks rooms that were created via `bm bridge room create` but `room list` queries the bridge directly for authoritative data.

3. **BridgeRunner trait vs plain functions**
   - What we know: This is Claude's discretion per CONTEXT.md.
   - What's unclear: Whether testability benefits justify the abstraction overhead.
   - Recommendation: Start with plain functions (matching start.rs pattern). The module is self-contained enough that functions can be unit tested with temp directories and mock Justfiles. A trait can be introduced later if Phase 9 integration requires it.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework (cargo test) |
| Config file | `crates/bm/Cargo.toml` |
| Quick run command | `cargo test -p bm` |
| Full suite command | `cargo test -p bm && cargo clippy -p bm -- -D warnings` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BRDG-05 | BridgeManifest parses bridge.yml correctly | unit | `cargo test -p bm bridge::tests::parse_manifest -x` | Wave 0 |
| BRDG-05 | BridgeState round-trip serialization | unit | `cargo test -p bm bridge::tests::state_round_trip -x` | Wave 0 |
| BRDG-06 | State persists across load/save cycles with 0600 perms | unit | `cargo test -p bm bridge::tests::state_persistence -x` | Wave 0 |
| BRDG-08 | No bridge configured returns graceful message | unit | `cargo test -p bm bridge::tests::no_bridge_graceful -x` | Wave 0 |
| BRDG-09 | Credential priority: env var overrides state file | unit | `cargo test -p bm bridge::tests::credential_priority -x` | Wave 0 |
| CLI-01 | Bridge start invokes recipe and persists state | integration | `cargo test -p bm --test integration bridge_start -x` | Wave 0 |
| CLI-02 | Bridge stop invokes recipe and updates state | integration | `cargo test -p bm --test integration bridge_stop -x` | Wave 0 |
| CLI-03 | Bridge status displays state info | integration | `cargo test -p bm --test integration bridge_status -x` | Wave 0 |
| CLI-04 | Identity add creates user and stores credentials | integration | `cargo test -p bm --test integration bridge_identity_add -x` | Wave 0 |
| CLI-05 | Identity rotate updates credentials | integration | `cargo test -p bm --test integration bridge_identity_rotate -x` | Wave 0 |
| CLI-06 | Identity list shows registered users | integration | `cargo test -p bm --test integration bridge_identity_list -x` | Wave 0 |
| CLI-07 | Identity remove deletes user from state | integration | `cargo test -p bm --test integration bridge_identity_remove -x` | Wave 0 |
| CLI-10 | Room create invokes recipe and stores room | integration | `cargo test -p bm --test integration bridge_room_create -x` | Wave 0 |
| CLI-11 | Room list shows rooms | integration | `cargo test -p bm --test integration bridge_room_list -x` | Wave 0 |
| ALL | CLI parsing for all bridge subcommands | unit | `cargo test -p bm --test cli_parsing bridge -x` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p bm`
- **Per wave merge:** `cargo test -p bm && cargo clippy -p bm -- -D warnings`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/bm/src/bridge.rs` -- BridgeManifest, BridgeState structs with unit tests (parse, round-trip, load/save)
- [ ] `crates/bm/tests/integration.rs` -- bridge integration tests using stub bridge fixture
- [ ] `crates/bm/tests/cli_parsing.rs` -- bridge subcommand parsing tests
- [ ] `.planning/specs/bridge/examples/stub/Justfile` -- needs room-create and room-list recipes
- [ ] `.planning/specs/bridge/examples/stub/bridge.yml` -- needs optional `spec.room` section

## Sources

### Primary (HIGH confidence)
- `crates/bm/src/cli.rs` -- DaemonCommand pattern for nested subcommands
- `crates/bm/src/state.rs` -- JSON state persistence with atomic writes
- `crates/bm/src/topology.rs` -- JSON state at `{workzone}/{team}/` with 0600 permissions
- `crates/bm/src/commands/start.rs` -- std::process::Command for subprocess invocation
- `crates/bm/src/commands/status.rs` -- comfy-table display pattern
- `crates/bm/src/commands/stop.rs` -- Graceful stop with state cleanup
- `crates/bm/src/config.rs` -- resolve_team() helper, config loading, 0600 permissions
- `.planning/specs/bridge/bridge-spec.md` -- Bridge plugin contract specification
- `.planning/specs/bridge/examples/stub/` -- Stub bridge fixture (bridge.yml, schema.json, Justfile)
- `.planning/adrs/0002-bridge-abstraction.md` -- Shell script bridge design decision
- `crates/bm/tests/conformance.rs` -- Existing conformance test patterns
- `crates/bm/Cargo.toml` -- Current dependency versions

### Secondary (MEDIUM confidence)
- `.planning/research/IMPLEMENTATION-NOTES.md` -- Design notes from milestone planning
- `.planning/research/PITFALLS.md` -- Domain pitfalls (stdout corruption, admin access, idempotency)
- `.planning/phases/08-bridge-abstraction-cli/08-CONTEXT.md` -- User decisions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in Cargo.toml, versions verified
- Architecture: HIGH -- every pattern directly observed in existing codebase
- Pitfalls: HIGH -- identified from bridge spec gaps (rooms) and existing pitfall research

**Research date:** 2026-03-08
**Valid until:** 2026-04-08 (stable codebase, no external dependency changes expected)
