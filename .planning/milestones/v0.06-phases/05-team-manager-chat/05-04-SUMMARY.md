---
phase: 05-team-manager-chat
plan: 04
subsystem: chat
tags: [skills, meta-prompt, yaml-frontmatter, scanning]

requires:
  - phase: 05-team-manager-chat
    provides: "bm chat meta-prompt rendering with SkillInfo struct (plan 03)"
provides:
  - "SkillsConfig parsing from ralph.yml skills.dirs"
  - "scan_skills() function for reading SKILL.md frontmatter"
  - "Skills table rendering in bm chat meta-prompt"
affects: [team-manager-chat, workspace]

tech-stack:
  added: []
  patterns: [frontmatter-extraction, description-truncation, template-path-skipping]

key-files:
  created: []
  modified:
    - crates/bm/src/commands/chat.rs
    - crates/bm/src/chat.rs

key-decisions:
  - "Used serde_yml for SKILL.md frontmatter parsing (already a dependency)"
  - "Truncate descriptions to first sentence or 120 chars for table readability"
  - "Skip template paths containing <project> placeholder since project context unavailable in chat"
  - "Deduplicate skills by name, keeping first occurrence across dirs"

patterns-established:
  - "Frontmatter extraction: split on --- markers, parse inner YAML"
  - "Description truncation: first sentence boundary or 120 char word-boundary cutoff"

requirements-completed: [CHAT-03]

duration: 5min
completed: 2026-03-07
---

# Phase 05 Plan 04: Skills Table in Chat Meta-Prompt Summary

**Skills scanning from ralph.yml dirs with SKILL.md frontmatter extraction, deduplication, and markdown table rendering in bm chat**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-07T06:38:40Z
- **Completed:** 2026-03-07T06:44:38Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- Added SkillsConfig struct to RalphConfig for parsing skills.enabled and skills.dirs from ralph.yml
- Implemented scan_skills() that reads SKILL.md frontmatter, truncates descriptions, deduplicates by name, and sorts results
- Integrated skills scanning into chat run() to populate MetaPromptParams with real skill data
- Added 14 new tests covering skills parsing, scanning, deduplication, truncation, and rendering

## Task Commits

Each task was committed atomically:

1. **Task 1: Add skills scanning to RalphConfig and skills field to MetaPromptParams** - `d3b9670` (feat)

## Files Created/Modified
- `crates/bm/src/commands/chat.rs` - Added SkillsConfig, SkillFrontmatter, scan_skills(), extract_frontmatter(), truncate_description() + 12 new tests
- `crates/bm/src/chat.rs` - Added skills_table_rendered_when_present and skills_section_omitted_when_empty tests

## Decisions Made
- Used serde_yml for SKILL.md frontmatter parsing since it is already a project dependency
- Truncate descriptions to first sentence (`. ` or `.\n` boundary) or 120 chars with word-boundary cutoff
- Skip template paths containing `<project>` placeholder since project substitution is not available in chat context
- Deduplicate skills by name keeping first occurrence (higher-priority dirs should be listed first in ralph.yml)
- Normalize multiline YAML descriptions (collapse whitespace) before truncation

## Deviations from Plan

None - plan executed exactly as written. The prior commit (05-03) had already added SkillInfo struct, skills field on MetaPromptParams, and the rendering logic. This plan added the scanning infrastructure, integration, and comprehensive tests.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Skills table is now fully functional in bm chat meta-prompt
- UAT re-validation for CHAT-03 should confirm the skills table appears in --render-system-prompt output

---
*Phase: 05-team-manager-chat*
*Completed: 2026-03-07*
