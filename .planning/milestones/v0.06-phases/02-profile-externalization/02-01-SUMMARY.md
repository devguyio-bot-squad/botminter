---
phase: 02-profile-externalization
plan: 01
subsystem: infra
tags: [profiles-init, disk-profiles, auto-prompt, ensure-initialized]

requires:
  - phase: 01-coding-agent-agnostic
    provides: CodingAgentDef model and unified-to-agent-specific extraction
provides:
  - bm profiles init command with --force flag
  - Disk-based profile API at ~/.config/botminter/profiles/
  - Auto-prompt pattern via ensure_profiles_initialized()
  - Two extraction modes: verbatim (profiles init to disk) vs unified-to-agent-specific (bm init to team repo)
affects: [03-workspace-repository, 05-team-manager-chat, 06-minty]

tech-stack:
  added: []
  patterns: [auto-prompt-initialization, two-layer-profile-model, verbatim-vs-filtered-extraction]

key-files:
  created:
    - crates/bm/src/commands/profiles_init.rs
  modified:
    - crates/bm/src/profile.rs
    - crates/bm/src/commands/profiles.rs
    - crates/bm/src/cli.rs

key-decisions:
  - "Two-layer model: embedded profiles for bootstrap, disk profiles for runtime editability"
  - "Verbatim extraction for profiles init (no filtering) vs filtered extraction for bm init/hire"
  - "Auto-prompt pattern: ensure_profiles_initialized() gates profile-reading commands"
  - "Per-profile overwrite prompting with --force flag for scripted use"

patterns-established:
  - "Auto-prompt initialization: commands that read profiles call ensure_profiles_initialized() first"
  - "Two extraction modes: verbatim (to disk) and filtered (to team repos)"

one-liner: "bm profiles init extracting to ~/.config/botminter/profiles/, disk-based profile API, and auto-prompt initialization pattern"

requirements-completed: [PROF-01, PROF-02, PROF-03, PROF-04, PROF-05]

completed: 2026-03-04
---

# Phase 2: Profile Externalization Completion Summary

**bm profiles init command extracting embedded profiles to ~/.config/botminter/profiles/, disk-based profile API replacing embedded reads, and auto-prompt initialization pattern for seamless first-run experience**

## Performance

- **Tasks:** 3 (steps 7-9 from original plan)
- **Files modified:** ~10

## Accomplishments
- Built `profiles_init.rs` (520 lines) — `bm profiles init [--force]` extracts all embedded profiles to disk
- Implemented per-profile overwrite prompting (TTY) with `--force` flag for non-interactive use
- Converted all profile operations to read from `~/.config/botminter/profiles/` via disk-based API
- Added `ensure_profiles_initialized()` auto-prompt gate — detects missing profiles, offers inline init (TTY) or auto-inits (non-TTY)
- Built embedded module with `include_dir!` for compile-time profile embedding and `extract_embedded_to_disk()` for verbatim extraction
- Added `profiles_dir()` and `profiles_dir_for()` path resolution helpers
- Added `check_schema_version()` to guard against drift between disk profiles and team repos
- Sprint 2 documentation updated

## Files Created/Modified
- `crates/bm/src/commands/profiles_init.rs` — Full init command with overwrite control, force flag, Minty co-extraction
- `crates/bm/src/profile.rs` — Disk-based API (profiles_dir, list_profiles, read_manifest, list_roles), embedded module, ensure_profiles_initialized(), check_schema_version()
- `crates/bm/src/commands/profiles.rs` — Updated list() and describe() to use disk-based API with auto-init gate
- `crates/bm/src/cli.rs` — Added ProfilesCommand::Init with force flag

## Decisions Made
- Two-layer model (embedded + disk) — embedded provides reliable bootstrap, disk enables customization without rebuilding
- Verbatim extraction for `profiles init` — users get exact profile source for editing; agent tag filtering only applies during team repo extraction
- Auto-prompt over hard failure — new users get guided init instead of cryptic "profiles not found" errors
- Minty config always co-extracted alongside profiles — single init step for everything

## Deviations from Plan

### Additions beyond plan scope

**1. Minty config co-extraction**
- **Issue:** `bm profiles init` also extracts Minty config (Sprint 6 scope) alongside profiles
- **Rationale:** Single init step for everything; natural extension of the extraction command
- **Impact:** Forward-pulled Sprint 6 dependency; no scope creep on Sprint 2 deliverables

**2. `check_schema_version()` function**
- **Issue:** Added schema version checking between disk profiles and team repos (not in plan Steps 7-9)
- **Rationale:** Prevents drift between edited disk profiles and generated team repos
- **Impact:** Defensive addition; no scope creep

## Next Phase Readiness
- Disk-based profile API complete, all downstream commands use it
- Auto-prompt pattern ensures profiles exist before any profile-reading operation
- Ready for workspace repository model changes

---
*Phase: 02-profile-externalization*
*Completed: 2026-03-04*
