---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Hat Instruction Updates and Sprint 4 Documentation

## Description
Update hat instructions in `ralph.yml` across all profiles to reference the shared `status-workflow` skill instead of inlining status transition logic. Update Sprint 4 documentation.

## Background
With the board-scanner skill (Step 13) and status-workflow skill (Task 1) extracted, hat instructions no longer need to inline scanning or status transition logic. They should reference the shared skills, keeping hats focused on their domain-specific logic.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (documentation impact matrix, Sprint 4)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 15)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update hat instructions in `ralph.yml` across all profiles:
   - Remove inlined status transition logic
   - Add reference to `status-workflow` skill
   - Preserve hat-specific domain logic
2. Verify `skills.dirs` in `ralph.yml` includes the path to the shared skills
3. Documentation updates:
   - `docs/content/reference/configuration.md` — document skill format and scoping
   - `docs/content/concepts/architecture.md` — add skills extraction concept

## Dependencies
- Task 1 of this step (status-workflow skill created)

## Implementation Approach
1. Read each hat's instructions in each profile's `ralph.yml`
2. Identify and remove duplicated status transition logic
3. Add skill references where logic was removed
4. Verify functionality preserved (skill provides same instructions)
5. Write documentation updates

## Acceptance Criteria

1. **No inline status logic in hats**
   - Given hat instructions in each profile's `ralph.yml`
   - When searching for board scanning / status transition GraphQL
   - Then it's not inlined — hats reference the shared skill instead

2. **Hat domain logic preserved**
   - Given each hat's instructions
   - When reading the remaining content
   - Then domain-specific logic (reviewing, building, etc.) is intact

3. **skills.dirs configured**
   - Given each profile's `ralph.yml`
   - When reading `skills.dirs` configuration
   - Then it includes the path to shared skills directory

4. **Configuration reference documents skills**
   - Given `docs/content/reference/configuration.md`
   - When reading the page
   - Then skill format and scoping are documented

5. **Architecture concept updated**
   - Given `docs/content/concepts/architecture.md`
   - When reading the page
   - Then skills extraction is explained as an architectural concept

## Metadata
- **Complexity**: Medium
- **Labels**: skills-extraction, documentation, sprint-4
- **Required Skills**: YAML, Ralph hat format, technical writing
