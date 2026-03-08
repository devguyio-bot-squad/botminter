//! E2E tests for daemon signal handling and lifecycle.
//!
//! These tests use stub `ralph` binaries to test daemon process management
//! without needing Claude API access. Tests that verify member lifecycle
//! (daemon_stop_terminates_running_members, daemon_log_created_on_poll)
//! use real GitHub repos and create real issues to trigger the daemon's
//! event detection and member launch.
//!
//! The `daemon_basic` suite combines 5 tests that share a single TempRepo
//! to reduce API rate limit consumption.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use bm::config::{BotminterConfig, Credentials, TeamEntry};
use bm::profile;
use libtest_mimic::Trial;

use super::helpers::{
    assert_cmd_success, bm_cmd, force_kill, is_alive, run_test, wait_for_exit, DaemonGuard,
    E2eConfig, GithubSuite,
};

// ── Stub Ralph ───────────────────────────────────────────────────────

/// Standard stub: traps SIGTERM, writes PID, sleeps forever.
const STUB_RALPH: &str = r#"#!/bin/bash
# Stub ralph binary for daemon E2E tests.
case "$1" in
  run)
    echo $$ > "$PWD/.ralph-stub-pid"
    echo "stub ralph started (PID $$)" >&2
    trap "rm -f \"$PWD/.ralph-stub-pid\"; exit 0" SIGTERM SIGINT
    while true; do sleep 1; done
    ;;
  *)
    exit 0
    ;;
esac
"#;

/// SIGTERM-ignoring stub: only dies to SIGKILL.
const STUB_RALPH_IGNORE_SIGTERM: &str = r#"#!/bin/bash
# Stub ralph that ignores SIGTERM (for SIGKILL escalation tests).
case "$1" in
  run)
    echo $$ > "$PWD/.ralph-stub-pid"
    echo "stub ralph (sigterm-immune) started (PID $$)" >&2
    trap "" SIGTERM  # ignore SIGTERM
    while true; do sleep 1; done
    ;;
  *)
    exit 0
    ;;
esac
"#;

/// Creates a stub `ralph` binary in a temp directory. Returns the directory path.
fn create_stub_ralph(tmp: &Path, script: &str) -> PathBuf {
    let stub_dir = tmp.join("stub-bin");
    fs::create_dir_all(&stub_dir).unwrap();

    let stub_path = stub_dir.join("ralph");
    fs::write(&stub_path, script).unwrap();
    fs::set_permissions(&stub_path, fs::Permissions::from_mode(0o755)).unwrap();

    stub_dir
}

/// Returns a PATH string with the stub directory prepended.
fn path_with_stub(stub_dir: &Path) -> String {
    format!(
        "{}:{}",
        stub_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

// ── Workspace Setup ──────────────────────────────────────────────────

/// Sets up a minimal team with one member workspace for daemon tests.
///
/// Returns `(team_name, member_dir_name)`.
fn setup_daemon_workspace(
    tmp: &Path,
    team_name: &str,
    github_repo: &str,
    gh_token: &str,
) -> (String, String) {
    let (profile_name, roles) = find_profile_with_role();
    let role = &roles[0];
    let member_name = "alice";
    let member_dir_name = format!("{}-{}", role, member_name);

    // Set up git auth in temp HOME (credential helper + user identity)
    super::helpers::setup_git_auth(tmp);

    let workzone = tmp.join("workspaces");
    let team_dir = workzone.join(team_name);
    let team_repo = team_dir.join("team");

    // Create team repo with botminter.yml
    fs::create_dir_all(&team_repo).unwrap();
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp);
    let manifest = profile::read_manifest_from(&profile_name, &profiles_base).unwrap();
    let manifest_yml = serde_yml::to_string(&manifest).unwrap();
    fs::write(team_repo.join("botminter.yml"), &manifest_yml).unwrap();

    // Member discovery: team_repo/members/<member>/
    let members_dir = team_repo.join("members");
    let member_config_dir = members_dir.join(&member_dir_name);
    fs::create_dir_all(&member_config_dir).unwrap();

    // Workspace: workzone/<team>/<member>/ (no-project mode)
    let workspace = team_dir.join(&member_dir_name);
    fs::create_dir_all(workspace.join(".botminter")).unwrap();
    fs::write(workspace.join("PROMPT.md"), "# E2E Daemon Test\n").unwrap();

    // Write config
    let config = BotminterConfig {
        workzone,
        default_team: Some(team_name.to_string()),
        teams: vec![TeamEntry {
            name: team_name.to_string(),
            path: team_dir,
            profile: profile_name,
            github_repo: github_repo.to_string(),
            credentials: Credentials {
                gh_token: Some(gh_token.to_string()),
                telegram_bot_token: None,
                webhook_secret: None,
            },
            coding_agent: None,
            project_number: None,
        }],
    };
    let config_path = tmp.join(".botminter").join("config.yml");
    bm::config::save_to(&config_path, &config).unwrap();

    (team_name.to_string(), member_dir_name)
}

fn find_profile_with_role() -> (String, Vec<String>) {
    for name in bm::profile::list_embedded_profiles() {
        let roles = bm::profile::list_embedded_roles(&name);
        if !roles.is_empty() {
            return (name, roles);
        }
    }
    panic!("No embedded profile has any roles");
}

/// Creates a bm command with HOME, PATH, and GH_TOKEN configured.
fn daemon_cmd(tmp: &Path, stub_dir: &Path, args: &[&str], gh_token: &str) -> Command {
    let mut cmd = bm_cmd();
    cmd.args(args)
        .env("HOME", tmp)
        .env("PATH", path_with_stub(stub_dir))
        .env("GH_TOKEN", gh_token);
    cmd
}

// ── Test registration ────────────────────────────────────────────────

pub fn tests(config: &E2eConfig) -> Vec<Trial> {
    let cfg = config.clone();

    let mut trials = Vec::new();

    // Suite: daemon_basic — 5 tests sharing 1 TempRepo
    trials.push(daemon_basic_suite(&cfg));

    // Isolated tests that need their own TempRepo (create issues, verify member launch)
    let isolated: Vec<Trial> = vec![
        Trial::test("daemon_stop_terminates_running_members", {
            let cfg = cfg.clone();
            move || run_test(|| daemon_stop_terminates_running_members_impl(&cfg))
        }),
        Trial::test("daemon_stop_timeout_escalates_to_sigkill", {
            let cfg = cfg.clone();
            move || run_test(|| daemon_stop_timeout_escalates_to_sigkill_impl(&cfg))
        }),
        Trial::test("daemon_log_created_on_poll", {
            let cfg = cfg.clone();
            move || run_test(|| daemon_log_created_on_poll_impl(&cfg))
        }),
    ];
    trials.extend(isolated);

    trials
}

// ── Suite: daemon_basic ───────────────────────────────────────────────

fn daemon_basic_suite(config: &E2eConfig) -> Trial {
    let gh_token = config.gh_token.clone();
    GithubSuite::new("daemon_basic", "bm-e2e-daemon")
        .case("start_stop_poll", {
            let gh_token = gh_token.clone();
            move |ctx| {
                let case_tmp = tempfile::tempdir().unwrap();
                let stub_dir = create_stub_ralph(case_tmp.path(), STUB_RALPH);

                let (team_name, _member) = setup_daemon_workspace(
                    case_tmp.path(),
                    "e2e-poll",
                    &ctx.repo.full_name,
                    &gh_token,
                );
                let _guard = DaemonGuard::new(
                    case_tmp.path(),
                    &team_name,
                    Some(&stub_dir),
                    Some(&gh_token),
                );

                // Start
                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "start", "--mode", "poll", "-t", &team_name],
                    &gh_token,
                ));
                assert!(out.contains("Daemon started"), "Expected started: {}", out);

                // Status shows running
                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "status", "-t", &team_name],
                    &gh_token,
                ));
                assert!(out.contains("running"), "Expected running: {}", out);
                assert!(out.contains("poll"), "Expected poll mode: {}", out);

                // Stop
                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "stop", "-t", &team_name],
                    &gh_token,
                ));
                assert!(out.contains("Daemon stopped"), "Expected stopped: {}", out);

                // Status shows not running
                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "status", "-t", &team_name],
                    &gh_token,
                ));
                assert!(
                    out.contains("not running"),
                    "Expected not running: {}",
                    out
                );

                // PID and config files cleaned up
                let pid_file = case_tmp
                    .path()
                    .join(format!(".botminter/daemon-{}.pid", team_name));
                assert!(!pid_file.exists(), "PID file should be cleaned up");
                let cfg_file = case_tmp
                    .path()
                    .join(format!(".botminter/daemon-{}.json", team_name));
                assert!(!cfg_file.exists(), "Config file should be cleaned up");
            }
        })
        .case("start_stop_webhook", {
            let gh_token = gh_token.clone();
            move |ctx| {
                let case_tmp = tempfile::tempdir().unwrap();
                let stub_dir = create_stub_ralph(case_tmp.path(), STUB_RALPH);

                let (team_name, _member) = setup_daemon_workspace(
                    case_tmp.path(),
                    "e2e-wh",
                    &ctx.repo.full_name,
                    &gh_token,
                );
                let _guard = DaemonGuard::new(
                    case_tmp.path(),
                    &team_name,
                    Some(&stub_dir),
                    Some(&gh_token),
                );

                let port = "19500";

                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &[
                        "daemon", "start", "--mode", "webhook", "--port", port, "-t",
                        &team_name,
                    ],
                    &gh_token,
                ));
                assert!(out.contains("Daemon started"), "Expected started: {}", out);

                std::thread::sleep(Duration::from_millis(500));

                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "status", "-t", &team_name],
                    &gh_token,
                ));
                assert!(out.contains("running"), "Expected running: {}", out);
                assert!(out.contains("webhook"), "Expected webhook mode: {}", out);

                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "stop", "-t", &team_name],
                    &gh_token,
                ));
                assert!(out.contains("Daemon stopped"), "Expected stopped: {}", out);
            }
        })
        .case("stale_pid", {
            let gh_token = gh_token.clone();
            move |ctx| {
                let case_tmp = tempfile::tempdir().unwrap();
                let stub_dir = create_stub_ralph(case_tmp.path(), STUB_RALPH);

                let (team_name, _member) = setup_daemon_workspace(
                    case_tmp.path(),
                    "e2e-stale",
                    &ctx.repo.full_name,
                    &gh_token,
                );
                let _guard = DaemonGuard::new(
                    case_tmp.path(),
                    &team_name,
                    Some(&stub_dir),
                    Some(&gh_token),
                );

                // Write a stale PID file with a PID that doesn't exist
                let pid_dir = case_tmp.path().join(".botminter");
                fs::create_dir_all(&pid_dir).unwrap();
                let pid_file = pid_dir.join(format!("daemon-{}.pid", team_name));
                fs::write(&pid_file, "99999").unwrap();

                // Start should succeed (cleans stale PID)
                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "start", "--mode", "poll", "-t", &team_name],
                    &gh_token,
                ));
                assert!(
                    out.contains("Daemon started"),
                    "Should start despite stale PID: {}",
                    out
                );
            }
        })
        .case("already_running", {
            let gh_token = gh_token.clone();
            move |ctx| {
                let case_tmp = tempfile::tempdir().unwrap();
                let stub_dir = create_stub_ralph(case_tmp.path(), STUB_RALPH);

                let (team_name, _member) = setup_daemon_workspace(
                    case_tmp.path(),
                    "e2e-dup",
                    &ctx.repo.full_name,
                    &gh_token,
                );
                let _guard = DaemonGuard::new(
                    case_tmp.path(),
                    &team_name,
                    Some(&stub_dir),
                    Some(&gh_token),
                );

                // First start
                assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "start", "--mode", "poll", "-t", &team_name],
                    &gh_token,
                ));

                // Second start should fail
                let output = daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "start", "--mode", "poll", "-t", &team_name],
                    &gh_token,
                )
                .output()
                .expect("failed to run second start");

                assert!(!output.status.success(), "Second start should fail");
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    stderr.contains("already running"),
                    "Should say already running: {}",
                    stderr
                );
            }
        })
        .case("crashed_status", {
            let gh_token = gh_token.clone();
            move |ctx| {
                let case_tmp = tempfile::tempdir().unwrap();
                let stub_dir = create_stub_ralph(case_tmp.path(), STUB_RALPH);

                let (team_name, _member) = setup_daemon_workspace(
                    case_tmp.path(),
                    "e2e-crash",
                    &ctx.repo.full_name,
                    &gh_token,
                );
                // No guard needed -- we're manually killing the daemon

                // Start
                assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "start", "--mode", "poll", "-t", &team_name],
                    &gh_token,
                ));

                // Read PID
                let pid_file = case_tmp
                    .path()
                    .join(format!(".botminter/daemon-{}.pid", team_name));
                let daemon_pid: u32 = fs::read_to_string(&pid_file)
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();

                // Force-kill the daemon (simulate crash)
                force_kill(daemon_pid);
                wait_for_exit(daemon_pid, Duration::from_secs(5));

                // Status should detect crash
                let out = assert_cmd_success(&mut daemon_cmd(
                    case_tmp.path(),
                    &stub_dir,
                    &["daemon", "status", "-t", &team_name],
                    &gh_token,
                ));
                assert!(
                    out.contains("not running") || out.contains("stale"),
                    "Status should show not running / stale PID: {}",
                    out
                );

                // PID file should be cleaned up by status
                assert!(
                    !pid_file.exists(),
                    "Stale PID file should be cleaned up by status"
                );
            }
        })
        .build(config)
}

// ── Isolated test implementations ─────────────────────────────────────

/// Start daemon -> daemon launches stub ralph via real GitHub event -> `bm daemon stop` -> both die.
fn daemon_stop_terminates_running_members_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path(), STUB_RALPH);

    // Create a REAL GitHub repo for the daemon to poll
    let repo = super::github::TempRepo::new_in_org("bm-e2e-daemon-term", &config.gh_org)
        .expect("Failed to create temp GitHub repo for daemon test");

    let (team_name, member) = setup_daemon_workspace(
        tmp.path(),
        "e2e-term",
        &repo.full_name,
        &config.gh_token,
    );
    let _guard = DaemonGuard::new(tmp.path(), &team_name, Some(&stub_dir), Some(&config.gh_token));

    // Start daemon in poll mode with short interval
    assert_cmd_success(&mut daemon_cmd(
        tmp.path(),
        &stub_dir,
        &[
            "daemon", "start", "--mode", "poll", "--interval", "2", "-t", &team_name,
        ],
        &config.gh_token,
    ));

    // Read daemon PID
    let pid_file = tmp
        .path()
        .join(format!(".botminter/daemon-{}.pid", team_name));
    let daemon_pid: u32 = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(is_alive(daemon_pid), "Daemon should be alive");

    // Create a GitHub issue to trigger an IssuesEvent -- this is what a real user does
    let create_output = Command::new("gh")
        .args([
            "issue",
            "create",
            "-R",
            &repo.full_name,
            "--title",
            "Trigger daemon member launch",
            "--body",
            "E2E test trigger",
        ])
        .env("GH_TOKEN", &config.gh_token)
        .output()
        .expect("Failed to create GitHub issue");
    assert!(
        create_output.status.success(),
        "gh issue create failed: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    // Wait for daemon to poll, detect the event, and launch stub ralph
    let workspace = tmp
        .path()
        .join("workspaces")
        .join(&team_name)
        .join(&member);
    let stub_pid_file = workspace.join(".ralph-stub-pid");

    // Daemon polls every 2s. Allow time for: poll cycle + event detection + member launch
    let poll_deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !stub_pid_file.exists() && std::time::Instant::now() < poll_deadline {
        std::thread::sleep(Duration::from_millis(500));
    }

    // UNCONDITIONAL: stub PID file MUST exist -- the daemon MUST have launched the member
    assert!(
        stub_pid_file.exists(),
        "Daemon did not launch member within 30s -- stub PID file never appeared at {}. \
         Check daemon log at ~/.botminter/logs/daemon-{}.log",
        stub_pid_file.display(),
        team_name
    );

    let stub_pid: u32 = fs::read_to_string(&stub_pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        is_alive(stub_pid),
        "Stub ralph PID {} should be alive before daemon stop",
        stub_pid
    );

    // Stop daemon
    assert_cmd_success(&mut daemon_cmd(
        tmp.path(),
        &stub_dir,
        &["daemon", "stop", "-t", &team_name],
        &config.gh_token,
    ));

    // Daemon should be dead
    wait_for_exit(daemon_pid, Duration::from_secs(10));
    assert!(
        !is_alive(daemon_pid),
        "Daemon PID {} should be dead after stop",
        daemon_pid
    );

    // Child member MUST also be dead -- this is the namesake claim
    wait_for_exit(stub_pid, Duration::from_secs(10));
    assert!(
        !is_alive(stub_pid),
        "Stub ralph PID {} should be dead after daemon stop",
        stub_pid
    );
    // TempRepo drops here -- GitHub repo cleaned up
}

/// Stub ralph ignores SIGTERM -> daemon escalates to SIGKILL -> processes die.
fn daemon_stop_timeout_escalates_to_sigkill_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path(), STUB_RALPH_IGNORE_SIGTERM);

    let repo = super::github::TempRepo::new_in_org("bm-e2e-dsigkill", &config.gh_org)
        .expect("Failed to create temp GitHub repo");

    let (team_name, _member) =
        setup_daemon_workspace(tmp.path(), "e2e-sigkill", &repo.full_name, &config.gh_token);
    let _guard = DaemonGuard::new(tmp.path(), &team_name, Some(&stub_dir), Some(&config.gh_token));

    // Start daemon
    assert_cmd_success(&mut daemon_cmd(
        tmp.path(),
        &stub_dir,
        &[
            "daemon", "start", "--mode", "poll", "--interval", "2", "-t", &team_name,
        ],
        &config.gh_token,
    ));

    let pid_file = tmp
        .path()
        .join(format!(".botminter/daemon-{}.pid", team_name));
    let daemon_pid: u32 = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    // Stop daemon -- will send SIGTERM, wait, then SIGKILL
    assert_cmd_success(&mut daemon_cmd(
        tmp.path(),
        &stub_dir,
        &["daemon", "stop", "-t", &team_name],
        &config.gh_token,
    ));

    // Daemon should be dead (SIGKILL escalation should have worked)
    wait_for_exit(daemon_pid, Duration::from_secs(5));
    assert!(!is_alive(daemon_pid), "Daemon should be dead after SIGKILL");
}

/// Start daemon -> verify daemon log is created and per-member log exists after member launch.
fn daemon_log_created_on_poll_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path(), STUB_RALPH);

    let repo = super::github::TempRepo::new_in_org("bm-e2e-daemon-log", &config.gh_org)
        .expect("Failed to create temp GitHub repo for daemon log test");

    let (team_name, member) = setup_daemon_workspace(
        tmp.path(),
        "e2e-memlog",
        &repo.full_name,
        &config.gh_token,
    );
    let _guard = DaemonGuard::new(tmp.path(), &team_name, Some(&stub_dir), Some(&config.gh_token));

    // Start daemon
    assert_cmd_success(&mut daemon_cmd(
        tmp.path(),
        &stub_dir,
        &[
            "daemon", "start", "--mode", "poll", "--interval", "2", "-t", &team_name,
        ],
        &config.gh_token,
    ));

    // Create issue to trigger member launch
    let create_output = Command::new("gh")
        .args([
            "issue",
            "create",
            "-R",
            &repo.full_name,
            "--title",
            "Trigger daemon log test",
            "--body",
            "E2E test trigger",
        ])
        .env("GH_TOKEN", &config.gh_token)
        .output()
        .expect("Failed to create GitHub issue");
    assert!(
        create_output.status.success(),
        "gh issue create failed: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    // Wait for member launch (stub PID file appears)
    let workspace = tmp
        .path()
        .join("workspaces")
        .join(&team_name)
        .join(&member);
    let stub_pid_file = workspace.join(".ralph-stub-pid");

    let poll_deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !stub_pid_file.exists() && std::time::Instant::now() < poll_deadline {
        std::thread::sleep(Duration::from_millis(500));
    }

    assert!(
        stub_pid_file.exists(),
        "Daemon did not launch member -- stub PID file never appeared"
    );

    // Daemon log MUST exist
    let daemon_log = tmp
        .path()
        .join(format!(".botminter/logs/daemon-{}.log", team_name));
    assert!(daemon_log.exists(), "Daemon log should exist");

    let log_content = fs::read_to_string(&daemon_log).unwrap();
    assert!(!log_content.is_empty(), "Daemon log should not be empty");

    // Per-member log MUST exist -- member was launched, this is UNCONDITIONAL
    let member_log = tmp
        .path()
        .join(format!(".botminter/logs/member-{}-{}.log", team_name, member));
    assert!(
        member_log.exists(),
        "Per-member log MUST exist after member launch at {}",
        member_log.display()
    );

    let member_log_content = fs::read_to_string(&member_log).unwrap();
    assert!(
        !member_log_content.is_empty(),
        "Per-member log should not be empty"
    );
}
