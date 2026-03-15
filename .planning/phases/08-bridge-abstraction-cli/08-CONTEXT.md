# Phase 8: Bridge Abstraction, CLI & Telegram - Context

**Gathered:** 2026-03-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the Rust bridge module (`crates/bm/src/bridge.rs`) with state management and all `bm bridge` CLI commands (`start`, `stop`, `status`, `identity add/rotate/remove/list`, `room create/list`). Wrap Telegram as the first real bridge implementation (external type, identity-only). Wire bridge lifecycle into `bm start/stop/status`. Validate end-to-end with both the stub bridge (for testing) and Telegram (as the real proof the abstraction works).

</domain>

<decisions>
## Implementation Decisions

### Bridge State Model
- Bridge state persisted at `{workzone}/{team}/bridge-state.json` (same directory level as `topology.json`)
- State tracks: bridge name, type (local/external), service URL, container IDs, health status, last health check timestamp, registered identities (username -> user_id + token mapping)
- State file created on first `bm bridge start` or `bm bridge identity add`; absent means no bridge active
- State uses `serde_json` for serialization (consistent with `state.json` and `topology.json`)
- File permissions: 0600 (contains credentials — same pattern as `config.yml`)

### CLI Command Structure
- Top-level: `bm bridge` with nested subcommands following `DaemonCommand` pattern
- Subcommand groups: `bm bridge start/stop/status` at top level, `bm bridge identity {add|rotate|remove|list}` and `bm bridge room {create|list}` as nested subgroups
- All bridge commands accept `-t/--team` flag (standard pattern)
- Output uses `comfy-table` for tabular data (`status`, `identity list`, `room list`), `println!` for single-value responses
- Error messages follow existing pattern: include what failed + what to do next (e.g., "No bridge configured for team 'X'. Configure one in the team repo or profile.")

### Bridge Discovery & Loading
- Bridge implementation lives as a directory in the team repo: `bridges/{bridge-name}/` containing `bridge.yml`, `schema.json`, `Justfile`
- Active bridge configured in team's `botminter.yml` manifest under a `bridge` key (name reference to `bridges/` directory)
- Bridge loading: parse `bridge.yml` with `serde_yml`, validate `schema.json` with `serde_json`, verify Justfile exists
- No bridge = no `bridge` key in manifest — all bridge commands return a clean "no bridge configured" message

### Command Invocation
- Bridge commands invoked via `just --justfile {bridge_dir}/Justfile {recipe} {args}`
- Environment variables set before invocation: `BRIDGE_CONFIG_DIR` (temp dir per invocation), `BM_TEAM_NAME`
- Config exchange: after command completes, read `$BRIDGE_CONFIG_DIR/config.json`, parse as JSON, merge into bridge state
- Command execution uses `std::process::Command` (same as Ralph launch in `start.rs`)

### Bridge Module Structure
- New core module: `crates/bm/src/bridge.rs` — bridge manifest parsing, state management, command invocation
- New command module: `crates/bm/src/commands/bridge.rs` — CLI handlers for all `bm bridge` subcommands
- Structs: `BridgeManifest` (parsed `bridge.yml`), `BridgeState` (persisted state), `BridgeConfig` (validated schema values), `BridgeIdentity` (per-user credentials)
- Bridge module is self-contained; other modules don't depend on it in this phase (integration with `start`/`status` is Phase 9)

### Graceful Degradation (BRDG-08)
- No bridge configured: all `bm bridge` commands print "No bridge configured for team '{name}'" and exit cleanly (not an error, exit 0)
- `bm status` and `bm start` work normally when no bridge exists — bridge features are additive
- Bridge state file simply doesn't exist — no sentinel values needed

### Telegram Bridge (TELE-01, TELE-02)
- Telegram bridge ships as an external-type bridge with `bridge.yml` + `schema.json` + `Justfile`
- Identity-only: onboard (register bot token), rotate-credentials, remove — no start/stop lifecycle
- Ships as a built-in bridge in the `scrum-compact` profile (and `scrum`)
- Validates the external bridge contract path end-to-end

### Start/Stop/Status Integration (CLI-08, CLI-09)
- `bm start` supports `--no-bridge` and `--bridge-only` flags
- Default behavior controlled by `bridge.auto_start` config (default: true if bridge configured)
- `bm status` team view shows member bridge identity mapping alongside agent status
- Bridge module is self-contained; `start.rs` and `status.rs` call into it for bridge-related operations

### Credential Handling (BRDG-09)
- Priority: env var → bridge state file → (future: system keyring)
- For Phase 8: env var and state file only — keyring is future work
- Credentials stored per-identity in bridge state (encrypted at rest is future work)

### Claude's Discretion
- Internal module organization within `bridge.rs` (helper functions, private types)
- Exact `comfy-table` column layout for `bridge status` and `identity list`
- Temp directory strategy for `BRIDGE_CONFIG_DIR` (system temp vs workspace-local)
- Test organization within `integration.rs` vs separate `bridge_tests.rs`
- Whether to use a `BridgeRunner` trait or plain functions for command invocation

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/bm/src/state.rs`: `RuntimeState`/`MemberRuntime` pattern for JSON-persisted state — bridge state follows same shape
- `crates/bm/src/cli.rs`: `DaemonCommand` enum pattern for nested subcommands with `start/stop/status` — bridge commands mirror this
- `crates/bm/src/commands/start.rs`: `std::process::Command` usage for spawning processes — bridge command invocation follows same approach
- `crates/bm/src/config.rs`: `resolve_team()` helper for `-t` flag resolution — all bridge commands reuse this
- `.planning/specs/bridge/examples/stub/`: Complete stub bridge fixture for testing

### Established Patterns
- All CLI subcommand groups use `#[derive(Subcommand)]` enum in `cli.rs` with handler dispatch in `main.rs`
- JSON state files at `{workzone}/{team}/` level (topology.json precedent)
- `anyhow::Result<()>` return type for all command handlers
- `comfy-table` with UTF-8 rounded corners for tabular output
- `bail!()` for user-facing errors with actionable guidance

### Integration Points
- `cli.rs`: Add `Bridge` variant to `Command` enum with nested `BridgeCommand`
- `commands/mod.rs`: Register new `bridge` module
- `main.rs`: Add dispatch arm for `Command::Bridge`
- `lib.rs`: Export new `bridge` module
- Team repo `botminter.yml`: Add optional `bridge` field to manifest schema

</code_context>

<specifics>
## Specific Ideas

- The `bm bridge` command group should feel like `bm daemon` — same start/stop/status shape, familiar to existing users
- Stub bridge from Phase 7 is the primary test fixture — all integration tests run against it
- Bridge state should be inspectable: `bm bridge status` shows everything an operator needs to debug (URL, uptime, identities, last health check)

</specifics>

<deferred>
## Deferred Ideas

- Profile-level bridge configuration and `bm init` wizard — Phase 9
- Real bridge implementations (Rocket.Chat) — Phase 10
- System keyring credential storage — future milestone
- Encrypted credentials at rest — future milestone
- Health check polling/auto-recovery — future milestone (BRDG-F01)

</deferred>

---

*Phase: 08-bridge-abstraction-cli*
*Context gathered: 2026-03-08*
