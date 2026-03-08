---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: bm start/stop Adaptation for Workspace Repos

## Description
Update `bm start` to discover workspaces by `.botminter.workspace` marker and launch Ralph from workspace repo roots. Update `bm stop` for the new workspace paths. Update PID tracking in `~/.botminter/state.json`.

## Background
`bm start` currently discovers workspaces using the old `.botminter/` directory model. With workspace repos, discovery uses the `.botminter.workspace` marker and Ralph launches from the workspace repo root (which has `PROMPT.md`, `ralph.yml`, and the agent directory).

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 11)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `bm start`:
   - Discover workspaces by scanning `workzone/<team>/` for directories with `.botminter.workspace`
   - Launch: `cd workzone/<team>/<member>/ && ralph run -p PROMPT.md --env GH_TOKEN=...`
   - Store PID in `~/.botminter/state.json` with new workspace paths
2. Update `bm stop`:
   - Read PIDs from state
   - Stop processes (same kill logic, just new paths in state)
3. Ensure `GH_TOKEN` is passed from `~/.botminter/config.yml` to Ralph instances

## Dependencies
- Task 1 of this step (sync flow working, workspaces discoverable)

## Implementation Approach
1. Update workspace discovery in start command
2. Update launch logic for new directory structure
3. Update state tracking with workspace repo paths
4. Update stop command to match
5. Write tests for discovery and launch setup

## Acceptance Criteria

1. **bm start discovers via marker**
   - Given workspace repos with `.botminter.workspace`
   - When `bm start` runs
   - Then it discovers all member workspaces

2. **Ralph launched at workspace root**
   - Given a discovered workspace
   - When Ralph is launched
   - Then the working directory is the workspace repo root (not a subdirectory)

3. **GH_TOKEN passed to Ralph**
   - Given a team with `GH_TOKEN` in config
   - When Ralph instances are launched
   - Then each receives `GH_TOKEN` in its environment

4. **PIDs tracked in state**
   - Given launched Ralph instances
   - When checking `~/.botminter/state.json`
   - Then PIDs are recorded with correct workspace paths

5. **bm stop terminates all members**
   - Given running Ralph instances tracked in state
   - When `bm stop` runs
   - Then all tracked processes are terminated

6. **bm stop --force available**
   - Given stuck Ralph instances
   - When `bm stop --force` runs
   - Then processes are forcefully killed

## Metadata
- **Complexity**: Medium
- **Labels**: workspace-model, runtime, sprint-3
- **Required Skills**: Rust, process management, state tracking
