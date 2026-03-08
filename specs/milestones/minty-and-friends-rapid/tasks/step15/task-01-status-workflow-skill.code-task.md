---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Create Status-Workflow Skill

## Description
Extract the status transition helpers (duplicated across hat instructions) into a shared `coding-agent/skills/status-workflow/` skill in each profile. The skill follows the SKILL.md format with YAML frontmatter, markdown instructions, and optional scripts/references.

## Background
Board scanning logic was already extracted into the `board-scanner` auto-inject skill (Step 13). What remains duplicated across hats in `ralph.yml` are status *mutation* helpers: field updates and label operations. Extracting these into a shared skill completes the skill-based architecture.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Skills Extraction")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 15)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Identify status *mutation* logic still duplicated across hats (board scanning is already in `board-scanner` skill):
   - Status field update mutations (`gh project item-edit`)
   - Label operations for status transitions
2. Create `coding-agent/skills/status-workflow/` in each profile:
   - `SKILL.md` — YAML frontmatter + markdown instructions
   - `scripts/` — shell scripts for status operations (if applicable)
   - `references/` — GraphQL query templates
3. Follow the same pattern as the existing `gh` skill
4. Ensure the skill is discoverable via `skills.dirs` in `ralph.yml`

## Dependencies
- Steps 1-13 complete (profiles on disk, workspace model working, board-scanner skill in place)

## Implementation Approach
1. Audit hat instructions across profiles for status transition logic
2. Extract common patterns into a unified skill definition
3. Create SKILL.md with clear instructions for the coding agent
4. Extract GraphQL templates into references/
5. Test that the skill follows the existing pattern

## Acceptance Criteria

1. **Skill exists in each profile**
   - Given each profile
   - When listing `coding-agent/skills/`
   - Then `status-workflow/` directory exists

2. **SKILL.md follows established format**
   - Given `status-workflow/SKILL.md`
   - When reading the file
   - Then it has YAML frontmatter and markdown instructions matching the `gh` skill pattern

3. **GraphQL templates in references/**
   - Given `status-workflow/references/`
   - When listing files
   - Then GraphQL query/mutation templates exist

4. **Skill discoverable by Ralph**
   - Given `ralph.yml` with `skills.dirs` configuration
   - When Ralph scans for skills
   - Then `status-workflow` is discoverable

5. **bm profiles init extracts skills**
   - Given `bm profiles init` running
   - When profiles are extracted
   - Then `coding-agent/skills/status-workflow/` is included

## Metadata
- **Complexity**: Medium
- **Labels**: skills-extraction, sprint-4
- **Required Skills**: GraphQL, GitHub Projects v2, skill format, profile structure
