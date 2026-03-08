---
phase: 01-coding-agent-agnostic
plan: 02
subsystem: testing
tags: [test-isolation, cli, tempdir, user-facing-labels]

# Dependency graph
requires:
  - phase: 01-coding-agent-agnostic
    provides: "Initial agent tag implementation with cli_parsing tests"
provides:
  - "Isolated cli_parsing tests that never pollute real HOME"
  - "User-friendly 'Coding-Agent Dependent Files' label in show-tags output"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns: ["bm(home: &Path) helper pattern for subprocess test isolation"]

key-files:
  created: []
  modified:
    - crates/bm/tests/cli_parsing.rs
    - crates/bm/src/commands/profiles.rs

key-decisions:
  - "Adopted same bm_cmd(home) pattern used in integration.rs for cli_parsing.rs consistency"

patterns-established:
  - "bm(home) helper: All subprocess tests must use isolated HOME via tempfile::tempdir()"

requirements-completed: [CAA-05, CAA-06]

# Metrics
duration: 15min
completed: 2026-03-05
---

# Phase 1 Plan 02: Fix Test Path Isolation + Rename Show-Tags Label Summary

**Isolated all cli_parsing.rs tests with temp HOME directories and renamed show-tags output to user-friendly "Coding-Agent Dependent Files" label**

## Performance

- **Duration:** 15 min
- **Started:** 2026-03-05T03:34:20Z
- **Completed:** 2026-03-05T03:50:05Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- All 49 cli_parsing tests now use isolated temp HOME directories via `bm(tmp.path())` pattern
- Running cli_parsing tests no longer creates `~/.config/botminter` on the real filesystem
- `bm profiles describe scrum --show-tags` output now shows "Coding-Agent Dependent Files" instead of internal "Agent Tags"

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix cli_parsing.rs test path isolation** - `7677884` (fix)
2. **Task 2: Rename show-tags label to user-friendly text** - `228944c` (feat)

## Files Created/Modified
- `crates/bm/tests/cli_parsing.rs` - Changed bm() helper to accept &Path for HOME isolation, updated all 49 test call sites
- `crates/bm/src/commands/profiles.rs` - Changed "Agent Tags" label to "Coding-Agent Dependent Files"

## Decisions Made
- Adopted the same `bm_cmd(home: &Path)` pattern from integration.rs for consistency across all test files

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 1 Plan 03 (profile staleness detection) can proceed
- All 91 tests passing, clippy clean

---
*Phase: 01-coding-agent-agnostic*
*Completed: 2026-03-05*
