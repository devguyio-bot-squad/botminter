---
status: diagnosed
trigger: "Investigate why tests write to real ~/.config/botminter, violating invariants/test-path-isolation.md"
created: 2026-03-05T00:00:00Z
updated: 2026-03-05T00:00:00Z
---

## Current Focus

hypothesis: cli_parsing.rs tests invoke bm as subprocess without HOME override; some commands trigger ensure_profiles_initialized() which writes to real ~/.config/botminter/profiles/
test: Traced code paths from cli_parsing.rs through to profile::ensure_profiles_initialized()
expecting: Tests that invoke commands listed under ensure_profiles_initialized callers will write to real config dir
next_action: Report findings

## Symptoms

expected: All tests use tempfile::tempdir() and override $HOME so ~/.config/botminter resolves to a temp path
actual: Tests in cli_parsing.rs invoke bm subprocess without HOME override, causing writes to real ~/.config/botminter
errors: N/A (silent pollution of user's filesystem)
reproduction: Run `cargo test -p bm` and check if ~/.config/botminter gets created/modified
started: Since cli_parsing.rs was created

## Eliminated

(none needed - root cause found on first hypothesis)

## Evidence

- timestamp: 2026-03-05T00:01:00Z
  checked: cli_parsing.rs helper function
  found: `fn bm() -> Command { Command::new(env!("CARGO_BIN_EXE_bm")) }` at line 10-12 creates commands with NO HOME override
  implication: Every test using this helper runs bm against the real user HOME

- timestamp: 2026-03-05T00:02:00Z
  checked: Which commands call ensure_profiles_initialized()
  found: init, hire, roles list, profiles list, profiles describe, teams sync, minty all call ensure_profiles_initialized()
  implication: Any cli_parsing test that invokes these commands past the parsing stage will trigger profile extraction to real ~/.config/botminter/profiles/

- timestamp: 2026-03-05T00:03:00Z
  checked: cli_parsing.rs tests that invoke commands with runtime side effects
  found: Multiple tests invoke commands WITHOUT HOME override that reach runtime (exit code 1, not 2). Key violators include team_flag_short_and_long (hire, status, start, stop, members list, roles list, teams sync, projects add), chat_with_member_parses, chat_with_all_flags_parses, minty_no_args_parses, minty_with_team_flag_parses, teams_show_with_name_parses, members_show_with_member_parses, projects_show_with_project_parses
  implication: These tests run bm commands that fail at runtime (config not found), but some may trigger ensure_profiles_initialized BEFORE failing

- timestamp: 2026-03-05T00:04:00Z
  checked: integration.rs test isolation
  found: integration.rs has a properly isolated bm_cmd(home) helper at line 156-161 that sets both HOME and XDG_CONFIG_HOME. All tests use tempfile::tempdir()
  implication: integration.rs is COMPLIANT

- timestamp: 2026-03-05T00:05:00Z
  checked: e2e test isolation
  found: All e2e tests use tempfile::tempdir() and set HOME via .env("HOME", tmp.path()). The helpers.rs bm_cmd() is HOME-free but all callers in e2e tests add .env("HOME", ...) explicitly
  implication: e2e tests are COMPLIANT (though e2e/helpers.rs bm_cmd() is a risk if used without adding HOME)

- timestamp: 2026-03-05T00:06:00Z
  checked: #[cfg(test)] modules in src/ files
  found: 20+ test modules exist. Checked key ones: config.rs tests use explicit paths via test_config_path(tmp). profile.rs tests use tempdir-based paths. workspace.rs tests use tempdir. None appear to use real HOME.
  implication: Unit tests in src/ are COMPLIANT

- timestamp: 2026-03-05T00:07:00Z
  checked: profiles_dir() and config_dir() path resolution
  found: profiles_dir() uses dirs::config_dir() which respects XDG_CONFIG_HOME, falling back to $HOME/.config. config_dir() uses dirs::home_dir() which reads $HOME.
  implication: Setting HOME env var on subprocess is sufficient to redirect both config paths to temp dirs

- timestamp: 2026-03-05T00:08:00Z
  checked: Specific cli_parsing.rs tests that DO have HOME override
  found: daemon_start_flags_parsed (line 365-387), daemon_stop_flags_parsed (line 391-402), daemon_status_flags_parsed (line 405-417) all use tempfile::tempdir() and .env("HOME", tmp.path()). profiles_describe_show_tags_outputs_tagged_files and similar profile tests use .env("XDG_CONFIG_HOME", ...).
  implication: Some tests in cli_parsing.rs are isolated but many are not

## Resolution

root_cause: The `bm()` helper in `crates/bm/tests/cli_parsing.rs` (line 10-12) creates a `Command` without setting HOME or XDG_CONFIG_HOME. Tests that use this helper to invoke commands that reach runtime execution (not just parse-level failures) can trigger `ensure_profiles_initialized()` which writes to the real `~/.config/botminter/profiles/` directory.

fix: (not applied - diagnosis only)
verification: (not applied - diagnosis only)
files_changed: []
