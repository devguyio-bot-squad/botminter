---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Workspace Edge-Case Tests

## Description
Expand unit test coverage for `workspace.rs` to cover edge cases: broken symlinks, `sync_workspace()` behavior when files change, `copy_if_newer()` timestamp logic, `assemble_claude_dir()` with agents from all three scopes, and `.git/info/exclude` content.

## Background
`workspace.rs` currently has 4 unit tests covering basic happy paths: gitignore content, claude dir assembly, surface files, and no-project workspace creation. The module has ~400 lines with several private helpers that need coverage for edge cases — particularly around symlink handling and idempotent sync behavior.

## Reference Documentation
**Required:**
- `crates/bm/src/workspace.rs` — all public and private functions
- Existing workspace tests in the `#[cfg(test)]` module at the bottom of workspace.rs
- CLAUDE.md workspace model diagram

## Technical Requirements

### Symlink edge cases
1. `surface_files` when target symlink already exists and points correctly — should be idempotent (no error)
2. `surface_files` when target symlink exists but points to wrong location — should update
3. `create_symlink` when target is a regular file (not a symlink) — should handle gracefully
4. `verify_symlink` with broken symlink (target deleted) — should report the issue

### Sync behavior
5. `sync_workspace` re-copies `ralph.yml` when source has changed
6. `sync_workspace` re-assembles `.claude/` dir when agents change
7. `sync_workspace` is idempotent — running twice produces same result

### copy_if_newer
8. `copy_if_newer` skips copy when destination is newer
9. `copy_if_newer` copies when source is newer
10. `copy_if_newer` copies when destination doesn't exist

### assemble_claude_dir multi-scope
11. `assemble_claude_dir` with agents from team scope, project scope, and member scope — all three should appear as symlinks
12. `assemble_claude_dir` with `settings.local.json` present — should be copied
13. `assemble_claude_dir` with no agents in any scope — `.claude/agents/` dir still created (empty)

### Gitignore/exclude
14. `write_git_exclude` creates `.git/info/exclude` with expected entries
15. `gitignore_content` includes all botminter-managed paths (`.botminter/`, `ralph.yml`, etc.)

## Dependencies
- Existing workspace test infrastructure in `workspace.rs`
- `tempfile` for filesystem isolation
- Some functions are private — tests must be inside the `#[cfg(test)]` module in `workspace.rs`

## Implementation Approach
1. Extend the existing `#[cfg(test)] mod tests` in `workspace.rs`
2. For symlink tests: create temp dir structures, create/break symlinks, call functions, verify
3. For sync tests: create initial workspace, modify source files, call `sync_workspace`, verify updates
4. For `copy_if_newer`: create files with controlled timestamps using `filetime` crate (add to dev-deps if needed) or by creating files in sequence
5. For multi-scope assembly: create team/project/member agent dirs with files, call `assemble_claude_dir`, verify all symlinks

## Acceptance Criteria

1. **Surface files idempotent**
   - Given a workspace with correctly symlinked PROMPT.md
   - When `surface_files()` is called again
   - Then no error occurs and symlink still points correctly

2. **Broken symlink detected**
   - Given a workspace where the PROMPT.md symlink target was deleted
   - When `verify_symlink()` is called
   - Then it reports the broken symlink

3. **Sync re-copies changed ralph.yml**
   - Given a workspace created by `create_workspace()`
   - When `ralph.yml` in the team repo is modified and `sync_workspace()` is called
   - Then the workspace copy reflects the new content

4. **Multi-scope agent assembly**
   - Given agents in `team/agents/`, `projects/<proj>/agents/`, and `team/<member>/agents/`
   - When `assemble_claude_dir()` is called
   - Then `.claude/agents/` contains symlinks to all three scopes' agents

5. **copy_if_newer skips when destination newer**
   - Given source file older than destination file
   - When `copy_if_newer()` is called
   - Then destination content is unchanged

6. **Git exclude content**
   - Given a workspace with `.git/` directory
   - When `write_git_exclude()` is called
   - Then `.git/info/exclude` contains botminter-managed entries

## Metadata
- **Complexity**: Medium
- **Labels**: test, unit-test, workspace
- **Required Skills**: Rust, filesystem operations, symlinks, tempfile
