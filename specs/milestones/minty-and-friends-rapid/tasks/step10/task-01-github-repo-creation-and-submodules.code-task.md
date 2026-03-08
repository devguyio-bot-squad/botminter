---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: GitHub Repo Creation and Submodule Setup

## Description
Implement the first half of the workspace repo creation flow: create a GitHub repo per member, clone it locally, add the team repo and project forks as submodules, and checkout member branches. This replaces the old `.botminter/` clone model.

## Background
The workspace repository model gives each member a dedicated GitHub repo containing submodules (team repo + project forks) rather than a simple clone of the team repo. This enables proper git workflows, independent commit histories per member, and submodule-based coordination.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Workspace Repository Model")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 10)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Create GitHub repo: `gh repo create <org>/<team>-<member> --private`
2. Clone locally: `git clone <url> workzone/<team>/<member>/`
3. Add team repo submodule: `git submodule add <team-repo-url> team`
4. Checkout member branch in team submodule: `git -C team checkout -b <member>`
5. For each assigned project: `git submodule add <fork-url> projects/<project>`
6. Checkout member branch in project submodules
7. Update `bm teams sync --push` to use this new flow for new workspaces

## Dependencies
- Steps 1-9 complete (Sprints 1-2 finished)

## Implementation Approach
1. Study existing workspace creation in `workspace.rs`
2. Implement repo creation and clone functions
3. Implement submodule addition for team repo and projects
4. Implement branch checkout in submodules
5. Wire into `bm teams sync --push` for new workspaces
6. Write integration tests with local repos (no GitHub) for structure validation

## Acceptance Criteria

1. **GitHub repo created with correct naming**
   - Given `bm teams sync --push` for a member
   - When creating a new workspace
   - Then a private repo named `<org>/<team>-<member>` is created on GitHub

2. **Local clone at correct path**
   - Given a created GitHub repo
   - When cloned locally
   - Then it exists at `workzone/<team>/<member>/`

3. **Team repo added as submodule**
   - Given the local workspace clone
   - When listing submodules
   - Then `team/` submodule points to the team repo URL

4. **Member branch checked out in team submodule**
   - Given the team submodule
   - When checking the current branch
   - Then it's on the `<member>` branch (not detached HEAD)

5. **Project submodules added**
   - Given a member with assigned projects
   - When listing submodules
   - Then `projects/<project>/` submodules exist for each assigned project

6. **Member branch in project submodules**
   - Given project submodules
   - When checking current branches
   - Then each is on the `<member>` branch

7. **Integration test validates structure**
   - Given a local integration test (no GitHub)
   - When the workspace creation flow runs
   - Then the directory structure matches expected layout

## Metadata
- **Complexity**: High
- **Labels**: workspace-model, github, sprint-3
- **Required Skills**: Rust, git submodules, gh CLI, workspace module
