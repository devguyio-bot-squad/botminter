# Milestone: Chat-First Member

**Status**: Implementation complete (2026-03-21)
**Date**: 2026-03-20
**Implementation**: Phases 1-6 committed on chat-first-member branch (2026-03-20)

## Vision

Flip the BotMinter member model from **"Ralph loop with chat sidecar"** to **"persistent Claude Code ACP brain that spawns Ralph loops as workers."**

Today, `bm start` launches a Ralph Orchestrator process per member. Chat is bolted on as a sidecar — when the loop ends, the bot goes silent. Between iterations, only deterministic responses; no reasoning, no conversation, no autonomy.

The chat-first model makes the **brain** the primary entity:

```
ACP Claude Code brain (always alive, autonomous)
    |-- Checks the board, picks tasks, starts Ralph loops
    |-- Monitors loop progress, handles blockers
    |-- Answers loop questions from knowledge (or escalates to human)
    |-- Picks the next task when a loop finishes
    |-- Available on bridge chat at all times
```

The human is a manager who can check in, reprioritize, or discuss approach — but the brain works autonomously when left alone.

## Architecture

### The Multiplexer Process

The core new component is a thin Rust process that replaces raw `ralph run` in `bm start`. It is the bridge between three async input streams and the sequential ACP session:

```
+---------------------------------------------+
|  BotMinter Multiplexer Process              |
|                                             |
|  Input streams:                             |
|  +---------------+                          |
|  | Bridge        |--- message arrives       |
|  | messages      |                          |
|  +---------------+                          |
|  | Loop events   |--- significant event  ---+--- prompt ACP session
|  | (file watch)  |    from a running loop   |
|  +---------------+                          |
|  | Heartbeat     |--- periodic timer fires  |
|  | timer         |                          |
|  +---------------+                          |
|                                             |
|  ACP Session (claude-code-acp-rs)           |
|  |-- Receives prompts from merged streams   |
|  |-- Responds conversationally              |
|  |-- Uses Bash to call ralph CLI            |
|  +-- Streaming responses -> bridge          |
|                                             |
+---------------------------------------------+
```

### ACP Protocol Surface

The brain runs via `claude-code-acp-rs` over stdio JSON-RPC. Key protocol methods:

| Method | Purpose |
|--------|---------|
| `initialize` | Initial handshake, capability negotiation |
| `session/new` | Create persistent session with CWD, system prompt, MCP servers, permission mode |
| `session/prompt` | Send prompt to session, get streaming response via notifications |
| `session/cancel` | Interrupt current prompt (human sends "stop") |
| `session/set_mode` | Change permission mode at runtime |
| `session/fork` | Fork session for parallel work |
| `session/list` | List active sessions |

Key features:
- **Session continuity** — multiple prompts on same session, full conversation history
- **Streaming** — `SessionNotification` with `AgentMessageChunk` content blocks
- **Permission callbacks** — `RequestPermission` for destructive commands, routed to bridge
- **~500ms to first token** — no process spawn per message

### Ralph Integration: CLI + Filesystem Only

**Hard constraint: No changes to Ralph Orchestrator.** The brain interacts with Ralph entirely through the CLI and the filesystem:

| Ralph surface | How the brain uses it |
|--------------|----------------------|
| `ralph run -p "..." --worktree` | Start loops via Bash |
| `ralph loops` | Check running loops via Bash |
| `ralph loops stop <id>` | Stop loops via Bash |
| `ralph tools task list` | Read the board via Bash |
| `ralph tools memory search` | Search memories via Bash |
| `ralph emit "human.guidance"` | Send guidance to loops via Bash |
| `.ralph/events-*.jsonl` | Multiplexer watches for loop events |
| `.ralph/loop.lock` | Multiplexer checks if a loop is active |
| `.ralph/agent/tasks.jsonl` | Brain reads tasks directly or via CLI |

Ralph's file-based coordination model ("Disk Is State") is the integration surface. The files are the API.

### The Critical Flow: Brain as Mediator

When a loop emits `human.interact`, the brain intercepts it:

```
Loop asks: "Should I use JWT or session cookies?"
    |
    v
Brain receives the question (via event watcher)
    |
    v
Brain decides:
    |-- "I know this -- we discussed it, memory says JWT"
    |   -> answers the loop directly (writes human.response)
    |   -> loop unblocks, human never bothered
    |
    +-- "This is a significant architectural decision"
        -> asks the human on bridge
        -> human responds
        -> brain translates response + adds context
        -> writes human.response to the loop
        -> loop unblocks
```

The brain acts as an intelligent mediator. It doesn't forward every question to the human — it uses judgment, like a good team member would.

## Design Decisions

These resolve the open questions from the design exploration:

| Question | Decision | Rationale |
|----------|----------|-----------|
| **Prompt priority** | Human > loop events > heartbeat | You don't ignore your manager |
| **Heartbeat frequency** | 60s default, configurable via `brain.heartbeat_secs` in profile | Balance cost vs. responsiveness |
| **Event significance filter** | `human.interact`, `build.blocked`, `task.close`, `LOOP_COMPLETE` | Only events that require brain attention |
| **Multiple loops** | Yes — Ralph already supports parallel worktree loops | Brain tracks all running loops; event watcher handles multiple files |
| **Autonomy boundaries** | System prompt / CLAUDE.md concern, not architecture | Users tune via brain prompt template in profile |
| **Dual-channel interaction** | GitHub for formal artifacts, bridge for informal chat | Brain prompt instructs contextual channel selection |
| **Cost management** | Configurable model per brain (`brain.model` in profile) | Default to sonnet for brain, opus for Ralph loops |

## Phased Implementation Plan

### Phase 1: ACP Client Library

**Goal:** Thin Rust wrapper around `claude-code-acp-rs` stdio JSON-RPC for use by the multiplexer.

**What to build:**
- `crates/bm/src/acp/` module with:
  - `AcpProcess` — spawn `claude-code-acp-rs` as child process over stdio
  - `AcpSession` — create/prompt/cancel sessions via JSON-RPC messages
  - `AcpStream` — typed streaming response reader (parse `SessionNotification` from stdout)
  - `AcpPermission` — permission request/response handling
- JSON-RPC message types (request, response, notification) matching ACP protocol
- Integration test: spawn ACP process, create session, send prompt, receive streaming response

**Acceptance criteria:**
- [x] Can spawn `claude-code-acp-rs` and perform initialize handshake (Phase 1, 43c0e79)
- [x] Can create a session with CWD and system prompt (Phase 1, 43c0e79)
- [x] Can send a prompt and receive streaming `AgentMessageChunk` notifications (Phase 1, 43c0e79)
- [x] Can cancel an in-progress prompt (Phase 1, 43c0e79)
- [x] Can handle `RequestPermission` callbacks (accept/deny) (Phase 1, 43c0e79)
- [x] Session stays alive across multiple prompts (Phase 1, 43c0e79)
- [x] Clean shutdown of child process (Phase 1, 43c0e79)
- [x] Unit tests for JSON-RPC message serialization/deserialization (Phase 1, 43c0e79)
- [x] Integration test against real `claude-code-acp-rs` binary (Phase 1, 43c0e79)

**Key reference:** `/opt/workspace/claude-code-acp-rs/src/agent/handlers.rs` for protocol methods, `/opt/workspace/claude-code-acp-rs/src/mcp/acp_server.rs` for JSON-RPC message routing.

### Phase 2: Multiplexer Process

**Goal:** The core process that manages the ACP session and merges input streams.

**What to build:**
- `crates/bm/src/brain/` module with:
  - `Multiplexer` — async event loop with `tokio::select!` over input channels
  - `PromptQueue` — priority queue: human messages (P0) > loop events (P1) > heartbeat (P2)
  - `BridgeInput` — receive messages from bridge (trait, initially Telegram polling)
  - `AcpOutput` — stream ACP responses back to bridge
- Prompt serialization: wrap each input with context prefix
  - Human message: `[Human on bridge]: <message>`
  - Loop event: `[Loop <id> event]: <event type> — <summary>`
  - Heartbeat: `[Heartbeat]: Check your loops. Check the board. Pick up new work if idle.`
- Concurrency: when a prompt is in-flight, queue incoming messages; when response completes, drain queue by priority

**Acceptance criteria:**
- [x] Multiplexer spawns ACP session on start (Phase 2, 604be17)
- [x] Human messages routed as prompts with correct prefix (Phase 2, 604be17)
- [x] ACP streaming responses forwarded to bridge in real-time (Phase 2, 604be17)
- [x] Priority queue respects human > events > heartbeat ordering (Phase 2, 604be17)
- [x] Queued messages are drained in priority order after current prompt completes (Phase 2, 604be17)
- [x] Clean shutdown: cancel in-flight prompt, close ACP session (Phase 2, 604be17)
- [x] Unit tests for priority queue (Phase 2, 604be17)
- [x] Integration test: mock bridge input -> ACP prompt -> mock bridge output (Phase 2, 604be17)

**Key reference:** Existing `crates/bm/src/daemon/` module for process lifecycle patterns.

### Phase 3: Event Watcher

**Goal:** Watch running Ralph loops' event files and inject significant events as prompts.

**What to build:**
- `crates/bm/src/brain/event_watcher.rs`:
  - File watcher (notify crate or poll-based) on `.ralph/events-*.jsonl`
  - Significance filter: only `human.interact`, `build.blocked`, `task.close`, `LOOP_COMPLETE`
  - Parse event JSON, extract type + summary
  - Push significant events into the multiplexer's prompt queue at P1 priority
  - Handle multiple concurrent loop event files (parallel worktree loops)
  - Detect new event files appearing (new loops started by the brain)

**Acceptance criteria:**
- [x] Detects new lines appended to existing event files (Phase 3, eb1d2ec)
- [x] Detects new event files appearing in `.ralph/` (Phase 3, eb1d2ec)
- [x] Filters to only significant event types (Phase 3, eb1d2ec)
- [x] Injects events into multiplexer prompt queue (Phase 3, eb1d2ec)
- [x] Handles multiple concurrent event files (Phase 3, eb1d2ec)
- [x] Does not re-inject events already seen (tracks file offset) (Phase 3, eb1d2ec)
- [x] Unit tests with synthetic event files (Phase 3, eb1d2ec)
- [x] Integration test: write event to file -> prompt appears in queue (Phase 3, eb1d2ec)

**Key reference:** Ralph's event file format at `.ralph/events-*.jsonl` — JSON lines with `{"ts": "...", "event": {"type": "..."}}`.

### Phase 4: Heartbeat Timer

**Goal:** Periodic autonomous prompts when the brain is idle.

**What to build:**
- `crates/bm/src/brain/heartbeat.rs`:
  - `tokio::time::interval` at configurable frequency (default 60s)
  - Skips firing when a prompt is currently being processed
  - Skips firing when a heartbeat was already fired within the interval
  - Heartbeat prompt: `[Heartbeat]: Check your loops. Check the board. Pick up new work if idle.`
  - Pushes to multiplexer prompt queue at P2 (lowest) priority

**Acceptance criteria:**
- [x] Fires at configured interval when idle (Phase 4, 2670296)
- [x] Does NOT fire when a prompt is in-flight (Phase 4, 2670296)
- [x] Configurable frequency via profile (`brain.heartbeat_secs`) (Phase 4, 2670296)
- [x] Can be disabled by setting frequency to 0 (Phase 4, 2670296)
- [x] Unit tests for timer logic and skip-when-busy behavior (Phase 4, 2670296)

### Phase 5: Brain System Prompt Template

**Goal:** Standing instructions that make the brain an autonomous team member.

**What to build:**
- Brain system prompt template in the profile (`profiles/scrum-compact/brain/system-prompt.md`):
  - Identity: "You are [member name], a team member on [team name]"
  - Board awareness: scan GitHub issues with status labels matching your role
  - Work loop: pick task -> start Ralph loop -> monitor -> pick next
  - Loop management: `ralph run`, `ralph loops`, `ralph loops stop`
  - Human interaction: answer from knowledge if confident, escalate to bridge if not
  - Dual-channel: GitHub comments for formal artifacts, bridge for informal chat
  - Current state awareness: "Check `.ralph/loop.lock` to see if a loop is running"
- Template variables: `{{member_name}}`, `{{team_name}}`, `{{role}}`, `{{gh_org}}`, `{{gh_repo}}`
- Brain prompt surfacing during `bm teams sync` (render template, write to workspace)

**Acceptance criteria:**
- [x] System prompt template exists in profile (Phase 5, 735df88)
- [x] Template renders with member-specific variables (Phase 5, 735df88)
- [x] Rendered prompt is surfaced to workspace during `bm teams sync` (Phase 5, 735df88)
- [ ] Brain acts autonomously: picks tasks, starts loops, monitors progress (requires ACP binary — deferred to integration testing)
- [ ] Brain responds conversationally to bridge messages (requires ACP binary — deferred to integration testing)
- [ ] Brain uses both GitHub and bridge channels contextually (requires ACP binary — deferred to integration testing)
- [ ] Manual E2E validation: start brain, verify it picks work and responds to chat (requires ACP binary — deferred to integration testing)

### Phase 6: `bm` CLI Integration

**Goal:** `bm start` launches the multiplexer instead of raw ralph, with clean lifecycle management.

**What to build:**
- Modify `crates/bm/src/formation/start_members.rs`:
  - Replace `formation::launch_ralph()` with `formation::launch_brain()` for chat-first members
  - Brain launch: spawn multiplexer process (not ralph directly)
  - Pass ACP binary path, system prompt path, bridge credentials, workspace path
- Modify `crates/bm/src/commands/stop.rs`:
  - Send shutdown signal to multiplexer (which cancels ACP session + stops running loops)
- Modify `crates/bm/src/commands/status.rs`:
  - Show brain health: ACP session alive, active loops, last heartbeat, last human interaction
- Modify `crates/bm/src/commands/chat.rs`:
  - `bm chat <member>` connects directly to the running ACP session (sends prompt, streams response)
  - Alternative: if multiplexer exposes a local socket, `bm chat` connects to that

**Acceptance criteria:**
- [x] `bm start` launches multiplexer per member (not raw ralph) (Phase 6, 224c305)
- [x] `bm stop` cleanly shuts down multiplexer + ACP session + running loops (Phase 6, 224c305)
- [x] `bm status` shows brain health alongside member info (Phase 6, 224c305)
- [x] `bm chat <member>` interacts with the running brain (Phase 6, 224c305)
- [x] State file tracks multiplexer PID (not ralph PID) (Phase 6, 224c305)
- [x] Stale state cleanup works with multiplexer processes (Phase 6, 224c305)
- [ ] E2E test: start -> status shows healthy -> chat works -> stop -> status shows stopped (requires ACP binary — deferred to integration testing)
- [x] Docs updated: `docs/content/reference/cli.md`, `docs/content/getting-started/index.md` (Phase 6, 224c305)

## Existing Code That Will Be Modified

| File | Change |
|------|--------|
| `crates/bm/src/session/mod.rs` | Add ACP session management alongside existing Ralph session |
| `crates/bm/src/formation/start_members.rs` | Replace `launch_ralph()` with `launch_brain()` for chat-first members |
| `crates/bm/src/formation/mod.rs` | Add `launch_brain()` function |
| `crates/bm/src/commands/start.rs` | Wire up brain launch path |
| `crates/bm/src/commands/stop.rs` | Handle multiplexer shutdown |
| `crates/bm/src/commands/status.rs` | Display brain health info |
| `crates/bm/src/commands/chat.rs` | Connect to running ACP session |
| `crates/bm/src/state.rs` | Track multiplexer process instead of ralph process |
| `profiles/scrum-compact/` | Add brain system prompt template |

## New Code

| Path | Purpose |
|------|---------|
| `crates/bm/src/acp/mod.rs` | ACP client library (JSON-RPC over stdio) |
| `crates/bm/src/acp/process.rs` | Spawn and manage ACP child process |
| `crates/bm/src/acp/session.rs` | Session create/prompt/cancel |
| `crates/bm/src/acp/stream.rs` | Streaming response parser |
| `crates/bm/src/acp/types.rs` | JSON-RPC and ACP message types |
| `crates/bm/src/brain/mod.rs` | Multiplexer module |
| `crates/bm/src/brain/multiplexer.rs` | Core event loop |
| `crates/bm/src/brain/prompt_queue.rs` | Priority prompt queue |
| `crates/bm/src/brain/event_watcher.rs` | Ralph event file watcher |
| `crates/bm/src/brain/heartbeat.rs` | Periodic autonomous prompts |
| `crates/bm/src/brain/bridge_input.rs` | Bridge message receiver |

## Dependencies

| Crate | Purpose | Phase |
|-------|---------|-------|
| `tokio` | Async runtime (already in deps) | 1 |
| `serde_json` | JSON-RPC messages (already in deps) | 1 |
| `notify` | File system watcher (new) | 3 |
| `tokio::sync::mpsc` | Inter-component channels | 2 |

## Constraints & Non-Goals

- **No changes to Ralph Orchestrator** — brain uses CLI + filesystem only
- **No new bridge backends** — reuse existing BotMinter bridge infrastructure
- **Profile-driven** — brain behavior is configurable via profile templates, not hardcoded
- **Backward compatible** — `bm start` for non-chat-first profiles still launches raw ralph
- **Alpha policy** — breaking changes expected, no migration tooling needed
