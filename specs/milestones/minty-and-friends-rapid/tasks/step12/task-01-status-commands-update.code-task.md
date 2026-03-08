---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Status Commands Update for Workspace Repo Model

## Description
Update `bm status`, `bm teams show`, and `bm members show` to display workspace repo information: repo name, branch, submodule status, resolved coding agent, and profile source.

## Background
The workspace repo model introduces new runtime information that operators need visibility into: which GitHub repo backs each member's workspace, what branch they're on, whether submodules are up-to-date, and which coding agent is resolved.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 12)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `bm status`:
   - Show workspace repo name and branch for each member
   - Show submodule status (up-to-date vs behind)
2. Update `bm teams show`:
   - Include resolved coding agent name
   - Include profile source path (disk path)
3. Update `bm members show <member>`:
   - Include workspace repo URL
   - Include checked-out branch
   - Include submodule status per submodule
   - Include resolved coding agent

## Dependencies
- Steps 10-11 complete (workspace repos created and syncable)

## Implementation Approach
1. Add git status queries for workspace repos (branch, submodule status)
2. Update status command formatter
3. Update teams show formatter
4. Update members show formatter
5. Write tests with mock workspace data

## Acceptance Criteria

1. **bm status shows workspace repo info**
   - Given running workspace repos
   - When `bm status` runs
   - Then each member shows workspace repo name and branch

2. **bm status shows submodule status**
   - Given a workspace with submodules
   - When `bm status` runs
   - Then submodule status (up-to-date/behind) is shown

3. **bm teams show includes coding agent**
   - Given `bm teams show` runs
   - When viewing team details
   - Then the resolved coding agent is displayed

4. **bm teams show includes profile source**
   - Given `bm teams show` runs
   - When viewing team details
   - Then the disk path to the active profile is shown

5. **bm members show includes workspace details**
   - Given `bm members show <member>`
   - When the command runs
   - Then it shows workspace repo URL, branch, submodule status, and coding agent

## Metadata
- **Complexity**: Medium
- **Labels**: workspace-model, cli, sprint-3
- **Required Skills**: Rust, git queries, CLI formatting
