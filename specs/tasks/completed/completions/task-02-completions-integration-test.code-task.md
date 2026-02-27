---
status: done
created: 2026-02-21
started: 2026-02-21
completed: 2026-02-21
---
# Task: Integration Tests for Completions Command

## Description
Add integration tests for the `bm completions` subcommand to verify it generates valid, non-empty completion scripts for all supported shells and handles edge cases correctly.

## Background
The existing integration tests in `crates/bm/tests/integration.rs` use `assert_cmd` patterns (running the `bm` binary as a subprocess and asserting on exit code and output). The completions tests should follow the same patterns established there.

## Technical Requirements
1. Add test cases in `crates/bm/tests/integration.rs` (or a new test file if the existing one is already large)
2. Test all five supported shells: bash, zsh, fish, powershell, elvish
3. Verify each produces non-empty stdout and exits successfully
4. Verify invalid shell names produce a non-zero exit code
5. Follow existing test patterns and conventions in the repo

## Dependencies
- Task 01 (add-completions-command) must be completed first
- Existing test infrastructure in `crates/bm/tests/`

## Implementation Approach
1. Review existing integration test patterns in `crates/bm/tests/integration.rs`
2. Add a test module or section for completions
3. For each shell, run `bm completions <shell>` via `Command::cargo_bin("bm")` and assert:
   - Exit status is success
   - Stdout is not empty
   - Stdout contains a shell-specific marker (e.g., `complete` for bash, `compdef` for zsh, `complete -c` for fish)
4. Add a negative test for an invalid shell name

## Acceptance Criteria

1. **All shells produce output**
   - Given the test binary is built
   - When the test runs `bm completions <shell>` for each of bash, zsh, fish, powershell, elvish
   - Then each invocation exits with code 0 and produces non-empty stdout

2. **Output contains shell-specific markers**
   - Given the test runs `bm completions bash`
   - When the output is inspected
   - Then it contains bash-specific completion syntax (e.g., the string `bm`)

3. **Invalid shell is rejected**
   - Given the test binary is built
   - When the test runs `bm completions notashell`
   - Then the process exits with a non-zero code

## Metadata
- **Complexity**: Low
- **Labels**: cli, testing, shell-completions
- **Required Skills**: Rust, assert_cmd, integration testing
