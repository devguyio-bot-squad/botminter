#!/usr/bin/env bash
# Phase S: Session Model Scenarios
# Tests session lifecycle, concurrency, recovery, and garbage collection.
# Prerequisite: Phases B-E must run first (team init + hire + bridge + workspace).
set -uo pipefail
source "$LIB"
ensure_gh_token
ensure_keyring

header "Phase S: Session Model"

# ── S.1: Session create + verify ──

echo "  S.1: Session create + verify..."

OUT=$(bm start superman-alice 2>&1)
EC=$?
if [ $EC -eq 0 ]; then
    STATUS_JSON=$(bm status --json 2>&1)
    if echo "$STATUS_JSON" | jq -e '.sessions[] | select(.member == "superman-alice" and .state == "Active")' >/dev/null 2>&1; then
        pass "S1" "Start alice creates active session"
    else
        fail "S1" "Session not active" "no Active session for alice in status output"
    fi
else
    fail "S1" "Start alice" "exit $EC: $(echo "$OUT" | tail -3)"
fi

# ── S.2: Concurrent sessions ──

echo "  S.2: Concurrent sessions..."

OUT=$(bm start superman-bob 2>&1)
EC=$?
if [ $EC -eq 0 ]; then
    STATUS_JSON=$(bm status --json 2>&1)
    ALICE_ACTIVE=$(echo "$STATUS_JSON" | jq -e '.sessions[] | select(.member == "superman-alice" and .state == "Active")' 2>/dev/null || true)
    BOB_ACTIVE=$(echo "$STATUS_JSON" | jq -e '.sessions[] | select(.member == "superman-bob" and .state == "Active")' 2>/dev/null || true)
    if [ -n "$ALICE_ACTIVE" ] && [ -n "$BOB_ACTIVE" ]; then
        pass "S2" "Concurrent sessions for alice + bob"
    else
        fail "S2" "Concurrent sessions" "alice=${ALICE_ACTIVE:+active}${ALICE_ACTIVE:-inactive} bob=${BOB_ACTIVE:+active}${BOB_ACTIVE:-inactive}"
    fi
else
    fail "S2" "Start bob (concurrent)" "exit $EC"
fi

# ── S.3: Kill agent → session Failed ──

echo "  S.3: Kill agent recovery..."

ALICE_PID=$(bm status --json 2>&1 | jq -r '.sessions[] | select(.member == "superman-alice") | .pid // empty' 2>/dev/null)
if [ -n "$ALICE_PID" ]; then
    kill "$ALICE_PID" 2>/dev/null || true
    sleep 3
    STATUS_JSON=$(bm status --json 2>&1)
    ALICE_STATE=$(echo "$STATUS_JSON" | jq -r '[.sessions[] | select(.member == "superman-alice")] | last | .state // empty' 2>/dev/null)
    if [ "$ALICE_STATE" = "Failed" ]; then
        pass "S3" "Kill agent → session Failed"
    else
        fail "S3" "Kill recovery" "expected Failed, got '$ALICE_STATE'"
    fi
else
    fail "S3" "Kill agent" "could not get alice PID from status --json"
fi

# ── S.4: Graceful stop → session completed ──

echo "  S.4: Graceful stop..."

OUT=$(bm stop superman-bob 2>&1)
EC=$?
STATUS_JSON=$(bm status --json 2>&1)
BOB_STATE=$(echo "$STATUS_JSON" | jq -r '[.sessions[] | select(.member == "superman-bob")] | last | .state // empty' 2>/dev/null)
if [ "$BOB_STATE" = "Completed" ] || [ "$BOB_STATE" = "Stopped" ]; then
    pass "S4" "Graceful stop → session $BOB_STATE"
else
    fail "S4" "Graceful stop" "expected Completed/Stopped, got '$BOB_STATE' (exit $EC)"
fi

# ── S.5: GC limits session history ──

echo "  S.5: Session GC..."

for i in 1 2 3; do
    bm start superman-alice 2>&1 >/dev/null || true
    bm stop superman-alice 2>&1 >/dev/null || true
done
STATUS_JSON=$(bm status --json 2>&1)
ALICE_SESSIONS=$(echo "$STATUS_JSON" | jq '[.sessions[] | select(.member == "superman-alice")] | length' 2>/dev/null || echo "0")
if [ "$ALICE_SESSIONS" -le 3 ]; then
    pass "S5" "GC limits session history (count=$ALICE_SESSIONS)"
else
    fail "S5" "GC" "expected ≤3 retained sessions, found $ALICE_SESSIONS"
fi

# ── S.6: Daemon restart recovery ──

echo "  S.6: Daemon restart recovery..."

bm start superman-alice 2>&1 >/dev/null || true
DAEMON_PID=$(bm status --json 2>&1 | jq -r '.daemon.pid // empty' 2>/dev/null)
if [ -n "$DAEMON_PID" ]; then
    kill "$DAEMON_PID" 2>/dev/null || true
    sleep 2
    OUT=$(bm start superman-bob 2>&1)
    EC=$?
    if [ $EC -eq 0 ]; then
        STATUS_JSON=$(bm status --json 2>&1)
        ALICE_FAILED=$(echo "$STATUS_JSON" | jq -e '.sessions[] | select(.member == "superman-alice" and .state == "Failed")' 2>/dev/null || true)
        BOB_ACTIVE=$(echo "$STATUS_JSON" | jq -e '.sessions[] | select(.member == "superman-bob" and .state == "Active")' 2>/dev/null || true)
        if [ -n "$ALICE_FAILED" ] && [ -n "$BOB_ACTIVE" ]; then
            pass "S6" "Daemon restart: stale→Failed, new→Active"
        else
            fail "S6" "Daemon restart" "alice_failed=${ALICE_FAILED:+yes}${ALICE_FAILED:-no} bob_active=${BOB_ACTIVE:+yes}${BOB_ACTIVE:-no}"
        fi
    else
        fail "S6" "Daemon restart" "bm start bob after daemon kill failed: exit $EC"
    fi
else
    note "S6" "Daemon restart" "no daemon PID in status --json — skipping"
fi

# ── Cleanup ──

bm stop --force 2>&1 || true

echo "Phase S complete."
