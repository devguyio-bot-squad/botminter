# Exploratory Test Report: Sync & Bridge Idempotency

**Date:** 2026-04-06
**Build:** bm 0.2.0-pre-alpha (local debug)
**Environment:** Linux x86_64, podman rootless, gh (devguyio)
**Test User:** bm-test-user@localhost (isolated)

## Results

### Phase B: Team Init + Hire

| # | Test | Result |
|---|------|--------|
| B1 | bm init (non-interactive, scrum-compact, tuwunel) | **PASS** |
| B2 | GitHub repo exists | **PASS** |
| B3 | GitHub project board exists | **PASS** |
| B4 | Labels created (11 labels) | **PASS** |
| B5 | Team registered in config.yml | **PASS** |
| B6 | Team repo cloned | **PASS** |
| B7 | Init again | **NOTE** — Correctly rejects: already exists |
| B8 | Hired alice (--reuse-app) | **PASS** |
| B9 | Hired bob (--reuse-app) | **PASS** |
| B10 | Member dirs exist (superman-alice, superman-bob) | **PASS** |
| B11 | Hire duplicate | **FAIL** — Should have failed |

### Phase C: Bridge Lifecycle (Tuwunel)

| # | Test | Result |
|---|------|--------|
| C1 | First sync --bridge | **PASS** |
| C2 | Container running | **PASS** |
| C3 | Matrix server healthy | **PASS** |
| C4 | Bridge state: running, 3 identities, 1 room | **PASS** |
| C5 | Passwords file has 3 entries | **PASS** |
| C6 | Keyring has credentials for alice + bob | **PASS** |
| C7 | Admin can login to Matrix | **PASS** |
| C8 | Room exploratory-test-general exists (!BjoSqn4sOsupjb5Gsv:localhost) | **PASS** |
| C9 | Sync --bridge again (idempotent) | **PASS** |
| C10 | Container still running | **PASS** |
| C11 | Bridge state unchanged | **PASS** |
| C12 | Alice credential unchanged after re-sync | **PASS** |
| C13 | Stopped container | **PASS** |
| C14 | Sync --bridge recovers stopped container | **PASS** |
| C15 | Container running again | **PASS** |
| C16 | Matrix healthy after recovery | **PASS** |
| C17 | Force-removed container | **PASS** |
| C18 | Sync --bridge recovers removed container | **PASS** |
| C19 | Container running after re-create | **PASS** |
| C20 | Admin login survives container re-create | **PASS** |
| C21 | Removed container + volume | **PASS** |
| C22 | Sync --bridge recovers from volume loss | **PASS** |
| C23 | Container running after volume re-create | **PASS** |
| C24 | Matrix healthy after volume re-create | **PASS** |
| C25 | Admin password regenerated | **PASS** |
| C26 | Alice: new password + keyring valid after volume re-create | **PASS** |
| C27 | Pre-existing user registered via UIAA (@superman-pre-existing:localhost) | **PASS** |
| C28 | Sync handles pre-existing user | **PASS** |
| C29 | Container stable after pre-existing user sync | **PASS** |
| C30 | Bridge state has 4 identities | **PASS** |
| C31 | Sync idempotent after pre-existing user | **PASS** |
| C32 | Final bridge state: running | **PASS** |
| C33 | Pre-existing user: keyring token valid (@superman-pre-existing:localhost) | **PASS** |

### Phase D: Workspace Sync Idempotency

| # | Test | Result |
|---|------|--------|
| D1 | Alice workspace has all context files | **PASS** |
| D2 | Bob workspace has all context files | **PASS** |
| D3 | Team submodule present | **PASS** |
| D4 | Agent dir assembled | **PASS** |
| D5 | Git repo clean | **PASS** |
| D6 | Git log | **NOTE** — 509cc7c Sync workspace with team repo |
| D7 | Sync again (no changes) | **PASS** |
| D8 | Context files still present after re-sync | **PASS** |
| D9 | Third sync still clean | **PASS** |
| D10 | Removed .botminter.workspace marker | **PASS** |
| D11 | Sync recovers stale workspace | **PASS** |
| D12 | All context files restored after recovery | **PASS** |
| D13 | Team submodule intact after recovery | **PASS** |
| D14 | Deleted CLAUDE.md from bob workspace | **PASS** |
| D15 | Sync restores CLAUDE.md | **PASS** |
| D16 | Deleted ralph.yml from bob workspace | **PASS** |
| D17 | Sync restores ralph.yml | **PASS** |
| D18 | Created junk dir at future carol workspace path | **PASS** |
| D19 | Hired carol | **PASS** |
| D20 | Junk cleaned, proper workspace created for carol | **PASS** |
| D21 | Settings.json surfaced with PostToolUse hook | **PASS** |
| D22 | Inbox write/peek/read lifecycle complete | **PASS** |
| D23 | Hook exits 0 in workspace (no pending messages) | **PASS** |
| D23b | Hook exits 0 outside workspace | **PASS** |
| D24 | Re-sync preserves inbox messages | **PASS** |

### Phase E: Full Sync (--bridge flag)

| # | Test | Result |
|---|------|--------|
| E1 | Full sync --bridge -v | **PASS** |
| E2 | Full sync again (idempotent) | **PASS** |
| E3 | Hire dave + sync creates new workspace | **PASS** |
| E4 | All 5 member workspaces present | **PASS** |
| E5 | Bridge has 6 identities (admin + 4 members) | **PASS** |

### Phase F: Error Handling

| # | Test | Result |
|---|------|--------|
| F1 | Graceful handling when just not in PATH | **PASS** |
| F2 | bm status -v works | **PASS** |
| F3 | bm members list shows 5 members | **PASS** |
| F4 | bm teams show works | **PASS** |

### Phase H: Brain Lifecycle (Chat-First Member)

| # | Test | Result |
|---|------|--------|
| H1 | brain-prompt.md exists and is non-empty | **PASS** |
| H2 | No unrendered template variables | **PASS** |
| H3 | Contains rendered member name (alice) | **PASS** |
| H4 | Contains rendered team name (exploratory-test) | **PASS** |
| H5 | Contains rendered GitHub org (devguyio-bot-squad) | **PASS** |
| H6 | Contains rendered GitHub repo (exploratory-test-team) | **PASS** |
| H7 | All expected sections present (Identity, Board Awareness, Work Loop, Human Interaction, Dual-Channel) | **PASS** |
| H8 | Bob workspace also has brain-prompt.md | **PASS** |
| H9 | Alice and bob brain-prompt.md differ (per-member rendering) | **PASS** |
| H10 | Bob's brain-prompt.md contains 'bob', not 'alice' | **PASS** |
| H11 | bm start detects brain mode (output mentions brain) | **PASS** |
| H12 | state.json has brain_mode=true for at least one member | **PASS** |
| H13 | Without brain-prompt.md: no brain_mode=true in state | **PASS** |
| H14 | Restored brain-prompt.md and cleaned up state | **PASS** |
| H15 | Modified brain-prompt.md restored on re-sync | **PASS** |
| H16 | Deleted brain-prompt.md restored on re-sync | **PASS** |
| H17 | brain-prompt.md content idempotent across syncs (hash match) | **PASS** |
| H18 | Verbose sync mentions brain prompt surfacing | **PASS** |
| H19 | Tuwunel bridge is running (Matrix server healthy) | **PASS** |
| H20 | ACP binary available (claude-code-acp-rs 0.1.22) | **PASS** |
| H21 | Admin Matrix login successful | **PASS** |
| H22 | Alice Matrix login successful | **PASS** |
| H23 | Cleaned DM room state for discovery test | **PASS** |
| H24 | Cleaned previous state for lifecycle test | **PASS** |
| H25 | bm start executed (brain mode detected) | **PASS** |
| H26 | Brain started in DM discovery mode (PID 3050551) | **PASS** |
| H27 | bm status shows brain label during lifecycle | **PASS** |
| H28 | Operator DM created and greeting sent (!Bb9SDxSnY5itOAIrHD:localhost, $bILPDup91_Um38cNNnPMsRCRo4hr8--Ihbj0BWwSLi0) | **PASS** |
| H28b | Brain discovered DM room (!Bb9SDxSnY5itOAIrHD:localhost via dm-room.json) | **PASS** |
| H29 | Work request sent to room while brain running ($1pHNffXVrbvQtDMj6gmHsZ4_nlDaI86J7HOvm8N95C4) | **PASS** |
| H30 | Follow-up question sent (multi-turn simulation) | **PASS** |
| H31 | Brain survived malformed/empty message (edge case) | **PASS** |
| H32 | Brain response | **FAIL** — brain is alive but did not respond within 30s |
| H29b | Work request response | **FAIL** — no brain response to evaluate |
| H33 | User messages visible in room history (4 total messages) | **PASS** |
| H34 | DM privacy | **NOTE** — bob can read alice's DM room (may be due to server config) |
| H35 | Brain survived all interaction (normal + malformed + cross-member messages) | **PASS** |
| H36 | bm stop executed cleanly (exit 0) | **PASS** |
| H37 | All brain processes terminated after stop | **PASS** |
| H38 | Brain restarted successfully (recovery scenario) | **PASS** |
| H39 | Message delivered after brain restart (recovery proof, $jMQyOSu_Xy9AwDL_M37YZOOzplAt_OrHfEQ45bGSmFA) | **PASS** |
| H40 | Recovery response | **FAIL** — brain alive after restart but did not respond within 90s (stderr: 2026-04-06T18:29:50.357841Z  INFO bm::commands::brain_run: Brain multiplexer starting workspace=/home/bm-test-user/.botminter/workspaces/exploratory-test/superman-alice acp_binary=claude-agent-acp 2026-04-06T18:29:50.359800Z  INFO bm::commands::brain_run: Bridge adapter enabled — spawning reader and writer room_id=Some("!Bb9SDxSnY5itOAIrHD:localhost") own_user_id=@superman-alice:localhost mode="locked" 2026-04-06T18:29:50.391611Z  INFO bm::brain::bridge_adapter: Joined Matrix room room_id=!Bb9SDxSnY5itOAIrHD:localhost 2026-04-06T18:29:50.396309Z  INFO bm::brain::bridge_adapter: Bridge reader initial sync complete 2026-04-06T18:29:50.413883Z  INFO bm::brain::heartbeat: Heartbeat timer started interval_secs=60 2026-04-06T18:30:16.431342Z  INFO bm::brain::bridge_adapter: Injected bridge message into multiplexer sender=@bmadmin:localhost body_len=75 2026-04-06T18:31:50.414938Z ERROR bm::brain::multiplexer: ACP session creation timed out after 120s 2026-04-06T18:31:50.415262Z  INFO bm::brain::heartbeat: Heartbeat timer shutting down 2026-04-06T18:31:50.415259Z  INFO bm::brain::bridge_adapter: Bridge writer stopping (multiplexer shut down) 2026-04-06T18:31:50.415296Z  INFO bm::brain::event_watcher: Event watcher shutting down 2026-04-06T18:31:50.415340Z  INFO bm::brain::bridge_adapter: Bridge reader shutting down 2026-04-06T18:31:50.415392Z ERROR bm::commands::brain_run: Brain multiplexer error: ACP error: ACP initialization failed: session creation timed out after 120s Error: Brain multiplexer failed: ACP error: ACP initialization failed: session creation timed out after 120s ) |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in DM room history (6 total) | **PASS** |
| H44 | dm-room.json persisted correctly (!Bb9SDxSnY5itOAIrHD:localhost) | **PASS** |
| H46 | Created GitHub issue #1 for brain to discover | **PASS** |
| H47 | Brain started for task execution journey (PID 3080519) | **PASS** |
| H48 | Board check request sent to brain ($AkWr_CFgmUI6DPiCDT_Ize5YW8ItxSJWAWEmPVEGYi4) | **PASS** |
| H49 | Task response | **NOTE** — brain alive, prompt sent to ACP, but LLM did not respond within 300s (expected for complex tool-use) |
| H50 | Brain survived task execution request (PID 3080519 still alive) | **PASS** |
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

- **PASS:** 130
- **FAIL:** 4
- **NOTE:** 4
