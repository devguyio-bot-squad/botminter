# Proposal: Brain Context Primer

**Date:** 2026-03-26
**Status:** Discussion — not yet implemented

## Problem

When the brain restarts (`bm stop` / `bm start`), it creates a fresh ACP session with zero conversation history. The operator and brain have been chatting — the operator remembers the context, but the brain doesn't. The operator has to re-explain what they were working on.

This is unlike human team communication: when you return to a Slack DM, you glance at the last few messages to orient yourself. You don't re-read the entire history, but you have a sense of "where we left off." The brain has no equivalent.

Within a single session lifetime, the ACP maintains full conversation history in Claude's context window. But that context is volatile — it's lost on restart and subject to compaction (lossy summarization) when the context window fills up.

## Current State

| What the brain knows | Source | Persistence |
|---------------------|--------|-------------|
| Identity, role, processes | `brain-prompt.md` (system prompt) | Always loaded, survives restarts |
| Current conversation | ACP session (Claude context window) | Volatile — lost on restart, compacted when full |
| Work state | GitHub board (issues, PRs) | Persistent — queried on demand |
| Team knowledge | CLAUDE.md, knowledge/ files | Persistent — loaded by Claude Code |
| Recent chat with operator | Matrix DM room history | Persistent — but brain never reads it on startup |

The gap: **recent chat history** exists in Matrix but the brain doesn't use it to prime a new session.

## Design Options

### Option A: Every message is a fresh conversation

The brain has no context per message. The operator must repeat themselves constantly.

**Verdict:** Clearly wrong for a team member relationship. Rejected.

### Option B: Single persistent conversation forever

The ACP session is persisted and resumed across restarts. Full history always available.

**Problems:**
- Claude's context window fills up, compaction kicks in (lossy)
- Resuming sessions after code changes or ACP upgrades is fragile
- Session state is tied to the ACP binary version — not portable
- Old context drifts and becomes misleading over time

**Verdict:** Fragile and impractical. The ACP session was not designed for infinite persistence.

### Option C: Session-scoped conversation with context primer (Recommended)

Each `bm start` creates a fresh ACP session. But before the first heartbeat or human message, the brain receives a **context primer** — a summary of recent interactions fetched from the Matrix DM room.

**Flow:**
1. Brain starts, creates ACP session
2. Brain fetches last N messages from the Matrix DM room (via the bridge adapter's existing Matrix client)
3. Formats them as a preamble prompt: "Here's your recent conversation history with the operator for context"
4. Sends this as the first prompt to the ACP session (before any heartbeat or human message)
5. The LLM now has recent context and can pick up where things left off

**Primer content example:**
```
<bm-context type="primer" channel="matrix">
<bm-message>
Here is your recent conversation history with the operator. Use this as context for the current session.

[20:15] Operator: Can you check the GitHub app branch and create a UAT plan?
[20:16] You: On it — checking the board now.
[20:17] You: Found the branch. It has 35 commits across 132 files. Creating the UAT epic with test cases.
[20:22] You: Done! Created epic #24 with 10 UAT test cases. Ready for you to run through them.
[20:25] Operator: Great, let's start with UAT-01
</bm-message>
</bm-context>
```

**Benefits:**
- Clean context window on every start (no stale accumulation)
- Recent context is available immediately (no "remind me what we were doing")
- Natural behavior — like a human glancing at recent Slack messages
- The full history is in Matrix if the LLM needs to reference older context (it can ask the operator)
- No dependency on ACP session persistence

**Open questions:**
- How many messages to include? (20? 50? Last N minutes?)
- Should the primer include tool call summaries or just chat text?
- Should the primer be fetched from Matrix or from the Claude JSONL session file?
- Should the brain announce to the operator that it restarted? ("I just restarted — here's what I remember from our last conversation")

## The Layered Memory Model

The brain's "memory" comes from multiple layers, each with different persistence and fidelity:

| Layer | What it stores | Persistence | Fidelity |
|-------|---------------|-------------|----------|
| **System prompt** | Identity, role, team, processes, response format | Loaded every session | Perfect — always current |
| **Context primer** | "Last time we talked about X, you were working on Y" | Fetched from Matrix on startup | Good — last N messages |
| **Session history** | Current conversation (full tool calls, reasoning) | In-memory, lost on restart | Perfect within session |
| **Board state** | What work exists, assignments, statuses | GitHub (queried on demand) | Perfect — source of truth |
| **Team knowledge** | Architecture, conventions, invariants | Files in workspace (CLAUDE.md, knowledge/) | Perfect — always loaded |

The context primer fills the gap between "system prompt" (who I am) and "session history" (current conversation). Without it, the brain knows who it is and what work exists, but not what it was just discussing with the operator.

## Analogy: Human Team Communication

When a human teammate returns to a Slack conversation:
- They **don't** re-read the entire history
- They **do** glance at the last few messages to orient themselves
- They **have** long-term memory of the project, relationships, decisions (= board state, knowledge files)
- They **can** scroll up if they need older context (= ask the operator to remind them)
- They **might** say "sorry, lost track — where were we?" (= natural, acceptable)

The context primer mimics the "glance at last few messages" step. The brain's long-term memory is the board + knowledge files. The "scroll up" equivalent is asking the operator.

## Implementation Sketch

### Where to fetch history

**Option 1: Matrix DM room messages**
- Use `GET /_matrix/client/v3/rooms/{roomId}/messages?dir=b&limit=N`
- Already have the Matrix client in the bridge adapter
- Gets exactly what the operator saw — both directions
- Needs filtering: skip bot's own messages that were just tool narration

**Option 2: Claude JSONL session file**
- Read the last session's JSONL from `.claude/projects/.../`
- Has full conversation including tool calls and reasoning
- More context but also more noise
- Session file might not exist (first start, or cleaned up)

**Recommendation:** Matrix DM messages. It's what the operator sees, it's persistent, and it's already accessible. The JSONL is too detailed for a primer — the operator doesn't see tool calls.

### When to send

After the ACP session is established, before the first heartbeat fires. The multiplexer could have a `primer_sent` flag and send the primer as the very first prompt.

### Priority

The primer should be sent at `Priority::Human` (highest) so it gets processed before any queued heartbeats.

## Not In Scope

- **Infinite session persistence** — too fragile, not worth the complexity
- **Cross-member context sharing** — each member has its own DM, no cross-contamination
- **Automated summarization of old sessions** — premature; let the operator handle it naturally
- **Memory/scratchpad persistence** — separate concern (Ralph's scratchpad model handles this)
