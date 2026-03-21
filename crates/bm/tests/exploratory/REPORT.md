# Exploratory Test Report: Sync, Bridge & Lima Idempotency

**Date:** 2026-03-21
**Build:** bm 0.2.0-pre-alpha (local debug)
**Environment:** Linux x86_64, podman rootless, limactl limactl version 2.1.0, gh (devguyio)

## Results

### Phase B: Team Init + Hire

| # | Test | Result |
|---|------|--------|
| B1 | Fresh init with tuwunel bridge | **PASS** |
| B2 | GitHub repo exists (private) | **PASS** |
| B3 | Project board exists | **PASS** |
| B4 | Kind labels bootstrapped | **PASS** |
| B5 | Team registered in config.yml | **PASS** |
| B6 | Team repo has profile content (PROCESS.md, knowledge/) | **PASS** |
| B7 | Init again | **NOTE** — Correctly rejects: 'directory exists' (exit 1) |
| B8 | Hire alice | **PASS** |
| B9 | Hire bob | **PASS** |
| B10 | Member config files present | **PASS** |
| B11 | Hire duplicate alice | **NOTE** — Correctly rejects: 'already exists' |

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
| C8 | Room exploratory-test-general exists (!j9dz7gzuyfu2ADtYT0:localhost) | **PASS** |
| C9 | Sync --bridge again (idempotent) | **PASS** |
| C10 | No duplicate identities (3) | **PASS** |
| C11 | No duplicate rooms (1) | **PASS** |
| C12 | Keyring token unchanged | **PASS** |
| C13 | Third sync --bridge still idempotent | **PASS** |
| C14 | Stopped container externally | **PASS** |
| C15 | Bridge state still says 'running' (stale) | **PASS** |
| C16 | Matrix server unreachable after stop | **PASS** |
| C17 | Sync --bridge recovers stopped container | **PASS** |
| C18 | Container running again | **PASS** |
| C19 | Matrix server healthy after recovery | **PASS** |
| C20 | Identities intact after recovery (3) | **PASS** |
| C21 | Force-removed container | **PASS** |
| C22 | Sync --bridge recreates from scratch | **PASS** |
| C23 | New container running | **PASS** |
| C24 | Identities re-provisioned (3) | **PASS** |
| C25 | Room recovered (1) | **PASS** |
| C26 | Keyring + Matrix login valid after full recreation | **PASS** |
| C27 | Removed container + volume | **PASS** |
| C28 | Removed bridge-state.json | **PASS** |
| C29 | Sync --bridge from scratch (full reset) | **PASS** |
| C30 | Full recovery: container up, Matrix healthy, 3 identities | **PASS** |
| C31 | Pre-registered user via Matrix API (@superman-pre-existing:localhost) | **PASS** |
| C32 | Sync onboards pre-existing user (M_USER_IN_USE handled) | **PASS** |
| C33 | Pre-existing user: keyring + Matrix login valid | **PASS** |

### Phase D: Workspace Sync Idempotency

| # | Test | Result |
|---|------|--------|
| D1 | Alice workspace has all context files | **PASS** |
| D2 | Bob workspace has all context files | **PASS** |
| D3 | Team submodule present | **PASS** |
| D4 | Agent dir assembled | **PASS** |
| D5 | Git repo clean | **PASS** |
| D6 | Git log | **NOTE** — 2540ad9 Sync workspace with team repo |
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

### Phase E: Full Sync (-a flag)

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
| H23 | Room resolved (!AkdrfRZAN910TYYVJp:localhost) | **PASS** |
| H24 | Cleaned previous state for lifecycle test | **PASS** |
| H25 | bm start executed (brain mode detected) | **PASS** |
| H26 | Brain member process is alive (PID verified) | **PASS** |
| H27 | bm status shows brain label during lifecycle | **PASS** |
| H28 | Greeting sent to room while brain running ($1n-PGBYVlvWr6SdpbsSWA22l-g7kTYIzwhGIThAj5a0) | **PASS** |
| H29 | Work request sent to room while brain running ($qQFE7mX-fkVKtYNDJ0_UJsEtnw65v5wfAiKwe30Brqc) | **PASS** |
| H30 | Follow-up question sent (multi-turn simulation) | **PASS** |
| H31 | Brain survived malformed/empty message (edge case) | **PASS** |
| H32 | Brain member responded autonomously! (response: 🤖 Ralph loop `main` connected via Matrix...) | **PASS** |
| H33 | User messages visible in room history (9 total messages) | **PASS** |
| H34 | Cross-member messaging while brain running (alice to bob, brain alive) | **PASS** |
| H35 | Brain survived all interaction (normal + malformed + cross-member messages) | **PASS** |
| H36 | bm stop executed cleanly (exit 0) | **PASS** |
| H37 | All brain processes terminated after stop | **PASS** |
| H38 | Brain restarted successfully (recovery scenario) | **PASS** |
| H39 | Message delivered after brain restart (recovery proof, $rVXwAxITmf_d19SPo-a6cAFDkmDRcDQuqi3HgV8yPn8) | **PASS** |
| H40 | Recovery response | **NOTE** — brain alive after restart but did not respond within 30s |
| H41 | Recovery start-stop cycle clean (brain lifecycle idempotent) | **PASS** |
| H42 | Status inquiry sent after brain lifecycle | **PASS** |
| H43 | All messages persist in room history incl. recovery + cross-member (12 total) | **PASS** |
| H44 | Bob sees all messages in room (12 messages) | **PASS** |
| H45 | Cleaned up brain lifecycle test artifacts | **PASS** |

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

- **PASS:** 122
- **FAIL:** 0
- **NOTE:** 4
