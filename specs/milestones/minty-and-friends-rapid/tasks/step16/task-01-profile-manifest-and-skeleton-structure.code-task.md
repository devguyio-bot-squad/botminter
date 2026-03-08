---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Team Manager Profile Manifest and Skeleton Structure

## Description
Add the `team-manager` role to the scrum profile's `botminter.yml`: role definition, `mgr:` statuses, and `role/team-manager` label. Create the member skeleton directory structure with all expected files (empty or minimal content — Task 2 fills them in).

## Background
The Team Manager is a new role that operates on the team repo itself (its default project) with a simple 3-status workflow (`mgr:todo` -> `mgr:in-progress` -> `mgr:done`). It's the first experiment with the role-as-skill pattern — any member can be interacted with via `bm chat`.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Team Manager Role")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 16)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `botminter.yml` in scrum profile:
   - Add `team-manager` to `roles:` list with description
   - Add statuses: `mgr:todo`, `mgr:in-progress`, `mgr:done`
   - Add label: `role/team-manager` (color: `"5319E7"`)
2. Create `profiles/scrum/roles/team-manager/` directory structure:
   ```
   .botminter.yml
   context.md
   coding-agent/
     agents/
     skills/
       board-scanner/    # Auto-inject skill scoped to mgr:* statuses
         SKILL.md
   hats/
     executor/
       knowledge/
   knowledge/
   invariants/
   ```
3. Write `.botminter.yml`: `role: team-manager`, `comment_emoji: "📋"`
4. Ensure `bm roles list` shows the new role
5. Ensure `bm hire team-manager` creates the skeleton

## Dependencies
- Steps 1-13 complete (Sprints 1-3 + board-scanner migration finished)

## Implementation Approach
1. Study existing role definitions in `botminter.yml` for pattern
2. Add team-manager role, statuses, and label
3. Create directory structure following existing member skeleton pattern
4. Write `.botminter.yml` with role metadata
5. Create placeholder files for structure validation
6. Test that hire command creates the skeleton correctly

## Acceptance Criteria

1. **Role listed in profiles**
   - Given `bm roles list`
   - When the command runs with the updated scrum profile
   - Then `team-manager` appears in the role list

2. **Statuses added to profile**
   - Given the scrum profile's `botminter.yml`
   - When reading the statuses section
   - Then `mgr:todo`, `mgr:in-progress`, `mgr:done` are present

3. **Label defined in profile**
   - Given the scrum profile's `botminter.yml`
   - When reading the labels section
   - Then `role/team-manager` with color `"5319E7"` is defined

4. **Skeleton directory structure correct**
   - Given `profiles/scrum/roles/team-manager/`
   - When listing the directory tree
   - Then all expected directories and files exist

5. **bm hire creates member from skeleton**
   - Given `bm hire team-manager --name bob`
   - When the command runs
   - Then a member is created with the correct skeleton files

6. **E2E: statuses bootstrapped on GitHub**
   - Given `bm init` with the updated scrum profile
   - When labels are bootstrapped
   - Then `mgr:todo`, `mgr:in-progress`, `mgr:done` status labels exist on GitHub

7. **E2E: role label bootstrapped on GitHub**
   - Given `bm init` with the updated scrum profile
   - When labels are bootstrapped
   - Then `role/team-manager` label exists on GitHub

## Metadata
- **Complexity**: Medium
- **Labels**: team-manager, profile, sprint-5
- **Required Skills**: Rust, YAML, profile structure, E2E testing
