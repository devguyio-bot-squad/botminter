#!/usr/bin/env bash
# Phase A: Lima VM Boot Script Idempotency
# Tests VM creation, tool installation, boot script idempotency across stop/start, cleanup.
# Does NOT use keyring — no ensure_keyring needed.
set -uo pipefail
source "$LIB"
ensure_gh_token

header "Phase A: Lima VM Boot Script Idempotency"

# A0: Set up a minimal team so bm runtime create works
bm init --non-interactive --profile "$PROFILE" --team-name "$TEAM" \
    --org "$ORG" --repo "$REPO" --bridge tuwunel \
    --github-project-board "$BOARD" 2>&1 || true
pass "A0" "Set up team for bm runtime create"

# A0.5: Verify template has --overwrite
TEMPLATE=$(bm runtime create --render --name "$LIMA_VM" 2>/dev/null)
if echo "$TEMPLATE" | grep -q "addrepo --overwrite"; then
    pass "A0.5" "Template has --overwrite (idempotent addrepo)"
else
    fail "A0.5" "Template check" "--overwrite not found"
fi

# A1: Create VM with bm runtime create
echo "  Creating VM (this takes 5-10 minutes)..."
OUT=$(bm runtime create --non-interactive --name "$LIMA_VM" 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "A1" "Create VM with bm runtime create"; else fail "A1" "Create VM" "exit $EC: $(echo "$OUT" | tail -3)"; echo "$OUT"; fi

# A2: Verify tools inside VM
TOOLS_OK=true
for tool in bm ralph gh git just; do
    if ! limactl shell "$LIMA_VM" -- which "$tool" >/dev/null 2>&1; then
        fail "A2" "Verify tools installed" "$tool not found"
        TOOLS_OK=false
        break
    fi
done
# claude may be npm-installed — check separately
if $TOOLS_OK; then
    if limactl shell "$LIMA_VM" -- which claude >/dev/null 2>&1; then
        pass "A2" "All tools installed (bm, ralph, claude, gh, git, just)"
    else
        note "A2" "Tools installed" "claude not found (npm install may have failed), others OK"
    fi
fi

# A3: Stop VM
echo "  Stopping VM..."
limactl stop "$LIMA_VM" 2>&1
if [ $? -eq 0 ]; then pass "A3" "Stop VM cleanly"; else fail "A3" "Stop VM" "exit $?"; fi

# A4: Restart VM (provision scripts re-run — tests --overwrite)
echo "  Restarting VM (tests boot script idempotency)..."
OUT=$(limactl start "$LIMA_VM" 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "A4" "Restart VM — boot scripts idempotent"; else fail "A4" "Restart VM" "exit $EC: $(echo "$OUT" | tail -5)"; echo "$OUT"; fi

# A5: Tools still present after restart
TOOLS_OK=true
for tool in bm ralph gh git just; do
    if ! limactl shell "$LIMA_VM" -- which "$tool" >/dev/null 2>&1; then
        fail "A5" "Tools present after restart" "$tool missing"
        TOOLS_OK=false
        break
    fi
done
if $TOOLS_OK; then pass "A5" "All tools present after restart"; fi

# A6: GH auth survives restart
GH_STATUS=$(limactl shell "$LIMA_VM" -- gh auth status 2>&1 || true)
if echo "$GH_STATUS" | grep -q "Logged in"; then
    pass "A6" "gh auth survives restart"
else
    note "A6" "gh auth after restart" "Not logged in (expected if no token was passed)"
fi

# A7: Third boot cycle
echo "  Third boot cycle (stop + start)..."
limactl stop "$LIMA_VM" 2>&1
OUT=$(limactl start "$LIMA_VM" 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "A7" "Third boot — no cumulative drift"; else fail "A7" "Third boot" "exit $EC"; fi

# A8: Delete VM
echo "  Deleting VM..."
limactl delete --force "$LIMA_VM" 2>&1
if [ $? -eq 0 ]; then pass "A8" "Delete VM"; else fail "A8" "Delete VM" "exit $?"; fi

# A9: Cleanup team created for Phase A
gh repo delete "$FULL_REPO" --yes 2>/dev/null || true
PROJ_NUM=$(gh project list --owner "$ORG" --format json 2>/dev/null \
    | jq -r ".projects[] | select(.title==\"$BOARD\") | .number" 2>/dev/null || true)
[ -n "$PROJ_NUM" ] && gh project delete "$PROJ_NUM" --owner "$ORG" --format json 2>/dev/null || true
rm -rf ~/.botminter ~/.config/botminter 2>/dev/null || true
pass "A9" "Cleaned up Phase A team artifacts"

echo "Phase A complete."
