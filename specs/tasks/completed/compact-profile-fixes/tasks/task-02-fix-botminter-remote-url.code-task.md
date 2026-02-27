---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Fix `.botminter/` remote to use GitHub URL

## Description
When `bm teams sync` creates a workspace, it clones the team repo into `.botminter/` using the local filesystem path. This means `.botminter/`'s `origin` remote points to e.g. `/home/user/.botminter/workspaces/my-team/team/` instead of the actual GitHub remote. All `gh` commands inside the workspace rely on `gh repo view` to auto-detect the team repo from `.botminter/`'s remote, which fails with a local path.

## Background
`workspace.rs:61-67` uses `fs::canonicalize(team_repo_path)` to get an absolute local path, then passes that to `git clone`. The resulting `.botminter/` clone has `origin` set to the local path. The team repo itself has the correct GitHub remote (set during `bm init` by `gh repo create --source . --push`), but `git clone` doesn't propagate the source repo's remotes — it sets origin to the clone source URL.

The `github_repo` identifier (e.g. `org/team-name`) is available in the calling context (`teams.rs:92` has access to `team.github_repo`), but it's never passed to `create_workspace`.

## Reference Documentation
**Required:**
- `crates/bm/src/workspace.rs` — lines 25-82 (`create_workspace` function)
- `crates/bm/src/commands/teams.rs` — lines 84-118 (calling context with `team.github_repo`)

**Additional References:**
- `crates/bm/src/config.rs` — `TeamEntry` struct with `github_repo` field

## Technical Requirements
1. Add a `github_repo: Option<&str>` parameter to `create_workspace`
2. After cloning `.botminter/` from the local path, set the remote origin to the GitHub URL if available
3. Update `sync_workspace` similarly — if `.botminter/` remote is a local path but a GitHub URL is known, fix it
4. Update all call sites to pass the GitHub repo identifier

## Dependencies
- None — standalone fix

## Implementation Approach
1. Add `github_repo: Option<&str>` parameter to `create_workspace` signature
2. After the `git clone` at line 63-67, add:
   ```rust
   if let Some(repo) = github_repo {
       let github_url = format!("https://github.com/{}.git", repo);
       git_cmd(&bm_dir, &["remote", "set-url", "origin", &github_url])?;
   }
   ```
3. Update `teams.rs` call sites to pass `Some(&team.github_repo)` (or `None` for tests without GitHub)
4. Update test calls in `workspace.rs` to pass `None`
5. Add a test that asserts `.botminter/` remote URL contains `github.com` when a GitHub repo is provided

## Acceptance Criteria

1. **Workspace `.botminter/` remote points to GitHub**
   - Given a team with `github_repo` set to `org/my-team`
   - When `bm teams sync` creates a workspace
   - Then `.botminter/`'s `origin` remote URL is `https://github.com/org/my-team.git`

2. **`gh repo view` works inside workspace**
   - Given a workspace created by `bm teams sync`
   - When running `cd .botminter && gh repo view --json nameWithOwner -q .nameWithOwner`
   - Then it returns the correct `org/my-team` identifier

3. **No-GitHub fallback still works**
   - Given a team without GitHub configured (local-only)
   - When `bm teams sync` creates a workspace
   - Then `.botminter/` is still created with the local path remote (no crash)

4. **Existing tests pass**
   - Given the updated code
   - When `just test` is run
   - Then all tests pass

## Metadata
- **Complexity**: Medium
- **Labels**: bugfix, cli, workspace
- **Required Skills**: Rust, git
