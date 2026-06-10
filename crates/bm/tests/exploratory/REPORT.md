# Exploratory Test Report: Sync & Bridge Idempotency

**Date:** 2026-06-10
**Build:** bm 0.2.0-pre-alpha (4d9763c-dirty) (local debug)
**Environment:** Linux x86_64, podman rootless, gh (devguyio)
**Test User:** bm-test-user@localhost (isolated)

## Results

### Phase B: Team Init + Hire

| # | Test | Result |
|---|------|--------|
| B1 | bm init | **FAIL** — exit 1: Error: Directory '/home/bm-test-user/.botminter/workspaces/exploratory-test' already exists. Choose a different team name. |
| B2 | GitHub repo exists | **PASS** |
| B3 | Project board | **FAIL** — not found |
| B4 | Labels created (17 labels) | **PASS** |
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
| D05 | Session creation latency: 793ms (AC-06) | **PASS** |
| D06 | Workspace marker has session_id + member fields (AC-01) | **PASS** |
| D07 | Project 'exploratory-test-project' provisioned in workspace (AC-01) | **PASS** |
| D08 | Config files (PROMPT.md, CLAUDE.md, ralph.yml) present (AC-01) | **PASS** |
| D09 | .claude/ fully assembled: agents(1), skills(3), settings.json (AC-08) | **PASS** |
| D10 | GH credentials (AC-09) | **NOTE** — D-02 credential path absent at /home/bm-test-user/.botminter/sessions/exploratory-test/credentials/engineer-alice/gh — App token provider not wired in run.rs (credential_resolver: None) |
| D11 | bm status --json has all fields: member=engineer-alice, state=Active (AC-10) | **PASS** |
| D12 | Two concurrent sessions active: alice + bob (AC-04) | **PASS** |
| D13 | Workspaces isolated: file in alice not visible in bob (AC-04) | **PASS** |
| B1 | bm init (non-interactive, agentic-sdlc-minimal, tuwunel) | **PASS** |
| B2 | GitHub repo exists | **PASS** |
| D14 | Stopped bob selectively, alice still Active (AC-15) | **PASS** |
| D15 | Stop returned in 0s (async deactivation) (AC-19) | **PASS** |
| B3 | GitHub project board exists | **PASS** |
| B4 | Labels created (22 labels) | **PASS** |
| B5 | Team registered in config.yml | **PASS** |
| B6 | Team repo cloned | **PASS** |
| B7 | Init again | **NOTE** — Correctly rejects: already exists |
| B8 | Hired alice (--reuse-app) | **PASS** |
| B9 | Hired bob (--reuse-app) | **PASS** |
| B10 | Member dirs exist (engineer-alice, engineer-bob) | **PASS** |
| B11 | Hire duplicate alice | **NOTE** — Correctly rejects: 'already exists' |
| B12 | Test project repo already exists (devguyio-bot-squad/exploratory-test-project) | **PASS** |
| B13 | Projects add | **FAIL** — exit 1: Error: Project 'exploratory-test-project' already exists in this team. |
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
| D16 | Force-stop session appears in bm session list as terminal (AC-15) | **PASS** |
| C33 | Pre-existing keyring | **FAIL** — no credential stored |
| D17 | bm session list shows sessions with session IDs (AC-17) | **PASS** |
| D18 | bm session list shows state and finalization status columns (AC-17) | **PASS** |
| D19 | Session inspect shows ID, member, type, state, workspace (AC-18) | **PASS** |
| D20 | Session cleanup completed for 70e2129f (AC-18) | **PASS** |
| D21 | Bulk cleanup --all completed (AC-18) | **PASS** |

### Phase D-Session: Ephemeral Session Lifecycle (all 25 ACs)

| # | Test | Result |
|---|------|--------|
| D01 | bm stop fails gracefully without daemon (AC-12) | **PASS** |
| D02 | bm status reports daemon not running (AC-12) | **PASS** |
| D03 | bm session inspect fails gracefully without daemon (AC-12) | **PASS** |
| D04 | Session started without prior bm teams sync (AC-22) | **PASS** |
| D05 | Session creation latency: 171ms (AC-06) | **PASS** |
| D06 | Workspace marker has session_id + member fields (AC-01) | **PASS** |
| D07 | Project 'exploratory-test-project' provisioned in workspace (AC-01) | **PASS** |
| D08 | Config files (PROMPT.md, CLAUDE.md, ralph.yml) present (AC-01) | **PASS** |
| D09 | .claude/ fully assembled: agents(1), skills(3), settings.json (AC-08) | **PASS** |
| D10 | GH credentials (AC-09) | **NOTE** — D-02 credential path absent at /home/bm-test-user/.botminter/sessions/exploratory-test/credentials/engineer-alice/gh — App token provider not wired in run.rs (credential_resolver: None) |
| D11 | bm status --json has all fields: member=engineer-bob, state=Completed (AC-10) | **PASS** |
| D12 | Two concurrent sessions active: alice + bob (AC-04) | **PASS** |
| D13 | Workspaces isolated: file in alice not visible in bob (AC-04) | **PASS** |
| D14 | Stopped bob selectively, alice still Active (AC-15) | **PASS** |
| D15 | Stop returned in 0s (async deactivation) (AC-19) | **PASS** |
| D16 | Force-stop session appears in bm session list as terminal (AC-15) | **PASS** |
| D17 | bm session list shows sessions with session IDs (AC-17) | **PASS** |
| D18 | bm session list shows state and finalization status columns (AC-17) | **PASS** |
| D19 | Session inspect shows ID, member, type, state, workspace (AC-18) | **PASS** |
| D20 | Session cleanup completed for dd16aac1 (AC-18) | **PASS** |
| D21 | Bulk cleanup --all completed (AC-18) | **PASS** |
| D22 | Finalization (AC-02) | **NOTE** — session be2eec2b did not reach Completed within 120s — finalization may be slow or stuck |
| D23 | Finalization results visible in inspect (AC-05) | **PASS** |
| D24 | Finalization re-trigger (AC-23) | **NOTE** — session found but finalize returned 1: Error: Daemon returned 500 Internal Server Error for retrigger finalization: {"ok":false,"error":"Cannot transition from Completed to Finalizing"} |
| D25 | Provision failure: non-zero exit, no partial session left (AC-07) | **PASS** |
| D27 | Crashed session workspace retained at /home/bm-test-user/.botminter/sessions/exploratory-test/engineer-alice/13c666ae (AC-26) | **PASS** |
| D22 | Finalization (AC-02) | **NOTE** — session 3519b68e did not reach Completed within 120s — finalization may be slow or stuck |
| D23 | Finalization results visible in inspect (AC-05) | **PASS** |
| D26 | Crash recovery | **FAIL** — exit 0: engineer-alice: already running

Started 0 member(s), skipped 1 (already running), 0 error(s). |
| D24 | Finalization re-trigger (AC-23) | **NOTE** — session eaac7953 not in Retained state — finalization completed before force-stop |
| D25 | Provision failure: non-zero exit, no partial session left (AC-07) | **PASS** |
| D26 | Start for crash test | **FAIL** — exit 0 |
| D27 | Retention | **FAIL** — session not started |
| D28 | Start for daemon test | **FAIL** — exit 0 |
| D28 | Stale recovery | **NOTE** — session list: Sessions: none |
| D29 | State machine | **NOTE** — unexpected state: Completed |
| D29 | Session state after start: Active (AC-11) | **PASS** |
| D30 | Session in bm session list after force-stop (terminal state) (AC-11) | **PASS** |
| D31 | Terminal state observed via inspect: Killed (AC-11) | **PASS** |
| D30 | Session in bm session list after force-stop (terminal state) (AC-11) | **PASS** |
| D31 | Terminal state observed via inspect: Killed (AC-11) | **PASS** |
| D32 | Session workspace retained after force-stop (retention policy) (AC-20) | **PASS** |
| D33 | Stopped session visible in bm session list (AC-20) | **PASS** |
| D34 | Individual session cleanup removed workspace (AC-21) | **PASS** |
| D32 | Retention | **NOTE** — workspace not found at '' after stop |
| D33 | Stopped session visible in bm session list (AC-20) | **PASS** |
| D34 | Cleanup | **NOTE** — no session ID to clean up |
| D35 | Work item lock lifecycle: A-acquire → B-contend(exit1) → A-release → B-acquire (AC-13) | **PASS** |
| D35 | Work item lock (AC-13) | **FAIL** — failed to start sessions: alice=0, bob=0 |
| D36 | Push test (AC-14a) | **NOTE** — failed to start alice(0) or bob(0) |
| D37 | Push conflict (AC-14b) | **NOTE** — sessions not started |
| D36 | Independent branches in isolated workspaces: alice=push-test-alice-1781085028, bob=push-test-bob-1781085028 (AC-14a) | **PASS** |
| D37 | Session inspect captures git/workspace state (AC-14b) | **PASS** |
| D38 | bm session list shows force-stopped session in output | **PASS** |
| D39 | bm session list --json has finalization_status field in all rows | **PASS** |
| D40 | bm status --history exits non-zero with migration hint to bm session list | **PASS** |
| D41 | .claude/ assembly with team-level coding-agent/ — no crash (workspace created successfully) | **PASS** |
| D42 | Lock parallel contention: exactly one session acquired (sum=1, product=0) | **PASS** |
| D43 | Lock release cycle: A-acquire → A-release → B-acquire | **PASS** |
| D38 | bm session list shows force-stopped session in output | **PASS** |
| D39 | bm session list --json has finalization_status field in all rows | **PASS** |
| D40 | bm status --history exits non-zero with migration hint to bm session list | **PASS** |
| D41 | .claude/ assembly with team-level coding-agent/ — no crash (workspace created successfully) | **PASS** |
| D42 | Lock parallel contention | **NOTE** — failed to start sessions: alice=0, bob=0 |
| D43 | Lock release cycle | **NOTE** — sessions not started |
| D44 | Lock cleanup on stop | **NOTE** — sessions not started |
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
| H11 | Brain mode detection | **NOTE** — output: Started 0 member(s), skipped 0 (already running), 2 error(s). Error: Some members failed to start. See errors above.  |
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
| H27 | Brain status | **NOTE** — output: │ d92571be   ┆ engineer-alice ┆ Loop ┆ Killed    ┆ 2026-06-10 09:50:35 ┆ 0h 0m   ┆ 0          │ │ c6ec2e23   ┆ engineer-alice ┆ Loop ┆ Killed    ┆ 2026-06-10 09:50:24 ┆ 0h 0m   ┆ 0          │ ╰────────────┴────────────────┴──────┴───────────┴─────────────────────┴─────────┴────────────╯  |
| H28 | Operator DM created and greeting sent (!B4UPdzIE7EuyIgodls:localhost, $Wp_qm8GsalmS2Tl8dNUNDrlJmjtq4DrO_cc7EVSiPN4) | **PASS** |
| H26 | Brain process | **NOTE** — not alive (ACP may have failed to authenticate) |
| H27 | Brain status | **NOTE** — output: │ d92571be   ┆ engineer-alice ┆ Loop ┆ Killed    ┆ 2026-06-10 09:50:35 ┆ 0h 0m   ┆ 0          │ │ c6ec2e23   ┆ engineer-alice ┆ Loop ┆ Killed    ┆ 2026-06-10 09:50:24 ┆ 0h 0m   ┆ 0          │ ╰────────────┴────────────────┴──────┴───────────┴─────────────────────┴─────────┴────────────╯  |
| H28 | Operator DM created and greeting sent (!JX6E87PojIKtehOxMU:localhost, $GUcYJZKLDBUneKBuQ_JhWDpG34vsmPB1jDLSZaA9ppU) | **PASS** |
| H28b | DM discovery | **FAIL** — dm-room.json not created within 60s (stderr: ) |
| H29 | Work request sent to room while brain running ($u30V2j04x31wT74Ykoc5CBLAd4P0OjSyy3aE-Xvc8tw) | **PASS** |
| H30 | Follow-up question sent (multi-turn simulation) | **PASS** |
| H28b | DM discovery | **FAIL** — dm-room.json not created within 60s (stderr: ) |
| H29 | Work request sent to room while brain running ($trMV5mi-8vJVDZN78CMIxjo6MaeiHpLQ-1iCQMuhTXI) | **PASS** |
| H30 | Follow-up question sent (multi-turn simulation) | **PASS** |
| H31 | Malformed message delivered to room (brain not alive to test survival) | **PASS** |
| H31 | Malformed message delivered to room (brain not alive to test survival) | **PASS** |
| H32 | Brain response | **FAIL** — brain process not alive, no response |
| H29b | Work request response | **FAIL** — no brain response to evaluate |
| H33 | Message visibility | **FAIL** — greeting=0 task=0 total=0 |
| H34 | DM privacy | **NOTE** — could not login as bob to test |
| H35 | Brain stability | **NOTE** — skipped (brain not alive) |
| H36 | bm stop executed cleanly (exit 0) | **PASS** |
| H32 | Brain response | **FAIL** — brain process not alive, no response |
| H29b | Work request response | **FAIL** — no brain response to evaluate |
| H33 | Message visibility | **FAIL** — greeting=0 task=0 total=0 |
| H34 | DM privacy | **NOTE** — could not login as bob to test |
| H35 | Brain stability | **NOTE** — skipped (brain not alive) |
| H36 | bm stop executed cleanly (exit 0) | **PASS** |
| H37 | All brain processes terminated after stop | **PASS** |
| H37 | All brain processes terminated after stop | **PASS** |
| H38 | Brain restarted successfully (recovery scenario) | **PASS** |
| H39 | Message delivered after brain restart (recovery proof, $a7JKetR-bcxwd0qJM-fa2ZkhxXpT3Xr_TKn_eI-ckoU) | **PASS** |
| H38 | Brain restarted successfully (recovery scenario) | **PASS** |
| H39 | Message delivered after brain restart (recovery proof, $9Pe6XQSaJ3YYPrJQ3sccFIrFkQzB06T-ToENn5C7a7I) | **PASS** |
| H40 | Recovery response | **FAIL** — brain not alive after restart, no response (stderr: no log) |
| H40 | Recovery response | **FAIL** — brain not alive after restart, no response (stderr: no log) |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in DM room history (6 total) | **PASS** |
| H44 | DM persistence | **FAIL** — dm-room.json not found in workspace |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in DM room history (6 total) | **PASS** |
| H44 | DM persistence | **FAIL** — dm-room.json not found in workspace |
| H46 | GitHub issue creation | **NOTE** — failed to create issue (gh auth may lack permissions) |
| H46 | GitHub issue creation | **NOTE** — failed to create issue (gh auth may lack permissions) |
| H47 | Task journey start | **NOTE** — brain not alive (ACP auth may have failed, stderr: no log) |
| H48 | Board check request sent to brain ($wfCiQPTdyP2PmTQhORGbu_yUOF_igezQ1dJb1IHcIb4) | **PASS** |
| H47 | Task journey start | **NOTE** — brain not alive (ACP auth may have failed, stderr: no log) |
| H48 | Board check request sent to brain ($-vNhkumbnpw-D-_iqXyLUoJ1kjnZtcJV3N9aXJX7pjc) | **PASS** |
| H49 | Task response | **FAIL** — brain not alive, no response (stderr: no log) |
| H50 | Brain stability | **NOTE** — skipped (brain not alive at start) |
| H49 | Task response | **FAIL** — brain not alive, no response (stderr: no log) |
| H50 | Brain stability | **NOTE** — skipped (brain not alive at start) |
| H51 | Task execution journey cleaned up | **PASS** |
| H52 | Cleaned up all brain lifecycle test artifacts | **PASS** |
| H51 | Task execution journey cleaned up | **PASS** |

### Phase G: Cleanup

| # | Test | Result |
|---|------|--------|
| H52 | Cleaned up all brain lifecycle test artifacts | **PASS** |

### Phase G: Cleanup

| # | Test | Result |
|---|------|--------|
| G1 | Removed bridge container | **PASS** |
| G1 | Removed bridge container | **PASS** |
| G2 | Removed bridge volume | **PASS** |
| G2 | Removed bridge volume | **PASS** |
| G3 | Deleted GitHub repo | **PASS** |
| G3 | Deleted GitHub repo | **PASS** |
| G4 | Deleted GitHub project | **PASS** |
| G5 | Removed local state | **PASS** |
| G6 | Cleared keyring entries | **PASS** |
| G4 | Deleted GitHub project | **PASS** |
| G5 | Removed local state | **PASS** |
| G6 | Keyring cleanup skipped (no isolated keyring running) | **PASS** |
| G8 | Verified clean: no containers, no repo, no local state | **PASS** |

---

## Summary

- **PASS:** 162
- **FAIL:** 114
- **NOTE:** 43
| G8 | Verified clean: no containers, no repo, no local state | **PASS** |

---

## Summary

- **PASS:** 164
- **FAIL:** 115
- **NOTE:** 44
