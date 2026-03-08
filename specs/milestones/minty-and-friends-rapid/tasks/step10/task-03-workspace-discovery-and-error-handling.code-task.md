---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Workspace Discovery Update and Error Handling

## Description
Update workspace discovery to use `.botminter.workspace` marker instead of `.botminter/` directory. Remove old `.botminter/` clone logic (Alpha policy: no backwards compat). Add actionable error handling for common failures. Add E2E tests.

## Background
The old workspace model discovered workspaces by the presence of a `.botminter/` directory (a clone of the team repo). The new model uses a `.botminter.workspace` marker file. The old logic must be removed entirely — Alpha policy means no migration path.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 10)
- Invariants: invariants/e2e-testing.md

**Note:** Read the design document and E2E testing invariant before beginning implementation.

## Technical Requirements
1. Update workspace discovery to scan for `.botminter.workspace` marker
2. Remove all old `.botminter/` clone logic from `workspace.rs`
3. Update old tests that reference `.botminter/` to use new model
4. Error handling per design:
   - Repo already exists -> actionable error with `gh repo delete` command
   - Submodule failure -> actionable error with `gh repo view` command
5. E2E tests (require `--features e2e`):
   - `bm teams sync --push` creates GitHub repo with `<team>-<member>` naming
   - Submodules properly initialized on GitHub

## Dependencies
- Tasks 1-2 of this step (workspace creation flow working)

## Implementation Approach
1. Replace workspace discovery function to use marker file
2. Remove deprecated clone logic
3. Add error types with actionable messages
4. Update all tests referencing old workspace model
5. Write E2E tests following patterns in `crates/bm/tests/e2e/`

## Acceptance Criteria

1. **Workspace discovered by marker**
   - Given a directory with `.botminter.workspace`
   - When workspace discovery runs
   - Then it's recognized as a valid workspace

2. **Old .botminter/ directory not recognized**
   - Given a directory with only `.botminter/` (old model)
   - When workspace discovery runs
   - Then it's NOT recognized as a workspace

3. **No .botminter/ clone code remains**
   - Given `workspace.rs` source
   - When searching for old clone logic
   - Then no references to `.botminter/` as workspace clone remain

4. **Repo exists error is actionable**
   - Given a GitHub repo that already exists with the workspace name
   - When `bm teams sync --push` tries to create it
   - Then the error includes the `gh repo delete` command to resolve it

5. **Submodule failure error is actionable**
   - Given a submodule URL that can't be added (e.g., no access)
   - When submodule setup fails
   - Then the error includes the `gh repo view` command for diagnosis

6. **E2E test: repo creation on GitHub**
   - Given E2E test with real GitHub API
   - When `bm teams sync --push` runs
   - Then a repo named `<team>-<member>` exists on GitHub with correct submodules

## Metadata
- **Complexity**: High
- **Labels**: workspace-model, e2e, sprint-3
- **Required Skills**: Rust, GitHub API, E2E testing, error handling
