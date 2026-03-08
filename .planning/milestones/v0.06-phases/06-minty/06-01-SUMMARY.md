---
phase: 06-minty
plan: 01
subsystem: cli
tags: [minty, bm-minty, skills, hire-guide, profile-browser, team-overview, workspace-doctor]

requires:
  - phase: 02-profile-externalization
    provides: Disk-based profile API and ensure_profiles_initialized() pattern
  - phase: 05-team-manager-chat
    provides: exec() pattern for launching coding agent sessions
provides:
  - bm minty command launching BotMinter interactive assistant
  - Minty config structure at ~/.config/botminter/minty/
  - 4 Minty skills (hire-guide, profile-browser, team-overview, workspace-doctor)
  - Auto-initialization of Minty config from embedded data
  - Profiles-only mode for users without configured teams
affects: []

tech-stack:
  added: []
  patterns: [thin-persona-shell, skill-driven-assistant, profiles-only-mode, auto-init]

key-files:
  created:
    - crates/bm/src/commands/minty.rs
    - minty/config.yml
    - minty/prompt.md
    - minty/skills/hire-guide/SKILL.md
    - minty/skills/profile-browser/SKILL.md
    - minty/skills/team-overview/SKILL.md
    - minty/skills/workspace-doctor/SKILL.md
  modified:
    - crates/bm/src/profile.rs
    - crates/bm/src/cli.rs
    - crates/bm/src/commands/mod.rs
    - crates/bm/src/commands/profiles_init.rs

key-decisions:
  - "Thin persona shell — Minty's capabilities come entirely from skills, not baked into prompt"
  - "Minty is NOT a team member and NOT a Ralph instance — separate entity for operator interaction"
  - "Profiles-only mode — Minty works even without any teams configured"
  - "Auto-init from embedded data — ensure_minty_initialized() extracts if prompt.md missing"
  - "Skills use YAML frontmatter with trigger phrases for discoverability"

patterns-established:
  - "Thin persona pattern: identity prompt + composable skills = assistant"
  - "ensure_minty_initialized() auto-init following same pattern as ensure_profiles_initialized()"
  - "Minty skills: YAML frontmatter with name, description, metadata (author, version, category, tags)"
  - "Profiles-only mode detection: no ~/.botminter/ config means first-time user experience"

one-liner: "bm minty interactive assistant with thin persona shell, 4 composable skills, and profiles-only mode"

requirements-completed: [MNTY-01, MNTY-02, MNTY-03, MNTY-04]

completed: 2026-03-04
---

# Phase 6: Minty Summary

**bm minty command launching BotMinter's interactive assistant with thin persona shell, 4 composable skills (hire-guide, profile-browser, team-overview, workspace-doctor), auto-initialization, and profiles-only mode for new users**

## Performance

- **Tasks:** 2 (steps 19-20 from original plan)
- **Files modified:** ~12

## Accomplishments
- Implemented `bm minty [-t team]` command (191 lines) — resolves coding agent, ensures Minty initialized, execs with system prompt
- Built `ensure_minty_initialized()` — auto-extracts embedded Minty config to `~/.config/botminter/minty/` if `prompt.md` missing
- Built `resolve_agent_from_profiles()` — iterates disk profiles to find default coding agent binary when no team specified
- Created Minty persona prompt (40 lines) — thin shell defining Minty as "the friendly interactive assistant for BotMinter operators"
- Built 4 composable Minty skills:
  - **team-overview** — shows teams, members, roles, workspaces, running state from `~/.botminter/config.yml`
  - **profile-browser** — browses profiles from disk, shows roles/statuses/agents, supports comparison
  - **hire-guide** — interactive hiring walkthrough with role selection, name conventions, edge case handling
  - **workspace-doctor** — diagnoses common workspace issues (stale submodules, broken symlinks, missing files) per design spec
- Implemented profiles-only mode — Minty detects absence of `~/.botminter/` config and adjusts behavior for first-time users
- Minty config embedded via `include_dir!` in `profile.rs` and co-extracted by `bm profiles init`
- Sprint 6 documentation updated

## Files Created/Modified
- `crates/bm/src/commands/minty.rs` — Minty command: ensure_minty_initialized(), resolve_agent_from_profiles(), profiles-only mode, exec
- `minty/config.yml` — Minty configuration (prompt path, skills dir)
- `minty/prompt.md` — Thin persona prompt defining Minty's identity and behavioral guidelines
- `minty/skills/team-overview/SKILL.md` — Team and member status display
- `minty/skills/profile-browser/SKILL.md` — Profile browsing and comparison
- `minty/skills/hire-guide/SKILL.md` — Interactive hiring walkthrough
- `minty/skills/workspace-doctor/SKILL.md` — Workspace diagnostic checks
- `crates/bm/src/profile.rs` — minty_embedded module with include_dir! and extract_minty_to_disk()
- `crates/bm/src/commands/profiles_init.rs` — Co-extracts Minty alongside profiles

## Decisions Made
- Thin persona shell — Minty's prompt defines only identity and style; all capabilities come from skills, making it extensible without prompt changes
- Minty is explicitly NOT a team member or Ralph instance — avoids the complexity of orchestration for what's fundamentally a help/guidance tool
- Profiles-only mode — new users who run `bm minty` before `bm init` still get value (profile browsing, hire guidance)
- Auto-init follows the same pattern as profile externalization — seamless first-run experience

## Deviations from Plan
None — followed plan as specified.

## Next Phase Readiness
- Milestone complete — all 6 phases delivered
- 91 tests passing, cargo clippy clean
- Ready for milestone completion and next milestone planning

---
*Phase: 06-minty*
*Completed: 2026-03-04*
