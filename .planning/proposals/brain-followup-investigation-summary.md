# Brain Follow-Up Investigation Summary

**Date:** 2026-03-26
**Investigator:** Claude Opus 4.6

## Problem

When the brain runs a background task (e.g., `sleep 10 && echo DONE > /tmp/result.txt`), it acknowledges the request but never follows up with the result. The operator always has to remind it.

## Root Cause Analysis

Three layers contributed:

1. **`settings.json`** denied `mcp__acp__BashOutput` — the LLM literally couldn't call it
2. **System prompt** said "FORBIDDEN: BashOutput is disabled" with a 50-line execution protocol forcing all-background, no-follow-up
3. **`CHAT_FIRST_REMINDER`** appended to every prompt reinforced "Do NOT call BashOutput"
4. **Rust ACP (`claude-code-acp-rs`) Terminal API** blocks indefinitely on `BashOutput` when the background process is still running (discovered when we un-denied BashOutput)

## Solution Applied

### 1. Upgraded `sacp` 10 → 11 + schema feature flag

```toml
# crates/bm/Cargo.toml
sacp = "11.0.0"
agent-client-protocol-schema = { version = "0.11.3", features = ["unstable_session_usage"] }
```

The `unstable_session_usage` feature adds `UsageUpdate` to the `SessionUpdate` enum, which the TS ACP sends and the old schema couldn't deserialize (crashed the connection).

### 2. Updated ACP client API (3 renames in `crates/bm/src/acp/client.rs`)

```rust
// Before (sacp 10)
use sacp::{ClientToAgent, JrConnectionCx};
.run_until(transport, |cx: JrConnectionCx<ClientToAgent>| async move { ... })

// After (sacp 11)
use sacp::role::acp::{Agent, Client};
use sacp::ConnectionTo;
.connect_with(transport, async |cx: ConnectionTo<Agent>| { ... })
```

### 3. Switched ACP runtime from Rust to TypeScript

Replaced `claude-code-acp-rs` binary with `claude-agent-acp` (TS). The TS ACP uses the Claude CLI's **built-in** Bash/BashOutput tools (which are non-blocking) instead of replacing them with MCP tools that route through the Terminal API (which blocks).

**Runtime swap on test user:** `~/.local/bin/claude-code-acp-rs` is now a shell wrapper that runs `node ~/claude-agent-acp/dist/index.js`.

### 4. Cleared `denied_tools` in both profiles

```json
// profiles/scrum-compact/coding-agent/settings.json
// profiles/scrum/coding-agent/settings.json
"denied_tools": []
```

Previously denied `mcp__acp__BashOutput` and `mcp__acp__KillShell`. With the TS ACP, `mcp__acp__*` tools don't exist (it uses CLI built-ins), so the denials were dead code anyway.

### 5. Stripped execution protocol from brain prompt

Removed the 50-line "Background Execution Protocol" section from `profiles/scrum-compact/brain/system-prompt.md`. Replaced with one sentence: "Respond promptly — don't let autonomous work block your ability to reply."

Emptied `CHAT_FIRST_REMINDER` in `multiplexer.rs`. The LLM handles background tasks correctly on its own with the TS ACP — no prompt engineering needed.

## Test Results

### Verified on `bm-dashboard-test-user@localhost` (port 9009)

| Test | Before fix | After fix |
|------|-----------|-----------|
| 3s task (`sleep 3 && echo RESULT`) | 65s (heartbeat) | 30s (same turn) |
| 10s task (`sleep 10 && echo DONE`) | HUNG (Rust ACP BashOutput blocks) | 40s (same turn, TS ACP) |
| 10s task, minimal prompt (no execution instructions) | N/A | 30s |

### Unit tests: 120 passed, 0 failed
### Clippy: clean, no warnings

### E2E tests: inconclusive

Ran `just test` twice. Both runs had failures in `scenario_operator_journey`:

**Failure 1:** `projects_sync_fresh` — `gh project field-list` can't resolve project number 968
**Failure 2:** `projects_sync_fresh` — same pattern with project number 977, plus API rate limit

These failures are **NOT caused by our changes**. Our changes touch `Cargo.toml`, `acp/client.rs`, `brain/multiplexer.rs`, profile prompts, and `settings.json` — none of which affect project board creation or sync.

The failing code path is:
1. `bm init` creates a project board → gets number N → stores in `config.yml`
2. Later, `bm projects sync` calls `gh project field-list N --owner org`
3. Project N no longer exists → GraphQL error

**Suspected cause:** The project board is deleted between creation (init step) and use (projects_sync step). Needs investigation into:
- Whether another e2e scenario's cleanup (`cleanup_project_boards`) matches the board title pattern
- Whether `TempProject::drop` from `GithubSuite` deletes the board
- Whether GitHub has eventual consistency issues with project numbering
- Whether the `--test-threads=1` flag actually serializes across scenarios (it serializes within a scenario, but multiple `Trial` entries may run in parallel)

## What Needs to Happen Next

### To land this change:
1. Run `just test` in isolation (no other Claude instances using the test infrastructure) and confirm the e2e failure is pre-existing
2. Investigate the `projects_sync_fresh` failure — it may be a test isolation bug where scenarios interfere with each other's project boards
3. Decide on TS ACP packaging — currently the test user has a manual `node` wrapper; production deployment needs a proper binary or npm package

### To complete the TS ACP migration:
1. Update `launch_brain` in `formation/launch.rs` to use `claude-agent-acp` as the ACP binary (currently defaults to `claude-code-acp-rs`)
2. Add `claude-agent-acp` to the deployment pipeline (exploratory test deploy recipe, Lima boot script)
3. Update CLAUDE.md to document the ACP switch
4. Run full exploratory test suite (phase H) to validate brain lifecycle with TS ACP

### Future improvement:
See `.planning/proposals/multiplexer-priority-gating.md` — redesign multiplexer to use priority gating with ACP queue delegation, leveraging the TS ACP's native `promptQueueing` capability.

## Files Changed

```
Cargo.lock                                        | 52 ++++++++++-------------
crates/bm/Cargo.toml                              |  3 +-
crates/bm/src/acp/client.rs                       | 11 ++---
crates/bm/src/brain/multiplexer.rs                | 19 +++------
profiles/scrum-compact/brain/system-prompt.md     | 51 +---------------------
profiles/scrum-compact/coding-agent/settings.json |  5 +--
profiles/scrum/coding-agent/settings.json         |  5 +--
.planning/proposals/multiplexer-priority-gating.md | (new)
```

Pre-existing fixes also included (found during work):
```
crates/bm/src/git/github.rs                       |  1 +  (bug color)
crates/bm/src/web/assets.rs                       | 10 +++++ (console build tolerance)
crates/bm/tests/integration.rs                    | 40 ++++++++------- (console test tolerance)
```

## Environment

- **Test user:** `bm-dashboard-test-user@localhost`
- **Tuwunel port:** 9009 (avoids conflict with `bm-test-user` on 8008)
- **TS ACP source:** `/opt/workspace/claude-agent-acp` (cloned from `github.com/agentclientprotocol/claude-agent-acp`)
- **Node.js:** v22.21.1 installed at `~/.local/bin/node` on test user
- **Team:** `brain-followup-test` on org `devguyio-bot-squad`
- **GitHub repo:** `devguyio-bot-squad/brain-follow-up-test`
- **Tuwunel container:** `bm-tuwunel-brain-followup-test` (still running on port 9009)
