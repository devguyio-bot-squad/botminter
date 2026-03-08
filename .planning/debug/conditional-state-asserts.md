---
status: diagnosed
trigger: "E2E state assertions are unconditional -- state.json existence is asserted, not conditionally checked"
created: 2026-03-05T12:00:00Z
updated: 2026-03-05T12:00:00Z
---

## Current Focus

hypothesis: confirmed -- three tests use `if state_path.exists()` guard instead of `assert!(state_path.exists())`, silently accepting file absence as success
test: verified production code paths in stop.rs and status.rs
expecting: n/a -- root cause confirmed
next_action: return diagnosis

## Symptoms

expected: After `bm stop` or `bm status` (crash detection), tests should unconditionally assert that state.json exists and has empty members.
actual: Three tests in start_to_stop.rs wrap members-empty assertion in `if state_path.exists()`. Since state::save() always writes the file after member removal, file absence would be a regression to catch, not an acceptable outcome.
errors: None -- tests pass but skip assertions when guard is false
reproduction: Lines 308, 402, 452 in start_to_stop.rs
started: Present since test creation

## Eliminated

(none needed -- root cause identified on first hypothesis)

## Evidence

- timestamp: 2026-03-05T12:00:00Z
  checked: stop.rs lines 46, 55, 63
  found: Every code path that removes a member from runtime_state immediately calls `state::save(&runtime_state)?`. Graceful stop (line 63), force stop (line 55), and already-exited (line 46) all save. The only path that skips save is graceful_stop failure (line 67-75), but that path returns an error and does NOT remove the member from state.
  implication: After a successful `bm stop`, state.json is always written to disk with the member removed.

- timestamp: 2026-03-05T12:00:00Z
  checked: status.rs lines 158-164
  found: When crashed members are detected, they are removed from runtime_state and `state::save(&runtime_state)?` is called unconditionally (line 163).
  implication: After `bm status` detects a crash, state.json is always written to disk with the crashed member removed.

- timestamp: 2026-03-05T12:00:00Z
  checked: start_to_stop.rs lines 306-317, 400-409, 450-460
  found: All three locations use identical pattern: `if state_path.exists() { ... assert members empty ... }`. Line 317 has comment "If state.json doesn't exist, that's also fine (clean state)" -- but this is incorrect given the production code always writes the file.
  implication: The `if` guard makes the assertion a no-op when the file is missing. If a regression caused stop/status to delete state.json instead of writing an empty-members version, these tests would silently pass.

- timestamp: 2026-03-05T12:00:00Z
  checked: state.rs save_to() lines 55-68
  found: save_to() creates parent directories, serializes to JSON, writes atomically via temp file + rename. It never conditionally skips writing. It always produces a file on disk.
  implication: state.json absence after any save() call is a bug, not an acceptable state.

## Resolution

root_cause: Defensive coding pattern -- the three test sites treat state.json absence as equivalent to "clean state" (empty members), but the production code (stop.rs and status.rs) always calls state::save() after removing members, which always writes the file. The `if state_path.exists()` guard makes the members-empty assertion conditional, meaning a regression that deletes the file instead of clearing it would go undetected.
fix: (diagnosis only)
verification: (diagnosis only)
files_changed: []
