# Chat-First Member Architecture

**Status**: Milestone planning
**Date**: 2026-03-20

## Core Concept

Flip the BotMinter member model from "Ralph loop with chat sidecar" to "persistent Claude Code ACP brain that spawns Ralph loops as workers." The brain is always alive, autonomous, and available on chat via the bridge (Telegram/RC/Tuwunel).

## Components

### 1. ACP Brain Process
A persistent `claude-code-acp-rs` session running in the member's workspace. Has full codebase access, conversation history, and standing instructions to work the board autonomously. Uses `ralph` CLI via Bash to manage loops.

### 2. BotMinter Multiplexer
A thin Rust process (replaces raw `ralph` in `bm start`) that:
- Spawns and maintains the ACP session (claude-code-acp-rs over stdio JSON-RPC)
- Routes bridge messages as ACP prompts
- Watches Ralph loop event files (.ralph/events-*.jsonl) for significant events
- Fires heartbeat prompts when idle (configurable, ~60s)
- Streams ACP responses back to the bridge
- Routes ACP permission callbacks to the bridge

### 3. Brain System Prompt
Standing instructions telling the brain to:
- Check the GitHub board for work (status labels matching role)
- Pick tasks and start Ralph loops via `ralph run -p "..." --worktree`
- Monitor loop progress via `ralph loops` and event files
- Handle loop questions — answer from knowledge if confident, escalate to human if not
- Pick next task when a loop finishes
- Respond conversationally to human messages

### 4. Event Watcher
File watcher on `.ralph/events-*.jsonl` that detects:
- `human.interact` — loop needs guidance
- `build.blocked` — build/test failure
- `task.close` — task completed
- `LOOP_COMPLETE` — loop finished
Injects these as prompts into the ACP session.

### 5. Heartbeat Timer
Periodic autonomous prompts when idle:
- "Heartbeat: Check your loops. Check the board."
- Configurable frequency (default 60s)
- Skipped when a prompt is already being processed

### 6. Bridge Integration
Reuses existing BotMinter bridge infrastructure:
- Human messages → ACP prompt
- ACP streaming response → bridge message chunks
- ACP permission request → bridge question → human answer → ACP permission response

## Key Design Constraint: No Changes to Ralph Orchestrator

The brain interacts with Ralph entirely through CLI and filesystem:
- `ralph run -p "..."` — start loops
- `ralph loops` — check running loops
- `ralph loops stop <id>` — stop loops
- `ralph tools task list` — read board
- `ralph emit "human.guidance"` — send guidance
- `.ralph/events-*.jsonl` — event files

## Integration with bm CLI

- `bm start` launches the multiplexer instead of raw ralph
- `bm stop` cleanly shuts down ACP session + running ralph loops
- `bm status` shows ACP session health + active loops
- `bm chat` connects directly to the ACP session

## ACP Protocol Surface (claude-code-acp-rs)

The ACP server runs over stdio JSON-RPC:
- `session/new` — create persistent session with CWD, system prompt, MCP servers
- `session/prompt` — send prompt, get streaming response via notifications
- `session/cancel` — interrupt current prompt
- Session stays alive across prompts — full conversation history
- Streaming: `SessionNotification` with `AgentMessageChunk` content blocks
- Permission callbacks: `session/request_permission` for destructive commands
- ~500ms to first token (no process spawn per message)

## Milestone Scope

1. Multiplexer process that manages ACP session lifecycle
2. Bridge message routing (at least Telegram)
3. Event file watcher with significant event injection
4. Heartbeat timer
5. Brain system prompt template in the profile
6. Integration with `bm start`/`bm stop`/`bm status`
