---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Command Helper Unit Tests

## Description
Add unit tests for pure/testable helper functions across command modules that currently have zero test coverage: `start.rs`, `stop.rs`, `status.rs`, and `members.rs`. These are functions that can be tested without spawning external processes.

## Background
Several command modules expose helper functions that are testable in isolation but have no unit tests. The existing unit test patterns in `config.rs` (7 tests), `profile.rs` (11 tests), and `state.rs` (5 tests) provide good templates — they use `tempfile::tempdir()` for filesystem isolation and test both happy paths and error cases.

## Reference Documentation
**Required:**
- `crates/bm/src/commands/start.rs` — `list_member_dirs()`, `find_workspace()`, `resolve_member_status()`, `require_gh_token()`, `MemberStatus::label()`
- `crates/bm/src/commands/status.rs` — `read_member_role()`, `format_timestamp()`
- `crates/bm/src/commands/members.rs` — `infer_role_from_dir()`
- Existing test patterns: `crates/bm/src/config.rs` (unit test module), `crates/bm/src/state.rs`

## Technical Requirements

### `start.rs` unit tests
1. `list_member_dirs` — returns sorted dir names, skips hidden dirs, skips files
2. `find_workspace` — finds project-mode workspace (subdir with `.botminter/`), finds no-project workspace (dir itself has `.botminter/`), returns None for missing dir, returns None for dir without `.botminter/`
3. `resolve_member_status` — returns Running for alive PID, returns Crashed for dead PID in state, returns Stopped for absent key
4. `require_gh_token` — returns token when present, errors with team name when absent
5. `MemberStatus::label` — returns correct string for each variant

### `status.rs` unit tests
6. `read_member_role` — reads role from `botminter.yml`, falls back to dir-name inference when YAML missing, falls back when role field absent
7. `format_timestamp` — formats RFC 3339 to display format, passes through unparseable strings unchanged

### `members.rs` unit tests
8. `infer_role_from_dir` — extracts role from `role-name` pattern, handles no-hyphen names, handles multiple hyphens (takes first segment)

## Dependencies
- Functions in `start.rs` must be made `pub(crate)` or the test module placed inside the file (`#[cfg(test)] mod tests`)
- `status.rs` helpers are currently private — tests should be `#[cfg(test)]` inside the file
- `tempfile` already in dev-dependencies

## Implementation Approach
1. Add `#[cfg(test)] mod tests` blocks inside each command file
2. For filesystem tests (`list_member_dirs`, `find_workspace`, `read_member_role`): create temp dirs with the expected structure
3. For state-dependent tests (`resolve_member_status`): construct `RuntimeState` in-memory with known PIDs
4. For pure functions (`format_timestamp`, `infer_role_from_dir`, `MemberStatus::label`): straightforward input/output assertions
5. Keep `require_gh_token` test as a unit test constructing `TeamEntry` directly

## Acceptance Criteria

1. **list_member_dirs correctness**
   - Given a directory with `alice/`, `bob/`, `.hidden/`, and `file.txt`
   - When `list_member_dirs()` is called
   - Then it returns `["alice", "bob"]` (sorted, no hidden, no files)

2. **find_workspace project mode**
   - Given `team_ws_base/member/project/.botminter/` exists
   - When `find_workspace(team_ws_base, "member")` is called
   - Then it returns `Some(team_ws_base/member/project)`

3. **find_workspace no-project mode**
   - Given `team_ws_base/member/.botminter/` exists (no project subdir)
   - When `find_workspace(team_ws_base, "member")` is called
   - Then it returns `Some(team_ws_base/member)`

4. **find_workspace missing**
   - Given `team_ws_base/member/` does not exist
   - When `find_workspace()` is called
   - Then it returns `None`

5. **resolve_member_status variants**
   - Given a RuntimeState with a known alive PID and a known dead PID
   - When `resolve_member_status()` is called for each
   - Then it returns `Running`, `Crashed`, and `Stopped` respectively

6. **read_member_role from YAML**
   - Given a member dir with `botminter.yml` containing `role: architect`
   - When `read_member_role()` is called
   - Then it returns `"architect"`

7. **read_member_role fallback**
   - Given a member dir named `architect-alice` with no `botminter.yml`
   - When `read_member_role()` is called
   - Then it returns `"architect"` (inferred from dir name)

8. **format_timestamp formatting**
   - Given `"2026-02-21T10:30:00+00:00"`
   - When `format_timestamp()` is called
   - Then it returns `"2026-02-21 10:30:00"`

9. **format_timestamp passthrough**
   - Given `"not-a-timestamp"`
   - When `format_timestamp()` is called
   - Then it returns `"not-a-timestamp"` unchanged

10. **infer_role_from_dir patterns**
    - Given dir names `"architect-alice"`, `"po-bob-senior"`, `"superman"`
    - When `infer_role_from_dir()` is called for each
    - Then it returns `"architect"`, `"po"`, `"superman"` respectively

## Metadata
- **Complexity**: Medium
- **Labels**: test, unit-test
- **Required Skills**: Rust, unit testing, tempfile, process state
