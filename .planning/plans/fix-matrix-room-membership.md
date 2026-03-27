# Plan: Fix Matrix room membership for brain members

## Context

### Problem
Brain members (chat-first members) never respond to Matrix messages. The brain process starts, creates an ACP session, but zero messages are ever delivered to the multiplexer. H32, H40, and H49 in exploratory tests all fail.

### Investigation trail
1. **Locked binaries / stale state** — exploratory tests couldn't deploy because previous runs left zombie processes holding binaries and port 8008. **Fixed** with `reset` recipe and `clean` bookends (committed: `202e310`).

2. **C5 password count=6** — stale password entries from previous runs. **Fixed** by the `clean` bookend.

3. **Missing Vertex credentials** — test user lacked GCP credentials file and `CLOUD_ML_REGION`. **Fixed** by deploying credentials in `deploy` recipe and adding Claude smoke test to preflight.

4. **Bridge adapter disabled** — when running `bm start` without the `bm()` wrapper (which sets `BM_KEYRING_DBUS`), the keyring lookup fails silently, so `RALPH_MATRIX_ACCESS_TOKEN` is never set on the brain child process. The bridge adapter disables itself. **When using the proper wrapper, the token IS set and bridge adapter IS enabled.**

5. **Root cause: members not joined to room** — even with bridge adapter enabled, alice has zero joined rooms. The bridge reader polls Matrix `/sync` which returns nothing because alice isn't in the room. No `"Injected bridge message"` log ever appears.

6. **Silent curl failures (exit 22)** — the `room-create` recipe used `curl -sf` which exits with code 22 on HTTP errors **without printing the response body**. Combined with `set -e`, every failure was completely silent — no way to diagnose. **Fixed** by replacing all `curl -sf` calls with a reusable `matrix_request()` function that captures HTTP status + body and prints verbose diagnostics.

7. **M_BAD_JSON on createRoom** — once verbose errors were visible, the actual error was `M_BAD_JSON: deserialization failed: leading sigil is incorrect or missing`. The invite list in `room-create` was built from `bridge-state.json` identities without filtering empty `user_id` values. Empty strings (missing `@` sigil) caused the Matrix server to reject the request. **Fixed** by adding `select(startswith("@"))` to the jq filter.

### Root cause gaps (4 found)
1. **`room-create` invite list reads from wrong path** — uses `BRIDGE_CONFIG_DIR` (temp dir) instead of `BM_BRIDGE_STATE_DIR` to find `bridge-state.json`, so invite list is always `[]`. **Fixed** in prior session.
2. **No room-join step after invite** — Matrix requires invited user to call `/join` to accept. No such recipe exists. **Fixed** by brain auto-join (Change 1).
3. **`provision()` in Rust never joins members to rooms** — onboards members and creates rooms but doesn't connect them. **Fixed** by brain auto-join (Change 1) — self-healing at startup.
4. **Brain reader assumes membership** — doesn't attempt to join the room before polling `/sync`. **Fixed** by brain auto-join (Change 1).

### E2E test gap
The E2E tests (`crates/bm/tests/e2e/scenarios/operator_journey.rs`) verify room creation by checking `bridge-state.json` has a rooms array, but **never verify that members are actually joined to the room**. The `bridge_room_create_fn` (line 286) checks `!rooms.is_empty()` but doesn't check any member can see messages. This allowed the room membership bug to ship undetected.

### Ralph Orchestrator precedent
Ralph uses an `ensure_room` pattern (`client.rs:106-118`):
```rust
pub async fn ensure_room(&self, room_id: &str) -> MatrixResult<()> {
    self.sync_once(Duration::from_secs(5)).await?;
    self.join_room(room_id).await?;
    Ok(())
}
```
Called at **every entry point** — daemon startup, loop runner, bot onboarding, interactive messaging. Idempotent: no-op if already joined, accepts pending invite if one exists. No reactive invite handling — just pre-join the configured room.

## Approach: Brain auto-join on startup (following Ralph's pattern) + fix invite path + E2E coverage

**Why auto-join as primary fix:**
- Follows Ralph's proven `ensure_room` pattern
- The room uses `preset: public_chat` — any registered user can join without an invite
- Self-healing: even if provisioning changes or rooms are recreated, the brain always ensures membership
- Minimal code change: one HTTP call in `bridge_adapter.rs` before the `/sync` loop
- No new Justfile recipes needed, no provisioning flow changes

**Why also fix the invite path (Gap 1):**
- Defense-in-depth for non-public rooms
- Bug regardless of the auto-join fix

**Why add E2E coverage:**
- This bug should have been caught by E2E tests
- After `sync --bridge`, E2E should verify a member can actually see messages in the room (not just that `bridge-state.json` has a rooms entry)

## Changes

### 1. `crates/bm/src/brain/bridge_adapter.rs` — auto-join room on reader startup [DONE]

In `MatrixBridgeReader::run()`, before the initial sync call, add a room join call. `join_room()` is idempotent — no-op if already joined, accepts pending invite if one exists.

### 2-3. `profiles/{scrum-compact,scrum}/bridges/tuwunel/Justfile` — `matrix_request()` + invite filter [DONE]

Two fixes applied:
- **`matrix_request()` function** — defined as Just variable `_MATRIX_FN`, interpolated into every recipe via `{{_MATRIX_FN}}`. Replaces all 17 `curl -sf` calls. Captures HTTP status + response body, prints verbose diagnostics on failure.
- **Invite list filter** — `select(startswith("@"))` in the jq filter so invalid user IDs are filtered out instead of causing `M_BAD_JSON`.

### 4. `crates/bm/src/acp/client.rs` — async ACP prompt processing [DONE, PARTIALLY EFFECTIVE]

Uses `cx.spawn()` to run prompt processing as a concurrent task. Replies to the multiplexer immediately so its `tokio::select!` loop stays responsive. `TurnComplete` arrives via the event channel when the LLM responds.

**Status:** Implemented and compiles. First lifecycle (H32) works. Recovery lifecycles (H40, H49) still fail — see "Remaining: Head-of-line blocking" below.

### 5. E2E: Add room membership verification [DONE, NOT YET RUN]

New test case `bridge_room_membership_verify` in `operator_journey.rs`.

### 6. `crates/bm/tests/exploratory/Justfile` — merge `reset` and `clean` [DONE]

## Implementation status

### Completed and validated
- [x] `just build` — compiles clean
- [x] `just clippy` — no warnings
- [x] `just unit` — 822 tests pass, 0 failures
- [x] Change 1 (bridge_adapter.rs auto-join) — **validated**: H23 passes, H28-H35 all pass
- [x] Changes 2-3 (Justfile matrix_request + invite filter) — **validated**: C1-C33, E1-E5, F1-F4 all pass
- [x] Change 4 (ACP async cx.spawn) — **validated**: H32 passes, H40 passes (after stderr drain fix)
- [x] Change 5 (E2E test) — implemented, `just test` not yet run
- [x] Change 6 (merge reset+clean) — **validated**: exploratory test cleanup works
- [x] Change 7 (stderr drain) — **root cause of H40 failure**, validated
- [x] Change 8 (readiness detection fix) — fixes stale `.ralph/` check + log truncation
- [x] Change 9 (chat-first system prompt + per-message reminder) — production responsiveness

### Exploratory test results (2026-03-23)
- **PASS:** 134
- **FAIL:** 1 (H49)
- **NOTE:** 3

H49 is a timeout issue — the LLM takes >300s for complex board analysis with tool use. Not a deadlock.

## Root cause analysis: H40/H49 failures

### Three distinct bugs were found and fixed

#### Bug 1: stderr pipe deadlock in ACP client (PRIMARY — caused H40 and H49 to hang forever)

`AcpClient::spawn()` in `client.rs` piped the ACP agent's stderr but never read from it:
```rust
cmd.stderr(std::process::Stdio::piped()); // piped but never consumed
```

The ACP agent (`claude-code-acp-rs`) writes tracing logs to stderr. When the OS pipe buffer fills (~64KB on Linux), the agent blocks on the next stderr write, deadlocking the entire process. No more prompt processing, no JSON-RPC responses.

**Why H32 works but H40 doesn't:** The first lifecycle processes short messages with minimal tool use — low stderr output, buffer doesn't fill. The second lifecycle involves a fresh Claude CLI startup with verbose initialization (MCP server discovery, tool registration), which fills the buffer.

**Fix:** Spawn a background task to drain stderr:
```rust
if let Some(stderr) = child.stderr.take() {
    tokio::spawn(async move {
        let mut stderr = stderr;
        let mut buf = vec![0u8; 8192];
        loop {
            match stderr.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });
}
```

#### Bug 2: stale readiness detection in exploratory tests (caused messages to arrive too late or too early)

The readiness check in `phase-h.sh` had two problems:
1. **Wrong signal:** Checked for `.ralph/` directory which never exists — brain uses ACP directly, not Ralph. The loop always ran for the full 120s.
2. **Stale log:** After fixing to grep `brain-stderr.log` for "Brain multiplexer session started", the log from the FIRST lifecycle matched immediately without waiting for the second lifecycle's session to actually start.

**Fix:** Grep `brain-stderr.log` for session started message, AND truncate the log before each brain restart:
```bash
: > "$ws/brain-stderr.log" 2>/dev/null || true  # truncate before restart
# Then in readiness loop:
grep -q "Brain multiplexer session started" "$ALICE_WS/brain-stderr.log"
```

#### Bug 3: synchronous `block_task()` in ACP command loop (caused head-of-line blocking)

The ACP client's command loop called `block_task().await` directly, making `client.prompt()` synchronous. While a prompt was processing, the multiplexer's `tokio::select!` loop couldn't receive new messages, process events, or handle shutdown.

**Fix:** `cx.spawn()` per sacp SDK docs — reply immediately, process prompt in concurrent task, send TurnComplete via event channel when done. This keeps the multiplexer responsive.

### Chat-first responsiveness (design improvement)

The brain's primary job is to be a responsive chat partner. Two changes enforce this:

1. **System prompt** (`profiles/scrum-compact/brain/system-prompt.md`): Added "Chat Responsiveness (NON-NEGOTIABLE)" section instructing the brain to always use background execution for autonomous work.

2. **Per-message reminder** (`multiplexer.rs`): `CHAT_FIRST_REMINDER` appended to every prompt sent to ACP, reinforcing the instruction on every turn since system prompts lose salience over time.

### Verification pipeline
- [x] Fix C1 room-create failure (matrix_request + invite filter)
- [x] `just exploratory-test` phases B-G all pass (0 failures)
- [x] Fix stderr pipe deadlock — **H40 passes** (brain responds in ~10s after restart)
- [x] Fix readiness detection — brain ready at check 1 (was check 24)
- [x] Fix head-of-line blocking (cx.spawn) — multiplexer stays responsive
- [x] `just exploratory-test` — **134 PASS, 1 FAIL (H49), 3 NOTE**
- [ ] H49 — LLM takes >300s for board analysis (timeout, not deadlock). Needs chat-first prompt to be effective.
- [ ] `just test` — full suite passes (unit + conformance + e2e)
