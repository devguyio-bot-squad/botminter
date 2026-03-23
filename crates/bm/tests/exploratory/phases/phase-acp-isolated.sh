#!/usr/bin/env bash
# Isolated ACP session test — diagnose why brain-run doesn't respond after restart.
#
# This test:
# 1. Sets up a minimal team + workspace with bridge
# 2. Starts brain-run for ONE member
# 3. Sends a Matrix message
# 4. Polls for response
# 5. Stops brain-run
# 6. Starts brain-run AGAIN (the restart that fails in H40)
# 7. Sends another Matrix message
# 8. Polls for response
# 9. Captures brain-stderr.log for diagnosis
#
# Run: ssh bm-test-user@localhost 'bash -l -c "source ~/.bm-exploratory-tests/env.sh && exec ~/.bm-exploratory-tests/phases/phase-acp-isolated.sh"'
set -uo pipefail
source "$LIB"
ensure_gh_token
ensure_keyring

echo "=== Isolated ACP Session Test ==="
echo ""

TEAM_NAME="$TEAM"
ALICE_WS="$TEAM_DIR/superman-alice"
STATE_FILE="$HOME/.botminter/state.json"
MATRIX_URL="http://127.0.0.1:${TUWUNEL_PORT:-8008}"
PWFILE="$TEAM_DIR/tuwunel-passwords.json"

# ── Prerequisites ──
echo "--- Prerequisites ---"

# Verify bridge is up
HTTP=$(curl -sf -o /dev/null -w "%{http_code}" "$MATRIX_URL/_matrix/client/versions" 2>/dev/null || echo "000")
if [ "$HTTP" != "200" ]; then
    echo "SKIP: Bridge not running (run full exploratory-test first to set up team)"
    exit 0
fi
echo "  Bridge: OK"

# Verify workspace exists
if [ ! -d "$ALICE_WS" ]; then
    echo "SKIP: Alice workspace not found (run full exploratory-test first)"
    exit 0
fi
echo "  Workspace: OK"

# Get Matrix auth
ADMIN_PASS=$(jq -r '.bmadmin' "$PWFILE" 2>/dev/null)
ADMIN_LOGIN=$(curl -sf -X POST -H "Content-Type: application/json" \
    -d "{\"type\":\"m.login.password\",\"identifier\":{\"type\":\"m.id.user\",\"user\":\"bmadmin\"},\"password\":\"$ADMIN_PASS\"}" \
    "$MATRIX_URL/_matrix/client/v3/login" 2>/dev/null || echo '{}')
ADMIN_TOKEN=$(echo "$ADMIN_LOGIN" | jq -r '.access_token // empty')
if [ -z "$ADMIN_TOKEN" ]; then
    echo "SKIP: Cannot login as admin"
    exit 0
fi
echo "  Admin auth: OK"

# Get room
ROOM_ALIAS="%23${TEAM_NAME}-general:localhost"
ROOM_RESP=$(curl -sf "$MATRIX_URL/_matrix/client/v3/directory/room/$ROOM_ALIAS" 2>/dev/null || echo '{}')
ROOM_ID=$(echo "$ROOM_RESP" | jq -r '.room_id // empty')
if [ -z "$ROOM_ID" ]; then
    echo "SKIP: Room not found"
    exit 0
fi
echo "  Room: $ROOM_ID"

# ── Kill ALL lingering processes ──
echo ""
echo "--- Cleanup stale processes ---"
for pid in $(ps aux | grep '[b]rain-run' | awk '{print $2}'); do kill "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude-code-acp-rs' | awk '{print $2}'); do kill "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude.*stream-json' | awk '{print $2}'); do kill "$pid" 2>/dev/null || true; done
sleep 3
for pid in $(ps aux | grep '[b]rain-run' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude-code-acp-rs' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude.*stream-json' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
sleep 2
echo "  Stale processes killed"

# Clean ACP/Claude state
rm -f "$STATE_FILE"
for ws in "$TEAM_DIR"/superman-*/; do
    rm -rf "$ws/.ralph" "$ws/.claude" "$ws/.claude-code-acp" "$ws/.cache" 2>/dev/null || true
done
rm -rf "$HOME/.cache/claude-cli-nodejs" "$HOME/.local/state/claude" "$HOME/.claude" 2>/dev/null || true
echo "  State cleaned"

# ── Test 1: First brain start + message + response ──
echo ""
echo "=== Test 1: First brain start ==="

# Record initial brain message count
INITIAL_HIST=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$MATRIX_URL/_matrix/client/v3/rooms/$ROOM_ID/messages?dir=b&limit=50" 2>/dev/null || echo '{}')
INITIAL_BRAIN_COUNT=$(echo "$INITIAL_HIST" | jq '[.chunk[] | select(.type == "m.room.message" and (.sender // "" | test("superman-")))] | length' 2>/dev/null || echo "0")
echo "  Initial brain message count: $INITIAL_BRAIN_COUNT"

# Start brain
bm teams sync --bridge 2>&1 >/dev/null || true
START_OUT=$(bm start superman-alice 2>&1 || true)
echo "  Start output: $(echo "$START_OUT" | tail -2 | tr '\n' ' ')"

# Wait for brain to be alive
sleep 5
BRAIN_PID=""
if [ -f "$STATE_FILE" ]; then
    BRAIN_PID=$(jq -r '.members // {} | to_entries[] | select(.value.brain_mode == true) | .value.pid' "$STATE_FILE" 2>/dev/null | head -1)
fi

if [ -n "$BRAIN_PID" ] && kill -0 "$BRAIN_PID" 2>/dev/null; then
    echo "  Brain alive: PID $BRAIN_PID"
    echo "  Process: $(cat /proc/$BRAIN_PID/cmdline 2>/dev/null | tr '\0' ' ' | head -c 100)"
else
    echo "  FAIL: Brain not alive (PID=$BRAIN_PID)"
    echo "  brain-stderr.log:"
    cat "$ALICE_WS/brain-stderr.log" 2>/dev/null || echo "  (no log)"
    exit 1
fi

# Send message
MSG_TXN="acp-test1-$(date +%s)"
SEND_RESP=$(curl -sf -X PUT \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"msgtype":"m.text","body":"ACP isolated test 1: Hello brain!"}' \
    "$MATRIX_URL/_matrix/client/v3/rooms/$ROOM_ID/send/m.room.message/$MSG_TXN" 2>/dev/null || echo '{}')
echo "  Message sent: $(echo "$SEND_RESP" | jq -r '.event_id // "FAILED"')"

# Poll for response (60s)
echo "  Polling for brain response (up to 60s)..."
RESPONDED=false
for attempt in $(seq 1 12); do
    sleep 5
    if ! kill -0 "$BRAIN_PID" 2>/dev/null; then
        echo "  Brain died at attempt $attempt"
        echo "  brain-stderr.log:"
        tail -20 "$ALICE_WS/brain-stderr.log" 2>/dev/null || echo "  (no log)"
        break
    fi
    HIST=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$MATRIX_URL/_matrix/client/v3/rooms/$ROOM_ID/messages?dir=b&limit=50" 2>/dev/null || echo '{}')
    BRAIN_MSGS=$(echo "$HIST" | jq '[.chunk[] | select(.type == "m.room.message" and (.sender // "" | test("superman-")))] | length' 2>/dev/null || echo "0")
    if [ "${BRAIN_MSGS:-0}" -gt "${INITIAL_BRAIN_COUNT:-0}" ]; then
        BODY=$(echo "$HIST" | jq -r '[.chunk[] | select(.type == "m.room.message" and (.sender // "" | test("superman-")))] | .[0].content.body // "empty"' 2>/dev/null)
        echo "  PASS: Brain responded! (body: $(echo "$BODY" | head -c 80)...)"
        RESPONDED=true
        break
    fi
    echo "    attempt $attempt/12: no response yet (msgs: $BRAIN_MSGS, initial: $INITIAL_BRAIN_COUNT)"
done
if ! $RESPONDED; then
    echo "  FAIL: No brain response within 60s"
fi

# Show brain stderr log
echo ""
echo "  --- brain-stderr.log (Test 1) ---"
tail -30 "$ALICE_WS/brain-stderr.log" 2>/dev/null || echo "  (empty)"
echo "  --- end ---"

# ── Stop brain ──
echo ""
echo "=== Stopping brain ==="
bm stop --force 2>/dev/null || true
sleep 2
# Kill ALL processes
for pid in $(ps aux | grep '[b]rain-run' | awk '{print $2}'); do kill "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude-code-acp-rs' | awk '{print $2}'); do kill "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude.*stream-json' | awk '{print $2}'); do kill "$pid" 2>/dev/null || true; done
sleep 3
for pid in $(ps aux | grep '[b]rain-run' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude-code-acp-rs' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude.*stream-json' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
sleep 2
echo "  All processes killed"

# Record pre-restart brain count
PRE_RESTART_HIST=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$MATRIX_URL/_matrix/client/v3/rooms/$ROOM_ID/messages?dir=b&limit=50" 2>/dev/null || echo '{}')
PRE_RESTART_BRAIN_COUNT=$(echo "$PRE_RESTART_HIST" | jq '[.chunk[] | select(.type == "m.room.message" and (.sender // "" | test("superman-")))] | length' 2>/dev/null || echo "0")
echo "  Pre-restart brain message count: $PRE_RESTART_BRAIN_COUNT"

# ── Test 2: Restart brain + message + response (the failing scenario) ──
echo ""
echo "=== Test 2: Brain RESTART (this is what fails in H40/H49) ==="

# Clean state for restart
rm -f "$STATE_FILE"
for ws in "$TEAM_DIR"/superman-*/; do
    rm -rf "$ws/.ralph" "$ws/.claude" "$ws/.claude-code-acp" "$ws/.cache" 2>/dev/null || true
done
rm -rf "$HOME/.cache/claude-cli-nodejs" "$HOME/.local/state/claude" "$HOME/.claude" 2>/dev/null || true

# Refresh bridge credentials
bm teams sync --bridge 2>&1 >/dev/null || true

# Start brain
START2_OUT=$(bm start superman-alice 2>&1 || true)
echo "  Start output: $(echo "$START2_OUT" | tail -2 | tr '\n' ' ')"

# Wait for brain to be alive
sleep 5
BRAIN_PID2=""
if [ -f "$STATE_FILE" ]; then
    BRAIN_PID2=$(jq -r '.members // {} | to_entries[] | select(.value.brain_mode == true) | .value.pid' "$STATE_FILE" 2>/dev/null | head -1)
fi

if [ -n "$BRAIN_PID2" ] && kill -0 "$BRAIN_PID2" 2>/dev/null; then
    echo "  Brain alive: PID $BRAIN_PID2"
    echo "  Process: $(cat /proc/$BRAIN_PID2/cmdline 2>/dev/null | tr '\0' ' ' | head -c 100)"
    echo "  Children: $(ps --ppid "$BRAIN_PID2" -o pid,comm 2>/dev/null | tail -n +2 | tr '\n' ' ')"
else
    echo "  FAIL: Brain not alive after restart (PID=$BRAIN_PID2)"
    echo "  brain-stderr.log:"
    cat "$ALICE_WS/brain-stderr.log" 2>/dev/null || echo "  (no log)"
    # Still continue to see the log
fi

# Wait longer for ACP session establishment
echo "  Waiting for ACP session (checking .ralph dir, up to 120s)..."
for check in $(seq 1 24); do
    if [ -n "$BRAIN_PID2" ] && ! kill -0 "$BRAIN_PID2" 2>/dev/null; then
        echo "  Brain died at check $check"
        echo "  brain-stderr.log (last 30 lines):"
        tail -30 "$ALICE_WS/brain-stderr.log" 2>/dev/null || echo "  (no log)"
        break
    fi
    if [ -d "$ALICE_WS/.ralph" ]; then
        echo "  ACP session ready at check $check (.ralph dir exists)"
        break
    fi
    echo "    check $check/24: .ralph not yet"
    sleep 5
done

# Send message
MSG_TXN="acp-test2-$(date +%s)"
SEND_RESP2=$(curl -sf -X PUT \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"msgtype":"m.text","body":"ACP isolated test 2: Restart test - are you operational?"}' \
    "$MATRIX_URL/_matrix/client/v3/rooms/$ROOM_ID/send/m.room.message/$MSG_TXN" 2>/dev/null || echo '{}')
echo "  Message sent: $(echo "$SEND_RESP2" | jq -r '.event_id // "FAILED"')"

# Poll for response (90s)
echo "  Polling for brain response after restart (up to 90s)..."
RESPONDED2=false
for attempt in $(seq 1 18); do
    sleep 5
    if [ -n "$BRAIN_PID2" ] && ! kill -0 "$BRAIN_PID2" 2>/dev/null; then
        echo "  Brain died at attempt $attempt"
        break
    fi
    HIST2=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$MATRIX_URL/_matrix/client/v3/rooms/$ROOM_ID/messages?dir=b&limit=50" 2>/dev/null || echo '{}')
    BRAIN_MSGS2=$(echo "$HIST2" | jq '[.chunk[] | select(.type == "m.room.message" and (.sender // "" | test("superman-")))] | length' 2>/dev/null || echo "0")
    if [ "${BRAIN_MSGS2:-0}" -gt "${PRE_RESTART_BRAIN_COUNT:-0}" ]; then
        BODY2=$(echo "$HIST2" | jq -r '[.chunk[] | select(.type == "m.room.message" and (.sender // "" | test("superman-")))] | .[0].content.body // "empty"' 2>/dev/null)
        echo "  PASS: Brain responded after restart! (body: $(echo "$BODY2" | head -c 80)...)"
        RESPONDED2=true
        break
    fi
    echo "    attempt $attempt/18: no response yet (msgs: $BRAIN_MSGS2, pre: $PRE_RESTART_BRAIN_COUNT)"
done
if ! $RESPONDED2; then
    echo "  FAIL: No brain response within 90s after restart"
fi

# Show brain stderr log
echo ""
echo "  --- brain-stderr.log (Test 2 - RESTART) ---"
cat "$ALICE_WS/brain-stderr.log" 2>/dev/null || echo "  (empty)"
echo "  --- end ---"

# Show child process tree
echo ""
echo "  --- Process tree ---"
if [ -n "$BRAIN_PID2" ] && kill -0 "$BRAIN_PID2" 2>/dev/null; then
    ps --forest -p "$BRAIN_PID2" $(ps --ppid "$BRAIN_PID2" -o pid= 2>/dev/null) 2>/dev/null || true
fi
echo "  All claude-related processes:"
ps aux | grep -E '[c]laude|[b]rain-run|[c]laude-code-acp' | head -10

# ── Cleanup ──
echo ""
echo "=== Cleanup ==="
bm stop --force 2>/dev/null || true
for pid in $(ps aux | grep '[b]rain-run' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude-code-acp-rs' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
for pid in $(ps aux | grep '[c]laude.*stream-json' | awk '{print $2}'); do kill -9 "$pid" 2>/dev/null || true; done
rm -f "$STATE_FILE"
echo "  Done"
