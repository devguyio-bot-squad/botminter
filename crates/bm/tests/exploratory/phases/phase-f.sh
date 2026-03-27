#!/usr/bin/env bash
# Phase F: Error Handling
# Tests graceful degradation without just, CLI display commands.
# Does NOT use keyring directly.
set -uo pipefail
source "$LIB"
ensure_gh_token
ensure_keyring

header "Phase F: Error Handling"

# F1: Without just
OUT=$(PATH=/usr/bin:/bin bm teams sync --bridge -v 2>&1)
# Should not crash — either skips or errors gracefully
if echo "$OUT" | grep -qi "just\|skip\|not found"; then
    pass "F1" "Graceful handling when just not in PATH"
else
    note "F1" "Without just" "Output: $(echo "$OUT" | tail -2)"
fi

# F2: bm status
OUT=$(bm status -v 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "F2" "bm status -v works"; else fail "F2" "bm status" "exit $EC"; fi

# F3: bm members list
OUT=$(bm members list 2>&1)
EC=$?
MEMBER_COUNT=$(echo "$OUT" | grep -c "superman-" || true)
if [ $EC -eq 0 ] && [ "$MEMBER_COUNT" -ge 3 ]; then pass "F3" "bm members list shows $MEMBER_COUNT members"; else fail "F3" "members list" "exit $EC, count=$MEMBER_COUNT"; fi

# F4: bm teams show
OUT=$(bm teams show 2>&1)
EC=$?
if [ $EC -eq 0 ] && echo "$OUT" | grep -q "$TEAM"; then pass "F4" "bm teams show works"; else fail "F4" "teams show" "exit $EC"; fi

echo "Phase F complete."
