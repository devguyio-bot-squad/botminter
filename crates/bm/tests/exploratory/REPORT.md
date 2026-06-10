# Exploratory Test Report: Sync & Bridge Idempotency

**Date:** 2026-06-10
**Build:** bm 0.2.0-pre-alpha (cd011f5-dirty) (local debug)
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
| D02 | bm status reports daemon not running (AC-12) | **PASS** |
| D03 | bm session inspect fails gracefully without daemon (AC-12) | **PASS** |
| D04 | Session started without prior bm teams sync (AC-22) | **PASS** |
| D05 | Session creation latency: 680ms (AC-06) | **PASS** |
| D06 | Workspace marker has session_id + member fields (AC-01) | **PASS** |
| D07 | Project 'exploratory-test-project' provisioned in workspace (AC-01) | **PASS** |
| D08 | Config files (PROMPT.md, CLAUDE.md, ralph.yml) present (AC-01) | **PASS** |
| D09 | .claude/ fully assembled: agents(1), skills(3), settings.json (AC-08) | **PASS** |
| D10 | GH credentials (AC-09) | **NOTE** — D-02 credential path absent at /home/bm-test-user/.botminter/sessions/exploratory-test/credentials/engineer-alice/gh — App token provider not wired in run.rs (credential_resolver: None) |
| D11 | bm status --json has all fields: member=engineer-alice, state=Active (AC-10) | **PASS** |
| D12 | Two concurrent sessions active: alice + bob (AC-04) | **PASS** |
| D13 | Workspaces isolated: file in alice not visible in bob (AC-04) | **PASS** |
| D14 | Stopped bob selectively, alice still Active (AC-15) | **PASS** |
| D15 | Stop returned in 0s (async deactivation) (AC-19) | **PASS** |
| D16 | Force-stop session appears in bm session list as terminal (AC-15) | **PASS** |
| D17 | bm session list shows sessions with session IDs (AC-17) | **PASS** |
| D18 | bm session list shows state and finalization status columns (AC-17) | **PASS** |
| D19 | Session inspect shows ID, member, type, state, workspace (AC-18) | **PASS** |
| D20 | Session cleanup completed for 440897c0 (AC-18) | **PASS** |
| D21 | Bulk cleanup --all completed (AC-18) | **PASS** |
| D22 | Finalization (AC-02) | **NOTE** — session ec15b166 did not reach Completed within 120s — finalization may be slow or stuck |
| D23 | Finalization results visible in inspect (AC-05) | **PASS** |
| D24 | Finalization re-trigger (AC-23) | **NOTE** — session found but finalize returned 1: Error: Daemon returned 500 Internal Server Error for retrigger finalization: {"ok":false,"error":"Cannot transition from Completed to Finalizing"} |
| D25 | Provision failure: non-zero exit, no partial session left (AC-07) | **PASS** |
| D27 | Crashed session workspace retained at /home/bm-test-user/.botminter/sessions/exploratory-test/engineer-alice/fc9cc2bb (AC-26) | **PASS** |
| D26 | New session starts after crash + force-stop (AC-03) | **PASS** |
| D28 | Daemon restart: stale sessions visible in bm session list (AC-25) | **PASS** |
| D29 | Session state after start: Killed (AC-11) | **PASS** |
| D30 | Session in bm session list after force-stop (terminal state) (AC-11) | **PASS** |
| D31 | Terminal state observed via inspect: Killed (AC-11) | **PASS** |
| D32 | Session workspace retained after force-stop (retention policy) (AC-20) | **PASS** |
| D33 | Stopped session visible in bm session list (AC-20) | **PASS** |
| D34 | Individual session cleanup removed workspace (AC-21) | **PASS** |
| D35 | Work item lock lifecycle: A-acquire → B-contend(exit1) → A-release → B-acquire (AC-13) | **PASS** |
| D36 | Independent branches in isolated workspaces: alice=push-test-alice-1781089218, bob=push-test-bob-1781089218 (AC-14a) | **PASS** |
| D37 | Session inspect captures git/workspace state (AC-14b) | **PASS** |
| D38 | bm session list shows force-stopped session in output | **PASS** |
| D39 | bm session list --json has finalization_status field in all rows | **PASS** |
| D40 | bm status --history exits non-zero with migration hint to bm session list | **PASS** |
| D41 | .claude/ assembly with team-level coding-agent/ — no crash (workspace created successfully) | **PASS** |
| D42 | Lock parallel contention: exactly one session acquired (sum=1, product=0) | **PASS** |
| D43 | Lock release cycle: A-acquire → A-release → B-acquire | **PASS** |
| D44 | Lock released when session stops — B acquired after A stopped | **PASS** |

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
| H27 | Brain status | **NOTE** — output: │ 701a74a7   ┆ engineer-bob   ┆ Loop ┆ Killed    ┆ 2026-06-10 11:00:17 ┆ 0h 0m   ┆ 0          │ │ b03a5c2f   ┆ engineer-alice ┆ Loop ┆ Completed ┆ 2026-06-10 10:59:33 ┆ 0h 1m   ┆ 0          │ ╰────────────┴────────────────┴──────┴───────────┴─────────────────────┴─────────┴────────────╯  |
| H28 | Operator DM created and greeting sent (!vJioV0pzUdV3RjuzkV:localhost, $8BX3eaQy6rbE35e8hAUlzBGKf87p0qBeNyi-Mw7iAxk) | **PASS** |
| H28b | DM discovery | **FAIL** — dm-room.json not created within 60s (stderr: ) |
| H29 | Work request sent to room while brain running ($c3-qeBzgtmwIae0-Bmaduz6Ly_oeb5Vz4SjMDpszb00) | **PASS** |
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
| H39 | Message delivered after brain restart (recovery proof, $P9dbtjCMoQ_dpqKJHF1ruHbCR1m91eXqf82lDWdyGKM) | **PASS** |
| H40 | Recovery response | **FAIL** — brain not alive after restart, no response (stderr: no log) |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in DM room history (6 total) | **PASS** |
| H44 | DM persistence | **FAIL** — dm-room.json not found in workspace |
| H46 | GitHub issue creation | **NOTE** — failed to create issue (gh auth may lack permissions) |
| H47 | Task journey start | **NOTE** — brain not alive (ACP auth may have failed, stderr: no log) |
| H48 | Board check request sent to brain ($O5noEdQrQaKtSjrVOzP9BlxayjMcMRys2_7NMVTEzss) | **PASS** |
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

- **PASS:** 90
- **FAIL:** 53
- **NOTE:** 17
