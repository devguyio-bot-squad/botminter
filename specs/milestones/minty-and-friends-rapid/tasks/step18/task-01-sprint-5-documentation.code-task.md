---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Sprint 5 Documentation Updates

## Description
Update documentation for the Team Manager role and `bm chat` command. Document the new role, its statuses, and the interactive session capability. Docs-only step.

## Background
Sprint 5 introduced the Team Manager role (new member type) and `bm chat` (interactive sessions with any member). These are significant new capabilities that operators need to understand.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (documentation impact matrix, Sprint 5)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 18)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. `docs/content/reference/member-roles.md` — add team-manager role: description, purpose, statuses, when to use
2. `docs/content/reference/process.md` — add `mgr:` statuses (`mgr:todo`, `mgr:in-progress`, `mgr:done`) and `role/team-manager` label
3. `docs/content/reference/cli.md` — document `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt]`
4. `docs/content/concepts/coordination-model.md` — add Team Manager to coordination model; introduce role-as-skill pattern concept

## Dependencies
- Steps 16-17 complete (Team Manager and bm chat implemented)

## Implementation Approach
1. Read each affected doc page
2. Add Team Manager role documentation
3. Add bm chat command documentation with examples
4. Explain role-as-skill pattern in coordination model
5. Verify no broken links

## Acceptance Criteria

1. **Team Manager role documented**
   - Given `docs/content/reference/member-roles.md`
   - When reading the page
   - Then team-manager role is described with its purpose and statuses

2. **mgr: statuses documented**
   - Given `docs/content/reference/process.md`
   - When reading the statuses section
   - Then `mgr:todo`, `mgr:in-progress`, `mgr:done` are listed

3. **bm chat documented in CLI reference**
   - Given `docs/content/reference/cli.md`
   - When reading the page
   - Then `bm chat` is fully documented with all flags and examples

4. **Role-as-skill pattern explained**
   - Given `docs/content/concepts/coordination-model.md`
   - When reading the page
   - Then the role-as-skill pattern is introduced and explained

## Metadata
- **Complexity**: Low
- **Labels**: documentation, sprint-5
- **Required Skills**: Technical writing, Markdown
