---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Existing Workspace Sync Flow

## Description
Implement the sync flow for existing workspace repos: update submodules, re-copy context files if newer, re-assemble agent directory, and commit+push changes. Add a `-v` verbose flag to show submodule update status.

## Background
Once workspace repos are created (Step 10), they need to stay in sync with the team repo. `bm teams sync` (without `--push`) handles existing workspaces: pulling latest submodule changes, refreshing context files, and rebuilding agent symlinks.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Workspace sync")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 11)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Implement sync flow for existing workspaces:
   - `git submodule update --remote` to fetch latest
   - Checkout member branch in each submodule (never leave detached HEAD)
   - Re-copy context files if team submodule versions are newer (content comparison)
   - Re-copy `ralph.yml` if newer
   - Re-assemble agent dir symlinks (idempotent operation)
   - Commit changes (if any) and push
2. Add `-v`/`--verbose` flag to `bm teams sync`:
   - Show submodule update status (ahead/behind/up-to-date)
   - Show branch checkout results
   - Show file copy decisions (newer/same/skipped)
   - Show errors per workspace

## Dependencies
- Step 10 complete (workspace repos exist on GitHub with submodules)

## Implementation Approach
1. Detect existing workspaces by `.botminter.workspace` marker
2. Implement submodule update and branch checkout
3. Implement content-based comparison for context files
4. Reuse agent dir assembly function (already idempotent from Step 10)
5. Add verbose output formatting
6. Write tests for sync scenarios (files changed, files same, new files added)

## Acceptance Criteria

1. **Submodules updated to latest**
   - Given a workspace with a team submodule behind upstream
   - When `bm teams sync` runs
   - Then submodules are updated to the latest remote content

2. **Member branch checked out (not detached HEAD)**
   - Given submodules after update
   - When checking branch state
   - Then each submodule is on the member's branch, not detached HEAD

3. **Context files re-copied when newer**
   - Given `CLAUDE.md` in team submodule has changed since last sync
   - When `bm teams sync` runs
   - Then the workspace root's `CLAUDE.md` is updated to match

4. **Context files skipped when same**
   - Given no changes in team submodule since last sync
   - When `bm teams sync` runs
   - Then context files are not re-copied (or content is identical)

5. **Agent dir symlinks rebuilt idempotently**
   - Given existing symlinks in the agent directory
   - When sync re-assembles the agent dir
   - Then symlinks are correct (added/removed/unchanged as needed)

6. **Verbose output shows status**
   - Given `bm teams sync -v`
   - When sync runs
   - Then output shows per-workspace status: submodule updates, file decisions, branch state

7. **Changes committed and pushed**
   - Given context files were re-copied during sync
   - When sync completes
   - Then changes are committed and pushed to the workspace repo

## Metadata
- **Complexity**: High
- **Labels**: workspace-model, sync, sprint-3
- **Required Skills**: Rust, git submodules, filesystem comparison
