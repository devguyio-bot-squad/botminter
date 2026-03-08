---
phase: 01-coding-agent-agnostic
plan: 05
subsystem: testing
tags: [e2e, github-api, bm-init, non-interactive, integration-test]

# Dependency graph
requires:
  - phase: 01-coding-agent-agnostic
    provides: "bm init --non-interactive with --skip-github (plan 03)"
provides:
  - "Real E2E test for bm init --non-interactive against GitHub API"
  - "Pattern for TempRepo RAII cleanup without pre-creating the repo"
affects: [e2e-tests, init-command]

# Tech tracking
tech-stack:
  added: []
  patterns: ["TempRepo struct construction for cleanup-only RAII (no pre-creation)"]

key-files:
  created: []
  modified:
    - crates/bm/tests/e2e/init_to_sync.rs

key-decisions:
  - "Used direct TempRepo struct construction for RAII cleanup instead of TempRepo::new_in_org (which pre-creates the repo conflicting with bm init)"
  - "Removed roles/ directory assertion since extract_profile_to intentionally skips roles/ (extracted on demand via extract_member_to)"
  - "Omitted --project flag since non-interactive mode always creates a new Project board automatically"

patterns-established:
  - "TempRepo cleanup-only pattern: construct TempRepo struct directly with full_name to get drop cleanup without pre-creating the repo"

requirements-completed: [CAA-05]

# Metrics
duration: 5min
completed: 2026-03-05
---

# Phase 01 Plan 05: Real E2E Init Test Summary

**Real E2E test exercising `bm init --non-interactive` against GitHub API with repo creation, label bootstrap, Project board creation, and team repo push**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-05T05:54:46Z
- **Completed:** 2026-03-05T05:59:23Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Added `e2e_init_non_interactive_full_github` test that exercises the actual user flow end-to-end
- Test creates a real GitHub repo, bootstraps labels, creates a Project board, pushes team repo content
- Verifies labels on GitHub, config.yml correctness, team repo structure, and git remote URL
- Automatic cleanup via TempRepo RAII (repo deletion) and manual project board cleanup

## Task Commits

Each task was committed atomically:

1. **Task 1: Add e2e_init_non_interactive_full_github test** - `4685322` (test)

## Files Created/Modified
- `crates/bm/tests/e2e/init_to_sync.rs` - Added e2e_init_non_interactive_full_github test function

## Decisions Made
- Used direct TempRepo struct construction for RAII cleanup instead of TempRepo::new_in_org, since the latter pre-creates the repo which conflicts with bm init creating it
- Removed the roles/ directory assertion from the plan since extract_profile_to intentionally excludes roles/ (role skeletons are extracted on demand via extract_member_to)
- Omitted the --project new flag since non-interactive mode always creates a new Project board automatically (--project is for fork URLs, not board selection)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed roles/ directory assertion**
- **Found during:** Task 1 (test implementation)
- **Issue:** Plan specified asserting `team_repo.join("roles").is_dir()` but `extract_profile_to` intentionally skips roles/ directory (extracted on demand via `extract_member_to`)
- **Fix:** Removed the roles/ assertion, added clarifying comment
- **Files modified:** crates/bm/tests/e2e/init_to_sync.rs
- **Verification:** Test passes with correct assertions
- **Committed in:** 4685322

**2. [Rule 1 - Bug] Used direct TempRepo construction instead of TempRepo::new_in_org**
- **Found during:** Task 1 (test implementation)
- **Issue:** Plan used TempRepo::new_in_org to pre-create the repo, but bm init also creates the repo, causing "Name already exists" conflict
- **Fix:** Construct TempRepo struct directly with the generated full_name for cleanup-only RAII
- **Files modified:** crates/bm/tests/e2e/init_to_sync.rs
- **Verification:** Test passes, repo is created by bm init and cleaned up by TempRepo drop
- **Committed in:** 4685322

**3. [Rule 1 - Bug] Removed --project new flag**
- **Found during:** Task 1 (test implementation)
- **Issue:** Plan suggested `--project new` but the CLI's --project flag is for fork URLs, not project board selection. Non-interactive mode always creates a new board automatically.
- **Fix:** Omitted the --project flag entirely
- **Files modified:** crates/bm/tests/e2e/init_to_sync.rs
- **Verification:** Test passes, project board is created automatically
- **Committed in:** 4685322

---

**Total deviations:** 3 auto-fixed (3 bugs in plan specification)
**Impact on plan:** All fixes were necessary for the test to work correctly against the actual CLI behavior. No scope creep.

## Issues Encountered
None beyond the plan specification bugs documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All E2E tests pass including the new real GitHub API test
- Gap closure for Phase 01 is complete (plans 04 and 05)
- Ready for UAT re-validation

---
*Phase: 01-coding-agent-agnostic*
*Completed: 2026-03-05*
