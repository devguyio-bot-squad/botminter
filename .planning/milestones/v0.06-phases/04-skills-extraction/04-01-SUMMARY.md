---
phase: 04-skills-extraction
plan: 01
subsystem: infra
tags: [skills, board-scanner, status-workflow, ralph-prompts, profile-restructuring]

requires:
  - phase: 01-coding-agent-agnostic
    provides: coding-agent/ directory structure in profiles
provides:
  - Board-scanner skill migrated to composable SKILL.md format
  - Ralph prompts shipped to ralph-prompts/ directory across all profiles
  - Status-workflow skill for status transitions and issue operations
  - gh skill with scripts and references
  - Profile directory restructuring with team-level and role-level skills
affects: [05-team-manager-chat, 06-minty]

tech-stack:
  added: []
  patterns: [composable-skills, auto-inject-skills, skill-scoping]

key-files:
  created:
    - profiles/scrum/ralph-prompts/guardrails.md
    - profiles/scrum/ralph-prompts/orientation.md
    - profiles/scrum/ralph-prompts/hat-template.md
    - profiles/scrum/ralph-prompts/reference/
    - profiles/scrum/coding-agent/skills/status-workflow/SKILL.md
    - profiles/scrum/coding-agent/skills/gh/SKILL.md
    - profiles/scrum/coding-agent/skills/gh/scripts/
    - profiles/scrum/coding-agent/skills/gh/references/
  modified:
    - profiles/scrum-compact/coding-agent/skills/board-scanner/SKILL.md
    - profiles/scrum/roles/architect/coding-agent/skills/board-scanner/SKILL.md

key-decisions:
  - "Skills as SKILL.md files discoverable by coding agent — composable, not hard-wired"
  - "Board-scanner auto-inject via ralph.yml overrides — special skill that bootstraps all others"
  - "Ralph prompts as reference material in ralph-prompts/ — not inlined into hats, available for chat"
  - "Two-level skill scoping: team-level (shared) and role-level (specialized)"

patterns-established:
  - "Composable skills: SKILL.md with YAML frontmatter (name, description, metadata) in coding-agent/skills/"
  - "Auto-inject pattern: ralph.yml overrides.board-scanner.auto_inject: true"
  - "Skills dirs config in ralph.yml: multiple scope directories for skill discovery"
  - "gh skill with scripts/ and references/ subdirectories for complex operations"

one-liner: "Board-scanner skill migration, ralph-prompts/ shipping, status-workflow and gh skills with two-level scoping"

requirements-completed: [SKIL-01, SKIL-02, SKIL-03, SKIL-04, SKIL-05]

completed: 2026-03-04
---

# Phase 4: Skills Extraction Summary

**Board-scanner skill migrated to composable SKILL.md format, Ralph prompts extracted to ralph-prompts/ directory, status-workflow and gh skills with scripts and references, profile directory restructured with two-level skill scoping**

## Performance

- **Tasks:** 3 (steps 13-15 from original plan)
- **Files modified:** ~30 (across all 3 profiles)

## Accomplishments
- Migrated board-scanner from embedded hat instructions to composable `SKILL.md` format (197 lines) with GitHub Projects v2 dispatch, auto-advance, priority tables, and error handling
- Created `ralph-prompts/` directory across all profiles with guardrails.md, orientation.md, hat-template.md, and reference/ subdirectory (workflows, event-writing, completion, ralph-tools, robot-interaction)
- Built status-workflow skill (175 lines) — composable skill for GitHub Projects v2 status field mutations, GraphQL verification, comment attribution, label operations
- Formalized pre-existing gh skill (design notes "already extracted") with SKILL.md + 10 shell scripts + 5 reference docs
- Established two-level skill scoping: team-level shared skills in `profiles/<name>/coding-agent/skills/` and role-level specialized skills in `profiles/<name>/roles/<role>/coding-agent/skills/`
- Configured `ralph.yml` skills dirs to discover from multiple scopes (team, project, member)
- Sprint 4 documentation updated

## Files Created/Modified
- `profiles/scrum/ralph-prompts/` — Guardrails, orientation, hat template, 5 reference docs
- `profiles/scrum/coding-agent/skills/status-workflow/SKILL.md` — Status transition skill
- `profiles/scrum/coding-agent/skills/gh/` — SKILL.md, 10 scripts, 5 reference docs
- `profiles/scrum-compact/coding-agent/skills/board-scanner/SKILL.md` — Migrated board scanner
- `profiles/scrum/roles/*/coding-agent/skills/board-scanner/SKILL.md` — Role-specific overrides
- `profiles/scrum-compact/ralph-prompts/` — Same structure
- `profiles/scrum-compact-telegram/ralph-prompts/` — Same structure

## Decisions Made
- Skills as discoverable SKILL.md files rather than hard-wired hat instructions — enables interactive sessions to access the same capabilities
- Board-scanner auto-inject pattern — special skill bootstrapped by ralph.yml override, dispatches work to other hats
- Ralph prompts as reference material (not inlined) — keeps hat instructions focused, provides depth when needed
- gh skill with shell scripts — complex GitHub operations benefit from dedicated scripts vs inline markdown

## Deviations from Plan

### Scope notes

**1. gh skill described as "Already extracted" in design**
- Design.md line 517 marks the gh skill as "Already extracted" / "Already a shared skill"
- Sprint 4 formalized its structure (SKILL.md, scripts/, references/) but did not create it from scratch
- The 10 scripts and 5 reference docs may have been enhanced during this sprint

**2. Step 13 completed early, reducing Step 15 scope**
- Board-scanner migration (Step 13) was completed before the rest of Sprint 4
- This reduced Step 15's scope — board scanning logic was already extracted

## Next Phase Readiness
- Skills infrastructure complete, ready for Team Manager role (which uses these skills)
- Ralph prompts available for `bm chat` meta-prompt reference section
- Board-scanner skill provides the dispatch mechanism for all roles

---
*Phase: 04-skills-extraction*
*Completed: 2026-03-04*
