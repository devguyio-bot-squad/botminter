---
phase: 01-coding-agent-agnostic
plan: 09
subsystem: testing
tags: [libtest-mimic, e2e, custom-harness, github-api, daemon]

requires:
  - phase: 01-coding-agent-agnostic (plan 08)
    provides: unconditional test assertions and daemon test namesake fixes
provides:
  - custom E2E test harness with mandatory --gh-token and --gh-org CLI args
  - real GitHub repo daemon tests with unconditional member lifecycle assertions
  - ephemeral TempRepo per test replacing persistent shared repo pattern
  - Justfile recipes with TESTS_GH_TOKEN/TESTS_GH_ORG env var validation
affects: [e2e-tests, ci-pipeline, daemon-lifecycle]

tech-stack:
  added: [libtest-mimic 0.8]
  patterns: [custom-harness-cli-args, catch-unwind-to-error, temprepo-per-test]

key-files:
  created: []
  modified:
    - crates/bm/Cargo.toml
    - crates/bm/tests/e2e/main.rs
    - crates/bm/tests/e2e/helpers.rs
    - crates/bm/tests/e2e/daemon_lifecycle.rs
    - crates/bm/tests/e2e/github.rs
    - crates/bm/tests/e2e/init_to_sync.rs
    - crates/bm/tests/e2e/start_to_stop.rs
    - crates/bm/tests/e2e/telegram.rs
    - Justfile

key-decisions:
  - "Used libtest-mimic with custom arg extraction before passing remaining args to harness"
  - "Used catch_unwind in run_test helper to convert panics to libtest-mimic errors"
  - "Replaced persistent shared repo with ephemeral TempRepo per test for isolation"
  - "Removed gh_auth_ok, require_gh_auth macro, PERSISTENT_REPO, E2E_ORG constants"
  - "DaemonGuard now accepts optional gh_token for proper cleanup"

patterns-established:
  - "E2eConfig struct: all E2E test configuration flows through a single config passed from CLI"
  - "Trial::test with move closures: each test clones config and runs via run_test wrapper"
  - "TempRepo per test: each test creates its own ephemeral repo, no shared mutable state"

requirements-completed: [CAA-05, CAA-06]

duration: 9min
completed: 2026-03-05
---

# Phase 01 Plan 09: Custom E2E Test Harness Summary

**Custom libtest-mimic E2E harness with mandatory --gh-token/--gh-org CLI args, real GitHub daemon tests with unconditional member lifecycle assertions**

## Performance

- **Duration:** 9 min
- **Started:** 2026-03-05T11:40:06Z
- **Completed:** 2026-03-05T11:49:18Z
- **Tasks:** 5
- **Files modified:** 9

## Accomplishments
- Built custom E2E test harness using libtest-mimic that accepts --gh-token and --gh-org as mandatory CLI arguments
- Rewrote all daemon tests to use real GitHub repos (TempRepo) with real tokens and unconditional assertions
- Eliminated persistent shared repo pattern, hardcoded org names, fake tokens, and conditional fallbacks
- All 10 static verification checks pass (no fake repos/tokens, no conditionals, no hardcoded orgs)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add libtest-mimic dependency and set harness = false** - `06cd9a8` (chore)
2. **Task 2: Rewrite E2E main.rs as custom test harness** - `d209af7` (feat)
3. **Task 3: Convert all modules to harness format, parameterize org** - `97803ff` (feat)
4. **Task 4: Rewrite daemon tests to use real GitHub repos** - `1d857b7` (feat)
5. **Task 5: Update Justfile and verify** - `edc9fd3` (feat)

**Cleanup:** `7ad79e8` (chore: suppress dead_code warning on TempRepo::new)

## Files Created/Modified
- `crates/bm/Cargo.toml` - Added libtest-mimic dev-dependency, set harness = false
- `crates/bm/tests/e2e/main.rs` - Custom harness with --gh-token/--gh-org arg parsing
- `crates/bm/tests/e2e/helpers.rs` - E2eConfig struct, run_test helper, DaemonGuard with gh_token
- `crates/bm/tests/e2e/daemon_lifecycle.rs` - Real GitHub repo daemon tests with unconditional assertions
- `crates/bm/tests/e2e/github.rs` - Removed PERSISTENT_REPO, clean_persistent_repo, gh_auth_ok
- `crates/bm/tests/e2e/init_to_sync.rs` - Converted to harness format, TempRepo per test
- `crates/bm/tests/e2e/start_to_stop.rs` - Converted to harness format, real token from config
- `crates/bm/tests/e2e/telegram.rs` - Converted to harness format
- `Justfile` - E2E recipes with TESTS_GH_TOKEN/TESTS_GH_ORG validation

## Decisions Made
- Used libtest-mimic for custom harness (lightweight, supports standard test filtering)
- Used catch_unwind in run_test helper so existing assert! calls work unchanged
- Replaced all persistent/shared repo usage with ephemeral TempRepo per test
- Removed gh_auth_ok/require_gh_auth since --gh-token CLI arg is the sole gate
- Added gh_token to DaemonGuard so drop() can pass GH_TOKEN to stop command

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- E2E test suite fully migrated to custom harness with real GitHub infrastructure
- UAT gaps #12 (daemon namesake claims) and #13 (runtime dependency gate) closed
- Ready for full E2E suite execution with real GitHub token via `just e2e`

---
*Phase: 01-coding-agent-agnostic*
*Completed: 2026-03-05*
