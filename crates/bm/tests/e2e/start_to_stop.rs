//! E2E tests for the start -> status -> stop lifecycle.
//!
//! These tests use a stub `ralph` binary (a bash script that sleeps) to test
//! `bm`'s process management without needing Claude API access.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use bm::config::{BotminterConfig, Credentials, TeamEntry};
use bm::profile;
use libtest_mimic::Trial;

use super::helpers::{
    assert_cmd_fails, assert_cmd_success, bm_cmd, force_kill, is_alive, run_test, wait_for_exit,
    E2eConfig,
};

// ── Stub Ralph ───────────────────────────────────────────────────────

const STUB_RALPH: &str = r#"#!/bin/bash
# Stub ralph binary for E2E testing.
case "$1" in
  run)
    echo $$ > "$PWD/.ralph-stub-pid"
    if [ -n "$RALPH_TELEGRAM_API_URL" ] && [ -n "$RALPH_TELEGRAM_BOT_TOKEN" ]; then
      curl -s "${RALPH_TELEGRAM_API_URL}/bot${RALPH_TELEGRAM_BOT_TOKEN}/getUpdates" \
        > "$PWD/.ralph-stub-tg-response" 2>&1
    fi
    env | grep -E '^(RALPH_|GH_TOKEN)' | sort > "$PWD/.ralph-stub-env"
    trap "rm -f \"$PWD/.ralph-stub-pid\"; exit 0" SIGTERM SIGINT
    while true; do sleep 1; done
    ;;
  loops)
    if [ "$2" = "stop" ]; then
      pid_file="$PWD/.ralph-stub-pid"
      if [ -f "$pid_file" ]; then
        kill "$(cat "$pid_file")" 2>/dev/null
        rm -f "$pid_file"
      fi
      exit 0
    fi
    ;;
  *)
    exit 0
    ;;
esac
"#;

fn create_stub_ralph(tmp: &Path) -> PathBuf {
    let stub_dir = tmp.join("stub-bin");
    fs::create_dir_all(&stub_dir).unwrap();
    let stub_path = stub_dir.join("ralph");
    fs::write(&stub_path, STUB_RALPH).unwrap();
    fs::set_permissions(&stub_path, fs::Permissions::from_mode(0o755)).unwrap();
    stub_dir
}

fn path_with_stub(stub_dir: &Path) -> String {
    format!(
        "{}:{}",
        stub_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

// ── Workspace Setup ──────────────────────────────────────────────────

fn setup_workspace_for_start(tmp: &Path, config: &E2eConfig) -> (String, String, PathBuf) {
    let team_name = "e2e-start";

    let (profile_name, roles) = find_profile_with_role();
    let role = &roles[0];
    let member_name = "alice";
    let member_dir_name = format!("{}-{}", role, member_name);

    let workzone = tmp.join("workspaces");
    let team_dir = workzone.join(team_name);
    let team_repo = team_dir.join("team");

    fs::create_dir_all(&team_repo).unwrap();
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp);
    let manifest = profile::read_manifest_from(&profile_name, &profiles_base).unwrap();
    let manifest_yml = serde_yml::to_string(&manifest).unwrap();
    fs::write(team_repo.join("botminter.yml"), &manifest_yml).unwrap();

    let members_dir = team_repo.join("members");
    let member_config_dir = members_dir.join(&member_dir_name);
    fs::create_dir_all(&member_config_dir).unwrap();

    let workspace = team_dir.join(&member_dir_name);
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        workspace.join(".botminter.workspace"),
        "member: workspace\n",
    )
    .unwrap();
    fs::write(workspace.join("PROMPT.md"), "# E2E Test Prompt\n").unwrap();

    let bm_config = BotminterConfig {
        workzone,
        default_team: Some(team_name.to_string()),
        teams: vec![TeamEntry {
            name: team_name.to_string(),
            path: team_dir,
            profile: profile_name,
            github_repo: format!("{}/e2e-placeholder", config.gh_org),
            credentials: Credentials {
                gh_token: Some(config.gh_token.clone()),
                telegram_bot_token: None,
                webhook_secret: None,
            },
            coding_agent: None,
            project_number: None,
        }],
    };
    let config_path = tmp.join(".botminter").join("config.yml");
    bm::config::save_to(&config_path, &bm_config).unwrap();

    (team_name.to_string(), member_dir_name, workspace)
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

fn start_cmd(tmp: &Path, stub_dir: &Path, args: &[&str]) -> Command {
    let mut cmd = bm_cmd();
    cmd.args(args)
        .env("HOME", tmp)
        .env("PATH", path_with_stub(stub_dir));
    cmd
}

fn read_pid_from_state(home: &Path) -> Option<u32> {
    let state_path = home.join(".botminter").join("state.json");
    if !state_path.exists() {
        return None;
    }
    let contents = fs::read_to_string(&state_path).ok()?;
    let state: bm::state::RuntimeState = serde_json::from_str(&contents).ok()?;
    state.members.values().next().map(|rt| rt.pid)
}

struct ProcessGuard {
    pid: Option<u32>,
    home: PathBuf,
    stub_dir: PathBuf,
    team_name: String,
}

impl ProcessGuard {
    fn new(home: &Path, stub_dir: &Path, team_name: &str) -> Self {
        ProcessGuard {
            pid: None,
            home: home.to_path_buf(),
            stub_dir: stub_dir.to_path_buf(),
            team_name: team_name.to_string(),
        }
    }

    fn set_pid(&mut self, pid: u32) {
        self.pid = Some(pid);
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = bm_cmd()
            .args(["stop", "--force", "-t", &self.team_name])
            .env("HOME", &self.home)
            .env("PATH", path_with_stub(&self.stub_dir))
            .output();
        if let Some(pid) = self.pid {
            if is_alive(pid) {
                force_kill(pid);
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

// ── Test registration ────────────────────────────────────────────────

pub fn tests(config: &E2eConfig) -> Vec<Trial> {
    let cfg = config.clone();
    vec![
        Trial::test("e2e_start_status_stop_lifecycle", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_start_status_stop_lifecycle_impl(&cfg))
        }),
        Trial::test("e2e_start_already_running_skips", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_start_already_running_skips_impl(&cfg))
        }),
        Trial::test("e2e_stop_force_kills", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_stop_force_kills_impl(&cfg))
        }),
        Trial::test("e2e_status_detects_crashed_member", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_status_detects_crashed_member_impl(&cfg))
        }),
        Trial::test("e2e_tg_mock_receives_bot_messages", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_tg_mock_receives_bot_messages_impl(&cfg))
        }),
        Trial::test("e2e_start_without_ralph_errors", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_start_without_ralph_errors_impl(&cfg))
        }),
    ]
}

// ── Test implementations ─────────────────────────────────────────────

fn e2e_start_status_stop_lifecycle_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path());
    let (team_name, member_dir_name, _workspace) =
        setup_workspace_for_start(tmp.path(), config);

    let mut guard = ProcessGuard::new(tmp.path(), &stub_dir, &team_name);

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["start", "-t", &team_name]);
    let out = assert_cmd_success(&mut cmd);
    assert!(out.contains("Started 1 member"), "Expected 'Started 1 member': {}", out);

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["status", "-t", &team_name]);
    let out = assert_cmd_success(&mut cmd);
    assert!(out.contains("running"), "Expected 'running': {}", out);
    assert!(out.contains(&member_dir_name));

    if let Some(pid) = read_pid_from_state(tmp.path()) {
        guard.set_pid(pid);
    }

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["stop", "-t", &team_name]);
    let out = assert_cmd_success(&mut cmd);
    assert!(out.contains("Stopped 1 member"), "Expected 'Stopped 1 member': {}", out);

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["status", "-t", &team_name]);
    let out = assert_cmd_success(&mut cmd);
    assert!(out.contains("stopped"));

    let state_path = tmp.path().join(".botminter").join("state.json");
    assert!(state_path.exists());
    let state: bm::state::RuntimeState =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert!(state.members.is_empty());
}

fn e2e_start_already_running_skips_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path());
    let (team_name, _member_dir, _workspace) =
        setup_workspace_for_start(tmp.path(), config);

    let mut guard = ProcessGuard::new(tmp.path(), &stub_dir, &team_name);

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["start", "-t", &team_name]);
    let out1 = assert_cmd_success(&mut cmd);
    assert!(out1.contains("Started 1 member"));

    let pid1 = read_pid_from_state(tmp.path());
    if let Some(pid) = pid1 {
        guard.set_pid(pid);
    }

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["start", "-t", &team_name]);
    let out2 = assert_cmd_success(&mut cmd);
    assert!(
        out2.contains("already running"),
        "Second start should say 'already running', got: {}",
        out2
    );

    let pid2 = read_pid_from_state(tmp.path());
    assert_eq!(pid1, pid2);
}

fn e2e_stop_force_kills_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path());
    let (team_name, _member_dir, _workspace) =
        setup_workspace_for_start(tmp.path(), config);

    let mut guard = ProcessGuard::new(tmp.path(), &stub_dir, &team_name);

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["start", "-t", &team_name]);
    assert_cmd_success(&mut cmd);

    let pid = read_pid_from_state(tmp.path()).expect("should have a PID");
    guard.set_pid(pid);
    assert!(is_alive(pid));

    let mut cmd = start_cmd(
        tmp.path(),
        &stub_dir,
        &["stop", "--force", "-t", &team_name],
    );
    assert_cmd_success(&mut cmd);

    wait_for_exit(pid, Duration::from_secs(5));
    assert!(!is_alive(pid));

    let state_path = tmp.path().join(".botminter").join("state.json");
    assert!(state_path.exists());
    let state: bm::state::RuntimeState =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert!(state.members.is_empty());
}

fn e2e_status_detects_crashed_member_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path());
    let (team_name, member_dir_name, _workspace) =
        setup_workspace_for_start(tmp.path(), config);

    let _guard = ProcessGuard::new(tmp.path(), &stub_dir, &team_name);

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["start", "-t", &team_name]);
    assert_cmd_success(&mut cmd);

    let pid = read_pid_from_state(tmp.path()).expect("should have a PID");
    force_kill(pid);
    wait_for_exit(pid, Duration::from_secs(5));

    let mut cmd = start_cmd(tmp.path(), &stub_dir, &["status", "-t", &team_name]);
    let out = assert_cmd_success(&mut cmd);
    assert!(out.contains("crashed"), "Expected 'crashed': {}", out);
    assert!(out.contains(&member_dir_name));

    let state_path = tmp.path().join(".botminter").join("state.json");
    assert!(state_path.exists());
    let state: bm::state::RuntimeState =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert!(state.members.is_empty());
}

fn e2e_tg_mock_receives_bot_messages_impl(config: &E2eConfig) {
    if !super::telegram::podman_available() {
        eprintln!("SKIP: podman not available");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let stub_dir = create_stub_ralph(tmp.path());
    let (team_name, _member_dir, workspace) =
        setup_workspace_for_start(tmp.path(), config);

    let mut guard = ProcessGuard::new(tmp.path(), &stub_dir, &team_name);

    let mock = super::telegram::TgMock::start();
    let bot_token = "123456789:ABCDEFGhijklmnopqrstuvwxyz";

    let mut cmd = bm_cmd();
    cmd.args(["start", "-t", &team_name])
        .env("HOME", tmp.path())
        .env("PATH", path_with_stub(&stub_dir))
        .env("RALPH_TELEGRAM_API_URL", mock.api_url())
        .env("RALPH_TELEGRAM_BOT_TOKEN", bot_token);
    let out = assert_cmd_success(&mut cmd);
    assert!(out.contains("Started 1 member"));

    if let Some(pid) = read_pid_from_state(tmp.path()) {
        guard.set_pid(pid);
    }

    std::thread::sleep(Duration::from_secs(3));

    let env_file = workspace.join(".ralph-stub-env");
    assert!(env_file.exists(), "Stub should have written .ralph-stub-env");
    let env_content = fs::read_to_string(&env_file).unwrap();
    assert!(env_content.contains("RALPH_TELEGRAM_API_URL="));
    assert!(env_content.contains("GH_TOKEN="));

    let tg_response_file = workspace.join(".ralph-stub-tg-response");
    assert!(tg_response_file.exists());
    let tg_response = fs::read_to_string(&tg_response_file).unwrap();
    assert!(tg_response.contains("ok"));
}

fn e2e_start_without_ralph_errors_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let (team_name, _member_dir, _workspace) =
        setup_workspace_for_start(tmp.path(), config);

    let restricted_path = "/usr/bin:/bin:/usr/sbin:/sbin";

    let mut cmd = bm_cmd();
    cmd.args(["start", "-t", &team_name])
        .env("HOME", tmp.path())
        .env("PATH", restricted_path);
    let stderr = assert_cmd_fails(&mut cmd);
    assert!(
        stderr.contains("ralph") && stderr.contains("not found"),
        "Error should mention 'ralph' and 'not found', got: {}",
        stderr
    );
}
