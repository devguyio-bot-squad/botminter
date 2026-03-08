# Research: Current Workspace Model Audit

> Complete analysis of the current workspace provisioning lifecycle for redesign to submodule-based workspace repos.

## Summary

The current model clones the team repo into `.botminter/` inside each member's workspace, then symlinks/copies files to the workspace root. The redesign replaces `.botminter/` with a dedicated workspace git repo containing the team repo and project forks as submodules.

## Current Workspace Structure

### No-Project Mode
```
workzone/<team>/
  team/                              # Team repo (control plane)
  <member>/                          # Member workspace
    .botminter/                      # Clone of team repo
    PROMPT.md → .botminter/team/<member>/PROMPT.md
    CLAUDE.md → .botminter/team/<member>/CLAUDE.md
    ralph.yml                        # Copied (mutable)
    .claude/agents/                  # Symlinks from 3 scopes
    .claude/settings.local.json      # Copied
    .git/                            # git init (for .gitignore support)
```

### With-Project Mode
```
workzone/<team>/
  team/                              # Team repo
  <member>/<project>/                # Member workspace (inside project fork)
    .botminter/                      # Clone of team repo
    <fork content>                   # Cloned from fork URL
    PROMPT.md → .botminter/team/<member>/PROMPT.md
    CLAUDE.md → .botminter/team/<member>/CLAUDE.md
    ralph.yml                        # Copied
    .claude/agents/                  # Symlinks from 3+1 scopes
```

## Creation Flow (`create_workspace()`)

1. Create member directory
2. Clone fork (project mode) or `git init` (no-project mode)
3. Clone team repo → `.botminter/`
4. `surface_files()`: symlink PROMPT.md/CLAUDE.md, copy ralph.yml
5. `assemble_claude_dir()`: create `.claude/agents/` with symlinks from team/project/member scopes
6. Write `.gitignore` + `.git/info/exclude` with BM entries
7. Hide tracked BM files via `git update-index --skip-worktree`

## Sync Flow (`sync_workspace()`)

1. Fix `.botminter/` remote URL if stale
2. `git pull` in `.botminter/` (non-fatal)
3. `git pull` in project fork (if applicable)
4. Re-copy `ralph.yml` if source newer
5. Re-copy `settings.local.json` if source newer
6. Re-assemble `.claude/agents/` (idempotent)
7. Verify PROMPT.md/CLAUDE.md symlinks
8. Update `.git/info/exclude`, re-hide tracked files

## Launch Flow (`bm start`)

1. Discover workspaces by scanning for `.botminter/` subdirs
2. `cd {workspace_root} && ralph run -p PROMPT.md --env GH_TOKEN=... --env RALPH_TELEGRAM_BOT_TOKEN=...`
3. Unsets `CLAUDECODE` env var, detaches stdio, records PID in `state.json`

## Key Design Properties

- **Idempotent sync**: safe to re-run, preserves local edits to ralph.yml
- **Relative symlinks**: portable across machines
- **Git safety**: triple-layer hiding (.gitignore, .git/info/exclude, skip-worktree)
- **Scope resolution**: team → project → member (additive, later scopes override)

## Problems Motivating Redesign

1. **Nested repo confusion**: `.botminter/` inside workspace creates nested `.git` — agents confused by multiple CLAUDE.md, skills, and git contexts
2. **Push ambiguity**: agents try to push to the wrong repo
3. **No multi-project support**: current with-project mode creates separate workspaces per project, not one workspace with multiple projects
4. **Team repo clone is heavy**: full clone of team repo per workspace

## New Model (from requirements)

Each agent gets a **dedicated GitHub-hosted workspace repo** (`<team>-<member>`):
- Team repo as git submodule
- Project fork(s) as git submodule(s)
- CLAUDE.md, ralph.yml, PROMPT.md at repo root (clean, unambiguous)
- `bm teams sync --push` creates the repo on GitHub
- Naming: `<team-name>-<member-name>` in same org

## Functions to Rewrite

| Function | Current | New |
|----------|---------|-----|
| `create_workspace()` | Clone team repo into `.botminter/`, clone fork, surface files | Create workspace repo, add submodules, surface files at root |
| `sync_workspace()` | Pull `.botminter/`, pull fork, re-assemble | Pull submodules, re-surface if needed |
| `assemble_claude_dir()` | Symlink from `.botminter/` paths | Symlink from submodule paths |
| `surface_files()` | Symlink to `.botminter/team/<member>/` | Files live at workspace root natively |
| `find_workspace()` | Look for `.botminter/` | Look for workspace repo markers |
