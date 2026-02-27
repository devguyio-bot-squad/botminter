---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: CLI Argument Parsing & Error Tests

## Description
Add tests verifying CLI argument parsing, flag threading, command aliases, and user-facing error messages. These tests exercise clap's argument definitions without requiring a real team setup — they focus on parsing correctness and help text.

## Background
The CLI uses `clap` derive for argument parsing (`cli.rs`). While completions are tested (6 tests), the rest of the argument surface — aliases, flag propagation, required args, help text — has no dedicated tests. Clap can silently break if derive attributes change.

## Reference Documentation
**Required:**
- `crates/bm/src/cli.rs` — clap derive definitions, all commands and flags
- `crates/bm/src/main.rs` — command dispatch

## Technical Requirements

### Command aliases (2 tests)
1. `start_and_up_are_aliases` — `bm up` and `bm start` parse to the same command
2. `aliases_shown_in_help` — `bm --help` lists `up` as an alias for `start`

### Flag parsing (5 tests)
3. `team_flag_short_and_long` — `-t myteam` and `--team myteam` both parse correctly for `hire`, `status`, `start`, `stop`, `members list`, `roles list`, `teams sync`, `projects add`
4. `force_flag_on_stop` — `bm stop --force` parses correctly
5. `push_flag_on_sync` — `bm teams sync --push` parses correctly
6. `verbose_flag_on_status` — `bm status -v` and `bm status --verbose` both parse
7. `name_flag_on_hire` — `bm hire architect --name alice` parses, `bm hire architect -n alice` also works

### Required arguments (3 tests)
8. `hire_requires_role_argument` — `bm hire` with no role arg exits with error mentioning required arg
9. `profiles_describe_requires_profile_name` — `bm profiles describe` with no arg errors
10. `projects_add_requires_url` — `bm projects add` with no URL errors

### Unknown/invalid input (3 tests)
11. `unknown_subcommand_errors` — `bm foobar` exits non-zero with helpful error
12. `unknown_flag_errors` — `bm status --foobar` exits non-zero
13. `completions_requires_valid_shell` — Already tested, verify it's still working

### Help text (2 tests)
14. `help_flag_shows_all_commands` — `bm --help` lists all top-level commands
15. `subcommand_help_works` — `bm teams --help` lists `list` and `sync` subcommands

## Dependencies
- `bm` binary must be built (`env!("CARGO_BIN_EXE_bm")`)
- No team setup needed — these tests only exercise argument parsing

## Implementation Approach
1. Create a new test file `crates/bm/tests/cli_parsing.rs` to keep these separate from integration tests (they don't need ENV_MUTEX or team setup)
2. Use `Command::new(env!("CARGO_BIN_EXE_bm"))` to invoke the binary
3. For parsing-only tests: check exit code and stderr/stdout content
4. For alias tests: compare behavior or help output
5. These tests are fast (no filesystem setup) and can run in parallel

## Acceptance Criteria

1. **Alias equivalence**
   - Given the `bm` binary
   - When `bm up --help` and `bm start --help` are run
   - Then both produce equivalent help text (same command)

2. **Flag threading**
   - Given the `bm` binary
   - When `-t myteam` is passed to any team-scoped command
   - Then the command accepts the flag without parsing errors (exit code may be non-zero for missing config, but NOT for parsing)

3. **Missing required args**
   - Given the `bm` binary
   - When `bm hire` is run with no arguments
   - Then exit code is non-zero and stderr mentions the missing `<ROLE>` argument

4. **Unknown subcommand**
   - Given the `bm` binary
   - When `bm foobar` is run
   - Then exit code is non-zero and stderr contains a suggestion or error

5. **Help completeness**
   - Given the `bm` binary
   - When `bm --help` is run
   - Then output contains all top-level commands: init, hire, start, stop, status, teams, members, roles, profiles, projects, completions

## Metadata
- **Complexity**: Low
- **Labels**: test, cli, argument-parsing
- **Required Skills**: Rust, clap, CLI testing
