# Exploratory Test Report: Sync & Bridge Idempotency

**Date:** 2026-06-07
**Build:** bm 0.2.0-pre-alpha (1c97853-dirty) (local debug)
**Environment:** Linux x86_64, podman rootless, gh (devguyio)
**Test User:** bm-test-user@localhost (isolated)

## Results

### Phase B: Team Init + Hire

| # | Test | Result |
|---|------|--------|
| B1 | bm init | **FAIL** — exit 1: Error: Directory '/home/bm-test-user/.botminter/workspaces/exploratory-test' already exists. Choose a different team name. |
| B2 | GitHub repo | **FAIL** — not found |
| B3 | Project board | **FAIL** — not found |
| B4 | Labels | **FAIL** — only 0 |
| B5 | Team registered in config.yml | **PASS** |
| B6 | Team repo cloned | **PASS** |
| B7 | Init again | **NOTE** — Correctly rejects: already exists |
| B8 | Hired alice (--reuse-app) | **PASS** |
| B9 | Hired bob (--reuse-app) | **PASS** |
| B10 | Member dirs exist (engineer-alice, engineer-bob) | **PASS** |
| B11 | Hire duplicate alice | **NOTE** — Correctly rejects: 'already exists' |
| B12 | Create project repo | **FAIL** — exit 1: GraphQL: API rate limit already exceeded for user ID 1930204. |
| B13 | Projects add | **FAIL** — exit 1: Error: Project 'exploratory-test-project' already exists in this team. |
| B14 | Project registered in botminter.yml | **PASS** |

### Phase C: Bridge Lifecycle (Tuwunel)

| # | Test | Result |
|---|------|--------|
| C1 | First sync --bridge | **FAIL** — exit 1: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| C2 | Container | **FAIL** — status= |
| C3 | Matrix health | **FAIL** — HTTP 000000 |
| C4 | Bridge state | **FAIL** — status=running ids=0 rooms=0 |
| C5 | Passwords | **FAIL** — count=1 |
| C6 | Keyring | **FAIL** — alice='empty' bob='empty' |
| C7 | Admin login | **FAIL** — no token |
| C8 | Room | **FAIL** — not found |
| C9 | Sync --bridge again | **FAIL** — exit 1 |
| C10 | Container | **FAIL** — status= |
| C11 | State | **FAIL** — status=running ids=0 |
| C12 | Alice credential unchanged after re-sync | **PASS** |
| C13 | Stopped container | **PASS** |
| C14 | Recovery | **FAIL** — exit 1 |
| C15 | Container | **FAIL** — status= |
| C16 | Matrix health | **FAIL** — HTTP 000000 |
| C17 | Force-removed container | **PASS** |
| C18 | Recovery | **FAIL** — exit 1 |
| C19 | Container | **FAIL** — status= |
| C20 | Admin login | **FAIL** — no token after re-create |
| C21 | Removed container + volume | **PASS** |
| C22 | Recovery | **FAIL** — exit 1: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| C23 | Container | **FAIL** — status= |
| C24 | Matrix health | **FAIL** — HTTP 000000 |
| C25 | Admin password regenerated | **PASS** |
| C26 | Keyring | **FAIL** — no credential after volume re-create |
| C27 | Pre-existing registration | **NOTE** — no session returned: {} |
| C28 | Pre-existing sync | **FAIL** — exit 1 |
| C29 | Container | **FAIL** — status= |
| C30 | Identities | **FAIL** — count=0 |
| C31 | Idempotent sync | **FAIL** — exit 1 |
| C32 | Final bridge state: running | **PASS** |
| C33 | Pre-existing keyring | **FAIL** — no credential stored |

### Phase D: Workspace Sync Idempotency

| # | Test | Result |
|---|------|--------|
| D1 | Alice workspace | **FAIL** — missing files |
| D2 | Bob workspace | **FAIL** — missing files |
| D3 | Team submodule | **FAIL** — team/members/ not found |
| D4 | Agent dir | **FAIL** — .claude/agents/ not found |
| D5 | Git repo clean | **PASS** |
| D6 | Git log | **NOTE** —  |
| D7 | Sync | **FAIL** — exit 1 |
| D8 | Context files | **FAIL** — missing after re-sync |
| D9 | Third sync | **FAIL** — exit 1 |
| D10 | Removed .botminter.workspace marker | **PASS** |
| D11 | Recovery | **FAIL** — exit 1: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| D12 | Recovery | **FAIL** — missing files |
| D13 | Team submodule | **FAIL** — missing |
| D14 | Deleted CLAUDE.md from bob workspace | **PASS** |
| D15 | Restore CLAUDE.md | **FAIL** — file still missing or sync failed |
| D16 | Deleted ralph.yml from bob workspace | **PASS** |
| D17 | Restore ralph.yml | **FAIL** — file still missing or sync failed |
| D18 | Created junk dir at future carol workspace path | **PASS** |
| D19 | Hired carol | **PASS** |
| D20 | Workspace creation | **FAIL** — exit 1 |
| D21 | Settings.json | **FAIL** — .claude/settings.json not found in workspace |
| D22 | Inbox write | **FAIL** — exit 1: Error: Not in a BotMinter workspace (no .botminter.workspace found) |
| D23 | Hook exits 0 in workspace (no pending messages) | **PASS** |
| D23b | Hook exits 0 outside workspace | **PASS** |
| D24 | Inbox after sync | **FAIL** — message lost: Error: Not in a BotMinter workspace (no .botminter.workspace found) |

### Phase E: Full Sync (--bridge flag)

| # | Test | Result |
|---|------|--------|
| E1 | Full sync | **FAIL** — exit 1: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| E2 | Idempotent sync | **FAIL** — exit 1 |
| E3 | Dave workspace | **FAIL** — exit 1 or missing marker |
| E4 | Workspaces | **FAIL** — only 1 found |
| E5 | Identities | **FAIL** — count=0 |

### Phase F: Error Handling

| # | Test | Result |
|---|------|--------|
| F1 | Without just | **NOTE** — Output: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| F2 | bm status -v works | **PASS** |
| F3 | members list | **FAIL** — exit 0, count=0 |
| F4 | bm teams show works | **PASS** |

### Phase H: Brain Lifecycle (Chat-First Member)

| # | Test | Result |
|---|------|--------|
| H1 | brain-prompt.md | **FAIL** — missing or empty in /home/bm-test-user/.botminter/workspaces/exploratory-test/superman-alice |
| H2 | No unrendered template variables | **PASS** |
| H3 | Member name | **FAIL** — alice not found in brain-prompt.md |
| H4 | Team name | **FAIL** — exploratory-test not found in brain-prompt.md |
| H5 | GitHub org | **FAIL** — devguyio-bot-squad not found in brain-prompt.md |
| H6 | GitHub repo | **FAIL** — exploratory-test-team not found in brain-prompt.md |
| H7 | Missing sections | **FAIL** —  Identity Board Awareness Work Loop Direct Chat with Operator Dual-Channel |
| H8 | Bob brain-prompt.md | **FAIL** — missing or empty in /home/bm-test-user/.botminter/workspaces/exploratory-test/superman-bob |
| H9 | Alice and bob brain-prompt.md differ (per-member rendering) | **PASS** |
| H10 | Bob content | **FAIL** — expected 'bob' only, got mixed or wrong names |
| H11 | Brain mode detection | **NOTE** — output:  Started 2 member(s), skipped 0 (already running), 0 error(s).  |
| H12 | State file | **NOTE** — brain_mode field not found (start may have failed before writing state) |
| H13 | Without brain-prompt.md: standard launch path (no state written) | **PASS** |
| H14 | Restored brain-prompt.md and cleaned up state | **PASS** |
| H15 | Re-sync restore | **FAIL** — brain-prompt.md not restored from template |
| H16 | Re-sync recreate | **FAIL** — brain-prompt.md not recreated |
| H17 | brain-prompt.md content idempotent across syncs (hash match) | **PASS** |
| H18 | Verbose output | **NOTE** — no brain-related output in sync -v |
| H19 | Tuwunel bridge is running (Matrix server healthy) | **PASS** |
| H20 | ACP binary | **FAIL** — claude-code-acp-rs not found in PATH |
| H21 | Admin Matrix login successful | **PASS** |
| H22 | Alice login | **FAIL** — no access token returned |
| H23 | Cleaned DM room state for discovery test | **PASS** |
| H24 | Cleaned previous state for lifecycle test | **PASS** |
| H25 | bm start executed (brain mode detected) | **PASS** |
| H26 | Brain process | **NOTE** — not alive (ACP may have failed to authenticate) |
| H27 | Brain status | **NOTE** — output: │ 67ce70eb   ┆ engineer-alice ┆ Loop ┆ Killed    ┆ 2026-06-07 16:08:36 ┆ 2h 14m  ┆ 0          │ │ abc83749   ┆ engineer-alice ┆ Loop ┆ Killed    ┆ 2026-06-07 16:08:47 ┆ 2h 13m  ┆ 0          │ ╰────────────┴────────────────┴──────┴───────────┴─────────────────────┴─────────┴────────────╯  |
| H28 | Operator DM created and greeting sent (!En1mSqpaZstV8NMMA4:localhost, $o_wGinVk6NSL-IhrL5HXGjMk41J9wuZL3egZ84YqKho) | **PASS** |
| H28b | DM discovery | **FAIL** — dm-room.json not created within 60s (stderr: ) |
| H29 | Work request sent to room while brain running ($gcilDrA4V8_sIt2ikmyl9_zoAHwbPl-dyUu9J_7hkjQ) | **PASS** |
| H30 | Follow-up question sent (multi-turn simulation) | **PASS** |
| H31 | Malformed message delivered to room (brain not alive to test survival) | **PASS** |
| H32 | Brain response | **FAIL** — brain process not alive, no response |
| H29b | Work request response | **FAIL** — no brain response to evaluate |
| H33 | Message visibility | **FAIL** — greeting=0 task=0 total=0 |
| H34 | DM privacy | **NOTE** — could not login as bob to test |
| H35 | Brain stability | **NOTE** — skipped (brain not alive) |
| H36 | bm stop executed cleanly (exit 0) | **PASS** |
| H37 | All brain processes terminated after stop | **PASS** |
| H38 | Brain restarted successfully (recovery scenario) | **PASS** |
| H39 | Message delivered after brain restart (recovery proof, $uOoY2bO8GwMHmOmpWC3qT3SmQCv7y_xwKx4St62xIHI) | **PASS** |
| H40 | Recovery response | **FAIL** — brain not alive after restart, no response (stderr: no log) |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in DM room history (6 total) | **PASS** |
| H44 | DM persistence | **FAIL** — dm-room.json not found in workspace |
| H46 | GitHub issue creation | **NOTE** — failed to create issue (gh auth may lack permissions) |
| H47 | Task journey start | **NOTE** — brain not alive (ACP auth may have failed, stderr: no log) |
| H48 | Board check request sent to brain ($jUj9mdkQTtB_cZl2OyVX6jaO0lQy9X0OlcLM8Hk8GEQ) | **PASS** |
| H49 | Task response | **FAIL** — brain not alive, no response (stderr: no log) |
| H50 | Brain stability | **NOTE** — skipped (brain not alive at start) |
| H51 | Task execution journey cleaned up | **PASS** |
| H52 | Cleaned up all brain lifecycle test artifacts | **PASS** |

### Phase G: Cleanup

| # | Test | Result |
|---|------|--------|
| G1 | Removed bridge container | **PASS** |
| G2 | Removed bridge volume | **PASS** |
| G3 | Deleted GitHub repo | **PASS** |
| G4 | Deleted GitHub project | **PASS** |
| G5 | Removed local state | **PASS** |
| G6 | Cleared keyring entries | **PASS** |
| G8 | Verified clean: no containers, no repo, no local state | **PASS** |

---

## Summary

- **PASS:** 53
- **FAIL:** 73
- **NOTE:** 15
