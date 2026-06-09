# Exploratory Test Report: Sync & Bridge Idempotency

**Date:** 2026-06-09
**Build:** bm 0.2.0-pre-alpha (1ae757b-dirty) (local debug)
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
| B7 | Init again correctly rejects existing team (exit 1) | **PASS** |
| B8 | Hired alice (--reuse-app) | **PASS** |
| B9 | Hired bob (--reuse-app) | **PASS** |
| B9b | Deployed brain-prompt.md to team repo (alice and bob, per-member rendered) | **PASS** |
| B10 | Member dirs exist (engineer-alice, engineer-bob) | **PASS** |
| B11 | Hire duplicate alice correctly rejects (exit 1) | **PASS** |
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
| C8 | Room exploratory-test-general exists (!PkpNYwwhEOKatOz8qV:localhost) | **PASS** |
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
| D05 | Session creation latency: 696ms (AC-06) | **PASS** |
| D06 | Workspace marker has session_id + member fields (AC-01) | **PASS** |
| D07 | Project 'exploratory-test-project' provisioned in workspace (AC-01) | **PASS** |
| D08 | Config files (PROMPT.md, CLAUDE.md, ralph.yml) present (AC-01) | **PASS** |
| D09 | .claude/ fully assembled: agents(1), skills(3), settings.json (AC-08) | **PASS** |
| D10 | GH credentials via system gh auth (AC-09) — ephemeral model inherits system credentials | **PASS** |
| D11 | bm status --json has all fields: member=engineer-alice, state=Active (AC-10) | **PASS** |
| D12 | Two concurrent sessions active: alice + bob (AC-04) | **PASS** |
| D13 | Workspaces isolated: file in alice not visible in bob (AC-04) | **PASS** |
| D14 | Stopped bob selectively, alice still Active (AC-15) | **PASS** |
| D15 | Stop returned in 0s (async deactivation) (AC-19) | **PASS** |
| D16 | Force-stop session appears in bm session list as terminal (AC-15) | **PASS** |
| D17 | bm session list shows sessions with session IDs (AC-17) | **PASS** |
| D18 | bm session list shows state and finalization status columns (AC-17) | **PASS** |
| D19 | Session inspect shows ID, member, type, state, workspace (AC-18) | **PASS** |
| D20 | Session cleanup completed for ca583c15 (AC-18) | **PASS** |
| D21 | Bulk cleanup --all completed (AC-18) | **PASS** |
| D22 | Graceful stop finalized (completed) and branch 'finalization-test-1780969580' confirmed pushed to remote (AC-02) | **PASS** |
| D23 | Finalization results visible in inspect (AC-05) | **PASS** |
| D24 | Finalization re-trigger correctly rejected: session already Completed (AC-23) | **PASS** |
| D25 | Provision failure: non-zero exit, no partial session left (AC-07) | **PASS** |
| D27 | Crashed session workspace retained at /home/bm-test-user/.botminter/sessions/exploratory-test/engineer-alice/24fd7127 (AC-26) | **PASS** |
| D26 | New session starts after crash + force-stop (AC-03) | **PASS** |
| D28 | Daemon restart: stale sessions visible in bm session list (AC-25) | **PASS** |
| D29 | Session state after start: Killed (AC-11) | **PASS** |
| D30 | Session in bm session list after force-stop (terminal state) (AC-11) | **PASS** |
| D31 | Terminal state observed via inspect: Killed (AC-11) | **PASS** |
| D32 | Session workspace retained after force-stop (retention policy) (AC-20) | **PASS** |
| D33 | Stopped session visible in bm session list (AC-20) | **PASS** |
| D34 | Individual session cleanup removed workspace (AC-21) | **PASS** |
| D35 | Work item lock lifecycle: A-acquire → B-contend(exit1) → A-release → B-acquire (AC-13) | **PASS** |
| D36 | Independent branches in isolated workspaces: alice=push-test-alice-1780969626, bob=push-test-bob-1780969626 (AC-14a) | **PASS** |
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
| F1 | Graceful handling when just not in PATH (output: Bridge 'tuwunel' already running.) | **PASS** |
| F2 | bm status -v works | **PASS** |
| F3 | bm members list shows 4 members | **PASS** |
| F4 | bm teams show works | **PASS** |

### Phase H: Brain Lifecycle (Chat-First Member)

| # | Test | Result |
|---|------|--------|
| H2 | No unrendered template variables in team repo brain-prompt.md | **PASS** |
| H9 | Alice and bob brain-prompt.md differ in team repo (per-member rendering) | **PASS** |
| H11 | bm start executed successfully (Started 4 member(s), skipped 0 (already running), 0 error(s). ) | **PASS** |
| H12 | brain_mode=true: active Brain session found in session registry | **PASS** |
| H13 | Without brain-prompt.md: no Brain session active (Loop sessions only) | **PASS** |
| H14 | Restored brain-prompt.md and cleaned up state | **PASS** |
| H19 | Tuwunel bridge is running (Matrix server healthy) | **PASS** |
| H20 | ACP binary available () | **PASS** |
| H21 | Admin Matrix login successful | **PASS** |
| H22 | Alice Matrix login successful | **PASS** |
| H23 | Cleaned DM room state for discovery test | **PASS** |
| H24 | Cleaned previous state for lifecycle test | **PASS** |
| H25 | bm start executed (brain mode detected) | **PASS** |
| H26 | Brain started in DM discovery mode (PID 1774539) | **PASS** |
| H27 | bm status shows brain label during lifecycle | **PASS** |
| H28 | Operator DM created and greeting sent (!Ddyj4JoE7lfZn4U2dI:localhost, $Tjem4ttESMoqI6IL32UW-HIcRar1QdY9hmZNhYiZBaE) | **PASS** |
| H28b | Brain discovered DM room (!Ddyj4JoE7lfZn4U2dI:localhost via dm-room.json) | **PASS** |
| H29 | Work request sent to room while brain running ($nsE4RjflTbQlw9LPmYD2iNYNmkW74VK9FapxS3MsU70) | **PASS** |
| H30 | Follow-up question sent (multi-turn simulation) | **PASS** |
| H31 | Brain survived malformed/empty message (edge case) | **PASS** |
| H32 | Brain responded with meaningful content (response: Hey! I'm Alice, the engineer on the exploratory-test team. Checking my current state now....) | **PASS** |
| H29b | Brain response addresses work request (mentions project/status/tools) | **PASS** |
| H33 | User messages visible in room history (5 total messages) | **PASS** |
| H34 | DM room is private — bob is not a member of alice's DM room (expected) | **PASS** |
| H35 | Brain survived all interaction (normal + malformed + cross-member messages) | **PASS** |
| H36 | bm stop executed cleanly (exit 0) | **PASS** |
| H37 | All brain processes terminated after stop | **PASS** |
| H38 | Brain restarted successfully (recovery scenario) | **PASS** |
| H39 | Message delivered after brain restart (recovery proof, $7ONvycwLsu-enfRvdCDOIzeYFjoYM7sXKSdeAvZqBuY) | **PASS** |
| H40 | Recovery response | **FAIL** — brain alive after restart but did not respond within 90s (stderr: 2026-06-09T01:48:05.823104Z  INFO bm::commands::brain_run: Brain multiplexer starting workspace=/home/bm-test-user/.botminter/sessions/exploratory-test/engineer-alice/b08cd443 acp_binary=claude-agent-acp 2026-06-09T01:48:05.824363Z  INFO bm::commands::brain_run: Bridge adapter enabled — spawning reader and writer room_id=None own_user_id=@engineer-alice:localhost mode="discovery" 2026-06-09T01:48:05.855776Z  INFO bm::brain::bridge_adapter: Bridge reader starting in DM discovery mode — waiting for operator invite 2026-06-09T01:48:05.860727Z  INFO bm::brain::bridge_adapter: Bridge reader initial sync complete 2026-06-09T01:48:05.882034Z  INFO bm::brain::heartbeat: Heartbeat timer started interval_secs=60 2026-06-09T01:48:06.905880Z  INFO bm::brain::multiplexer: Brain multiplexer session started session_id=3393db55-fb4f-4ea0-b5dd-3c95f69af0dc 2026-06-09T01:48:06.906008Z  INFO bm::brain::types: Loaded brain envelope template path=/home/bm-test-user/.botminter/sessions/exploratory-test/engineer-alice/b08cd443/brain-envelope.md 2026-06-09T01:49:05.881999Z  INFO bm::brain::multiplexer: Sending prompt to ACP priority=heartbeat prompt_len=552 2026-06-09T01:49:05.882181Z  INFO connection{name="botminter"}: bm::acp::client: ACP prompt task: sending request 2026-06-09T01:49:42.407451Z  INFO connection{name="botminter"}: bm::acp::client: ACP prompt completed stop_reason=EndTurn 2026-06-09T01:49:42.407762Z  INFO bm::brain::multiplexer: Turn complete, draining queue stop_reason=EndTurn queue_len=0 2026-06-09T01:51:05.882274Z  INFO bm::brain::multiplexer: Sending prompt to ACP priority=heartbeat prompt_len=552 2026-06-09T01:51:05.882520Z  INFO connection{name="botminter"}: bm::acp::client: ACP prompt task: sending request ) |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in DM room history (7 total) | **PASS** |
| H44 | DM persistence | **FAIL** — dm-room.json not found — brain must discover and persist DM room (ACP auth must succeed) |
| H46 | Created GitHub issue #1 for brain to discover | **PASS** |
| H47 | Brain started for task execution journey (PID 1795231) | **PASS** |
| H48 | Board check request sent to brain ($FDFmCGs-FRvkSf4wnsXdX11_F7ire6hyckciAE8xgoE) | **PASS** |
| H49 | Task response | **FAIL** — brain alive but LLM did not respond within 300s — brain must respond to task requests |
| H50 | Brain survived task execution request (PID 1795231 still alive) | **PASS** |
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

- **PASS:** 151
- **FAIL:** 3
- **NOTE:** 0
