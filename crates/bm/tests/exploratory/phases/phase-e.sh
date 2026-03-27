#!/usr/bin/env bash
# Phase E: Full Sync (-a flag)
# Tests combined bridge + workspace sync, idempotency, new member addition.
set -uo pipefail
source "$LIB"
ensure_gh_token
ensure_keyring

header "Phase E: Full Sync (--bridge flag)"

# Note: -a includes --repos which requires GitHub workspace repos per member.
# For local-only teams, use --bridge (bridge + workspace, no git push).

OUT=$(bm teams sync --bridge -v 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "E1" "Full sync --bridge -v"; else fail "E1" "Full sync" "exit $EC: $(echo "$OUT" | tail -5)"; echo "$OUT"; fi

OUT=$(bm teams sync --bridge -v 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "E2" "Full sync again (idempotent)"; else fail "E2" "Idempotent sync" "exit $EC"; fi

bm hire superman --name dave 2>&1
OUT=$(bm teams sync --bridge -v 2>&1)
EC=$?
if [ $EC -eq 0 ] && [ -f "$TEAM_DIR/superman-dave/.botminter.workspace" ]; then
    pass "E3" "Hire dave + sync creates new workspace"
else
    fail "E3" "Dave workspace" "exit $EC or missing marker"
fi

# Count workspaces
WS_COUNT=$(ls -d "$TEAM_DIR"/superman-*/. 2>/dev/null | wc -l)
if [ "$WS_COUNT" -ge 4 ]; then pass "E4" "All $WS_COUNT member workspaces present"; else fail "E4" "Workspaces" "only $WS_COUNT found"; fi

# Count bridge identities
ID_COUNT=$(jq '.identities | length' "$TEAM_DIR/bridge-state.json" 2>/dev/null || echo "0")
if [ "$ID_COUNT" -ge 5 ]; then pass "E5" "Bridge has $ID_COUNT identities (admin + 4 members)"; else fail "E5" "Identities" "count=$ID_COUNT"; fi

echo "Phase E complete."
