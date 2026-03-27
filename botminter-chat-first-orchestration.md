# BotMinter: Chat-First Orchestration

**Status**: Design exploration
**Date**: 2026-03-19

## The Idea

Instead of a loop that has a chat sidecar, have a **chat brain that spawns loops as workers**.

The brain is an ACP Claude Code instance — always alive, autonomous, and available on chat. It works like a human team member: checks the board, picks tasks, starts work, monitors progress, and is reachable on Slack anytime.

## Current Model vs. BotMinter

### Current: Loop-First

```
Human starts a loop → Ralph runs iterations → Telegram is bolted on as sidecar
```

The loop is primary. Chat is secondary. When the loop stops, the bot goes silent. Between iterations, only deterministic `/status` and `/tasks` responses — no reasoning.

### Current Daemon Mode: Still Loop-First

The existing `ralph bot daemon` is a thin message-to-loop dispatcher:

```
Daemon polls Telegram
    Free text arrives → start_loop(text)
    Daemon STOPS polling, loop takes over
    Loop finishes → daemon resumes
```

No intelligence in the idle state. No conversation. No autonomy.

### BotMinter: Chat-First

```
ACP Claude Code brain (always alive, autonomous)
    ├── Checks the board, picks tasks, starts Ralph loops
    ├── Monitors loop progress, handles blockers
    ├── Picks the next task when a loop finishes
    └── Available on Telegram chat at all times
```

The brain is the primary entity. Loops are ephemeral work units it dispatches. The human is a manager who can check in, reprioritize, or discuss approach — but the brain works autonomously when left alone.

## How It Works

### The Brain

A persistent Claude Code instance running via ACP (`claude-code-acp-rs`). It has:

- Full codebase access (Read, Grep, Glob, Bash)
- Conversation history (ACP session stays alive)
- Standing instructions to work the board autonomously
- The `ralph` CLI for managing loops, tasks, and memories

It doesn't need special tools. Claude Code + Bash + the existing Ralph CLI gives it everything:

```bash
ralph run -p "implement auth module" --worktree    # start a loop
ralph loops                                         # check running loops
ralph loops stop auth-module                        # stop a loop
ralph tools task list                               # read the board
ralph tools memory search "auth"                    # check memories
ralph emit "human.guidance" "use JWT not sessions"  # send guidance to a loop
```

### The Human

Like a manager on Slack talking to a team member:

| Human says | Brain does |
|-----------|-----------|
| *[silence for 8 hours]* | Works autonomously — picks tasks, runs loops, handles blockers |
| "What are you working on?" | Checks active loops, responds conversationally |
| "How's the auth module going?" | Reads loop progress, explains status with context |
| "Stop that, this P0 bug needs fixing first" | Stops current loop, starts one for the bug |
| "I think we should use Redis here" | Sends guidance to the relevant loop |
| "Good morning, what happened overnight?" | Summarizes completions, blockers, decisions made |
| "Let's discuss the API design before you start" | Has a back-and-forth conversation, then starts work |

### Loop Progress Feeds Back to the Brain

The brain needs to know what its workers are doing. Three ways:

**Pull**: Brain calls `ralph loops` or reads `events.jsonl` whenever it wants a status update.

**Push**: The BotMinter process watches each loop's `events.jsonl` for significant events (`human.interact`, `build.blocked`, `LOOP_COMPLETE`) and injects them as prompts into the ACP session.

**Heartbeat**: When nothing else is happening, the BotMinter process periodically prompts the brain: "Heartbeat. Check on your loops. Check the board."

### The Critical Flow: Loop Asks a Question

Today, when a loop emits `human.interact`, it blocks waiting for the human to respond on Telegram. In BotMinter:

```
Loop asks: "Should I use JWT or session cookies?"
    │
    ▼
Brain receives the question (via event watcher or status check)
    │
    ▼
Brain decides:
    ├── "I know this — we discussed it, memory #12 says JWT"
    │   → answers the loop directly (writes human.response)
    │   → loop unblocks, human never bothered
    │
    └── "This is a significant architectural decision"
        → asks the human on Telegram
        → human responds
        → brain translates response + adds context
        → writes human.response to the loop
        → loop unblocks
```

The brain acts as an intelligent mediator. It doesn't forward every question to the human — it uses judgment, like a good team member would.

## The BotMinter Process

The Rust process is a thin multiplexer with three input streams:

```
┌─────────────────────────────────────────────┐
│  BotMinter Process                          │
│                                             │
│  Input streams:                             │
│  ┌──────────────┐                           │
│  │ Telegram      │──► message arrives       │
│  │ messages      │                          │
│  ├──────────────┤                           │
│  │ Loop events   │──► significant event     │──► prompt ACP session
│  │ (file watch)  │    from a running loop   │
│  ├──────────────┤                           │
│  │ Heartbeat     │──► periodic timer fires  │
│  │ timer         │                          │
│  └──────────────┘                           │
│                                             │
│  ACP Session (claude-code-acp-rs)           │
│  ├── Receives prompts from merged streams   │
│  ├── Responds conversationally              │
│  ├── Uses Bash to call ralph CLI            │
│  └── Streaming responses → Telegram         │
│                                             │
└─────────────────────────────────────────────┘
```

It doesn't need to be smart. It just:
1. Spawns and maintains the ACP session
2. Routes Telegram messages as prompts
3. Watches loop event files and injects significant events as prompts
4. Fires a heartbeat when idle
5. Streams ACP responses back to Telegram

## BotMinter Is a Separate Project

BotMinter is **not part of Ralph**. It's a separate project that consumes Ralph as a CLI tool — the same way a human developer would.

### Ralph Orchestrator: No Changes Needed

The brain interacts with Ralph entirely through the CLI and the file system:

| Ralph surface | How BotMinter uses it |
|--------------|----------------------|
| `ralph run -p "..." --worktree` | Brain starts loops via Bash |
| `ralph loops` | Brain checks running loops via Bash |
| `ralph loops stop <id>` | Brain stops loops via Bash |
| `ralph tools task list` | Brain reads the board via Bash |
| `ralph tools memory search` | Brain searches memories via Bash |
| `ralph emit "human.guidance"` | Brain sends guidance to loops via Bash |
| `.ralph/events-*.jsonl` | BotMinter process watches for loop events |
| `.ralph/loop.lock` | BotMinter process checks if a loop is active |
| `.ralph/agent/tasks.jsonl` | Brain reads tasks directly or via CLI |
| `.ralph/stop-requested` | Brain signals a loop to stop |

Ralph's file-based coordination model (Tenet 4: "Disk Is State") is the integration surface. The files are the API. No changes, no new protocols, no dependencies to add.

### What BotMinter Builds

| Component | Description |
|-----------|------------|
| ACP session management | Spawn `claude-code-acp-rs`, maintain persistent session |
| Telegram routing | Poll Telegram, route messages as ACP prompts |
| Event file watcher | Watch running loops' `events.jsonl` for significant events |
| Heartbeat timer | Periodic autonomous prompts when idle |
| Telegram streaming bridge | Forward ACP `AgentMessageChunk` to Telegram |
| Permission routing | Forward ACP `RequestPermission` callbacks to Telegram |
| Brain system prompt | Standing instructions for autonomous board-driven work |

BotMinter can reference Ralph's `AcpExecutor` pattern and `TelegramDaemon` as architectural inspiration, but it builds its own implementations.

## What ACP Gives Us

The brain runs via ACP (`claude-code-acp-rs`), not `claude -p --resume`, because:

- **No process spawn per message** — the session stays alive, ~500ms to first token vs 2-5s
- **Streaming** — typed `AgentMessageChunk` notifications forwarded to Telegram in real-time
- **Permission callbacks** — when Claude wants to run a destructive command, it asks the human via Telegram before proceeding
- **Cancellation** — human sends "stop" on Telegram, brain gets a clean interrupt, session stays alive
- **Session continuity** — multiple prompts on the same session, full conversation history

See `.ralph/specs/research/acp-protocol-deep-dive.md` for the full ACP protocol analysis.

## Why This Is Different From Just Running Claude Code

A raw Claude Code instance doing `ralph run` via Bash would block until the loop finishes (could be 30 minutes) and be completely unavailable for chat.

BotMinter starts loops as **background processes** via the Ralph CLI. The brain returns immediately — still available for chat. The Rust process watches loop events and pushes updates into the session. The brain can react to loop progress, answer loop questions, send guidance — all while chatting with the human.

The Rust process is the multiplexer that makes this concurrency possible. Claude Code processes prompts sequentially, but the Rust process handles the async fan-in of Telegram + loop events + heartbeat and feeds them as sequential prompts.

## Open Design Questions

**Prompt merging**: When a Telegram message and a loop event arrive simultaneously, which goes first? Probably human messages take priority — you don't ignore your manager.

**Heartbeat frequency**: How often should the brain autonomously check on things? Too frequent = expensive and noisy. Too infrequent = slow to react. Probably configurable, starting at 60 seconds.

**Event significance filtering**: Which loop events are worth prompting the brain about? Every event would be noisy. Likely just: `human.interact`, `build.blocked`, `task.close`, `LOOP_COMPLETE`, and maybe periodic summaries.

**Autonomy boundaries**: How much should the brain decide on its own vs. escalate to the human? This is a system prompt / CLAUDE.md concern, not an architecture concern. Let users tune it.

**Multiple loops**: Can the brain run multiple loops in parallel? Ralph already supports parallel worktree loops. The brain just needs to track them. The event watcher handles multiple files.

**Cost**: The brain consumes tokens for every prompt (Telegram messages, loop events, heartbeats). Using a cheaper model (Sonnet/Haiku) for the brain while loops use Opus could manage costs.
