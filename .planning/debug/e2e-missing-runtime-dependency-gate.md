---
status: diagnosed
trigger: "E2E test suite fails early when runtime dependencies (GH_TOKEN, gh auth) are missing"
created: 2026-03-05T10:00:00Z
updated: 2026-03-05T10:00:00Z
---

## Current Focus

hypothesis: confirmed — daemon tests hardcode fake token, daemon subprocess calls `gh api` with that fake token, API call fails, no events returned, no members launched, test assertions conditionally skipped
test: n/a — root cause confirmed through code reading
expecting: n/a
next_action: return diagnosis

## Symptoms

expected: The E2E test suite should validate runtime dependencies at startup and fail fast with actionable error messages. Tests that need GitHub auth should not silently degrade by using fake tokens that cause API calls to fail.
actual: No suite-level runtime dependency validation exists. init_to_sync tests use per-test `require_gh_auth!()` macro but daemon tests don't use it at all — they hardcode fake 'ghp_test_token' in setup_daemon_workspace. The fake token causes `gh api` calls inside the daemon to fail silently.
errors: None — tests pass but with degraded coverage
reproduction: Test 13 in UAT. Review daemon_lifecycle.rs setup_daemon_workspace and compare with init_to_sync.rs require_gh_auth!() pattern
started: Discovered during UAT re-verification

## Eliminated

(none — root cause found on first hypothesis)

## Evidence

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon_lifecycle.rs setup_daemon_workspace (line 121)
  found: Hardcodes `gh_token: Some("ghp_test_token".to_string())` in BotminterConfig
  implication: Daemon subprocess reads this fake token from config and passes it to `gh api` calls

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon.rs poll_github_events (line 817)
  found: Calls `Command::new("gh").args(["api", ...])` which uses ambient gh auth or GH_TOKEN
  implication: The daemon process inherits the test's environment but the config has the fake token

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon.rs launch_members_oneshot (line 624-628)
  found: Reads gh_token from team config (`team.credentials.gh_token`) and passes it to ralph via `.env("GH_TOKEN", gh_token)` at line 751
  implication: The fake "ghp_test_token" is passed as GH_TOKEN to ralph subprocess

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon.rs resolve_github_repo (line 854)
  found: Uses `config::load()` which reads from HOME-based config path
  implication: Daemon subprocess reads from the test's fake HOME, gets "test-org/test-repo" as github_repo

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon.rs run_poll_mode (line 488-518)
  found: When poll_github_events fails, it logs ERROR and continues to next sleep cycle — never launches members
  implication: Fake token -> gh api failure -> no events -> no member launch -> stub PID file never written

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon_lifecycle.rs daemon_stop_terminates_running_members (lines 292-311)
  found: Conditional `if stub_pid_file.exists()` fallback with eprintln note about "gh auth likely unavailable"
  implication: Test knows member launch may not happen and silently degrades — the namesake claim "terminates running members" is not actually verified

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon_lifecycle.rs daemon_log_created_on_poll (lines 446-460)
  found: Conditional `if member_log.exists()` with comment "member launch is environment-dependent"
  implication: Same silent degradation pattern — per-member log assertion is skipped

- timestamp: 2026-03-05T10:00:00Z
  checked: main.rs require_gh_auth! macro (line 20-27)
  found: Macro calls `github::gh_auth_ok()` and returns early with SKIP message if no auth
  implication: This macro exists but is only used in init_to_sync.rs — daemon tests don't use it

- timestamp: 2026-03-05T10:00:00Z
  checked: start_to_stop.rs setup_workspace_for_start (line 152)
  found: Also hardcodes `gh_token: Some("ghp_e2e_test_token".to_string())`
  implication: start_to_stop tests also use a fake token, but this is less problematic because `bm start` launches ralph directly without polling GitHub events first — the stub ralph runs regardless of token validity

- timestamp: 2026-03-05T10:00:00Z
  checked: daemon.rs poll flow architecture
  found: Two separate token-dependent code paths: (1) poll_github_events uses `gh` CLI with ambient auth to query events API, (2) launch_ralph_oneshot passes config's gh_token as GH_TOKEN env var to ralph
  implication: Even if ambient gh auth works, poll queries "test-org/test-repo" which doesn't exist — would fail anyway

## Resolution

root_cause: |
  The daemon E2E tests have TWO interacting problems that prevent member launch:

  1. **Fake GitHub repo in config:** setup_daemon_workspace sets `github_repo: "test-org/test-repo"` — a repo that doesn't exist. The daemon's poll_github_events calls `gh api repos/test-org/test-repo/events` which fails even with valid gh auth because the repo doesn't exist.

  2. **Fake GH_TOKEN in config:** setup_daemon_workspace sets `gh_token: Some("ghp_test_token")`. This token is passed to ralph subprocess as GH_TOKEN env var (line 751). Even if events somehow arrived, ralph would get an invalid token.

  The poll_github_events function uses the ambient `gh` CLI auth (not the config's gh_token), so if the test environment has valid gh auth, the `gh api` call authenticates fine but queries a non-existent repo ("test-org/test-repo") and gets a 404.

  The result: daemon starts, polls, gets no events (or errors), never launches members. Tests that depend on member launch (daemon_stop_terminates_running_members, daemon_log_created_on_poll) have conditional fallbacks that silently skip their namesake assertions.

  There is no suite-level gate. The require_gh_auth!() macro exists but is only used in init_to_sync.rs. Daemon tests don't call it.

  **The right fix pattern:**
  - Daemon tests that need member launch must either:
    (a) Use a real GitHub repo + real GH_TOKEN (via require_gh_auth!() gate) so events flow naturally, OR
    (b) Bypass the poll mechanism entirely — inject/simulate events so the daemon launches members without calling gh api. This is the better approach since daemon lifecycle tests should test process management, not GitHub API integration.
  - Option (b) could be implemented by adding a `--trigger-now` or `--launch-immediately` flag to the daemon that skips event polling and launches members on first cycle, or by using webhook mode with a local HTTP POST to trigger launch.
  - A suite-level dependency gate is a nice-to-have but the real fix is making daemon tests not depend on external services for their core assertions (process lifecycle).

fix: ""
verification: ""
files_changed: []
