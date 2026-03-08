---
phase: 01-coding-agent-agnostic
plan: 10
subsystem: testing
tags: [e2e, github-api, rate-limiting, libtest-mimic, test-suites]

# Dependency graph
requires:
  - phase: 01-coding-agent-agnostic (plan 09)
    provides: custom E2E test harness with libtest-mimic
provides:
  - GithubSuite abstraction for shared TempRepo test suites
  - team_lifecycle suite (5 tests sharing 1 repo)
  - daemon_basic suite (5 tests sharing 1 repo)
  - ~45% reduction in TempRepo creations per E2E run
affects: [e2e-tests, github-api-usage]

# Tech tracking
tech-stack:
  added: []
  patterns: [GithubSuite builder pattern, SuiteCtx shared context, per-case TempDir isolation]

key-files:
  created: []
  modified:
    - crates/bm/tests/e2e/helpers.rs
    - crates/bm/tests/e2e/init_to_sync.rs
    - crates/bm/tests/e2e/daemon_lifecycle.rs

key-decisions:
  - "Used closure-based builder pattern for GithubSuite (new/setup/case/build) producing a single libtest-mimic Trial"
  - "Kept project_ops tests isolated since TempProject savings is only 2 API calls vs complexity cost"
  - "Used per-case TempDir in daemon_basic for filesystem isolation while sharing the GitHub repo"

patterns-established:
  - "GithubSuite pattern: tests that only read repo state can share a TempRepo via suite"
  - "Per-case isolation: daemon tests create local TempDir but share the GitHub repo reference"

requirements-completed: [CAA-05]

# Metrics
duration: 5min
completed: 2026-03-06
---

# Phase 01 Plan 10: E2E Test Suite Optimization Summary

**GithubSuite abstraction reducing TempRepo creations from 19 to ~11 per E2E run via shared-repo test suites**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-06T11:29:52Z
- **Completed:** 2026-03-06T11:35:16Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added GithubSuite struct with builder pattern (new/setup/case/build) and SuiteCtx shared context
- Refactored 5 init_to_sync tests into team_lifecycle suite sharing 1 TempRepo
- Refactored 5 daemon tests into daemon_basic suite sharing 1 TempRepo with per-case filesystem isolation
- Reduced TempRepo creations from 19 to ~11 per E2E run (~45% reduction, saving ~16 API calls)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add GithubSuite abstraction and team_lifecycle suite** - `7b1ae37` (feat)
2. **Task 2: Add daemon_basic suite and verify repo count** - `b3425c5` (feat)

## Files Created/Modified
- `crates/bm/tests/e2e/helpers.rs` - Added GithubSuite, SuiteCtx structs and panic_to_string helper
- `crates/bm/tests/e2e/init_to_sync.rs` - Refactored 5 tests into team_lifecycle suite, kept 7 isolated
- `crates/bm/tests/e2e/daemon_lifecycle.rs` - Refactored 5 tests into daemon_basic suite, kept 3 isolated

## Decisions Made
- Used closure-based builder pattern for GithubSuite producing a single libtest-mimic Trial, keeping the interface simple
- Kept project_ops tests (list_gh_projects, sync_status_on_existing_project) as isolated tests since they use TempProject not TempRepo and savings is minimal (2 API calls)
- Used per-case TempDir in daemon_basic suite for filesystem isolation (each daemon test needs its own workspace, stub ralph, PID files) while sharing the GitHub repo reference

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- GithubSuite pattern can be reused for future test grouping
- E2E test suite is more rate-limit friendly for iterative development
- All 454 unit/integration tests continue to pass

## Self-Check: PASSED

- All 3 modified files exist on disk
- Both task commits verified (7b1ae37, b3425c5)
- GithubSuite struct present in helpers.rs
- team_lifecycle suite present in init_to_sync.rs
- daemon_basic suite present in daemon_lifecycle.rs
- 454 unit/integration tests passing, clippy clean

---
*Phase: 01-coding-agent-agnostic*
*Completed: 2026-03-06*
