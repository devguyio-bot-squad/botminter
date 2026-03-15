---
phase: 09-profile-integration-cleanup
plan: 02
subsystem: bridge
tags: [init-wizard, bridge-selection, hire-token, profile-cleanup, cliclack]

# Dependency graph
requires:
  - phase: 09-profile-integration-cleanup
    provides: "CredentialStore, ProfileManifest.bridges, CLI --bridge flag, BridgeDef struct"
provides:
  - "Init wizard bridge selection (interactive cliclack::select + non-interactive --bridge flag)"
  - "Bridge validation against profile's bridges list"
  - "Bridge name recorded in team botminter.yml during init"
  - "Hire token prompt for external bridges (interactive mode only)"
  - "scrum-compact-telegram profile removed from codebase"
affects: [09-03, 09-04]

# Tech tracking
tech-stack:
  added: []
  patterns: [validate_bridge_selection() for profile-aware bridge validation, record_bridge_in_manifest() for serde_yml::Value mutation]

key-files:
  created: []
  modified:
    - "crates/bm/src/commands/init.rs"
    - "crates/bm/src/commands/hire.rs"
    - "crates/bm/src/profile.rs"
    - "crates/bm/tests/integration.rs"
    - "profiles/scrum-compact-telegram/ (deleted)"
    - "README.md"
    - "RELEASE_NOTES.md"
    - "docs/content/getting-started/index.md"
    - "docs/content/getting-started/bootstrap-your-team.md"
    - "docs/content/concepts/profiles.md"
    - "docs/content/concepts/coordination-model.md"
    - "docs/content/how-to/generate-team-repo.md"
    - "docs/content/reference/configuration.md"
    - "docs/content/reference/member-roles.md"
    - "docs/content/roadmap.md"

key-decisions:
  - "Bridge selection in interactive wizard uses cliclack::select with 'No bridge' option (not a separate step)"
  - "Bridge recorded in team botminter.yml BEFORE initial commit (part of team repo initial state)"
  - "Hire token prompt only shown for external bridges in interactive mode (stdin.is_terminal())"
  - "scrum-compact-telegram deleted without migration path per Alpha policy"

patterns-established:
  - "validate_bridge_selection(): validates bridge name against profile manifest bridges list"
  - "record_bridge_in_manifest(): serde_yml::Value mutation to add bridge key to botminter.yml"

requirements-completed: [PROF-05, PROF-06]

# Metrics
duration: 12min
completed: 2026-03-08
---

# Phase 9 Plan 02: Init/Hire Bridge Wiring + Profile Cleanup Summary

**Init wizard bridge selection (interactive + non-interactive), hire token prompt for external bridges, scrum-compact-telegram profile removed**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-08T19:23:15Z
- **Completed:** 2026-03-08T19:36:10Z
- **Tasks:** 2
- **Files modified:** 16 modified, 84 deleted (profile directory)

## Accomplishments
- Init wizard offers bridge selection after profile selection (interactive: cliclack::select, non-interactive: --bridge flag)
- Bridge name validated against profile's bridges list with clear error messages
- Selected bridge recorded in team botminter.yml (readable by bridge::discover())
- Hire prompts for optional bridge token on external bridges (interactive only, stores via CredentialStore)
- scrum-compact-telegram profile completely removed from codebase (84 files deleted)
- All 573 tests pass (371 unit + 10 bridge_sync + 66 cli_parsing + 12 conformance + 114 integration), clippy clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Init wizard bridge selection and hire token prompt** - `1e21332` (feat)
2. **Task 2: Remove scrum-compact-telegram profile and update all references** - `6fd7291` (chore)

## Files Created/Modified
- `crates/bm/src/commands/init.rs` - validate_bridge_selection(), record_bridge_in_manifest(), interactive bridge step, non-interactive bridge wiring
- `crates/bm/src/commands/hire.rs` - Bridge token prompt for external bridges after hire
- `crates/bm/src/profile.rs` - Removed scrum_compact_telegram_has_views test
- `crates/bm/tests/integration.rs` - 3 new integration tests for init bridge flow + setup_git_config helper
- `profiles/scrum-compact-telegram/` - Entire directory deleted (84 files)
- `README.md` - Updated profile table (2 profiles, Telegram as bridge option)
- `RELEASE_NOTES.md` - Updated profiles description
- `docs/content/` - 8 files updated to remove scrum-compact-telegram references

## Decisions Made
- Bridge selection in interactive wizard uses cliclack::select with display_name and description from BridgeDef, plus "No bridge" option
- Bridge is recorded in team botminter.yml before initial commit so it's part of the repo from the start
- Hire token prompt only appears for external bridges in interactive mode (stdin.is_terminal() check)
- Token storage errors are handled gracefully with env var fallback guidance
- scrum-compact-telegram deleted without migration per Alpha policy (operators recreate teams)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Removed untracked bridge_sync.rs test file**
- **Found during:** Task 1 (compilation phase)
- **Issue:** An untracked bridge_sync.rs test file referenced bm::bridge_sync module that didn't exist yet, blocking compilation
- **Fix:** Determined it was created by a concurrent Plan 03 execution and was later auto-fixed; cleaned cargo cache to resolve stale compilation artifacts
- **Files modified:** None (file was untracked/transient)
- **Verification:** Full test suite passes
- **Committed in:** N/A (transient issue)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Transient build issue from concurrent plan execution. No scope creep.

## Issues Encountered
- The `profiles init --force` command does not remove profiles that were deleted from embedded source -- operators must manually delete stale profiles from ~/.config/botminter/profiles/ after upgrading. This is acceptable under Alpha policy.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Init wizard bridge selection complete, ready for Plan 03 (sync --bridge provisioning, per-member credential resolution)
- Hire token prompt wired, credentials flow through CredentialStore to keyring
- Profile cleanup complete, Telegram is now purely a bridge option

---
*Phase: 09-profile-integration-cleanup*
*Completed: 2026-03-08*
