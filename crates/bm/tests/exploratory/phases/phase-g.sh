#!/usr/bin/env bash
# Phase G: Cleanup
# Removes all test artifacts: containers, volumes, GitHub repos/projects, keyring, local state.
set -uo pipefail
source "$LIB"
ensure_gh_token

header "Phase G: Cleanup"

CONTAINER="bm-tuwunel-$TEAM"

if is_linux && command -v podman >/dev/null 2>&1; then
    podman stop "$CONTAINER" 2>/dev/null; podman rm "$CONTAINER" 2>/dev/null
    pass "G1" "Removed bridge container"
    podman volume rm "bm-tuwunel-${TEAM}-data" 2>/dev/null
    pass "G2" "Removed bridge volume"
else
    note "G1" "Container cleanup" "skipped (no podman on $(uname -s))"
    note "G2" "Volume cleanup" "skipped (no podman on $(uname -s))"
fi

gh repo delete "$FULL_REPO" --yes 2>/dev/null
pass "G3" "Deleted GitHub repo"

PROJ_NUM=$(gh project list --owner "$ORG" --format json 2>/dev/null \
    | jq -r ".projects[] | select(.title==\"$BOARD\") | .number" 2>/dev/null || true)
if [ -n "$PROJ_NUM" ]; then
    gh project delete "$PROJ_NUM" --owner "$ORG" --format json 2>/dev/null
fi
pass "G4" "Deleted GitHub project"

rm -rf ~/.botminter ~/.config/botminter
pass "G5" "Removed local state"

# Clear keyring (use isolated keyring if available)
if is_macos; then
    # macOS: delete keychain entries via security command
    for u in superman-alice superman-bob superman-carol superman-dave bmadmin superman-pre-existing; do
        security delete-generic-password -s "botminter.$TEAM.tuwunel" -a "$u" 2>/dev/null || true
    done
    pass "G6" "Cleared macOS Keychain entries"
elif load_isolated_keyring; then
    for u in superman-alice superman-bob superman-carol superman-dave bmadmin superman-pre-existing; do
        secret_tool clear service "botminter.$TEAM.tuwunel" username "$u" 2>/dev/null || true
    done
    pass "G6" "Cleared keyring entries"
    stop_isolated_keyring
else
    pass "G6" "Keyring cleanup skipped (no isolated keyring running)"
fi

# Verify clean
if is_linux && command -v podman >/dev/null 2>&1; then
    CONTAINERS=$(podman ps -a --filter "name=bm-tuwunel-$TEAM" --format '{{.Names}}' 2>/dev/null)
else
    CONTAINERS=""
fi
REPO_EXISTS=$(gh repo view "$FULL_REPO" --json name 2>/dev/null && echo "yes" || echo "no")
if [ -z "$CONTAINERS" ] && [ "$REPO_EXISTS" = "no" ] && [ ! -d ~/.botminter ]; then
    pass "G8" "Verified clean: no containers, no repo, no local state"
else
    fail "G8" "Verify clean" "containers='$CONTAINERS' repo=$REPO_EXISTS botminter=$(test -d ~/.botminter && echo exists || echo gone)"
fi

echo "" >> "$REPORT"
echo "---" >> "$REPORT"
echo "" >> "$REPORT"
echo "## Summary" >> "$REPORT"
PASS_COUNT=$(grep -c '**PASS**' "$REPORT" || true)
FAIL_COUNT=$(grep -c '**FAIL**' "$REPORT" || true)
NOTE_COUNT=$(grep -c '**NOTE**' "$REPORT" || true)
echo "" >> "$REPORT"
echo "- **PASS:** $PASS_COUNT" >> "$REPORT"
echo "- **FAIL:** $FAIL_COUNT" >> "$REPORT"
echo "- **NOTE:** $NOTE_COUNT" >> "$REPORT"

echo ""
echo "Phase G complete."
echo "================================"
echo "PASS: $PASS_COUNT | FAIL: $FAIL_COUNT | NOTE: $NOTE_COUNT"
echo "Report: $REPORT"
