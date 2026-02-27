---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: E2E — Start to Stop Lifecycle

## Description
Write E2E tests covering `bm start` → `bm status` → `bm stop` with a real Ralph instance using `tg-mock` as the Telegram backend. Verify that members launch, status reports correctly, and stop cleans up state.

## Background
`bm start` spawns Ralph instances (one per member) with `GH_TOKEN` and optional `RALPH_TELEGRAM_BOT_TOKEN` in the environment. It sets `RALPH_TELEGRAM_API_URL` to redirect Telegram calls. `bm status` reads PID state and reports running/stopped/crashed. `bm stop` sends signals and cleans up state.

These tests need:
- A real (or mock) Ralph binary available in PATH
- `tg-mock` Docker container for Telegram API mocking
- A provisioned workspace (from init → hire → sync)

### tg-mock Integration
From the research doc: Point Ralph at `http://localhost:<port>` via `RALPH_TELEGRAM_API_URL`. The mock validates Bot API requests and tracks them via `/__control/requests`. Inject fake user messages via `/__control/updates`.

## Reference Documentation
**Required:**
- `crates/bm/src/commands/start.rs` — launch_ralph(), credential setup, PID tracking
- `crates/bm/src/commands/stop.rs` — graceful_stop(), force_stop()
- `crates/bm/src/commands/status.rs` — status display, crash detection
- `crates/bm/src/state.rs` — RuntimeState, is_alive(), cleanup_stale()
- `specs/milestone-2-architect-first-epic/sprint-4/research/mock-telegram-server.md` — tg-mock API
- E2E harness from task-06: `TgMock`, `TempRepo`, helpers

## Technical Requirements

### Test: `e2e_start_status_stop_lifecycle`
1. Provision a workspace (init → hire → sync) with one member
2. Start `tg-mock` via `TgMock::start()`
3. Set `RALPH_TELEGRAM_API_URL` to tg-mock's URL and `RALPH_TELEGRAM_BOT_TOKEN` to a test token
4. Run `bm start -t <team>`
5. Verify `bm status -t <team>` shows the member as "running" with a PID
6. Run `bm stop -t <team>`
7. Verify `bm status -t <team>` shows the member as "stopped"
8. Verify state file (`~/.botminter/state.json`) no longer contains the member

### Test: `e2e_start_already_running_skips`
9. Start a member
10. Run `bm start` again
11. Verify output says "already running" and no duplicate process is spawned

### Test: `e2e_stop_force_kills`
12. Start a member
13. Run `bm stop --force -t <team>`
14. Verify the process is terminated (PID no longer alive)
15. Verify state is cleaned up

### Test: `e2e_status_detects_crashed_member`
16. Start a member, then kill the Ralph process externally (kill PID)
17. Run `bm status -t <team>`
18. Verify output shows "crashed" for that member
19. Verify state is cleaned up (crashed entry removed)

### Test: `e2e_tg_mock_receives_bot_messages`
20. After starting a member with tg-mock configured
21. Wait briefly for Ralph to initialize
22. Query `tg-mock` control API: `GET /__control/requests?token=<token>`
23. Verify at least one Bot API call was made (e.g., `getUpdates` or `sendMessage`)

### Test: `e2e_start_without_ralph_errors`
24. Temporarily remove `ralph` from PATH (or use a non-existent binary name)
25. Run `bm start -t <team>`
26. Verify clear error: "ralph not found in PATH"

## Dependencies
- Task-06 E2E harness (`TgMock`, `TempRepo`, helpers)
- Task-07 or equivalent workspace provisioning
- `ralph` binary in PATH (for start/stop tests)
- Docker available (for tg-mock)
- Feature-gated behind `e2e`

## Implementation Approach
1. Create `crates/bm/tests/e2e/start_to_stop.rs`
2. Each test provisions its own workspace using the programmatic setup
3. Configure `RALPH_TELEGRAM_API_URL` to point to the tg-mock instance
4. Use `bm_cmd()` helper with env overrides for each CLI invocation
5. For crash detection test: use `libc::kill(pid, SIGKILL)` to simulate crash
6. For "ralph not found" test: temporarily override PATH
7. All assertions include timeouts (Ralph needs 2-3 seconds to start)

## Acceptance Criteria

1. **Full start-stop lifecycle**
   - Given a provisioned workspace with one member
   - When `bm start` → `bm status` → `bm stop` → `bm status` is executed
   - Then status transitions: stopped → running → stopped, and state.json is clean

2. **Already running detection**
   - Given a running member
   - When `bm start` is run again
   - Then output indicates "already running" and no duplicate process exists

3. **Force stop**
   - Given a running member
   - When `bm stop --force` is run
   - Then the process is killed immediately and state is cleaned

4. **Crash detection**
   - Given a member whose Ralph process was killed externally
   - When `bm status` is run
   - Then output shows "crashed" and state is cleaned up

5. **tg-mock integration**
   - Given a member started with `RALPH_TELEGRAM_API_URL` pointing to tg-mock
   - When Ralph initializes
   - Then tg-mock's control API shows at least one Bot API request from the bot token

6. **Missing ralph binary**
   - Given `ralph` is not in PATH
   - When `bm start` is run
   - Then a clear error message is shown

## Metadata
- **Complexity**: High
- **Labels**: test, e2e, lifecycle, docker, telegram, ralph
- **Required Skills**: Rust, E2E testing, Docker, process management, HTTP APIs
