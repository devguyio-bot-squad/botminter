---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Fix Member Discovery Path Bug

## Description
Fix the off-by-one directory level bug in `status.rs` and `start.rs` where `team_repo` is derived as `team.path` instead of `team.path.join("team")`. After this fix, the regression test from task-01 should pass (green phase).

## Background
`team.path` in the config points to the **team directory** (e.g., `workzone/my-team/`), not the team repo. The team repo lives at `workzone/my-team/team/`. Two commands derive `team_repo` incorrectly:

- `status.rs:23`: `let team_repo = &team.path;` — should be `team.path.join("team")`
- `start.rs:16`: `let team_repo = &team.path;` — should be `team.path.join("team")`

This causes:
- `status.rs` scans `workzone/my-team/team/` (team repo root) instead of `workzone/my-team/team/team/` (members dir), listing structural dirs as members
- `start.rs` looks for `botminter.yml` at `workzone/my-team/botminter.yml` instead of `workzone/my-team/team/botminter.yml`, and scans the wrong directory for members

`members.rs` is correct: `let team_repo = team.path.join("team");`

## Reference Documentation
**Required:**
- `crates/bm/src/commands/status.rs:23` — bug location
- `crates/bm/src/commands/start.rs:16` — bug location
- `crates/bm/src/commands/members.rs:22` — correct pattern to match

## Technical Requirements
1. In `status.rs`: Change `let team_repo = &team.path;` to `let team_repo = team.path.join("team");`
2. In `start.rs`: Change `let team_repo = &team.path;` to `let team_repo = team.path.join("team");`
3. Update any downstream references that relied on the old (incorrect) `team_repo` value — adjust `.join("team")` calls on `members_dir` since `team_repo` now already includes the `team/` segment
4. Verify all references to `team_repo` in both files still resolve correctly:
   - `team_repo.join("botminter.yml")` → should find the manifest
   - `team_repo.join("team")` → should find the members directory
5. Run the regression test from task-01 — it should now pass
6. Run the full test suite — no regressions

## Dependencies
- Task-01 regression test must exist (to validate the fix)

## Implementation Approach
1. Fix `status.rs:23`: `let team_repo = team.path.join("team");`
2. Fix `start.rs:16`: `let team_repo = team.path.join("team");`
3. Audit both files for any `.join("team")` calls that would now double-join — verify each resolves correctly:
   - `status.rs`: `members_dir = team_repo.join("team")` → `workzone/<team>/team/team/` (correct members dir)
   - `start.rs:19`: `team_repo.join("botminter.yml")` → `workzone/<team>/team/botminter.yml` (correct manifest)
   - `start.rs:48`: `team_repo.join("team")` → `workzone/<team>/team/team/` (correct members dir)
4. Run `cargo test -p bm` — all tests should pass
5. Run `cargo clippy -p bm -- -D warnings` — no warnings

## Acceptance Criteria

1. **Status lists only hired members**
   - Given a team with any profile and one hired member
   - When `bm status` is run
   - Then only the hired member appears (no schema-level structural dirs)

2. **Start discovers correct members**
   - Given a team with hired members and provisioned workspaces
   - When `bm start` is run (with ralph unavailable — expected error)
   - Then only hired member names appear in output, not structural dirs

3. **Regression test passes**
   - Given the regression test from task-01
   - When `cargo test -p bm status_only_lists_hired_members` is run
   - Then the test passes

4. **No test regressions**
   - Given the full test suite
   - When `cargo test -p bm` is run
   - Then all existing tests still pass

5. **Clippy clean**
   - Given the fixed code
   - When `cargo clippy -p bm -- -D warnings` is run
   - Then no warnings are emitted

## Metadata
- **Complexity**: Low
- **Labels**: bug-fix, member-discovery
- **Required Skills**: Rust, path resolution
