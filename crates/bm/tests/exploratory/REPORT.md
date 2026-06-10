# Exploratory Test Report: Sync & Bridge Idempotency

**Date:** 2026-06-10
**Build:** bm 0.2.0-pre-alpha (f2b964a-dirty) (local debug)
**Environment:** Linux x86_64, podman rootless, gh (devguyio)
**Test User:** bm-test-user@localhost (isolated)

## Results

### Phase B: Team Init + Hire

| # | Test | Result |
|---|------|--------|
| B1 | bm init | **FAIL** — exit 1: Error: gh repo create failed: HTTP 401: Requires authentication (https://api.github.com/graphql)
Try authenticating with:  gh auth login

To fix, run manually:
  gh repo create devguyio-bot-squad/exploratory-test-team --private --source . --push |
| B2 | GitHub repo | **FAIL** — not found |
| B3 | GitHub project board exists | **PASS** |
| B4 | Labels | **FAIL** — only 0 |
| B5 | Config | **FAIL** — team not in config.yml |
| B6 | Team repo cloned | **PASS** |
| B7 | Init again | **NOTE** — Correctly rejects: already exists |
| B8 | Hire alice | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| B9 | Hire bob | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| B10 | Member dirs | **FAIL** — missing |
| B11 | Hire duplicate alice | **NOTE** — Correctly rejects: 'already exists' |
| B12 | Test project repo already exists (devguyio-bot-squad/exploratory-test-project) | **PASS** |
| B13 | Projects add | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| B14 | Project config | **FAIL** — not found in botminter.yml |

### Phase C: Bridge Lifecycle (Tuwunel)

| # | Test | Result |
|---|------|--------|
| C1 | First sync --bridge | **FAIL** — exit 1: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| C2 | Container | **FAIL** — status= |
| C3 | Matrix health | **FAIL** — HTTP 000000 |
| C4 | Bridge state | **FAIL** — status= ids= rooms= |
| C5 | Passwords | **FAIL** — count=0 |
| C6 | Keyring | **FAIL** — alice='empty' bob='empty' |
| C7 | Admin login | **FAIL** — no token |
| C8 | Room | **FAIL** — not found |
| C9 | Sync --bridge again | **FAIL** — exit 1 |
| C10 | Container | **FAIL** — status= |
| C11 | State | **FAIL** — status= ids= |
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
| C25 | Password | **FAIL** — no admin password |
| C26 | Keyring | **FAIL** — no credential after volume re-create |
| C27 | Pre-existing registration | **NOTE** — no session returned: {} |
| C28 | Pre-existing sync | **FAIL** — exit 1 |
| C29 | Container | **FAIL** — status= |
| C30 | Identities | **FAIL** — count=0 |
| C31 | Idempotent sync | **FAIL** — exit 1 |
| C32 | Final state | **FAIL** — status= |
| C33 | Pre-existing keyring | **FAIL** — no credential stored |

### Phase D-Session: Ephemeral Session Lifecycle (all 25 ACs)

| # | Test | Result |
|---|------|--------|
| D01 | bm stop fails gracefully without daemon (AC-12) | **PASS** |
| D02 | No-daemon status | **NOTE** — Error: No teams configured. Run `bm init` first. |
| D03 | bm session inspect fails gracefully without daemon (AC-12) | **PASS** |
| D04 | Start session | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| D05 | Session creation latency: 7ms (AC-06) | **PASS** |
| D06 | Workspace path | **FAIL** — not found or empty: '' |
| D07 | Projects dir | **FAIL** — workspace not found |
| D08 | Config files | **FAIL** — workspace not found |
| D09 | Skill dirs | **FAIL** — workspace not found |
| D10 | GH credentials | **FAIL** — workspace not found |
| D11 | Status JSON | **FAIL** — no valid sessions array |
| D12 | Start bob | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| D13 | Workspace isolation | **FAIL** — bob session not started |
| D14 | Stop bob | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| D15 | Stop alice | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| D16 | Force-stop | **FAIL** — exit 1: Error: No teams configured. Run `bm init` first. |
| D17 | Session list | **FAIL** — no session IDs found in bm session list |
| D18 | Session list columns | **NOTE** — expected state/finalization columns — output: Error: No teams configured. Run `bm init` first. |
| D19 | Inspect | **FAIL** — no session ID available |
| D21 | Bulk cleanup | **NOTE** — exit 1: Error: No teams configured. Run `bm init` first. |
| D22 | Start session for finalization | **FAIL** — exit 1 |
| D23 | Finalization results | **FAIL** — session not started |
| D24 | Finalization re-trigger (AC-23) | **NOTE** — session start failed: exit 1 |
| D25 | Error handling | **FAIL** — partial session left after failure |
| D26 | Start for crash test | **FAIL** — exit 1 |
| D27 | Retention | **FAIL** — session not started |
| D28 | Start for daemon test | **FAIL** — exit 1 |
| D29 | Start for state test | **FAIL** — exit 1 |
| D30 | State machine | **FAIL** — session not started |
| D31 | State machine | **FAIL** — session not started |
| D32 | Retention | **NOTE** — workspace not found at '' after stop |
| D33 | Stopped session visible in bm session list (AC-20) | **PASS** |
| D34 | Cleanup | **NOTE** — no session ID to clean up |
| D35 | Work item lock (AC-13) | **FAIL** — failed to start sessions: alice=1, bob=1 |
| D36 | Push test (AC-14a) | **NOTE** — failed to start alice(1) or bob(1) |
| D37 | Push conflict (AC-14b) | **NOTE** — sessions not started |
| D38 | bm session list shows force-stopped session in output | **PASS** |
| D39 | bm session list --json returns valid empty JSON array | **PASS** |
| D40 | bm status --history exits non-zero with migration hint to bm session list | **PASS** |
| D41 | .claude/ assembly | **NOTE** — WS_A= — .claude/ not found (session may have been cleaned up) |
| D42 | Lock parallel contention | **NOTE** — failed to start sessions: alice=1, bob=1 |
| D43 | Lock release cycle | **NOTE** — sessions not started |
| D44 | Lock cleanup on stop | **NOTE** — sessions not started |

### Phase E: Full Sync (--bridge flag)

| # | Test | Result |
|---|------|--------|
| E1 | Full sync | **FAIL** — exit 1: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| E2 | Idempotent sync | **FAIL** — exit 1 |
| E3 | Dave workspace | **FAIL** — exit 1 or missing marker |
| E4 | Workspaces | **FAIL** — only 0 found |
| E5 | Identities | **FAIL** — count=0 |

### Phase F: Error Handling

| # | Test | Result |
|---|------|--------|
| F1 | Without just | **NOTE** — Output: Error: bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed. Run `bm minty` to migrate existing workspaces, or `bm start` to create a new session. |
| F2 | bm status | **FAIL** — exit 1 |
| F3 | members list | **FAIL** — exit 1, count=0 |
| F4 | teams show | **FAIL** — exit 1 |

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
| H11 | Brain mode detection | **NOTE** — output: Error: No teams configured. Run `bm init` first.  |
| H12 | State file | **NOTE** — brain_mode field not found (start may have failed before writing state) |
| H13 | Ralph fallback | **NOTE** — start output: Error: No teams configured. Run `bm init` first.  |
| H14 | Restored brain-prompt.md and cleaned up state | **PASS** |
| H15 | Re-sync restore | **FAIL** — brain-prompt.md not restored from template |
| H16 | Re-sync recreate | **FAIL** — brain-prompt.md not recreated |
| H17 | brain-prompt.md content idempotent across syncs (hash match) | **PASS** |
| H18 | Verbose output | **NOTE** — no brain-related output in sync -v |
| H19 | Bridge prerequisite | **FAIL** — Matrix server not reachable (HTTP 000000) |

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

- **PASS:** 25
- **FAIL:** 80
- **NOTE:** 20
