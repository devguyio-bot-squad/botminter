# Integration Test Guide

## Architecture

Tests use full process isolation — no global environment variable mutation.

### Test helpers

| Helper | Purpose |
|--------|---------|
| `setup_team(tmp, name, profile)` | Creates team repo + config in a temp directory. No env vars. |
| `bm_cmd(home)` | Creates a `Command` with HOME/XDG_CONFIG_HOME set for isolation. |
| `bm_run(home, args)` | Runs `bm` and asserts success. |
| `bm_run_fail(home, args)` | Runs `bm` and asserts failure. Returns stderr. |
| `bm_hire(home, role, name, team)` | Hires a member via subprocess. |
| `bm_sync(home, team)` | Runs `bm teams sync` via subprocess. |
| `bm_add_project(home, url, team)` | Adds a project via subprocess. |
| `get_free_port()` | Returns an OS-assigned free port (avoids port conflicts). |
| `wait_for_port(port, timeout)` | Polls until a TCP port accepts connections. |

### Isolation model

- **No `env::set_var`**: Tests never mutate process-global environment variables.
- **Explicit paths**: Profile resolution uses `profiles_dir_for(home)` and `_from` variants.
- **Subprocess isolation**: Commands that resolve config via `dirs::home_dir()` run as subprocesses with per-test HOME via `.env("HOME", tmp)`.
- **OS-assigned ports**: Webhook tests use `get_free_port()` instead of hardcoded port numbers.
- **Readiness polling**: `wait_for_port()` replaces fixed-duration `thread::sleep()` calls.

## Known Flaky Tests

| Test | Failure mode | Suspected cause | Date |
|------|-------------|-----------------|------|
| `daemon_poll_launches_member_existing` | "Daemon did not launch member within 30s" | Race between daemon poll interval and 30s timeout; under CI load the daemon may not complete a full poll cycle + member launch within the window | 2026-03-17 |
