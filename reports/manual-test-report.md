# Manual Test Report: Sync, Bridge & Lima Idempotency

**Date:** 2026-03-20
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
| C8 | Room manual-test-general exists (!ethyfnIemWvDOlp3PB:localhost) | **PASS** |
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
| D6 | Git log | **NOTE** — 6880918 Sync workspace with team repo |
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

- **PASS:** 78
- **FAIL:** 0
- **NOTE:** 3
