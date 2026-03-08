---
phase: 01-coding-agent-agnostic
plan: 03
subsystem: cli
tags: [profile, staleness-detection, non-interactive, init, testing]

# Dependency graph
requires:
  - phase: 01-coding-agent-agnostic (plan 01)
    provides: "Profile extraction, CodingAgentDef, agent tags"
provides:
  - "Profile staleness detection via .profiles_version marker"
  - "bm init --non-interactive for scripted team creation"
  - "--skip-github flag for testing init without network"
affects: [e2e-testing, deployment-automation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Version marker file for embedded resource staleness detection"
    - "Non-interactive CLI mode with --skip-github for test isolation"

key-files:
  created: []
  modified:
    - "crates/bm/src/profile.rs"
    - "crates/bm/src/cli.rs"
    - "crates/bm/src/main.rs"
    - "crates/bm/src/commands/init.rs"
    - "crates/bm/src/completions.rs"
    - "crates/bm/tests/integration.rs"

key-decisions:
  - "Used PROFILES_FORMAT_VERSION constant (manually bumped) instead of content hash for simplicity"
  - "Added --skip-github hidden flag to enable testing init without GitHub API"
  - "Placed init smoke tests in integration.rs (not e2e/) since they need no network"

patterns-established:
  - "Version marker pattern: write .profiles_version alongside extracted profiles for staleness checks"
  - "Non-interactive CLI pattern: --non-interactive flag with required sub-args and --skip-github for tests"

requirements-completed: [CAA-03]

# Metrics
duration: 28min
completed: 2026-03-05
---

# Phase 1 Plan 3: Profile Staleness Detection + Non-Interactive Init Summary

**Profile version marker staleness detection auto-re-extracts stale profiles, with --non-interactive init flag for scripted team creation**

## Performance

- **Duration:** 28 min
- **Started:** 2026-03-05T03:34:45Z
- **Completed:** 2026-03-05T04:02:38Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Stale profiles (missing or outdated .profiles_version marker) are now automatically re-extracted from the embedded binary
- `bm init --non-interactive` accepts CLI args for fully scripted team creation without TTY
- `--skip-github` flag enables testing init flow without GitHub API access
- 8 new tests covering staleness detection and non-interactive init scenarios

## Task Commits

Each task was committed atomically:

1. **Task 1: Add version marker staleness detection** - `2d24e65` (feat)
2. **Task 2: Add --non-interactive flag and E2E smoke tests** - `8e45841` (feat)

_Note: TDD tasks had tests and implementation combined in single commits for efficiency_

## Files Created/Modified
- `crates/bm/src/profile.rs` - Added PROFILES_FORMAT_VERSION, PROFILES_VERSION_MARKER, staleness check in ensure_profiles_initialized_with, marker writing in extract_embedded_to_disk, 4 new tests
- `crates/bm/src/cli.rs` - Changed Init from bare variant to struct with --non-interactive, --profile, --team-name, --org, --repo, --project, --skip-github, --workzone flags
- `crates/bm/src/main.rs` - Updated Command::Init dispatch to pass new fields, route to run_non_interactive
- `crates/bm/src/commands/init.rs` - Added run_non_interactive() function and detect_gh_token_non_interactive()
- `crates/bm/src/completions.rs` - Updated Init variant pattern match for struct variant
- `crates/bm/tests/integration.rs` - Added 4 integration tests for non-interactive init

## Decisions Made
- Used a simple format version constant (PROFILES_FORMAT_VERSION = "2") instead of content hashing. Developers bump this when profile structure changes. This is simpler and more predictable than hash-based detection.
- Added `--skip-github` as a hidden flag rather than mocking HTTP, allowing full init testing without network access.
- Integration tests (not e2e-gated) since no real GitHub or external services are needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed completions.rs pattern match for Init variant**
- **Found during:** Task 2 (--non-interactive flag)
- **Issue:** Changing Init from bare to struct variant broke completions.rs variant exhaustiveness check
- **Fix:** Updated `Command::Init` to `Command::Init { .. }` in completions.rs
- **Files modified:** crates/bm/src/completions.rs
- **Verification:** cargo build succeeds
- **Committed in:** 8e45841 (Task 2 commit)

**2. [Rule 3 - Blocking] Fixed integration test git identity for init**
- **Found during:** Task 2 (E2E smoke test)
- **Issue:** git commit failed in test subprocess because no git user.name/email configured in isolated HOME
- **Fix:** Added GIT_AUTHOR_NAME/EMAIL and GIT_COMMITTER_NAME/EMAIL env vars to test commands
- **Files modified:** crates/bm/tests/integration.rs
- **Verification:** All 4 tests pass
- **Committed in:** 8e45841 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes necessary for compilation and test correctness. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 1 (Coding-Agent-Agnostic) gap closure complete -- all 3 UAT gaps addressed
- Ready for Phase 1 UAT re-validation
- All 452 tests passing (308 unit + 49 cli_parsing + 95 integration)
- Clippy clean

---
*Phase: 01-coding-agent-agnostic*
*Completed: 2026-03-05*
