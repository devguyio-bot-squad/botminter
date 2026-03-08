---
status: diagnosed
trigger: "daemon_stop_terminates_running_members conditional fallback — daemon never launches members due to fake GH_TOKEN"
created: 2026-03-05T09:30:00Z
updated: 2026-03-05T09:45:00Z
---

## Current Focus

hypothesis: CONFIRMED — poll_github_events calls `gh api` which inherits the process environment; the daemon subprocess inherits the test's env but `gh api` uses GH_TOKEN from inherited env. The daemon `start()` spawns `bm daemon-run` via Command::new(exe) without setting GH_TOKEN — it inherits whatever is in the parent environment. BUT the test sets HOME to tmp, and config has fake 'ghp_test_token'. The `gh` CLI picks up GH_TOKEN from env OR from `gh auth` config (in HOME). Since HOME is tmp (no gh auth config), and no GH_TOKEN env var is set by the test, `gh api` either uses no auth or the parent process's GH_TOKEN — BUT this is for `poll_github_events` which runs INSIDE the daemon subprocess, not during member launch.
test: Traced the full chain
expecting: n/a — confirmed
next_action: Return diagnosis

## Symptoms

expected: daemon_stop_terminates_running_members should unconditionally poll for stub PID file with timeout and assert child process termination after daemon stop
actual: The test has a conditional `if stub_pid_file.exists()` fallback at line 293 that silently skips the child termination assertion
errors: None — test passes but skips its namesake claim
reproduction: cargo test -p bm --test e2e -- daemon_stop_terminates_running_members
started: After plan 01-08 execution

## Eliminated

- hypothesis: The daemon subprocess doesn't have access to GH_TOKEN at all
  evidence: The daemon is spawned via Command::new(exe) at line 135 of daemon.rs with NO explicit env manipulation — it inherits the full parent environment. The `gh` CLI inside `poll_github_events` (line 817) also inherits environment. So if the test process has a real GH_TOKEN, the daemon subprocess DOES have access to it.
  timestamp: 2026-03-05T09:35:00Z

## Evidence

- timestamp: 2026-03-05T09:32:00Z
  checked: daemon.rs start() function (lines 78-180)
  found: `bm daemon start` spawns `bm daemon-run` as a detached child via Command::new(exe). No explicit env vars are set or removed — the subprocess inherits the full parent environment.
  implication: The daemon subprocess CAN access GH_TOKEN from the environment if the test process has it.

- timestamp: 2026-03-05T09:33:00Z
  checked: daemon.rs run_poll_mode() function (lines 460-526)
  found: run_poll_mode calls resolve_github_repo() which reads config from disk (HOME-relative), then calls poll_github_events() which runs `gh api repos/{repo}/events`. The `gh` command inherits the daemon process's environment.
  implication: `gh api` will use whatever auth is available — GH_TOKEN env var or gh auth login state in HOME.

- timestamp: 2026-03-05T09:34:00Z
  checked: daemon.rs poll_github_events() function (lines 813-851)
  found: Runs `gh api repos/{github_repo}/events --paginate --jq ...`. The github_repo comes from config (resolve_github_repo reads team.github_repo from config.yml). The test config sets github_repo to "test-org/test-repo" — a FAKE repo that doesn't exist on GitHub.
  implication: CRITICAL — Even with a real GH_TOKEN, `gh api repos/test-org/test-repo/events` will return a 404 error because "test-org/test-repo" doesn't exist. The daemon will log "Failed to poll GitHub events" and never call handle_member_launch().

- timestamp: 2026-03-05T09:36:00Z
  checked: daemon.rs handle_member_launch → launch_members_oneshot (lines 529-692)
  found: Member launch only happens when poll_github_events returns relevant events (line 495: `if relevant_count > 0`). It then calls launch_members_oneshot which discovers members, finds workspaces, and spawns `ralph run` in each workspace.
  implication: The chain is: poll finds events → launch members → ralph writes .ralph-stub-pid. If poll fails (fake repo) or returns no events, members are never launched.

- timestamp: 2026-03-05T09:38:00Z
  checked: daemon_lifecycle.rs setup_daemon_workspace() (lines 84-132)
  found: Config uses github_repo "test-org/test-repo" (fake) and gh_token "ghp_test_token" (fake). The daemon_cmd() sets HOME to tmp and PATH to include stub ralph, but does NOT set GH_TOKEN.
  implication: Two independent problems: (1) fake repo name means gh api always 404s even with real auth, (2) fake gh_token in config is only used for member launch (launch_ralph_oneshot sets env GH_TOKEN from config), not for gh api polling.

- timestamp: 2026-03-05T09:40:00Z
  checked: How gh CLI resolves authentication
  found: `gh` CLI checks GH_TOKEN env var first, then GITHUB_TOKEN, then gh auth login state (stored in $HOME/.config/gh/). Since the test sets HOME to tmp (no gh auth config), and the daemon subprocess inherits the parent env, gh will use GH_TOKEN from the parent env if present. BUT even with valid auth, the repo "test-org/test-repo" doesn't exist.
  implication: The fundamental problem is the FAKE REPO, not the fake token. Even injecting a real GH_TOKEN won't help because test-org/test-repo returns 404.

- timestamp: 2026-03-05T09:42:00Z
  checked: Whether there's any mechanism to trigger member launch without real GitHub events
  found: run_poll_mode has exactly one path to member launch: poll_github_events returns events with relevant types. There is no "launch all members on startup" or "launch on first poll regardless of events" behavior.
  implication: The test CANNOT trigger member launch through the normal poll path without either (a) a real repo with real events, or (b) modifying the daemon to support a test/dry-run mode that launches members immediately.

## Resolution

root_cause: |
  The daemon_stop_terminates_running_members test cannot verify child termination because the daemon never launches members. This is caused by TWO compounding issues in the test setup:

  1. **Fake repository name**: setup_daemon_workspace sets `github_repo: "test-org/test-repo"` — a non-existent GitHub repository. When the daemon's `poll_github_events()` calls `gh api repos/test-org/test-repo/events`, it gets a 404 error regardless of authentication, so no events are found.

  2. **No alternative launch trigger**: `run_poll_mode` only launches members when `poll_github_events` returns relevant events (issues, issue_comment, pull_request). There is no "launch on startup" or "dry-run" mode. The only path to member launch is: successful poll → relevant events found → handle_member_launch().

  The fake GH_TOKEN ("ghp_test_token") in the config is a secondary issue — it's only used by `launch_ralph_oneshot` to set GH_TOKEN for the ralph subprocess (line 751), NOT by `poll_github_events`. The `gh api` call in poll_github_events inherits auth from the daemon process's environment (GH_TOKEN env var or gh auth state), but even with valid auth, the fake repo guarantees failure.

  **Why "just use real GH_TOKEN" won't fix it**: Even with a real token from the environment, `gh api repos/test-org/test-repo/events` returns 404 because that repo doesn't exist. You'd need BOTH a real token AND a real repo with actual events.

  **The real fix direction**: The test needs to bypass the GitHub polling layer entirely. Options:
  - (a) Add a `--launch-immediately` or `--dry-run` flag to daemon poll mode that launches members on the first cycle without requiring GitHub events
  - (b) Create a real test repo (like init_to_sync tests do with TempRepo) and create a real event in it before starting the daemon — heavyweight but tests the full path
  - (c) Refactor poll_github_events to be injectable/mockable so tests can provide fake events
  - (d) Add a file-based event injection mechanism (daemon watches a local file for events in addition to GitHub)

fix: ""
verification: ""
files_changed: []
