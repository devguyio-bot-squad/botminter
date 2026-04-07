# Sprint 2: Daemon Supervisor + CLI Through Team + Brain Model

## Checklist

- [x] Define daemon RESTful HTTP API with OpenAPI schema
- [x] Implement daemon API endpoints (start/stop/status members)
- [x] Transfer `state.json` ownership to daemon (CLI reads only)
- [x] `formation.start_members()` spawns daemon + communicates via HTTP
- [x] `formation.stop_members()` communicates via HTTP — *deviation: uses local PID signals instead of HTTP (pragmatic)*
- [x] `formation.member_status()` queries daemon via HTTP — *deviation: reads state.json directly*
- [x] Migrate `bm start` to go through `team.start()`
- [x] Migrate `bm stop` to go through `team.stop()`, add `--all` flag
- [ ] Migrate `bm status` to go through `team.status()` — *bypasses Team struct, calls state::gather_status directly*
- [x] Migrate `bm chat` through `formation.exec_in()`
- [x] Migrate `bm attach` through `formation.shell()`
- [x] Implement `bm env create/delete` replacing `bm runtime create/delete`
- [x] Update Brain to delegate loop spawning to daemon HTTP API
- [x] Update Brain system prompt
- [x] Tests: daemon HTTP API unit tests
- [x] Tests: CLI → Team → Formation → Daemon integration tests
- [x] Tests: verify all existing E2E and integration tests pass

## Steps (Sequential)

### 1. Daemon RESTful HTTP API

**Objective:** Add member lifecycle endpoints to the existing daemon axum server.

**Implementation:**
- Define OpenAPI schema for daemon API
- `POST /api/members/start` — accepts `{ member: Option<String> }`, launches member(s) by calling existing `start_local_members()` internally
- `POST /api/members/stop` — accepts `{ member: Option<String>, force: bool }`, stops member(s)
- `GET /api/members` — returns member status with PID, workspace, brain_mode, started_at
- `GET /api/health` — enhance existing health endpoint
- Request/response types with serde serialization
- Daemon state extended to hold formation instance

**Tests:** Unit tests for each endpoint with mock formation.

### 2. State Ownership Transfer

**Objective:** Daemon owns `state.json` — CLI reads only.

**Implementation:**
- Daemon writes `state.json` after every member launch/stop
- CLI commands that currently write `state.json` delegate to daemon HTTP API instead
- Daemon state file (`daemon.pid`, `daemon.port`) written on startup
- CLI reads daemon state file to discover daemon address

**Tests:** Verify no CLI code path writes `state.json` directly after this step.

### 3. CLI Migration Through Team

**Objective:** `bm start/stop/status` go through `team.start()`/`team.stop()`/`team.status()`.

**Implementation:**
- `team.start()` → `formation.start_members()` → spawns daemon if needed → `POST /api/members/start`
- `team.stop()` → `formation.stop_members()` → `POST /api/members/stop`
- `team.stop(all=true)` → stop members + stop daemon process
- `team.status()` → `formation.member_status()` → `GET /api/members`
- Add `--all` flag to `bm stop`
- Remove direct `start_local_members()` / `stop_local_members()` calls from command handlers
- `bm chat` → `team.chat()` → `formation.exec_in(workspace, &["claude", ...])`
- `bm attach` → `team.attach()` → `formation.shell()`

**Tests:** Integration tests for full CLI → Team → Formation → Daemon → member lifecycle.

### 4. `bm env create/delete`

**Objective:** Replace `bm runtime create/delete` with formation-aware environment commands.

**Implementation:**
- `bm env create` → resolves team → `team.setup_env()` → `formation.setup()`
- `bm env delete` → teardown
- For local formation: `setup()` verifies prerequisites (ralph, keyring, gh auth)
- For Lima: `setup()` delegates to existing Lima VM creation code
- Deprecate `bm runtime create/delete` (or alias to `bm env`)
- Update `bm attach` to delegate to formation

**Tests:** Existing Lima tests pass through new path. Local formation setup verification works.

### 5. Brain Model Change

**Objective:** Brain delegates loop spawning to daemon instead of spawning directly.

**Implementation:**
- Add `POST /api/loops/start` endpoint to daemon (launches Ralph loop as child process)
- Brain's system prompt updated: instead of spawning background Bash commands for loops, Brain uses `bm-agent` to request loop launch from daemon
- `bm-agent` gains `loop start` subcommand that calls daemon HTTP API
- Event watcher updated to receive loop events from daemon-managed processes

**Tests:** Brain integration test — loop request goes through daemon, not direct spawn.

## Deviations from Design

| Deviation | Rationale | Resolved in |
|-----------|-----------|-------------|
| `gh_token` still used for member launch | Auth model unchanged in this sprint | Sprint 3 |
| `setup_token_delivery()` / `refresh_token()` still no-ops | Token delivery is Sprint 3 | Sprint 3 |
| Daemon token refresh loop not implemented | No per-member tokens yet | Sprint 3 |
