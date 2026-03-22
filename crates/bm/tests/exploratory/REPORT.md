# Exploratory Test Report: Sync, Bridge & Lima Idempotency

**Date:** 2026-03-22
**Build:** bm 0.2.0-pre-alpha (local debug)
**Environment:** Linux x86_64, podman rootless, limactl limactl version 2.1.0, gh (devguyio)
**Test User:** bm-test-user@localhost (isolated)

## Results

### Phase B: Team Init + Hire

| # | Test | Result |
|---|------|--------|
| B1 | bm init | **FAIL** — exit 1: Error: Directory '/home/bm-test-user/.botminter/workspaces/exploratory-test' already exists. Choose a different team name. |
| B2 | GitHub repo exists | **PASS** |
| B3 | GitHub project board exists | **PASS** |
| B4 | Labels created (13 labels) | **PASS** |
| B5 | Team registered in config.yml | **PASS** |
| B6 | Team repo cloned | **PASS** |
| B7 | Init again | **NOTE** — Correctly rejects: already exists |
| B8 | Hire alice | **FAIL** — exit 1: Error: Member directory 'superman-alice' already exists. Choose a different name. |
| B9 | Hire bob | **FAIL** — exit 1: Error: Member directory 'superman-bob' already exists. Choose a different name. |
| B10 | Member dirs exist (superman-alice, superman-bob) | **PASS** |
| B11 | Hire duplicate alice | **NOTE** — Correctly rejects: 'already exists' |

### Phase C: Bridge Lifecycle (Tuwunel)

| # | Test | Result |
|---|------|--------|
| C1 | First sync --bridge | **PASS** |
| C2 | Container running | **PASS** |
| C3 | Matrix server healthy | **PASS** |
| C4 | Bridge state | **FAIL** — status=running ids=6 rooms=1 |
| C5 | Passwords | **FAIL** — count=6 |
| C6 | Keyring has credentials for alice + bob | **PASS** |
| C7 | Admin can login to Matrix | **PASS** |
| C8 | Room exploratory-test-general exists (!IUzWXhP9FtnYoEfmFY:localhost) | **PASS** |
| C9 | Sync --bridge again (idempotent) | **PASS** |
| C10 | Container still running | **PASS** |
| C11 | State | **FAIL** — status=running ids=6 |
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
| C27 | Pre-existing user already exists | **PASS** |
| C28 | Sync handles pre-existing user | **PASS** |
| C29 | Container stable after pre-existing user sync | **PASS** |
| C30 | Bridge state has 6 identities | **PASS** |
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
| D5 | Git status | **NOTE** — not clean:  M brain-stderr.log |
| D6 | Git log | **NOTE** — 246c2cd Sync workspace with team repo |
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
| D20 | Junk cleanup | **FAIL** — junk.txt still present |
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
| H23 | Room resolved (!yYUPBflXBmpnMVF48O:localhost) | **PASS** |
| H24 | Cleaned previous state for lifecycle test | **PASS** |
| H25 | bm start executed (brain mode detected) | **PASS** |
| H26 | Brain process verified (PID 1662949, command contains brain-run/acp) | **PASS** |
| H27 | bm status shows brain label during lifecycle | **PASS** |
| H28 | Greeting sent to room while brain running ($P9GGErv7oKmYrluxZ9I4E77y-OaiD-eDA6-KnT0FlAA) | **PASS** |
| H29 | Work request sent to room while brain running ($GXaP4xuwgeBuxGTupxtXLpdA0S-sUHo5lFqcp95mO-o) | **PASS** |
| H30 | Follow-up question sent (multi-turn simulation) | **PASS** |
| H31 | Brain survived malformed/empty message (edge case) | **PASS** |
| H32 | Brain responded with meaningful content (response: 🤖 Ralph loop `main` connected via Matrix...) | **PASS** |
| H29b | Brain response addresses work request (mentions project/status/tools) | **PASS** |
| H33 | User messages visible in room history (9 total messages) | **PASS** |
| H34 | Cross-member messaging while brain running (alice to bob, brain alive) | **PASS** |
| H35 | Brain survived all interaction (normal + malformed + cross-member messages) | **PASS** |
| H36 | bm stop graceful | **NOTE** — exit 1, retrying with --force |
| H37 | All brain processes terminated after stop | **PASS** |
| H38 | Brain restarted successfully (recovery scenario) | **PASS** |
| H39 | Message delivered after brain restart (recovery proof, $G0iJj3Zv-Oo0eudYIi7a874K7bTDLib-eStNv7ig0P8) | **PASS** |
| H40 | Recovery response | **FAIL** — brain alive after restart but did not respond within 90s (stderr: 2026-03-22T17:03:41.618797Z  INFO bm::brain::heartbeat: Heartbeat timer started interval_secs=60 2026-03-22T17:03:41.633076Z  INFO bm::brain::multiplexer: Brain multiplexer session started session_id=bce00470-50d2-4eae-88b7-5fda5a55c5d6 2026-03-22T17:05:43.641679Z  INFO bm::brain::bridge_adapter: Injected bridge message into multiplexer sender=@bmadmin:localhost body_len=75 ) |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in room history incl. recovery + cross-member (12 total) | **PASS** |
| H44 | Bob sees all messages in room (12 messages) | **PASS** |
| H46 | Created GitHub issue #2 for brain to discover | **PASS** |
| H47 | Brain started for task execution journey (PID 1737817) | **PASS** |
| H48 | Board check request sent to brain ($MSClY4pz8UuCfX-mYDD2cH7JBchJtUlOyz_362suNTk) | **PASS** |
| H49 | Task response | **FAIL** — brain alive but did not respond about board within 120s (stderr: 2026-03-22T17:07:25.717837Z  INFO bm::brain::heartbeat: Heartbeat timer started interval_secs=60 2026-03-22T17:07:25.731753Z  INFO bm::brain::multiplexer: Brain multiplexer session started session_id=fe66e013-1fe1-42ae-ae88-941fed5f8e5e 2026-03-22T17:09:27.769901Z  INFO bm::brain::bridge_adapter: Injected bridge message into multiplexer sender=@bmadmin:localhost body_len=140 ) |
| H50 | Brain survived task execution request (PID 1737817 still alive) | **PASS** |
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
| G7 | Deleted Lima VM (if exists) | **PASS** |
| G8 | Verified clean: no containers, no repo, no local state | **PASS** |

---

## Summary

- **PASS:** 124
- **FAIL:** 9
- **NOTE:** 5
