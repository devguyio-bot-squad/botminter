# Codebase Concerns

**Analysis Date:** 2026-03-10

## Tech Debt

**Profile staleness detection missing:**
- Issue: `ensure_profiles_initialized()` in `crates/bm/src/profile.rs` (line 585-599) returns early if ANY profile subdirectory exists on disk, with no version or content staleness check. When the embedded profile structure changes (e.g., `members/` renamed to `roles/` in commit 6499c62), stale disk profiles cause silent failures.
- Files: `crates/bm/src/profile.rs`, `crates/bm/src/commands/profiles_init.rs`
- Impact: Users with stale profiles get confusing errors like "Available roles: (empty list)" because the disk layout no longer matches what the code expects. Diagnosed in `.planning/debug/scrum-compact-roles-empty.md`.
- Fix approach: Compare embedded profile version against an on-disk version marker. If mismatched, auto-update or prompt. The `embedded_profile_version()` function (line 568) already exists but is unused for staleness detection.

**Profile-core coupling:**
- Issue: `bm` CLI source contains hardcoded assumptions about profile conventions (label bootstrapping, status field configurations, workspace assembly patterns). Adding a new profile requires changes to CLI source code.
- Files: `crates/bm/src/commands/init.rs`, `crates/bm/src/workspace.rs`, `crates/bm/src/profile.rs`
- Impact: Profile extensibility is blocked. Detailed in `.planning/todos/pending/2026-03-06-decouple-profile-specific-logic-from-botminter-core.md`.
- Fix approach: Research and design a hook/lifecycle model where profiles declare what happens at each stage (init, hire, sync, start) and the CLI executes those declarations.

**GH_TOKEN passed to `gh` CLI via `.env()` in 19+ call sites:**
- Issue: Every function that shells out to `gh` manually adds `.env("GH_TOKEN", token)`. This is repeated across `crates/bm/src/commands/init.rs` (15 sites), `crates/bm/src/workspace.rs` (2 sites), `crates/bm/src/commands/start.rs` (1 site), `crates/bm/src/commands/daemon.rs` (1 site). No shared helper abstracts this.
- Files: `crates/bm/src/commands/init.rs`, `crates/bm/src/workspace.rs`, `crates/bm/src/commands/start.rs`, `crates/bm/src/commands/daemon.rs`
- Impact: Easy to forget the `.env()` call when adding new `gh` invocations, leading to auth failures. Maintenance burden for any auth model change.
- Fix approach: Create a `gh_command(token: Option<&str>) -> Command` helper that pre-configures `GH_TOKEN` when present, and use it everywhere.

**`init.rs` is 1792 lines with mixed concerns:**
- Issue: `crates/bm/src/commands/init.rs` contains the full interactive wizard, non-interactive mode, GitHub API helpers (label creation, project creation, repo management), org/repo selection UI, and project URL validation. This is the largest source file.
- Files: `crates/bm/src/commands/init.rs`
- Impact: Difficult to test individual concerns in isolation. Hard to navigate and modify safely.
- Fix approach: Extract GitHub API helpers into a `github.rs` module, and interactive selection UI into a `wizard.rs` or `prompts.rs` module.

**Executable permissions lost during profile extraction:**
- Issue: The `include_dir` crate used for compile-time embedding does not preserve Unix file permissions. Shell scripts in profiles (e.g., `profiles/scrum/coding-agent/skills/gh/scripts/`) are extracted as `-rw-r--r--` instead of `-rwxr-xr-x`.
- Files: `crates/bm/src/commands/profiles_init.rs`, `crates/bm/src/profile.rs`
- Impact: Direct execution of extracted scripts fails (`./scripts/foo.sh`). Workaround exists (`bash scripts/foo.sh`). Detailed in `.planning/todos/pending/2026-03-07-preserve-executable-permissions-in-bm-profiles-init-extraction.md`.
- Fix approach: After extraction, scan for `.sh` extension and `chmod +x`, or store a manifest of executable files.

**Formation credential resolution is incomplete:**
- Issue: Two TODO comments in `crates/bm/src/commands/start.rs` (lines 113 and 430) note that bridge env var naming is hardcoded to `RALPH_TELEGRAM_BOT_TOKEN` and that the formation manager should resolve per-member credentials via CredentialStore.
- Files: `crates/bm/src/commands/start.rs`
- Impact: Adding a second bridge type (e.g., Rocket.Chat) will require refactoring credential resolution to be bridge-type-aware.
- Fix approach: Make the env var name bridge-type-driven from the bridge manifest rather than hardcoded.

## Known Bugs

**No known production bugs at this time.** Clippy passes clean with `-D warnings`. All diagnosed issues in `.planning/debug/` are test-level defects (see Test Coverage Gaps below).

## Security Considerations

**GH_TOKEN stored in plaintext config file:**
- Risk: `~/.botminter/config.yml` contains `gh_token` in plaintext YAML. While the file is written with `0o600` permissions (`crates/bm/src/config.rs` line 106-108), the token is still plaintext on disk.
- Files: `crates/bm/src/config.rs` (lines 44-45, 96-111)
- Current mitigation: File permissions set to 0600; `check_permissions()` warns on load if permissions are wrong (line 143-157). Bridge tokens (Telegram) use system keyring via `LocalCredentialStore`.
- Recommendations: Migrate GH_TOKEN to the system keyring alongside bridge tokens. The `keyring` crate infrastructure already exists in `crates/bm/src/bridge.rs`.

**Unsafe signal handling in daemon:**
- Risk: The daemon uses `libc::signal()` with a bare function pointer for SIGTERM/SIGINT handling (`crates/bm/src/commands/daemon.rs` lines 314-322). The handler writes to an `AtomicBool` which is safe, but `libc::signal()` itself is not async-signal-safe on all platforms and the handler function is a bare `extern "C"` function.
- Files: `crates/bm/src/commands/daemon.rs` (lines 310-352)
- Current mitigation: Handler only writes to `AtomicBool`, which is signal-safe. A polling thread reads the flag.
- Recommendations: Consider using `signal-hook` crate for more robust signal handling, or `ctrlc` crate for portable handling.

**Unsafe `libc::kill()` calls without return value checks:**
- Risk: Multiple `libc::kill()` calls ignore the return value: `crates/bm/src/commands/daemon.rs` (lines 213-215, 227-229), `crates/bm/src/commands/stop.rs` (lines 158-160). If `kill()` fails (e.g., EPERM), the code proceeds as if the signal was delivered.
- Files: `crates/bm/src/commands/stop.rs` (line 158-160), `crates/bm/src/commands/daemon.rs` (lines 213, 227, 578, 589)
- Current mitigation: `is_alive()` polling after kill catches most cases (process still alive = retry). `force_stop()` sleeps 500ms after SIGTERM but does not verify death.
- Recommendations: Check `kill()` return value and log/handle EPERM or ESRCH errors explicitly.

**PID reuse vulnerability in state management:**
- Risk: `state::is_alive()` uses `kill(pid, 0)` to check process liveness. Between process death and stale-entry cleanup, the PID could be reassigned to an unrelated process. `force_stop()` would then SIGTERM a random process.
- Files: `crates/bm/src/state.rs` (line 72-75), `crates/bm/src/commands/stop.rs` (line 157-163)
- Current mitigation: None. The window is small on modern systems but nonzero.
- Recommendations: Store process start time alongside PID and verify it matches before sending signals. On Linux, read `/proc/<pid>/stat` start time.

## Performance Bottlenecks

**Sequential workspace sync:**
- Problem: `bm teams sync` provisions workspaces one member at a time, each involving git clone, submodule add, and potentially GitHub API calls.
- Files: `crates/bm/src/commands/teams.rs`, `crates/bm/src/workspace.rs`
- Cause: Serial iteration over members with blocking `Command::new("gh")` and `Command::new("git")` calls.
- Improvement path: Parallelize workspace creation with `rayon` or `tokio`. Already identified in `.planning/todos/pending/2026-03-06-improve-teams-sync-push-speed-with-concurrency-and-progress-bar.md`.

**Sequential member launch in `bm start`:**
- Problem: Members are launched one at a time in a for loop (`crates/bm/src/commands/start.rs` lines 152-232). Each launch involves process spawning and state file writing.
- Files: `crates/bm/src/commands/start.rs`
- Cause: Serial loop with `state::save()` after each launch.
- Improvement path: Launch processes in parallel, batch state saves.

## Fragile Areas

**State file (state.json) has no locking:**
- Files: `crates/bm/src/state.rs`, `crates/bm/src/commands/start.rs`, `crates/bm/src/commands/stop.rs`, `crates/bm/src/commands/status.rs`
- Why fragile: Multiple `bm` commands (`start`, `stop`, `status`, `members list`) read and write `~/.botminter/state.json` without file locking. If `bm start` and `bm stop` run concurrently, they can overwrite each other's changes. `start.rs` does load -> modify -> save in multiple places (lines 140-146, 218, 231); `stop.rs` does the same (lines 21, 43, 52, 60).
- Safe modification: Always load state immediately before modifying and save immediately after. Consider adding `flock()`-based file locking.
- Test coverage: Unit tests cover save/load round-trip but not concurrent access.

**Daemon signal handling with global static:**
- Files: `crates/bm/src/commands/daemon.rs` (lines 347-352)
- Why fragile: `SHUTDOWN_FLAG` is a global `AtomicBool`. The `sigterm_handler` is a bare `extern "C"` function. Multiple daemon instances in the same process (tests) would share this global state.
- Safe modification: Do not run multiple daemon instances in-process. Tests that test daemon behavior should use subprocess isolation.
- Test coverage: No unit tests for signal handling path.

**`let _ =` error suppression in daemon (21 sites):**
- Files: `crates/bm/src/commands/daemon.rs` (lines 108, 182, 183, 234, 236, 238, 261, 269, 271, 384, 394, 411, 426, 592, 878, 898, 906)
- Why fragile: File removal, HTTP response sending, log rotation, and child process wait errors are silently discarded. If `fs::remove_file` fails on a PID file, the daemon may fail to start next time because the stale PID file remains.
- Safe modification: Log errors via `daemon_log()` instead of discarding. For PID file cleanup, at minimum log the failure.
- Test coverage: None for error paths in daemon cleanup.

## Scaling Limits

**Single state.json file for all teams:**
- Current capacity: Handles tens of members across a few teams.
- Limit: All teams share one `~/.botminter/state.json`. No partitioning. Concurrent multi-team operations risk write conflicts.
- Scaling path: Partition state per team (`~/.botminter/<team>/state.json`) or use a database.

**GitHub API rate limits during init/sync:**
- Current capacity: Works for teams with < 50 labels and a few projects.
- Limit: `bm init` creates labels one at a time via `gh label create` (loop in `bootstrap_labels`). Large profile with many labels could hit GitHub API rate limits.
- Scaling path: Batch label creation via GraphQL API instead of REST.

## Dependencies at Risk

**`keyring` crate platform sensitivity:**
- Risk: The `keyring` crate's Linux backend requires a running Secret Service provider (gnome-keyring-daemon) with an initialized and unlocked login collection. This is unreliable on headless/SSH/su access.
- Impact: `bridge identity add` fails on fresh Linux installs or headless servers. Detailed diagnosis in `.planning/debug/keyring-report.md` and fix proposal in `.planning/todos/pending/2026-03-09-improve-local-formation-keyring-ux.md`.
- Migration plan: Improve error messages to distinguish failure modes. Consider offering a fallback encrypted file store for headless environments.

**`include_dir` crate limitations:**
- Risk: Does not preserve file permissions (executable bits lost). Does not support symlinks.
- Impact: Extracted shell scripts are not executable. See executable permissions concern above.
- Migration plan: Post-extraction `chmod` for known executable extensions, or switch to `rust-embed` which has similar limitations but wider adoption.

## Missing Critical Features

**No `bm doctor` / prerequisite checker:**
- Problem: No single command validates that all runtime prerequisites are met (gh CLI installed, gh authenticated, keyring unlocked, ralph installed, required env vars set).
- Blocks: Users encounter failures deep in `bm start` or `bm init` instead of getting upfront guidance.

**No config migration tooling:**
- Problem: Alpha policy says breaking changes expected, but there is no tooling to detect that config.yml or on-disk profiles are from an incompatible version.
- Blocks: Users hit confusing runtime errors after binary upgrades instead of clear "re-run bm init" guidance.

## Test Coverage Gaps

**CLI parsing tests lack HOME isolation:**
- What's not tested: `crates/bm/tests/cli_parsing.rs` uses a `bm()` helper (line 10-12) that creates `Command` without setting `HOME` or `XDG_CONFIG_HOME`. Tests that reach runtime trigger `ensure_profiles_initialized()` which writes to the real `~/.config/botminter/profiles/`.
- Files: `crates/bm/tests/cli_parsing.rs`
- Risk: Tests pollute the developer's real config directory. Violates `invariants/test-path-isolation.md`. Diagnosed in `.planning/debug/test-path-isolation.md`.
- Priority: Medium -- causes filesystem side effects in dev environments.

**Daemon E2E tests use fake tokens and conditionally skip assertions:**
- What's not tested: Daemon lifecycle tests in E2E hardcode `gh_token: "ghp_test_token"` and `github_repo: "test-org/test-repo"`. When daemon polls GitHub, it gets errors, never launches members, and tests skip their namesake assertions via conditional guards.
- Files: `crates/bm/tests/e2e/` (daemon-related scenarios)
- Risk: Daemon member-launch, child termination, and per-member logging are not actually verified. Diagnosed in `.planning/debug/daemon-namesake-claims.md` and `.planning/debug/e2e-missing-runtime-dependency-gate.md`.
- Priority: High -- core daemon functionality is untested end-to-end.

**Conditional state assertions in start-to-stop E2E tests:**
- What's not tested: Three test sites wrap `state.json` member-empty assertions in `if state_path.exists()` guards instead of unconditional `assert!`. Since production code always writes state.json after member removal, file absence would be a regression these tests cannot catch.
- Files: `crates/bm/tests/e2e/` (start-to-stop scenarios)
- Risk: A regression that deletes state.json instead of clearing it would go undetected. Diagnosed in `.planning/debug/conditional-state-asserts.md`.
- Priority: Medium -- silently weakened assertions.

**No tests for concurrent state.json access:**
- What's not tested: Simultaneous `bm start` + `bm stop` or `bm status` + `bm stop` operating on the same state file.
- Files: `crates/bm/src/state.rs`
- Risk: Race conditions could corrupt state or cause one command's changes to be lost.
- Priority: Low -- unlikely in normal usage (single operator), but possible.

**No tests for `force_stop()` behavior:**
- What's not tested: The `force_stop()` function in `crates/bm/src/commands/stop.rs` (lines 157-163) sends SIGTERM and sleeps 500ms but never verifies the process died. No test exercises this path.
- Files: `crates/bm/src/commands/stop.rs`
- Risk: Force stop could silently fail to kill a process. No feedback to user.
- Priority: Low -- rare code path, SIGTERM almost always works.

---

*Concerns audit: 2026-03-10*
