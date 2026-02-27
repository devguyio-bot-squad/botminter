---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Regression Test — Member Discovery Bug

## Description
Write a failing integration test that proves `bm status` (and by extension `bm start`) incorrectly lists structural directories (`knowledge/`, `invariants/`, `projects/`, `team/`, `agent/`) as team members. Only directories under `team/team/` that were created by `bm hire` should appear.

This test **must fail** against the current codebase — it is the red phase of TDD for the fix in task-09.

## Background
`bm status` resolves `team_repo` as `team.path` (the team directory, e.g. `workzone/my-team/`), then scans `team.path.join("team")` — which is the **team repo root** (`workzone/my-team/team/`). This root contains structural directories like `knowledge/`, `invariants/`, etc., all of which get listed as "members."

`bm members list` correctly does a double-join: `team.path.join("team").join("team")` → `workzone/my-team/team/team/`, which is where hired member directories actually live.

The same off-by-one affects `bm start` (uses same `team_repo = &team.path` pattern).

### Root Cause Location
- `crates/bm/src/commands/status.rs:23` — `let team_repo = &team.path;` should be `team.path.join("team")`
- `crates/bm/src/commands/start.rs:16` — same issue

## Reference Documentation
**Required:**
- Bug analysis: `status.rs:23-47` vs `members.rs:22-23` path resolution comparison
- Existing test helpers: `crates/bm/tests/integration.rs` (see `setup_team()`, `add_team_to_config()`)

## Technical Requirements
1. Add an integration test `status_only_lists_hired_members` to `crates/bm/tests/integration.rs`
2. The test must:
   - Use `setup_team()` to create a team (any profile — schema-level structural dirs exist regardless)
   - Read a valid role dynamically via `profile::list_roles(profile)` and hire one member
   - Run `bm status` and capture stdout
   - Assert the output contains the hired member's name
   - Assert the output does NOT contain schema-level structural dirs (`knowledge`, `invariants`, `projects`, `agent`, `team`) as member entries
3. Add a complementary test `status_matches_members_list` that asserts `bm status` and `bm members list` agree on member count
4. Mark both tests with `#[should_panic]` or use `assert!` logic that documents the expected failure, so CI shows the regression clearly

## Dependencies
- Existing test infrastructure in `crates/bm/tests/integration.rs`
- `setup_team()` helper already extracts a profile, which creates the structural dirs

## Implementation Approach
1. Read the existing integration test patterns (ENV_MUTEX, setup_team, git helpers)
2. Write `status_only_lists_hired_members`:
   - Setup team with any profile
   - Read a role via `profile::list_roles()`, hire a member with that role
   - Run `bm status -t <team>` via `Command::new(env!("CARGO_BIN_EXE_bm"))`
   - Parse stdout, assert only the hired member appears as a member row
   - Assert schema-level structural dirs are absent from member rows
3. Write `status_matches_members_list`:
   - Same setup
   - Run both `bm status` and `bm members list`, extract member names
   - Assert sets are equal
4. Verify both tests fail against current code (red phase)

## Acceptance Criteria

1. **Structural dirs excluded from status**
   - Given a team with one hired member (role chosen dynamically)
   - When `bm status` is run
   - Then output contains the hired member but NOT schema-level structural dirs (`knowledge`, `invariants`, `projects`, `agent`, `team`) as member entries

2. **Status and members list agree**
   - Given a team with hired members
   - When both `bm status` and `bm members list` are run
   - Then both commands report the same set of member names

3. **Tests fail against current code**
   - Given the current (unfixed) codebase
   - When `cargo test -p bm status_only_lists_hired_members` is run
   - Then the test fails, proving the bug exists

## Metadata
- **Complexity**: Low
- **Labels**: test, regression, bug
- **Required Skills**: Rust, integration testing, CLI output parsing
