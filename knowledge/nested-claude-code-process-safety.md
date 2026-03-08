# Nested Claude Code & Process Safety

When developing botminter via Ralph, the implementing agent runs inside Claude Code which is managed by Ralph. Several `bm` commands (`bm chat`, `bm minty`, `bm start`) launch new Claude Code instances, creating a Claude-inside-Claude situation.

## The CLAUDECODE Environment Variable

Claude Code sets `CLAUDECODE=1` in its own environment. A nested Claude Code instance detects this and refuses to start.

**Three call sites in `bm` already handle this for production use:**

| File | Context |
|------|---------|
| `crates/bm/src/commands/start.rs:239-240` | `bm start` launching Ralph per member |
| `crates/bm/src/commands/daemon.rs:752` | Daemon mode launching Ralph per member |
| `crates/bm/src/session.rs:69-70` | Interactive sessions (`bm knowledge`) |

All use `.env_remove("CLAUDECODE")` before spawning the child process.

**For manual testing during development**, the agent must unset the variable itself:

```bash
CLAUDECODE= bm chat bob --render-system-prompt   # inline unset
env -u CLAUDECODE bm chat bob                     # explicit unset
```

## Process Safety Rules

The implementing agent is itself a process managed by Ralph. Careless process management can kill the orchestrator.

1. **NEVER kill by name or pattern.** No `pkill ralph`, no `killall claude`, no `kill $(pgrep ...)`. These can match the agent's own Ralph or parent Claude Code.
2. **Kill only by specific PID.** If you spawn a process for testing, capture its PID and kill only that PID.
3. **NEVER run `bm stop` during implementation.** `bm stop` sends signals to Ralph processes tracked in `~/.botminter/state.json` — this includes the agent's own orchestrator.
4. **Use `just dev-launch` for launching team members.** The root Justfile's `dev-launch` recipe unsets `CLAUDECODE` automatically. Never use the team repo's `just launch` from inside a Ralph session.

## How `bm stop` Works (Why It's Dangerous)

`bm stop` reads PIDs from `~/.botminter/state.json` and sends:
- Graceful: writes `STOP` to Ralph's event file, waits up to 30s
- Force (`-f`): sends `SIGTERM`, waits 500ms, then `SIGKILL`

Source: `crates/bm/src/commands/stop.rs`

If the agent's own Ralph PID is in `state.json`, running `bm stop` terminates itself.
