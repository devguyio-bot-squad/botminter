# Sprint 2: Daemon Supervisor + CLI Through Team + Brain Model

## Objective

Transform the daemon into the formation's internal member supervisor with a RESTful HTTP API. Migrate all CLI member lifecycle commands (`bm start/stop/status`) to go through `team.start()`/`team.stop()`/`team.status()`. Replace `bm runtime` with `bm env`. Update Brain to delegate loop spawning to daemon. Auth model remains unchanged (`gh_token`).

## Prerequisites

Sprint 1 delivered: Formation trait, key-value CredentialStore, Team struct, LinuxLocalFormation wrapping existing code.

## Deviations from Design

- `gh_token` is still used — daemon reads from config and injects as `GH_TOKEN` env var. Auth swap is Sprint 3.
- `setup_token_delivery()` / `refresh_token()` remain no-ops. Token lifecycle is Sprint 3.
- Daemon does not run token refresh loops yet — no per-member App tokens exist.

## Key References

- Design: `specs/github-app-identity/design.md` (Architecture, CLI ↔ Daemon Communication sections)
- ADR-0008: `.planning/adrs/0008-team-runtime-architecture.md` (daemon as implementation detail)
- Sprint plan: `specs/github-app-identity/sprint-2/plan.md`

## Requirements

1. The daemon MUST expose a RESTful HTTP API with OpenAPI schema on `127.0.0.1:{port}` using the existing axum server. Endpoints: `POST /api/members/start`, `POST /api/members/stop`, `GET /api/members`, `GET /api/health`. Ref: design.md "CLI ↔ Daemon Communication" section.

2. The daemon MUST own `state.json` — all mutations go through the daemon. CLI MUST read `state.json` for display only, never write. Ref: design.md, requirements.md R8.

3. The daemon MUST write PID + port to a state file on startup. The CLI MUST read this file to discover the daemon's address.

4. `formation.start_members()` MUST spawn the daemon process if not already running, then communicate via HTTP to launch members. It MUST NOT launch member processes directly.

5. `formation.stop_members()` MUST communicate via HTTP to stop members. The daemon MUST keep running after members stop.

6. `bm stop --all` MUST stop both members and daemon.

7. `bm start` called when daemon is already running MUST communicate with the existing daemon (no double-spawn). Ref: requirements.md I7 note about idempotency.

8. `bm env create` MUST replace `bm runtime create`, delegating to `formation.setup()`. `bm env delete` MUST replace `bm runtime delete`. Ref: design.md "Operator-Facing Commands" section.

9. `bm chat` MUST delegate to `formation.exec_in()`. `bm attach` MUST delegate to `formation.shell()`.

10. Brain MUST delegate loop spawning to the daemon via HTTP API instead of spawning background Bash commands directly. Brain's system prompt MUST be updated accordingly. Ref: requirements.md Q10b.

11. `bm-agent` SHOULD gain a `loop start` subcommand for Brain → daemon communication.

12. All existing E2E, integration, and unit tests MUST pass (behavior preserved, auth unchanged).

## Acceptance Criteria

1. **Given** `bm start`, **when** no daemon is running, **then** the formation starts the daemon first, then launches members via HTTP API.

2. **Given** `bm start` called a second time, **when** daemon is already running, **then** the CLI communicates with the existing daemon without starting a new one.

3. **Given** `bm stop`, **when** members are running, **then** members stop but the daemon keeps running.

4. **Given** `bm stop --all`, **when** daemon is running, **then** both members and daemon stop.

5. **Given** `bm status`, **when** daemon is running with members, **then** status is returned via daemon HTTP API including member PIDs, workspaces, and brain mode.

6. **Given** `bm env create`, **when** run for a local formation, **then** prerequisites are verified (ralph, keyring, gh auth).

7. **Given** `bm chat superman`, **when** run, **then** `formation.exec_in()` is called with the member's workspace.

8. **Given** Brain receives a work request, **when** it needs to spawn a Ralph loop, **then** it delegates to the daemon via HTTP API (not direct Bash spawn).

9. **Given** `GET /api/members`, **when** called on the daemon, **then** it returns JSON with member status, PIDs, workspaces, brain mode, and started_at timestamps.

10. (Regression) **Given** `just test`, **when** run, **then** all existing tests pass.

11. (Regression) **Given** the daemon manages member processes, **when** a member is launched, **then** it receives `GH_TOKEN` env var from the team config (same auth model as before).
