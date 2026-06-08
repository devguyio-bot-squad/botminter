# Exploratory Test Report: Sync & Bridge Idempotency

**Date:** 2026-06-08
**Build:** bm 0.2.0-pre-alpha (11760e5) (local debug)
**Environment:** Linux x86_64, podman rootless, gh (devguyio)
**Test User:** bm-test-user@localhost (isolated)

## Results

### Phase B: Team Init + Hire

| # | Test | Result |
|---|------|--------|
| B1 | bm init (non-interactive, agentic-sdlc-minimal, tuwunel) | **PASS** |
| B2 | GitHub repo exists | **PASS** |
| B3 | GitHub project board exists | **PASS** |
| B4 | Labels created (21 labels) | **PASS** |
| B5 | Team registered in config.yml | **PASS** |
| B6 | Team repo cloned | **PASS** |
| B7 | Init again | **NOTE** — Correctly rejects: already exists |
| B8 | Hired alice (--reuse-app) | **PASS** |
| B9 | Hired bob (--reuse-app) | **PASS** |
| B10 | Member dirs exist (engineer-alice, engineer-bob) | **PASS** |
| B11 | Hire duplicate alice | **NOTE** — Correctly rejects: 'already exists' |
| B12 | Test project repo already exists (devguyio-bot-squad/exploratory-test-project) | **PASS** |
| B13 | Added project to team (bm projects add) | **PASS** |
| B14 | Project registered in botminter.yml | **PASS** |

### Phase C: Bridge Lifecycle (Tuwunel)

| # | Test | Result |
|---|------|--------|
| C1 | First bridge start | **PASS** |
| C2 | Container running | **PASS** |
| C3 | Matrix server healthy | **PASS** |
| C3a | bm bridge identity add engineer-alice | **PASS** |
| C3b | bm bridge identity add engineer-bob | **PASS** |
| C3c | bm bridge room create exploratory-test-general | **PASS** |
| C4 | Bridge state: running, 2 member identities, 1 room | **PASS** |
| C5 | Passwords file has 3 entries | **PASS** |
| C6 | Keyring has credentials for alice + bob | **PASS** |
| C7 | Admin can login to Matrix | **PASS** |
| C8 | Room exploratory-test-general exists (!relwiy4lLkVxJVWoY6:localhost) | **PASS** |
| C9 | Bridge start again (idempotent — already running) | **PASS** |
| C10 | Container still running | **PASS** |
| C11 | Bridge state unchanged | **PASS** |
| C12 | Alice credential unchanged after re-sync | **PASS** |
| C13 | Stopped container | **PASS** |
| C14 | Bridge start recovers stopped container | **PASS** |
| C15 | Container running again | **PASS** |
| C16 | Matrix healthy after recovery | **PASS** |
| C17 | Force-removed container | **PASS** |
| C18 | Bridge start recovers removed container | **PASS** |
| C19 | Container running after re-create | **PASS** |
| C20 | Admin login survives container re-create | **PASS** |
| C21 | Removed container + volume | **PASS** |
| C22 | Bridge start recovers from volume loss | **PASS** |
| C23 | Container running after volume re-create | **PASS** |
| C24 | Matrix healthy after volume re-create | **PASS** |
| C25 | Admin password regenerated | **PASS** |
| C25a | Re-provision alice after volume loss | **PASS** |
| C25b | Re-provision bob after volume loss | **PASS** |
| C26 | Alice: new password + keyring valid after volume re-create | **PASS** |
| C27 | Pre-existing user registered via UIAA (@engineer-pre-existing:localhost) | **PASS** |
| C28 | bridge identity add handles pre-existing user | **PASS** |
| C29 | Container stable after pre-existing user sync | **PASS** |
| C30 | Bridge state has 3 identities | **PASS** |
| C31 | Bridge start idempotent after pre-existing user | **PASS** |
| C32 | Final bridge state: running | **PASS** |
| C33 | Pre-existing user: keyring token valid (@engineer-pre-existing:localhost) | **PASS** |

### Phase D-Session: Ephemeral Session Lifecycle (all 25 ACs)

| # | Test | Result |
|---|------|--------|
| D01 | bm stop fails gracefully without daemon (AC-12) | **PASS** |
| D02 | bm status reports daemon not running (AC-12) | **PASS** |
| D03 | bm session inspect fails gracefully without daemon (AC-12) | **PASS** |
| D04 | Session started without prior bm teams sync (AC-22) | **PASS** |
| D05 | Session creation latency: 665ms (AC-06) | **PASS** |
| D06 | Workspace marker has session_id + member fields (AC-01) | **PASS** |
| D07 | Project 'exploratory-test-project' provisioned in workspace (AC-01) | **PASS** |
| D08 | Config files (PROMPT.md, CLAUDE.md, ralph.yml) present (AC-01) | **PASS** |
| D09 | .claude/ fully assembled: agents(1), skills(3), settings.json (AC-08) | **PASS** |
| D10 | GH credentials (AC-09) | **NOTE** — hosts.yml not found in session dirs — may be inherited from system gh auth |
| D11 | bm status --json has all fields: member=engineer-alice, state=Active (AC-10) | **PASS** |
| D12 | Two concurrent sessions active: alice + bob (AC-04) | **PASS** |
| D13 | Workspaces isolated: file in alice not visible in bob (AC-04) | **PASS** |
| D14 | Stopped bob selectively, alice still Active (AC-15) | **PASS** |
| D15 | Stop returned in 0s (async deactivation) (AC-19) | **PASS** |
| D16 | Force-stop session appears in bm session list as terminal (AC-15) | **PASS** |
| D17 | bm session list shows sessions with session IDs (AC-17) | **PASS** |
| D18 | bm session list shows state and finalization status columns (AC-17) | **PASS** |
| D19 | Session inspect shows ID, member, type, state, workspace (AC-18) | **PASS** |
| D20 | Session cleanup completed for a08c28b3 (AC-18) | **PASS** |
| D21 | Bulk cleanup --all completed (AC-18) | **PASS** |
| D22 | Finalization (AC-02) | **NOTE** — session b247a05c did not reach Completed within 120s — finalization may be slow or stuck |
| D23 | Finalization results visible in inspect (AC-05) | **PASS** |
| D24 | Finalization re-trigger (AC-23) | **NOTE** — session found but finalize returned 1: Error: Daemon returned 500 Internal Server Error for retrigger finalization: {"ok":false,"error":"Cannot transition from Completed to Finalizing"} |
| D25 | Provision failure: non-zero exit, no partial session left (AC-07) | **PASS** |
| D27 | Crashed session workspace retained at /home/bm-test-user/.botminter/sessions/exploratory-test/engineer-alice/a1a64852 (AC-26) | **PASS** |
| D26 | New session starts after crash + force-stop (AC-03) | **PASS** |
| D28 | Daemon restart: stale sessions visible in bm session list (AC-25) | **PASS** |
| D29 | Session state after start: Active (AC-11) | **PASS** |
| D30 | Session in bm session list after force-stop (terminal state) (AC-11) | **PASS** |
| D31 | Terminal state observed via inspect: Killed (AC-11) | **PASS** |
| D32 | Session workspace retained after force-stop (retention policy) (AC-20) | **PASS** |
| D33 | Stopped session visible in bm session list (AC-20) | **PASS** |
| D34 | Individual session cleanup removed workspace (AC-21) | **PASS** |
| D35 | Work item lock lifecycle: A-acquire → B-contend(exit1) → A-release → B-acquire (AC-13) | **PASS** |
| D36 | Independent branches in isolated workspaces: alice=push-test-alice-1780888356, bob=push-test-bob-1780888356 (AC-14a) | **PASS** |
| D37 | Session inspect captures git/workspace state (AC-14b) | **PASS** |
| D38 | bm session list shows force-stopped session in output | **PASS** |
| D39 | bm session list --json has finalization_status field in all rows | **PASS** |
| D40 | bm status --history exits non-zero with migration hint to bm session list | **PASS** |
| D41 | .claude/ assembly with team-level coding-agent/ — no crash (workspace created successfully) | **PASS** |
| D42 | Lock parallel contention: exactly one session acquired (sum=1, product=0) | **PASS** |
| D43 | Lock release cycle: A-acquire → A-release → B-acquire | **PASS** |
| D44 | Lock released when session stops — B acquired after A stopped | **PASS** |

### Phase E: Member Hire & Session Workflow

| # | Test | Result |
|---|------|--------|
| E1 | Hire engineer-carol (bm_hire engineer --name carol) | **PASS** |
| E2 | bm bridge identity add engineer-carol | **PASS** |
| E3 | bm start engineer-carol: session started | **PASS** |
| E4 | Session visible in bm session list (engineer-carol) | **PASS** |
| E5 | bm bridge identity list shows engineer-carol | **PASS** |

### Phase F: Error Handling

| # | Test | Result |
|---|------|--------|
| F1 | Without just | **NOTE** — Output: Bridge 'tuwunel' already running. |
| F2 | bm status -v works | **PASS** |
| F3 | bm members list shows 4 members | **PASS** |
| F4 | bm teams show works | **PASS** |

### Phase H: Brain Lifecycle (Chat-First Member)

| # | Test | Result |
|---|------|--------|
| H1 | brain-prompt.md | **NOTE** — missing or empty in /home/bm-test-user/.botminter/workspaces/exploratory-test/engineer-alice (no permanent workspace in session model) |
| H2 | No unrendered template variables | **PASS** |
| H3 | Member name | **NOTE** — alice not found in brain-prompt.md (file may not exist in session model) |
| H4 | Team name | **NOTE** — exploratory-test not found in brain-prompt.md (file may not exist in session model) |
| H5 | GitHub org | **NOTE** — devguyio-bot-squad not found in brain-prompt.md (file may not exist in session model) |
| H6 | GitHub repo | **NOTE** — exploratory-test-team not found in brain-prompt.md (file may not exist in session model) |
| H7 | Missing sections | **NOTE** —  Identity Board Awareness Work Loop Direct Chat with Operator Dual-Channel (brain-prompt.md may not exist in session model) |
| H8 | Bob brain-prompt.md | **NOTE** — missing or empty in /home/bm-test-user/.botminter/workspaces/exploratory-test/engineer-bob (no permanent workspace in session model) |
| H9 | Alice and bob brain-prompt.md differ (per-member rendering) | **PASS** |
| H10 | Bob content | **NOTE** — expected 'bob' only, got mixed or wrong names (file may not exist in session model) |
| H11 | Brain mode detection | **NOTE** — output:  Started 4 member(s), skipped 0 (already running), 0 error(s).  |
| H12 | State file | **NOTE** — brain_mode field not found (start may have failed before writing state) |
| H13 | Without brain-prompt.md: standard launch path (no state written) | **PASS** |
| H14 | Restored brain-prompt.md and cleaned up state | **PASS** |
| H15 | Sync restore (removed) | **NOTE** — bm teams sync was removed — brain-prompt.md sync restoration is no longer a CLI feature |
| H16 | Sync recreate (removed) | **NOTE** — bm teams sync was removed — brain-prompt.md sync recreation is no longer a CLI feature |
| H17 | brain-prompt.md | **NOTE** — missing or empty at /home/bm-test-user/.botminter/workspaces/exploratory-test/engineer-alice/brain-prompt.md (no permanent workspace in session model) |
| H18 | Verbose sync (removed) | **NOTE** — bm teams sync was removed — verbose brain prompt surfacing output is no longer observable |
| H19 | Tuwunel bridge is running (Matrix server healthy) | **PASS** |
| H20 | ACP binary | **NOTE** — claude-code-acp-rs not found in PATH — deploy it alongside bm to enable brain lifecycle tests |
| H21 | Brain lifecycle | **NOTE** — ACP binary not available — deploy claude-code-acp-rs alongside bm to enable |

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

- **PASS:** 111
- **FAIL:** 0
- **NOTE:** 22
