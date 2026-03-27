#!/usr/bin/env bash
# Phase D: Workspace Sync Idempotency
# Tests initial workspace state, sync idempotency, stale/missing file recovery,
# junk cleanup, settings.json, and inbox lifecycle.
set -uo pipefail
source "$LIB"
ensure_gh_token
ensure_keyring

header "Phase D: Workspace Sync Idempotency"

ALICE_WS="$TEAM_DIR/superman-alice"
BOB_WS="$TEAM_DIR/superman-bob"

# ── D.1: Verify initial state (workspaces created by phase C) ──

check_ws() {
    local WS="$1"
    local OK=true
    for f in ralph.yml CLAUDE.md PROMPT.md .botminter.workspace; do
        if [ ! -f "$WS/$f" ]; then OK=false; break; fi
    done
    echo "$OK"
}

if [ "$(check_ws "$ALICE_WS")" = "true" ]; then pass "D1" "Alice workspace has all context files"; else fail "D1" "Alice workspace" "missing files"; fi
if [ "$(check_ws "$BOB_WS")" = "true" ]; then pass "D2" "Bob workspace has all context files"; else fail "D2" "Bob workspace" "missing files"; fi

if [ -d "$ALICE_WS/team/members" ]; then pass "D3" "Team submodule present"; else fail "D3" "Team submodule" "team/members/ not found"; fi

if [ -d "$ALICE_WS/.claude/agents" ]; then pass "D4" "Agent dir assembled"; else fail "D4" "Agent dir" ".claude/agents/ not found"; fi

GIT_STATUS=$(git -C "$ALICE_WS" status --porcelain 2>/dev/null)
if [ -z "$GIT_STATUS" ]; then pass "D5" "Git repo clean"; else note "D5" "Git status" "not clean: $GIT_STATUS"; fi

GIT_LOG=$(git -C "$ALICE_WS" log --oneline -1 2>/dev/null)
if echo "$GIT_LOG" | grep -q "Initial workspace setup"; then pass "D6" "Git has initial commit"; else note "D6" "Git log" "$GIT_LOG"; fi

# ── D.2: Sync idempotency ──

echo "  D.2: Sync idempotency..."
OUT=$(bm teams sync -v 2>&1)
if [ $? -eq 0 ]; then pass "D7" "Sync again (no changes)"; else fail "D7" "Sync" "exit $?"; fi

if [ "$(check_ws "$ALICE_WS")" = "true" ]; then pass "D8" "Context files still present after re-sync"; else fail "D8" "Context files" "missing after re-sync"; fi

OUT=$(bm teams sync -v 2>&1)
if [ $? -eq 0 ]; then pass "D9" "Third sync still clean"; else fail "D9" "Third sync" "exit $?"; fi

# ── D.3: Stale workspace recovery ──

echo "  D.3: Stale workspace recovery..."
rm -f "$ALICE_WS/.botminter.workspace"
pass "D10" "Removed .botminter.workspace marker"

OUT=$(bm teams sync -v 2>&1)
if [ $? -eq 0 ]; then pass "D11" "Sync recovers stale workspace"; else fail "D11" "Recovery" "exit $?: $(echo "$OUT" | tail -3)"; fi

if [ "$(check_ws "$ALICE_WS")" = "true" ]; then
    pass "D12" "All context files restored after recovery"
else
    fail "D12" "Recovery" "missing files"
fi

if [ -d "$ALICE_WS/team/members" ]; then pass "D13" "Team submodule intact after recovery"; else fail "D13" "Team submodule" "missing"; fi

# ── D.4: Missing context file recovery ──

echo "  D.4: Missing context file recovery..."
rm -f "$BOB_WS/CLAUDE.md"
pass "D14" "Deleted CLAUDE.md from bob workspace"

OUT=$(bm teams sync -v 2>&1)
if [ $? -eq 0 ] && [ -f "$BOB_WS/CLAUDE.md" ]; then
    pass "D15" "Sync restores CLAUDE.md"
else
    fail "D15" "Restore CLAUDE.md" "file still missing or sync failed"
fi

rm -f "$BOB_WS/ralph.yml"
pass "D16" "Deleted ralph.yml from bob workspace"

OUT=$(bm teams sync -v 2>&1)
if [ $? -eq 0 ] && [ -f "$BOB_WS/ralph.yml" ]; then
    pass "D17" "Sync restores ralph.yml"
else
    fail "D17" "Restore ralph.yml" "file still missing or sync failed"
fi

# ── D.5: Junk directory cleanup ──

echo "  D.5: Junk directory cleanup..."
CAROL_WS="$TEAM_DIR/superman-carol"
mkdir -p "$CAROL_WS"
echo "leftover junk" > "$CAROL_WS/junk.txt"
pass "D18" "Created junk dir at future carol workspace path"

bm_hire superman --name carol 2>&1
pass "D19" "Hired carol"

OUT=$(bm teams sync -v 2>&1)
EC=$?
if [ $EC -eq 0 ] && [ ! -f "$CAROL_WS/junk.txt" ] && [ -f "$CAROL_WS/.botminter.workspace" ]; then
    pass "D20" "Junk cleaned, proper workspace created for carol"
elif [ $EC -eq 0 ] && [ -f "$CAROL_WS/junk.txt" ]; then
    fail "D20" "Junk cleanup" "junk.txt still present"
else
    fail "D20" "Workspace creation" "exit $EC"
fi

# ── D.6: Settings.json & Inbox ──

echo "  D.6: Settings.json & Inbox..."

# D21: Settings.json surfaced after sync
if [ -f "$ALICE_WS/.claude/settings.json" ]; then
    HOOK_CONTENT=$(cat "$ALICE_WS/.claude/settings.json")
    if echo "$HOOK_CONTENT" | grep -q "bm-agent claude hook post-tool-use"; then
        pass "D21" "Settings.json surfaced with PostToolUse hook"
    else
        fail "D21" "Settings.json content" "missing PostToolUse hook reference"
    fi
else
    fail "D21" "Settings.json" ".claude/settings.json not found in workspace"
fi

# D22: Inbox write/peek/read lifecycle
cd "$ALICE_WS"
WRITE_OUT=$(bm_agent inbox write "exploratory test message" --from brain 2>&1)
WRITE_EC=$?
if [ $WRITE_EC -eq 0 ]; then
    PEEK_OUT=$(bm_agent inbox peek 2>&1)
    if echo "$PEEK_OUT" | grep -q "exploratory test message"; then
        READ_OUT=$(bm_agent inbox read --format json 2>&1)
        if echo "$READ_OUT" | grep -q "exploratory test message"; then
            PEEK_AFTER=$(bm_agent inbox peek 2>&1)
            if echo "$PEEK_AFTER" | grep -qi "no pending"; then
                pass "D22" "Inbox write/peek/read lifecycle complete"
            else
                fail "D22" "Inbox consume" "messages still present after read: $PEEK_AFTER"
            fi
        else
            fail "D22" "Inbox read" "message not in JSON output: $READ_OUT"
        fi
    else
        fail "D22" "Inbox peek" "message not visible: $PEEK_OUT"
    fi
else
    fail "D22" "Inbox write" "exit $WRITE_EC: $WRITE_OUT"
fi

# D23: Hook graceful degradation
# In workspace with no inbox file — should exit 0, empty output
HOOK_OUT=$(bm_agent claude hook post-tool-use 2>&1)
HOOK_EC=$?
if [ $HOOK_EC -eq 0 ]; then
    pass "D23" "Hook exits 0 in workspace (no pending messages)"
else
    fail "D23" "Hook in workspace" "exit $HOOK_EC: $HOOK_OUT"
fi
# In non-workspace dir — should also exit 0
cd /tmp
HOOK_OUT2=$(bm_agent claude hook post-tool-use 2>&1)
HOOK_EC2=$?
cd "$ALICE_WS"
if [ $HOOK_EC2 -eq 0 ]; then
    pass "D23b" "Hook exits 0 outside workspace"
else
    fail "D23b" "Hook outside workspace" "exit $HOOK_EC2: $HOOK_OUT2"
fi

# D24: Re-sync preserves inbox messages
bm_agent inbox write "survive sync" --from brain 2>&1
# Navigate to project root for sync, then back
ORIG_DIR=$(pwd)
cd "$CARGO_ROOT"
OUT=$(bm teams sync -v 2>&1)
cd "$ALICE_WS"
PEEK_SYNC=$(bm_agent inbox peek 2>&1)
if echo "$PEEK_SYNC" | grep -q "survive sync"; then
    pass "D24" "Re-sync preserves inbox messages"
    # Clean up
    bm_agent inbox read > /dev/null 2>&1
else
    fail "D24" "Inbox after sync" "message lost: $PEEK_SYNC"
fi

echo "Phase D complete."
