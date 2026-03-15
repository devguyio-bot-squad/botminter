---
phase: 09-profile-integration-cleanup
plan: 03
subsystem: bridge
tags: [bridge-provisioning, credential-store, robot-injection, keyring, per-member-credentials]

# Dependency graph
requires:
  - phase: 09-01
    provides: "CredentialStore trait, LocalCredentialStore, CLI sync flags (--repos/--bridge/--all)"
provides:
  - "provision_bridge() for managed and external bridge identity provisioning"
  - "inject_robot_enabled() for ralph.yml RObot.enabled injection"
  - "Per-member credential resolution in bm start via CredentialStore"
  - "Diagnostic warning when credentials exist but RObot.enabled is false"
affects: [09-04, 10-rocketchat]

# Tech tracking
tech-stack:
  added: []
  patterns: [provision_bridge with CredentialStore, inject_robot_enabled via serde_yml::Value mutation, per-member credential resolution replacing team-wide token]

key-files:
  created:
    - "crates/bm/tests/bridge_sync.rs"
  modified:
    - "crates/bm/src/bridge.rs"
    - "crates/bm/src/workspace.rs"
    - "crates/bm/src/commands/teams.rs"
    - "crates/bm/src/commands/start.rs"

key-decisions:
  - "provision_bridge lives in bridge.rs (bridge logic), called from commands/teams.rs"
  - "inject_robot_enabled runs on ALL syncs when bridge configured (not just --bridge)"
  - "Per-member credential resolution replaces team-wide telegram_bot_token in bm start"
  - "Formation manager retains legacy telegram_bot_token fallback (different code path)"

patterns-established:
  - "provision_bridge(): idempotent bridge provisioning with state tracking"
  - "inject_robot_enabled(): serde_yml::Value mutation for ralph.yml without touching secrets"
  - "check_robot_enabled_mismatch(): diagnostic warning for credential/config mismatches"

requirements-completed: [PROF-03]

# Metrics
duration: 10min
completed: 2026-03-08
---

# Phase 9 Plan 03: Sync Bridge Provisioning & Start Credential Resolution Summary

**Bridge provisioning via provision_bridge() during sync --bridge, ralph.yml RObot.enabled injection, per-member credential resolution replacing team-wide token in bm start**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-08T19:23:32Z
- **Completed:** 2026-03-08T19:33:32Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- provision_bridge() handles managed (auto-create user+token) and external (validate existing) bridge provisioning with idempotent state tracking
- inject_robot_enabled() sets RObot.enabled in ralph.yml based on credential availability -- no secrets in ralph.yml (per ADR-0003)
- bm start resolves per-member credentials via CredentialStore (env var priority, then keyring) instead of team-wide telegram_bot_token
- Diagnostic warning when member has credentials but RObot.enabled is false (forgotten sync)
- 573 tests passing (371 unit + 10 bridge_sync + 66 cli_parsing + 12 conformance + 114 integration), clippy clean

## Task Commits

Each task was committed atomically (TDD: test then implementation):

1. **Task 1: Bridge provisioning and RObot injection**
   - `3621fb2` (test): add failing tests for bridge provisioning and RObot injection
   - `8325de0` (feat): implement bridge provisioning and RObot.enabled injection

2. **Task 2: Per-member credential resolution in bm start**
   - `e9f05be` (test): add failing tests for per-member credential resolution
   - `73e65df` (feat): implement per-member credential resolution in bm start

## Files Created/Modified
- `crates/bm/src/bridge.rs` - provision_bridge() for managed/external bridge identity provisioning with CredentialStore
- `crates/bm/src/workspace.rs` - inject_robot_enabled() for ralph.yml RObot.enabled injection via serde_yml::Value
- `crates/bm/src/commands/teams.rs` - Wire provision_bridge into sync --bridge, inject_robot_enabled into all syncs
- `crates/bm/src/commands/start.rs` - Per-member credential resolution replacing team-wide token, check_robot_enabled_mismatch diagnostic
- `crates/bm/tests/bridge_sync.rs` - 10 new tests for provisioning and RObot injection

## Decisions Made
- provision_bridge() placed in bridge.rs module (bridge provisioning logic) rather than commands/teams.rs (keeps module boundaries clean)
- inject_robot_enabled runs during ALL sync operations when a bridge is configured, not just when --bridge flag is set -- ensures `bm teams sync` after `bm bridge identity add` updates RObot.enabled
- Per-member credential resolution replaces team-wide telegram_bot_token in the local member launch path
- Formation manager retains legacy telegram_bot_token usage (separate code path for non-local formations, to be updated when formation bridge integration lands)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Clippy violations in workspace.rs and teams.rs**
- **Found during:** Task 1 (implementation)
- **Issue:** map_or should be is_some_and, unwrap after is_some should use if-let
- **Fix:** Used is_some_and and if-let pattern per clippy recommendations
- **Files modified:** crates/bm/src/workspace.rs, crates/bm/src/commands/teams.rs
- **Verification:** cargo clippy -p bm -- -D warnings clean
- **Committed in:** 8325de0 (Task 1 commit)

**2. [Rule 1 - Bug] Pre-existing clippy error in hire.rs from Plan 02 changes**
- **Found during:** Task 1 (clippy check)
- **Issue:** Unnecessary reference creation in hire.rs (uncommitted Plan 02 code)
- **Fix:** Extracted to local variable to avoid unnecessary borrow
- **Files modified:** crates/bm/src/commands/hire.rs
- **Verification:** cargo clippy clean
- **Committed in:** 8325de0 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs/clippy)
**Impact on plan:** Minor clippy fixes, no scope change.

## Issues Encountered
- Test file (bridge_sync.rs) was repeatedly deleted from disk by an external process between tool calls, requiring git checkout restoration before each compilation. Worked around by chaining commands.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Bridge provisioning and credential injection complete, ready for Plan 04 (documentation and scrum-compact-telegram cleanup)
- Per-member credential flow: bm hire (token prompt) -> bm teams sync --bridge (provisioning) -> bm start (env var injection) is now fully wired

## Self-Check: PASSED

---
*Phase: 09-profile-integration-cleanup*
*Completed: 2026-03-08*
