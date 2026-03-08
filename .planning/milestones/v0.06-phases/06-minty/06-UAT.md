---
status: complete
phase: 06-minty
source: 06-01-SUMMARY.md, 06-02-SUMMARY.md
started: 2026-03-08T12:30:00Z
updated: 2026-03-08T13:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Build & Clippy Clean After Gap Closure
expected: Run `just build` and `just clippy` from the repo root. Both complete without errors. No warnings related to minty, ensure_minty_initialized, or resolve_agent_from_profiles.
result: pass

### 2. Minty Auto-Initialization Sets CWD (re-verify fix)
expected: Delete `~/.config/botminter/minty/` if it exists, then run `bm minty`. The directory `~/.config/botminter/minty/` is created with `prompt.md`, `config.yml`, and `.claude/skills/` extracted. Claude Code launches with CWD set to `~/.config/botminter/minty/` (not the caller's directory).
result: pass

### 3. Skills at .claude/skills/ Path (re-verify fix)
expected: After `bm minty` triggers extraction, check `~/.config/botminter/minty/.claude/skills/`. Four skill directories exist: hire-guide, profile-browser, team-overview, workspace-doctor. Each contains a SKILL.md. The old path `~/.config/botminter/minty/skills/` does NOT exist.
result: pass

### 4. Profiles-Only Mode Without Teams (re-verify fix)
expected: With no `~/.botminter/config.yml` present (no teams configured), `bm minty` should launch successfully in profiles-only mode with CWD set to `~/.config/botminter/minty/`. Minty should detect absence of team configuration and offer profile browsing and hire guidance. Skills should be discoverable at `.claude/skills/`.
result: pass

### 5. Unit Tests Pass After Gap Closure
expected: Run `just test`. All existing tests pass including minty-related tests (ensure_minty_initialized, resolve_agent_from_profiles, skills path assertions).
result: pass

## Summary

total: 5
passed: 5
issues: 0
pending: 0
skipped: 0

## Gaps

[none yet]
