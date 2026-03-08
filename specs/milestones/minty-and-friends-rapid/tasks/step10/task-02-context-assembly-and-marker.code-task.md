---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Context File Copy, Agent Dir Assembly, and Marker File

## Description
Complete the workspace repo creation by copying context files to the workspace root, assembling the agent directory with symlinks into submodule paths, writing `.gitignore` and `.botminter.workspace` marker, and committing+pushing.

## Background
After submodules are set up, the workspace needs: context files (CLAUDE.md, PROMPT.md, ralph.yml) at the root for the coding agent to discover, an agent directory (e.g., `.claude/agents/`) assembled from three scopes via symlinks, a gitignore, and a marker file for workspace discovery.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Workspace Repository Model")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 10)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Copy context files from `team/team/<member>/` to workspace root:
   - `CLAUDE.md` (or resolved context_file), `PROMPT.md`, `ralph.yml`
2. Assemble agent dir (e.g., `.claude/agents/`) with symlinks:
   - Team-level: `team/coding-agent/agents/*.md`
   - Project-level: `team/projects/<project>/coding-agent/agents/*.md`
   - Member-level: `team/team/<member>/coding-agent/agents/*.md`
3. Write `.gitignore` for `.ralph/`, agent dir, and other runtime files
4. Write `.botminter.workspace` marker file (can contain workspace metadata as YAML)
5. Commit all files and push to remote
6. Use parameterized paths from `CodingAgentDef` (no hardcoded agent strings)

## Dependencies
- Task 1 of this step (repo created, submodules in place)

## Implementation Approach
1. Implement context file copy function
2. Implement agent dir assembly with symlink creation
3. Implement gitignore and marker file writing
4. Add commit and push step
5. Write tests for file layout, symlink correctness, marker content

## Acceptance Criteria

1. **Context files at workspace root**
   - Given a workspace with team submodule
   - When context files are copied
   - Then `CLAUDE.md`, `PROMPT.md`, `ralph.yml` exist at the workspace root

2. **Agent dir assembled from three scopes**
   - Given team, project, and member-level agent files
   - When the agent dir is assembled
   - Then symlinks exist for all three scopes' agent files

3. **Symlinks point into submodule paths**
   - Given the assembled agent dir
   - When following symlinks
   - Then they resolve to paths inside `team/` submodule

4. **.gitignore contains expected entries**
   - Given the workspace root
   - When reading `.gitignore`
   - Then it includes the agent dir name and `.ralph/`

5. **.botminter.workspace marker exists**
   - Given the workspace root
   - When checking for the marker file
   - Then `.botminter.workspace` exists

6. **Changes committed and pushed**
   - Given all workspace files written
   - When the creation flow completes
   - Then a commit exists with all files and it's pushed to the remote

7. **Parameterized paths used throughout**
   - Given the assembly functions
   - When inspecting code
   - Then agent dir and context file names come from `CodingAgentDef`

8. **No `.botminter/` path references in profile content**
   - Given all profile files (ralph.yml, hat instructions, context.md, PROMPT.md)
   - When searching for `.botminter/` path references
   - Then none exist — all paths use `team/` (e.g., `team/invariants/`, `team/knowledge/`, `team/coding-agent/skills`)

## Metadata
- **Complexity**: High
- **Labels**: workspace-model, sprint-3
- **Required Skills**: Rust, git, symlinks, filesystem operations
