# Milestone: Brain Bridge I/O

**Status**: Not started
**Date**: 2026-03-22
**Depends on**: Chat-First Member (complete), Bridge Lifecycle (complete)

## Problem

The brain multiplexer (`brain_run.rs`) has no bridge I/O. The multiplexer channels exist and work (`MultiplexerInput` / `MultiplexerOutput`), but nothing connects them to the Matrix bridge:

```
Heartbeat (60s) ──┐
                   ├──► Multiplexer ──► ACP/Claude ──► _output (DROPPED)
EventWatcher ──────┘

Matrix room ──► [MISSING] ──► input
_output ──► [MISSING] ──► Matrix room
```

The original chat-first-member PROMPT planned `bridge_input.rs` and `BridgeInput` trait (Phase 2), but they were never implemented. All 6 phases were marked complete without closing this loop.

## Goal

Wire the brain to the Matrix bridge so that:
1. Human messages in the team's Matrix room are received and routed to the ACP session as `BrainMessage::human()`
2. ACP responses stream back to the Matrix room as messages from the member's identity
3. The brain is a real chat participant — not a closed loop processing only heartbeats

## Architecture

### What exists

| Component | Location | Status |
|-----------|----------|--------|
| `MultiplexerInput` | `brain/multiplexer.rs` | Ready — accepts `BrainMessage` via `mpsc::Sender` |
| `MultiplexerOutput` | `brain/multiplexer.rs` | Ready — emits `BridgeOutput` via `mpsc::Receiver` |
| `BrainMessage::human()` | `brain/types.rs` | Ready — P0 priority, `[Human on bridge]: <msg>` prompt format |
| `BridgeOutput::Text` | `brain/types.rs` | Ready — streaming text chunks from ACP |
| `BridgeOutput::TurnComplete` | `brain/types.rs` | Ready — signals end of response |
| Matrix access token | `launch_brain()` → `RALPH_MATRIX_ACCESS_TOKEN` env var | Passed to brain process |
| Matrix homeserver URL | `launch_brain()` → `RALPH_MATRIX_HOMESERVER_URL` env var | Passed to brain process |
| Room ID | `bridge-state.json` → `rooms[0].room_id`, also in `ralph.yml` → `RObot.matrix.room_id` | Available but not passed to brain |
| Member user ID | `ralph.yml` → `RObot.matrix.bot_user_id` | Available but not passed to brain |

### What needs to be built

```
Matrix room ──► MatrixBridgeAdapter (poll /sync) ──► MultiplexerInput
MultiplexerOutput ──► MatrixBridgeAdapter (PUT /send) ──► Matrix room
```

A single `MatrixBridgeAdapter` component that:
- **Reads**: Polls the Matrix room for new `m.room.message` events using the member's access token. Filters out messages from the member's own user ID (to avoid echo loops). Sends each new message as `BrainMessage::human_from(body, sender)` into the multiplexer.
- **Writes**: Reads `BridgeOutput` events from the multiplexer output channel. Accumulates `BridgeOutput::Text` chunks. On `BridgeOutput::TurnComplete`, sends the accumulated text as a single `m.room.message` to the room.

### Matrix Client-Server API surface needed

Only 2 endpoints:

| Endpoint | Purpose | Auth |
|----------|---------|------|
| `GET /_matrix/client/v3/sync` | Long-poll for new room events | `Authorization: Bearer <token>` |
| `PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message/{txnId}` | Send message to room | `Authorization: Bearer <token>` |

The `/sync` endpoint returns events since a `since` token (initially empty for first sync). The response includes a `next_batch` token for subsequent requests. Filter to `m.room.message` events in the target room. Use `timeout=30000` for long-polling (30s).

No external crate needed — `reqwest` is already a dev dependency; promote it to a regular dependency, or use raw `tokio` + `hyper`. Given the simplicity (2 endpoints, JSON parsing), raw HTTP via `reqwest` is the right choice.

## Design Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| How to pass room ID to brain? | New env var `BM_BRAIN_ROOM_ID` set by `launch_brain()` | Consistent with existing env var pattern for bridge config. Read from `bridge-state.json` at launch time. |
| How to pass member user ID? | New env var `BM_BRAIN_USER_ID` set by `launch_brain()` | Needed to filter out own messages from /sync |
| How to get room ID at launch? | `start_members.rs` reads `bridge-state.json` via existing `Bridge::default_room_id()` | Already available in the launch context |
| HTTP client? | `reqwest` (async) | Already a dev-dependency, minimal API surface needed, async-native |
| Sync strategy? | Long-poll `/sync` with `timeout=30000`, `since` token tracking | Standard Matrix pattern, no extra deps, responsive enough |
| Message batching? | Accumulate `Text` chunks, send on `TurnComplete` | Avoids flooding the room with partial responses |
| Echo suppression? | Filter out messages where `sender == own_user_id` | Standard Matrix bot pattern |
| Error handling? | Log and retry on transient errors, shutdown on auth failure | Brain should be resilient to brief network issues |
| What about non-Matrix bridges? | Bridge adapter is behind a trait; Matrix is the first impl | Future: Telegram, Rocket.Chat adapters |

## Phased Implementation Plan

### Phase 1: Pass bridge config to brain process

**Goal:** `launch_brain()` passes room ID and member user ID as env vars. `collect_env_vars()` forwards them to the ACP process (not needed for bridge adapter, but keeps the env consistent).

**What to build:**
- Modify `start_members.rs`: resolve `default_room_id()` and `member_user_id()` from bridge state, pass as new params to `launch_brain()`
- Modify `launch_brain()` in `launch.rs`: set `BM_BRAIN_ROOM_ID` and `BM_BRAIN_USER_ID` env vars on the child process
- Modify `collect_env_vars()` in `brain_run.rs`: add `BM_BRAIN_ROOM_ID`, `BM_BRAIN_USER_ID` to the allowlist

**Acceptance criteria:**
- [ ] `launch_brain()` sets `BM_BRAIN_ROOM_ID` when bridge has a room
- [ ] `launch_brain()` sets `BM_BRAIN_USER_ID` when bridge has a member user ID
- [ ] `brain_run.rs` can read these env vars at startup
- [ ] Existing unit tests still pass
- [ ] No behavioral change when env vars are absent (graceful degradation)

### Phase 2: Matrix bridge adapter — reader (room → multiplexer)

**Goal:** Poll Matrix for new messages and inject them into the multiplexer.

**What to build:**
- `crates/bm/src/brain/bridge_adapter.rs`:
  - `MatrixBridgeConfig` struct: `homeserver_url`, `access_token`, `room_id`, `own_user_id`
  - `MatrixBridgeReader` struct with async `run()` method
  - Long-poll `/sync` with `since` token, `timeout=30000`, room filter
  - Extract `m.room.message` events, filter out own messages
  - Send each as `BrainMessage::human_from(body, sender_display_name)` into `mpsc::Sender<BrainMessage>`
  - Shutdown via `mpsc::Receiver<()>` (same pattern as EventWatcher)
- Add `reqwest` as a regular dependency (move from dev-dependencies)
- Wire into `brain_run.rs`: spawn reader task alongside event watcher and heartbeat, pass `MultiplexerInput.sender()`

**Acceptance criteria:**
- [ ] Reader polls Matrix `/sync` and receives new messages
- [ ] Own messages are filtered out (no echo loop)
- [ ] Messages arrive in the multiplexer as P0 human priority
- [ ] Reader handles transient HTTP errors (retry with backoff)
- [ ] Reader stops cleanly on shutdown signal
- [ ] Unit tests with mock HTTP responses
- [ ] Integration test: send message to Matrix room → appears in multiplexer input

### Phase 3: Matrix bridge adapter — writer (multiplexer → room)

**Goal:** Send ACP responses back to the Matrix room.

**What to build:**
- `MatrixBridgeWriter` struct in `bridge_adapter.rs`:
  - Async `run()` method that reads from `MultiplexerOutput`
  - Accumulates `BridgeOutput::Text` chunks into a buffer
  - On `BridgeOutput::TurnComplete`, sends accumulated text as `m.room.message` via `PUT /rooms/{roomId}/send/m.room.message/{txnId}`
  - On `BridgeOutput::Error`, sends error as a message (prefixed with indicator)
  - Transaction ID: UUID per message for idempotency
  - Shutdown when output channel closes (multiplexer dropped)
- Wire into `brain_run.rs`: spawn writer task, pass `MultiplexerOutput`

**Acceptance criteria:**
- [ ] ACP text responses are sent to the Matrix room
- [ ] Streaming chunks are accumulated into a single message (not one per chunk)
- [ ] Error messages are sent with a distinguishing prefix
- [ ] Transaction IDs prevent duplicate sends
- [ ] Writer handles transient HTTP errors (retry)
- [ ] Writer stops cleanly when multiplexer shuts down
- [ ] Unit tests with mock HTTP
- [ ] Integration test: prompt multiplexer → response appears in Matrix room

### Phase 4: End-to-end validation

**Goal:** The brain is a real chat participant. Exploratory tests H40 and H49 pass.

**What to build:**
- Update `brain_run.rs` to wire reader + writer only when bridge env vars are present (graceful degradation for non-bridge teams)
- Run `just test` — all unit + e2e tests pass
- Run `just exploratory-test` — H40 and H49 pass (brain responds to Matrix messages after restart and to board check requests)
- Update `phase-acp-isolated.sh` to validate the full round-trip

**Acceptance criteria:**
- [ ] `just test` passes (all unit + conformance + e2e)
- [ ] `just exploratory-test` passes with 0 FAIL
- [ ] Brain responds to Matrix messages sent by admin
- [ ] Brain responses appear in room history from the member's identity
- [ ] Stop + restart cycle works (H40 scenario)
- [ ] Board check request gets a meaningful response (H49 scenario)
- [ ] Non-bridge teams still work (brain runs without bridge adapter)

## Existing Code That Will Be Modified

| File | Change |
|------|--------|
| `crates/bm/src/formation/launch.rs` | Add `room_id` and `user_id` params to `launch_brain()`, set env vars |
| `crates/bm/src/formation/start_members.rs` | Resolve room ID and user ID from bridge state, pass to `launch_brain()` |
| `crates/bm/src/commands/brain_run.rs` | Read bridge env vars, spawn reader + writer tasks, wire to multiplexer |
| `crates/bm/src/brain/mod.rs` | Export new `bridge_adapter` module |
| `crates/bm/Cargo.toml` | Move `reqwest` from dev-dependencies to dependencies |

## New Code

| Path | Purpose |
|------|---------|
| `crates/bm/src/brain/bridge_adapter.rs` | Matrix bridge reader + writer |

## Prior Work (already done)

Two preparatory changes were made during the investigation that diagnosed this gap:

### 1. Tracing subscriber for brain-run

`brain_run.rs` had no `tracing_subscriber` initialized, so all `tracing::info!()` / `tracing::error!()` calls in the multiplexer, ACP client, and heartbeat were silently dropped. The brain's `brain-stderr.log` was always empty, making debugging impossible.

**Fix applied:** Added `tracing-subscriber` dependency to `Cargo.toml` and initialized a stderr subscriber at the top of `brain_run::run()`. Now `brain-stderr.log` captures all diagnostic output (ACP spawn, session creation, errors, shutdown).

**Files changed:**
- `crates/bm/Cargo.toml` — added `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
- `crates/bm/src/commands/brain_run.rs` — added `tracing_subscriber::fmt()` initialization

### 2. Isolated ACP diagnostic test

An isolated test script (`phase-acp-isolated.sh`) was created to diagnose the H40/H49 failures outside the full exploratory suite. It proved that the brain failure is not restart-specific — even the first start fails to send Matrix messages. The test:
1. Starts brain for one member (alice)
2. Sends a Matrix message, polls for response (60s)
3. Stops brain, kills all processes
4. Restarts brain (the H40 scenario)
5. Sends another message, polls for response (90s)
6. Captures `brain-stderr.log` at each stage

**Key finding:** Both starts show the brain successfully creating an ACP session (`session_id=...` in logs) but never sending anything to Matrix — confirming the missing bridge I/O gap.

**Files added:**
- `crates/bm/tests/exploratory/phases/phase-acp-isolated.sh`
- `crates/bm/tests/exploratory/Justfile` — added `phase-acp-isolated` recipe

## Validation Infrastructure

### Isolated ACP test (`phase-acp-isolated.sh`)

A standalone exploratory test at `crates/bm/tests/exploratory/phases/phase-acp-isolated.sh` that validates brain ↔ Matrix round-trip independently from the full suite. Run via:

```bash
just -f crates/bm/tests/exploratory/Justfile phase-acp-isolated
```

Requires phases B-E to have run first (team + bridge + workspace setup). The test:

1. Verifies prerequisites (bridge up, workspace exists, admin auth, room resolved)
2. Kills all lingering brain/ACP/claude processes
3. **Test 1 — First start:** starts `bm start superman-alice`, sends a Matrix message as admin, polls 60s for a brain response from `superman-*` identity, captures `brain-stderr.log`
4. Stops brain, kills all processes
5. **Test 2 — Restart (H40 scenario):** cleans all ACP/Claude state, restarts brain, sends another message, polls 90s, captures stderr log and process tree

**Current output (before bridge adapter):** Both tests FAIL — brain creates ACP session successfully but never sends to Matrix:

```
brain-stderr.log:
  INFO bm::commands::brain_run: Brain multiplexer starting ...
  INFO bm::brain::heartbeat: Heartbeat timer started interval_secs=60
  INFO bm::brain::multiplexer: Brain multiplexer session started session_id=...
```

**Expected output (after bridge adapter):** Both tests PASS — brain responds to Matrix messages within the polling window.

The test also validates the tracing fix by checking that `brain-stderr.log` contains diagnostic output (previously always empty).

### Full exploratory test suite

The full suite (`just exploratory-test`) runs phases B-H + G (cleanup). The specific tests that validate bridge I/O:

| Test | What it validates | Current | Expected |
|------|-------------------|---------|----------|
| H32 | Brain responds to messages sent while running | PASS (false positive — coincidental Claude Bash activity) | PASS (real bridge response) |
| H40 | Brain responds after stop/restart cycle | FAIL | PASS |
| H49 | Brain responds to board check request | FAIL | PASS |

## Constraints

- **No changes to the multiplexer** — the channel API is correct as-is
- **No changes to the ACP client** — the brain's prompt processing is correct
- **No changes to the bridge plugin** — bridge provisioning and lifecycle are separate concerns
- **Bridge adapter is optional** — brain still works without it (heartbeat + event watcher only), just without chat
- **Tuwunel/Matrix only for now** — other bridges (Telegram, Rocket.Chat) can add their own adapters later
- **reqwest is the HTTP client** — no need for a full Matrix SDK; we need exactly 2 endpoints
