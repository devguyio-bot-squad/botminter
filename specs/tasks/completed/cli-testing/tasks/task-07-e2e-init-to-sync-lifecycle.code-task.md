---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: E2E — Init to Sync Lifecycle

## Description
Write E2E tests covering the full `bm init` → `bm hire` → `bm projects add` → `bm teams sync` lifecycle against a real (temporary) GitHub repo. Verify that workspace directories, symlinks, GitHub labels, and repo structure are all correct.

## Background
`bm init` creates a team repo, optionally creates a GitHub repo, and bootstraps labels. `bm hire` adds member skeletons. `bm projects add` registers fork URLs. `bm teams sync` provisions workspaces with symlinks, `.botminter/` clones, and `.claude/agents/` assembly. This test validates the entire pipeline end-to-end.

Since `bm init` is interactive (uses `cliclack`), E2E tests should either:
- Script the init flow (if stdin piping works with cliclack), OR
- Use the programmatic equivalents: manually call `profile::extract_profile_to()`, create config, etc. (matching what integration tests do) but with a real GitHub repo

## Reference Documentation
**Required:**
- `crates/bm/src/commands/init.rs` — init flow, label bootstrapping, GitHub repo creation
- `crates/bm/src/commands/hire.rs` — member skeleton extraction
- `crates/bm/src/commands/teams.rs` — sync/workspace provisioning
- `crates/bm/src/workspace.rs` — workspace creation and assembly
- E2E harness from task-06: `TempRepo`, `bm_cmd()`, helpers

## Technical Requirements

### Test: `e2e_init_hire_sync_lifecycle`
1. Create a temp GitHub repo via `TempRepo`
2. Choose a profile (any embedded profile works — test is profile-agnostic)
3. Set up team programmatically (profile extraction, git init, push to GitHub repo)
4. Read available roles dynamically via `profile::list_roles(profile)` — pick the first two
5. Run `bm hire <role_1> --name alice -t <team>` via CLI
6. Run `bm hire <role_2> --name bob -t <team>` via CLI
7. Run `bm projects add https://github.com/example/project -t <team>` via CLI
8. Run `bm teams sync -t <team>` via CLI
9. Verify:
   - Two workspace directories exist: `<role_1>-alice/` and `<role_2>-bob/`
   - Each workspace has `.botminter/` (team repo clone)
   - Each workspace has `PROMPT.md` symlink pointing into `.botminter/`
   - Each workspace has `CLAUDE.md` symlink pointing into `.botminter/`
   - Each workspace has `ralph.yml` (copied, not symlinked)
   - Each workspace has `.claude/agents/` with assembled agent symlinks

### Test: `e2e_labels_bootstrapped_on_github`
10. Read expected labels from `profile::read_manifest(profile).labels` — this is the source of truth
11. After init, query GitHub labels via `gh label list -R <repo> --json name,color`
12. Verify every label from the manifest exists on GitHub with matching color
13. Verify no extra unexpected labels exist (beyond GitHub defaults)

### Test: `e2e_sync_idempotent_with_github`
11. Run `bm teams sync` twice
12. Verify no errors on second run
13. Verify workspace structure unchanged after second sync

### Test: `e2e_members_list_after_full_setup`
14. After hiring and syncing, run `bm members list -t <team>`
15. Verify output lists exactly the hired members with correct roles

### Test: `e2e_teams_list_shows_github_repo`
16. Run `bm teams list`
17. Verify output includes the GitHub repo URL/name

## Dependencies
- Task-06 E2E harness (`TempRepo`, helpers)
- `gh` CLI authenticated with permissions to create/delete repos
- Feature-gated behind `e2e`

## Implementation Approach
1. Create `crates/bm/tests/e2e/init_to_sync.rs`
2. Each test creates its own `TempRepo` (RAII cleanup)
3. **Profile-agnostic:** Read roles via `profile::list_roles()` and labels via `profile::read_manifest().labels` — never hardcode role names or label values
4. Use programmatic team setup (matching integration test patterns) but with real GitHub repo
5. Push team repo to GitHub after setup
6. Use `bm_cmd()` helper for all CLI invocations
7. Parse CLI output and filesystem state for assertions
8. Use `gh label list --json name,color` for structured label verification against manifest

## Acceptance Criteria

1. **Full lifecycle produces correct workspace**
   - Given a fresh team with any embedded profile, 2 members (roles chosen dynamically from `profile::list_roles()`), and 1 project
   - When init → hire → hire → projects add → sync is executed
   - Then both workspaces exist with correct symlinks, `.botminter/`, `.claude/agents/`, and `ralph.yml`

2. **Labels match profile definition**
   - Given a team initialized with any profile
   - When `gh label list --json` is queried for the GitHub repo
   - Then labels match exactly what `profile::read_manifest(profile).labels` defines (name and color)

3. **Sync is idempotent**
   - Given a fully synced team
   - When `bm teams sync` is run again
   - Then it succeeds without errors and workspace structure is unchanged

4. **Members list accurate**
   - Given a team with 2 hired members after sync
   - When `bm members list` is run
   - Then output lists exactly 2 members with their correct roles

## Metadata
- **Complexity**: High
- **Labels**: test, e2e, lifecycle, github
- **Required Skills**: Rust, E2E testing, GitHub CLI, filesystem verification
