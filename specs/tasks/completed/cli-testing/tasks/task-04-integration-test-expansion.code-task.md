---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Integration Test Expansion

## Description
Expand `crates/bm/tests/integration.rs` with cross-command consistency tests, multi-member/multi-project scenarios, and error path coverage. These tests exercise the CLI binary end-to-end but don't require external services (no GitHub, no Docker).

## Background
The existing 28 integration tests cover individual commands well but lack cross-command validation (e.g., does `status` agree with `members list`?), complex scenarios (multiple members + projects + sync), and error paths (corrupt config, missing directories). The test infrastructure (`setup_team()`, `ENV_MUTEX`, `git()` helper) is mature and ready to support these additions.

## Reference Documentation
**Required:**
- `crates/bm/tests/integration.rs` — existing test patterns and helpers
- `crates/bm/src/commands/` — all command implementations
- CLAUDE.md workspace model and knowledge scoping sections

## Technical Requirements

### Cross-command consistency (3 tests)
1. `status_and_members_list_agree` — After hiring members, both commands report the same member set
2. `roles_list_matches_profile_describe` — `bm roles list` output matches roles from `bm profiles describe`
3. `hire_then_members_list_shows_role` — Hired member appears in `members list` with correct role

### Multi-member/multi-project scenarios (4 tests)
4. `hire_multiple_roles_then_sync` — Dynamically read available roles from `profile::list_roles(profile)`, hire 3 members (picking roles from that list), add 2 projects, sync, verify all 3 workspaces with correct structure
5. `sync_with_multiple_projects_creates_project_workspaces` — Members with projects get `member/project/` workspace layout
6. `hire_same_role_twice_auto_suffix` — Pick the first available role from `profile::list_roles(profile)`, hire it twice — get `role-01` and `role-02`, both appear in members list
7. `sync_after_second_hire_creates_new_workspace` — Sync, hire another member (role picked dynamically), sync again — new workspace appears without disturbing existing ones

### Error paths (5 tests)
8. `status_missing_team_repo_dir_errors` — Config points to nonexistent team dir
9. `hire_with_corrupt_manifest_errors` — Team repo's `botminter.yml` is malformed YAML
10. `sync_missing_workzone_creates_it` — Workzone dir doesn't exist yet — sync should create it
11. `teams_list_with_empty_config` — No teams registered, helpful message shown
12. `projects_add_invalid_url_format` — URL with no path component still derives a reasonable project name or errors helpfully

### Output format verification (2 tests)
13. `status_table_has_expected_columns` — Output contains headers: Member, Role, Status, Started, PID
14. `members_list_table_has_expected_columns` — Output contains headers: Member, Role, Status

## Dependencies
- Existing test helpers in `integration.rs`
- Some tests need the `bm` binary built (`env!("CARGO_BIN_EXE_bm")`)

## Implementation Approach
1. Add tests to the existing `integration.rs` file, following established patterns
2. Use `setup_team()` for standard team setup, `add_team_to_config()` for multi-team scenarios
3. **Profile-agnostic role selection:** Always use `profile::list_roles(profile)` to discover available roles at runtime — never hardcode role names like `architect` or `developer`
4. For multi-member tests: call `bm hire` multiple times before sync, using roles from the dynamic list
5. For error tests: manually corrupt or remove files after setup
6. For output verification: capture stdout and assert on table structure
7. All tests must use `ENV_MUTEX` when mutating HOME

## Acceptance Criteria

1. **Cross-command consistency**
   - Given a team with 2 hired members
   - When `bm status` and `bm members list` are both run
   - Then both report exactly the same member names

2. **Multi-member sync**
   - Given 3 members hired with roles dynamically chosen from `profile::list_roles()` and 2 projects added
   - When `bm teams sync` runs
   - Then 3 workspace directories exist with correct structure (symlinks, `.botminter/`, `.claude/agents/`)

3. **Incremental sync**
   - Given a synced team where a new member is hired
   - When `bm teams sync` runs again
   - Then the new workspace is created and existing workspaces are unchanged

4. **Corrupt manifest error**
   - Given a team repo with malformed `botminter.yml`
   - When `bm hire` is attempted
   - Then a clear error message is shown (not a panic)

5. **Status table format**
   - Given any team with members
   - When `bm status` is run
   - Then output contains column headers: Member, Role, Status, Started, PID

## Metadata
- **Complexity**: Medium
- **Labels**: test, integration-test
- **Required Skills**: Rust, CLI testing, process spawning, output parsing
