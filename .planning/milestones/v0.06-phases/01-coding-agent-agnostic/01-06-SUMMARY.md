---
phase: 01-coding-agent-agnostic
plan: 06
subsystem: testing
tags: [e2e, profile-isolation, embedded-data, path-isolation]

# Dependency graph
requires: []
provides:
  - "list_embedded_roles function for disk-free role discovery in embedded module"
  - "bootstrap_profiles_to_tmp E2E helper for isolated profile operations"
  - "All E2E tests use embedded data and _from variants instead of real HOME"
affects: [testing, profile]

# Tech tracking
tech-stack:
  added: []
  patterns: [embedded-data-for-test-discovery, profiles-base-parameter-pattern]

key-files:
  modified:
    - "crates/bm/src/profile.rs"
    - "crates/bm/tests/e2e/helpers.rs"
    - "crates/bm/tests/e2e/init_to_sync.rs"
    - "crates/bm/tests/e2e/start_to_stop.rs"
    - "crates/bm/tests/e2e/daemon_lifecycle.rs"

key-decisions:
  - "Used list_embedded_roles with PROFILES.get_dir for disk-free role discovery"
  - "Passed profiles_base as parameter to setup helpers rather than extracting inside each"

patterns-established:
  - "Embedded data pattern: E2E tests use list_embedded_profiles/list_embedded_roles for discovery, never profile::list_profiles/list_roles"
  - "Profiles base pattern: E2E helpers accept profiles_base parameter, each test calls bootstrap_profiles_to_tmp once"

requirements-completed: [CAA-05]

# Metrics
duration: 10min
completed: 2026-03-05
---

# Phase 01 Plan 06: E2E Profile Path Isolation Summary

**Eliminated all real-HOME profile access from E2E tests using embedded data for discovery and _from variants with temp dirs for operations**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-05T07:19:24Z
- **Completed:** 2026-03-05T07:29:30Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Added `list_embedded_roles` function to embedded module for disk-free role discovery
- Added `bootstrap_profiles_to_tmp` shared helper for E2E tests
- Converted all 3 E2E test files (init_to_sync, start_to_stop, daemon_lifecycle) to use embedded data
- All 14 non-GitHub-dependent E2E tests pass in clean environments (previously 0/27 passed)
- Zero calls to real-HOME profile functions remain in E2E test files

## Task Commits

Each task was committed atomically:

1. **Task 1: Add list_embedded_roles and bootstrap helper** - `2c2c9fe` (feat)
2. **Task 2: Fix all E2E test files to use embedded data** - `e11fa30` (fix)

## Files Created/Modified
- `crates/bm/src/profile.rs` - Added list_embedded_roles function to embedded module, re-exported it
- `crates/bm/tests/e2e/helpers.rs` - Added bootstrap_profiles_to_tmp shared helper
- `crates/bm/tests/e2e/init_to_sync.rs` - Replaced 10+ real-HOME calls with embedded/temp variants
- `crates/bm/tests/e2e/start_to_stop.rs` - Replaced find_profile_with_role and read_manifest calls
- `crates/bm/tests/e2e/daemon_lifecycle.rs` - Replaced find_profile_with_role and read_manifest calls

## Decisions Made
- Used `PROFILES.get_dir` for embedded role discovery (consistent with existing list_embedded_profiles pattern)
- Passed `profiles_base` as explicit parameter to helpers rather than extracting inside each helper (avoids redundant extractions)
- Re-exported `list_embedded_roles` alongside existing re-exports for test accessibility

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing workspace directory creation in start_to_stop.rs and daemon_lifecycle.rs**
- **Found during:** Task 2 (fixing E2E test files)
- **Issue:** `setup_workspace_for_start` and `setup_daemon_workspace` called `fs::write` on a non-existent directory path without prior `create_dir_all`
- **Fix:** Added `fs::create_dir_all(&workspace)` before writing workspace marker files
- **Files modified:** start_to_stop.rs, daemon_lifecycle.rs
- **Verification:** All 14 non-GitHub-dependent E2E tests pass
- **Committed in:** e11fa30

**2. [Rule 1 - Bug] Wrong workspace path nesting in start_to_stop.rs**
- **Found during:** Task 2
- **Issue:** Workspace path had extra `/workspace/` level that didn't match `find_workspace` expectation (`team_dir/<member>/` not `team_dir/<member>/workspace/`)
- **Fix:** Removed extra nesting level, workspace now at `team_dir/<member>/`
- **Files modified:** start_to_stop.rs
- **Committed in:** e11fa30

**3. [Rule 1 - Bug] Wrong workspace marker type in daemon_lifecycle.rs**
- **Found during:** Task 2
- **Issue:** Daemon `find_workspace` checks for `.botminter` directory but test created `.botminter.workspace` file
- **Fix:** Changed to create `.botminter` directory matching daemon's expected marker
- **Files modified:** daemon_lifecycle.rs
- **Committed in:** e11fa30

---

**Total deviations:** 3 auto-fixed (3 bugs exposed by fixing profile discovery)
**Impact on plan:** All auto-fixes necessary for test correctness. These were latent bugs only visible now that profile discovery succeeds. No scope creep.

## Issues Encountered
- 8 init_to_sync tests that use `PERSISTENT_REPO` fail due to GitHub environment issue (repo gets deleted by `clean_persistent_repo` but is not recreated before push). This is a pre-existing test environment issue, not caused by this plan's changes. The 4 init_to_sync tests that create their own repos all pass.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- UAT gap #9 fully closed
- All E2E tests are now self-contained for profile operations
- invariants/test-path-isolation.md fully satisfied in E2E tests

## Self-Check: PASSED

All 5 modified files exist. Both task commits (2c2c9fe, e11fa30) verified in git log.

---
*Phase: 01-coding-agent-agnostic*
*Completed: 2026-03-05*
