#!/usr/bin/env bash
# Phase B: Team Init + Hire
# Tests bm init, GitHub repo/project/labels verification, bm hire, idempotency rejection.
set -uo pipefail
source "$LIB"
ensure_gh_token
ensure_keyring

header "Phase B: Team Init + Hire"

# B1: Init team
OUT=$(bm init --non-interactive --profile "$PROFILE" --team-name "$TEAM" \
    --org "$ORG" --repo "$REPO" --bridge tuwunel \
    --github-project-board "$BOARD" 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "B1" "bm init (non-interactive, scrum-compact, tuwunel)"; else fail "B1" "bm init" "exit $EC: $(echo "$OUT" | tail -5)"; echo "$OUT"; fi

# B2: GitHub repo exists
if gh repo view "$FULL_REPO" --json name >/dev/null 2>&1; then pass "B2" "GitHub repo exists"; else fail "B2" "GitHub repo" "not found"; fi

# B3: GitHub project board exists
PROJ=$(gh project list --owner "$ORG" --format json 2>/dev/null | jq -r ".projects[] | select(.title==\"$BOARD\") | .title" 2>/dev/null || true)
if [ "$PROJ" = "$BOARD" ]; then pass "B3" "GitHub project board exists"; else fail "B3" "Project board" "not found"; fi

# B4: Labels created
LABEL_COUNT=$(gh label list -R "$FULL_REPO" --json name --jq 'length' 2>/dev/null || echo "0")
if [ "$LABEL_COUNT" -ge 4 ]; then pass "B4" "Labels created ($LABEL_COUNT labels)"; else fail "B4" "Labels" "only $LABEL_COUNT"; fi

# B5: Team registered in config
if [ -f "$HOME/.botminter/config.yml" ] && grep -q "$TEAM" "$HOME/.botminter/config.yml" 2>/dev/null; then
    pass "B5" "Team registered in config.yml"
else
    fail "B5" "Config" "team not in config.yml"
fi

# B6: Team repo cloned
if [ -d "$TEAM_REPO/.git" ]; then pass "B6" "Team repo cloned"; else fail "B6" "Team repo" "not cloned at $TEAM_REPO"; fi

# B7: Init again (should detect existing)
OUT=$(bm init --non-interactive --profile "$PROFILE" --team-name "$TEAM" \
    --org "$ORG" --repo "$REPO" --bridge tuwunel \
    --github-project-board "$BOARD" 2>&1)
EC=$?
if [ $EC -ne 0 ]; then note "B7" "Init again" "Correctly rejects: already exists"; else pass "B7" "Init again (idempotent or re-init)"; fi

# B8: Hire alice (with --reuse-app via bm_hire wrapper)
OUT=$(bm_hire superman --name alice 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "B8" "Hired alice (--reuse-app)"; else fail "B8" "Hire alice" "exit $EC: $(echo "$OUT" | tail -3)"; fi

# B9: Hire bob (with --reuse-app via bm_hire wrapper)
OUT=$(bm_hire superman --name bob 2>&1)
EC=$?
if [ $EC -eq 0 ]; then pass "B9" "Hired bob (--reuse-app)"; else fail "B9" "Hire bob" "exit $EC: $(echo "$OUT" | tail -3)"; fi

# B10: Member dirs exist
if [ -d "$TEAM_REPO/members/superman-alice" ] && [ -d "$TEAM_REPO/members/superman-bob" ]; then
    pass "B10" "Member dirs exist (superman-alice, superman-bob)"
else
    fail "B10" "Member dirs" "missing"
fi

# B11: Hire duplicate without --reuse-app (should fail because member dir exists)
OUT=$(bm hire superman --name alice -t "$TEAM" 2>&1)
EC=$?
if [ $EC -ne 0 ]; then note "B11" "Hire duplicate alice" "Correctly rejects: 'already exists'"; else fail "B11" "Hire duplicate" "Should have failed"; fi

echo "Phase B complete."
