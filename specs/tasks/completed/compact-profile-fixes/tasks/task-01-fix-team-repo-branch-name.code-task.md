---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Fix team repo branch name (`master` → `main`)

## Description
`bm init` creates the team repo with a bare `git init`, which uses the system's default branch name. On many systems this is still `master`, but the entire profile ecosystem (PR operations, PROCESS.md, gh skill) assumes `main`. The team repo must be initialized with `main` as the default branch.

## Background
When `bm init` runs `git init` at `init.rs:174`, it doesn't specify `-b main`. If the user's system has `init.defaultBranch = master` (or no config at all), the team repo ends up on `master`. This cascades: `gh repo create --source . --push` pushes `master` to GitHub, making it the default branch there too. All downstream `gh pr create --base main` commands then fail.

## Reference Documentation
**Required:**
- `crates/bm/src/commands/init.rs` — lines 172-174 (git init), lines 469-490 (create_github_repo)

**Additional References:**
- `profiles/compact/agent/skills/gh/SKILL.md` — PR operations section uses `--base main`
- `profiles/compact/PROCESS.md` — process assumes `main` branch

## Technical Requirements
1. Change `run_git(&team_repo, &["init"])` to `run_git(&team_repo, &["init", "-b", "main"])` in `init.rs:174`
2. Update the test helpers in `workspace.rs` that also use bare `git init` (lines 492, 647) to use `-b main`

## Dependencies
- None — standalone fix

## Implementation Approach
1. Update `init.rs:174` to pass `-b main` to `git init`
2. Update test helper `git init` calls at `workspace.rs:492` and `workspace.rs:647` to also use `-b main`
3. Add an assertion in the workspace tests that the branch name after init is `main`

## Acceptance Criteria

1. **Team repo initializes on main branch**
   - Given a system with `init.defaultBranch` unset or set to `master`
   - When `bm init` creates a new team repo
   - Then the default branch is `main`

2. **Test helpers use main branch**
   - Given workspace unit tests in `workspace.rs`
   - When the test helper initializes a git repo
   - Then the repo is on the `main` branch

3. **Existing tests pass**
   - Given the updated code
   - When `just test` is run
   - Then all tests pass

## Metadata
- **Complexity**: Low
- **Labels**: bugfix, cli, git
- **Required Skills**: Rust, git
