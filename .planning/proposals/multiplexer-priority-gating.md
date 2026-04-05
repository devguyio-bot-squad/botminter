# Proposal: Multiplexer Priority Gating with ACP Queue Delegation

**Date:** 2026-03-26
**Context:** Brain follow-up investigation — discovered the TS ACP supports native prompt queueing (`promptQueueing: true`). Current multiplexer does classification + queueing + serialization. This proposal separates concerns.

## Current Architecture

```
Human msg ──┐                                    ┌──► ACP (one prompt at a time)
Loop event ─┤──► Multiplexer ──► PromptQueue ───►│
Heartbeat ──┘    (priority sort)  (BinaryHeap)   └──► waits for TurnComplete, then drains next
```

The multiplexer tracks `prompt_in_flight`, holds all messages in a priority queue, and drains one at a time on `TurnComplete`. This works but couples priority logic with serialization.

## Proposed Architecture

```
Human msg ──┐                                         ┌──► ACP (queues internally, processes FIFO)
Loop event ─┤──► Multiplexer ──► Priority Gate ───────►│
Heartbeat ──┘    (classify)      (hold lower priority) └──► pendingMessages handles serialization
```

**Multiplexer becomes a priority gate:**

1. **Classify** incoming messages by priority tier (Human > LoopEvent > Heartbeat)
2. **Forward immediately** to ACP if no higher-priority messages are pending
3. **Hold lower-priority messages** while higher-priority ones are queued at the ACP
4. **Release** lower-priority messages once higher-priority tier drains

**Rules:**

| Higher-priority pending at ACP? | Incoming message | Action |
|------|------|--------|
| No | Any | Forward to ACP immediately |
| Yes (human) | Human | Forward to ACP (same tier) |
| Yes (human) | LoopEvent / Heartbeat | Hold locally until human tier drains |
| Yes (loop) | Human | Forward to ACP (higher tier) |
| Yes (loop) | LoopEvent | Forward to ACP (same tier) |
| Yes (loop) | Heartbeat | Hold locally |

**Configurable limits:**
- Max prompts queued at ACP per tier (e.g., 3 human, 2 loop, 1 heartbeat)
- Max age before dropping stale messages (e.g., 1-2 min configurable)
- Heartbeat deduplication: only one pending heartbeat at a time (existing `HeartbeatPending` flag)

## What Changes

**Remove from multiplexer:**
- `PromptQueue` (BinaryHeap) — ACP handles FIFO within each tier
- `prompt_in_flight` flag — ACP handles serialization
- `TurnComplete` drain logic — no need to wait and pop

**Add to multiplexer:**
- Per-tier pending count (how many of each priority are queued at ACP)
- Gate logic: hold lower tiers while higher tiers have pending prompts
- `TurnComplete` still received — decrement pending count for the completed tier

**Keep in multiplexer:**
- Message classification (Human / LoopEvent / Heartbeat)
- Bridge reader/writer routing
- Heartbeat deduplication

## Benefits

- **Simpler state management** — no manual queue drain, no prompt_in_flight tracking
- **Natural preemption** — human messages flow through even when loop events are processing
- **Better responsiveness** — multiple human messages can queue at ACP without waiting for each TurnComplete
- **Separation of concerns** — classification (multiplexer) vs serialization (ACP)

## Prerequisites

- TS ACP (`claude-agent-acp`) with `promptQueueing: true` capability
- `sacp` 11+ for protocol compatibility
- Multiplexer needs to detect ACP queueing capability (graceful fallback to current approach if ACP doesn't support it)

## Risks

- **ACP dependency** — relies on TS ACP's specific queueing behavior; if behavior changes or we switch ACPs, need fallback
- **Observability** — queue state split between multiplexer (held messages) and ACP (pending prompts); harder to debug
- **Message ordering across tiers** — a held loop event sent after human messages drain may reference stale state

## Status

Proposal — not yet implemented. Current priority is landing the sacp 11 + TS ACP migration.
