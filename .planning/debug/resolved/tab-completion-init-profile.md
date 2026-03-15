---
status: diagnosed
trigger: "Tab auto-completion not working for bm init --non-interactive --profile"
created: 2026-03-09T00:00:00Z
updated: 2026-03-09T00:00:00Z
---

## Current Focus

hypothesis: The `init` subcommand is missing from `build_cli_with_completions()` — its `--profile` and `--bridge` args have no `ArgValueCandidates` attached
test: Read completions.rs and verify init is absent from mut_subcommand calls
expecting: init not listed
next_action: document root cause

## Symptoms

expected: Pressing tab after `bm init --non-interactive --profile ` should auto-complete profile names
actual: No completions offered for --profile or --bridge on the init subcommand
errors: None (silent — just no completions)
reproduction: Run `eval "$(bm completions zsh)"` then type `bm init --non-interactive --profile ` and press tab
started: Always — init was never wired into completions

## Eliminated

(none needed — root cause found on first hypothesis)

## Evidence

- timestamp: 2026-03-09T00:00:00Z
  checked: crates/bm/Cargo.toml
  found: clap_complete v4 with "unstable-dynamic" feature is a dependency — dynamic completions infrastructure exists
  implication: The framework for dynamic completions is in place

- timestamp: 2026-03-09T00:00:00Z
  checked: crates/bm/src/completions.rs — build_cli_with_completions()
  found: Function attaches ArgValueCandidates to hire, chat, start, stop, status, teams, members, roles, profiles, projects, knowledge, minty, daemon subcommands — but NOT init
  implication: The init subcommand's --profile and --bridge args never get completion candidates

- timestamp: 2026-03-09T00:00:00Z
  checked: crates/bm/src/completions.rs — CompletionContext
  found: profile_names() method exists and calls profile::list_profiles() — the data source is already implemented
  implication: Adding completions for init --profile only requires wiring, not new data-fetching code

- timestamp: 2026-03-09T00:00:00Z
  checked: crates/bm/src/main.rs line 14
  found: CompleteEnv::with_factory(completions::build_cli_with_completions).complete() is called — dynamic completion dispatch is active
  implication: The shell integration is correct; only the init subcommand mapping is missing

- timestamp: 2026-03-09T00:00:00Z
  checked: crates/bm/src/cli.rs — Command::Init
  found: --profile is Option<String>, --bridge is Option<String> — both are plain string args with no value hints
  implication: Neither clap-level hints nor dynamic candidates are set for these args

- timestamp: 2026-03-09T00:00:00Z
  checked: crates/bm/src/completions.rs — guard test all_commands_covered_by_completions
  found: Test only checks that Command variants can be named in an exhaustive match and that subcommand names exist — it does NOT verify that each subcommand's args have ArgValueCandidates attached
  implication: The guard test gave a false sense of coverage; init passes the test despite having no completions

## Resolution

root_cause: |
  The `build_cli_with_completions()` function in `crates/bm/src/completions.rs` does not include
  a `.mut_subcommand("init", ...)` block. Every other subcommand with dynamic values (hire, chat,
  start, profiles, etc.) has its args wired to `ArgValueCandidates`, but `init` was omitted.

  The `--profile` arg on init should complete with `profiles` (from `ctx.profile_names()`), and
  the `--bridge` arg should complete with bridge names (currently no bridge-name lister exists,
  but profile names could serve as a starting point). The `--team` related args don't apply to
  init since the team doesn't exist yet.

fix: (not applied — diagnosis only)
verification: (not applied — diagnosis only)
files_changed: []
