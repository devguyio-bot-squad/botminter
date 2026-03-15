//! Unified Operator Journey
//!
//! One suite, two passes: fresh start → daemon → reset HOME → same journey again.
//! The second pass exercises idempotency — `bm init` hits an existing repo.
//! Cases that expect failure on second pass use `case_expect_error`.

use std::fs;
use std::process::Command;
use std::time::Duration;

use bm::profile;
use libtest_mimic::Trial;

use super::super::helpers::{
    assert_cmd_fails, assert_cmd_success, bm_cmd, bootstrap_profiles_to_tmp,
    cleanup_project_boards, force_kill, install_stub_ralph, is_alive, path_with_stub,
    read_pid_from_state, repo_from_config, reset_keyring, setup_git_auth,
    DaemonGuard, E2eConfig, GithubSuite, ProcessGuard, SuiteCtx,
};
use super::super::telegram;

// ── Constants ─────────────────────────────────────────────────────────

const TEAM_NAME: &str = "e2e-fresh";
const PROFILE: &str = "scrum-compact";
const ROLE: &str = "superman";
const MEMBER_NAME: &str = "alice";
const MEMBER_DIR: &str = "superman-alice";
const BOT_TOKEN: &str = "123456789:ABCDEFGhijklmnopqrstuvwxyz-e2e";

// ── Reusable case functions ───────────────────────────────────────────
//
// Each function takes captured values and returns a closure suitable for .case().
// This allows the same logic to be registered in both passes with different names.

fn init_with_bridge_fn(gh_org: String, gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let workzone = ctx.home.join("workspaces");
        let repo_name = ctx.repo_full_name.split('/').next_back().unwrap();

        let mut cmd = bm_cmd();
        cmd.args([
            "init", "--non-interactive",
            "--profile", PROFILE,
            "--team-name", TEAM_NAME,
            "--org", &gh_org,
            "--repo", repo_name,
            "--bridge", "telegram",
            "--workzone", &workzone.to_string_lossy(),
        ])
        .env("HOME", &ctx.home)
        .env("GH_TOKEN", &gh_token)
        .env("GIT_AUTHOR_NAME", "BM E2E")
        .env("GIT_AUTHOR_EMAIL", "e2e@botminter.test")
        .env("GIT_COMMITTER_NAME", "BM E2E")
        .env("GIT_COMMITTER_EMAIL", "e2e@botminter.test");
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("init: {}", stdout.trim());

        let team_repo = workzone.join(TEAM_NAME).join("team");
        assert!(team_repo.join(".git").is_dir(), "team repo should have .git");
        assert!(team_repo.join("botminter.yml").exists(), "should have botminter.yml");
        assert!(team_repo.join("PROCESS.md").exists(), "should have PROCESS.md");

        let repo = repo_from_config(&ctx.home);
        let labels = super::super::github::list_labels(&repo);
        let profiles_base = ctx.home.join(".config/botminter/profiles");
        let manifest = profile::read_manifest_from(PROFILE, &profiles_base).unwrap();
        for expected in &manifest.labels {
            assert!(labels.contains(&expected.name), "Label '{}' missing", expected.name);
        }

        setup_git_auth(&ctx.home);
    }
}

fn hire_member_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["hire", ROLE, "--name", MEMBER_NAME, "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token)
            .env("GIT_AUTHOR_NAME", "BM E2E")
            .env("GIT_AUTHOR_EMAIL", "e2e@botminter.test")
            .env("GIT_COMMITTER_NAME", "BM E2E")
            .env("GIT_COMMITTER_EMAIL", "e2e@botminter.test");
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains(MEMBER_DIR) || stdout.contains(MEMBER_NAME));
    }
}

fn projects_add_fn(gh_org: String, gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        // Read existing project URL from team repo if available (second pass)
        let team_repo = ctx.home.join("workspaces").join(TEAM_NAME).join("team");
        let manifest_path = team_repo.join("botminter.yml");
        let existing_url = if manifest_path.exists() {
            let contents = fs::read_to_string(&manifest_path).unwrap();
            let manifest: serde_yml::Value = serde_yml::from_str(&contents).unwrap();
            manifest["projects"].as_sequence()
                .and_then(|ps| ps.first())
                .and_then(|p| p["fork_url"].as_str().map(String::from))
        } else {
            None
        };

        let project_url = if let Some(url) = existing_url {
            url
        } else {
            // First pass — create a GitHub repo for the project
            let fork = ctx.home.join("test-project");
            fs::create_dir_all(&fork).unwrap();
            Command::new("git").args(["init", "-b", "main"]).current_dir(&fork).output().unwrap();
            Command::new("git").args(["config", "user.email", "e2e@test"]).current_dir(&fork).output().unwrap();
            Command::new("git").args(["config", "user.name", "E2E"]).current_dir(&fork).output().unwrap();
            fs::write(fork.join("README.md"), "# test").unwrap();
            Command::new("git").args(["add", "-A"]).current_dir(&fork).output().unwrap();
            Command::new("git").args(["commit", "-m", "init"]).current_dir(&fork).output().unwrap();

            let full_name = format!("{}/bm-e2e-project-{}", gh_org,
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let output = Command::new("gh")
                .args(["repo", "create", &full_name, "--private", "--source", ".", "--push"])
                .current_dir(&fork)
                .env("GH_TOKEN", &gh_token)
                .output().unwrap();
            assert!(output.status.success(), "gh repo create failed: {}",
                String::from_utf8_lossy(&output.stderr));
            format!("https://github.com/{}.git", full_name)
        };

        let project_name = bm::commands::init::derive_project_name(&project_url);

        let mut cmd = bm_cmd();
        cmd.args(["projects", "add", &project_url, "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("GH_TOKEN", &gh_token);
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("projects add: {}", stdout.trim());

        let repo = repo_from_config(&ctx.home);
        let labels = super::super::github::list_labels(&repo);
        let expected_label = format!("project/{}", project_name);
        assert!(labels.contains(&expected_label), "Label '{}' missing: {:?}", expected_label, labels);
    }
}

fn teams_show_fn() -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["teams", "show", "-t", TEAM_NAME]).env("HOME", &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains("Bridge:"));
        assert!(stdout.contains(MEMBER_DIR) || stdout.contains(MEMBER_NAME));
    }
}

fn bridge_identity_add_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["bridge", "identity", "add", MEMBER_DIR, "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token)
            .env(format!("BM_BRIDGE_TOKEN_{}", MEMBER_DIR.to_uppercase().replace('-', "_")), BOT_TOKEN);
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains(MEMBER_DIR));

        // Verify token was stored by running `bm bridge identity list` (subprocess,
        // not in-process — the test process's D-Bus connection is isolated)
        let mut list_cmd = bm_cmd();
        list_cmd.args(["bridge", "identity", "list", "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token);
        let list_out = assert_cmd_success(&mut list_cmd);
        assert!(list_out.contains(MEMBER_DIR), "identity should appear in list after add");
    }
}

fn bridge_identity_list_fn() -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["bridge", "identity", "list", "-t", TEAM_NAME]).env("HOME", &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains(MEMBER_DIR));
    }
}

fn sync_bridge_and_repos_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["teams", "sync", "--bridge", "--repos", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("GH_TOKEN", &gh_token);
        let stdout = assert_cmd_success(&mut cmd);
        assert!(!stdout.contains("No bridge configured"));

        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        assert!(ws.join(".botminter.workspace").exists());
        assert!(ws.join("team").is_dir());
        for file in ["PROMPT.md", "CLAUDE.md", "ralph.yml"] {
            assert!(ws.join(file).exists(), "{} missing", file);
        }
    }
}

fn sync_idempotent_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["teams", "sync", "--bridge", "--repos", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);

        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        assert!(ws.join(".botminter.workspace").exists());
        assert!(ws.join("PROMPT.md").exists());
    }
}

fn projects_sync_fn(gh_org: String, gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["projects", "sync", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("GH_TOKEN", &gh_token);
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains("Status field synced"));

        let _projects = bm::commands::init::list_gh_projects(&gh_token, &gh_org)
            .expect("list_gh_projects should succeed");

        // Idempotency
        let mut cmd = bm_cmd();
        cmd.args(["projects", "sync", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("GH_TOKEN", &gh_token);
        let stdout2 = assert_cmd_success(&mut cmd);
        assert!(stdout2.contains("Status field synced"));
    }
}

fn start_without_ralph_errors_fn() -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["start", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        let stderr = assert_cmd_fails(&mut cmd);
        assert!(stderr.contains("ralph") && stderr.contains("not found"));
    }
}

fn start_status_healthy_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut guard = ProcessGuard::new(&ctx.home, TEAM_NAME);
        let mut cmd = bm_cmd();
        cmd.args(["start", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let tg_url_file = ctx.home.join(".tg-mock-url");
        if tg_url_file.exists() {
            let url = fs::read_to_string(&tg_url_file).unwrap();
            cmd.env("RALPH_TELEGRAM_API_URL", url.trim()).env("RALPH_TELEGRAM_BOT_TOKEN", BOT_TOKEN);
        }
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains("Started 1 member"));

        if let Some(pid) = read_pid_from_state(&ctx.home) { guard.set_pid(pid); }

        let mut cmd = bm_cmd();
        cmd.args(["status", "-t", TEAM_NAME]).env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home));
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains("running") && stdout.contains(MEMBER_DIR));

        std::mem::forget(guard);
    }
}

fn bridge_functional_fn() -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        if !ctx.home.join(".tg-mock-url").exists() {
            eprintln!("SKIP: tg-mock not available");
            return;
        }
        std::thread::sleep(Duration::from_secs(3));
        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        let env_content = fs::read_to_string(ws.join(".ralph-stub-env")).unwrap();
        assert!(env_content.contains("RALPH_TELEGRAM_API_URL="));
        assert!(env_content.contains(&format!("RALPH_TELEGRAM_BOT_TOKEN={}", BOT_TOKEN)));
        assert!(env_content.contains("GH_TOKEN="));
        let tg_response = fs::read_to_string(ws.join(".ralph-stub-tg-response")).unwrap();
        assert!(tg_response.contains("ok"));
    }
}

fn stop_clean_shutdown_fn() -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let pid_before = read_pid_from_state(&ctx.home);
        let mut cmd = bm_cmd();
        cmd.args(["stop", "-t", TEAM_NAME]).env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home));
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains("Stopped 1 member"));
        if let Some(pid) = pid_before {
            super::super::helpers::wait_for_exit(pid, Duration::from_secs(5));
            assert!(!is_alive(pid));
        }
    }
}

fn stop_force_kills_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut guard = ProcessGuard::new(&ctx.home, TEAM_NAME);
        let mut cmd = bm_cmd();
        cmd.args(["start", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);
        let pid = read_pid_from_state(&ctx.home).expect("should have PID");
        guard.set_pid(pid);
        let mut cmd = bm_cmd();
        cmd.args(["stop", "--force", "-t", TEAM_NAME]).env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home));
        assert_cmd_success(&mut cmd);
        super::super::helpers::wait_for_exit(pid, Duration::from_secs(5));
        assert!(!is_alive(pid));
    }
}

fn status_detects_crashed_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["start", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);
        let pid = read_pid_from_state(&ctx.home).expect("should have PID");
        force_kill(pid);
        super::super::helpers::wait_for_exit(pid, Duration::from_secs(5));
        let mut cmd = bm_cmd();
        cmd.args(["status", "-t", TEAM_NAME]).env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home));
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains("crashed"));
    }
}

fn members_list_fn() -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["members", "list", "-t", TEAM_NAME]).env("HOME", &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        assert!(stdout.contains(MEMBER_DIR) && stdout.contains(ROLE));
    }
}

fn teams_list_fn() -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["teams", "list"]).env("HOME", &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        let repo = repo_from_config(&ctx.home);
        assert!(stdout.contains(&repo), "teams list should show repo '{}'", repo);
    }
}

fn daemon_start_poll_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        let _ = fs::remove_file(ws.join(".ralph-stub-pid"));
        let _ = fs::remove_file(ws.join(".ralph-stub-env"));
        let _ = fs::remove_file(ws.join(".ralph-stub-tg-response"));

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "start", "--mode", "poll", "--interval", "2", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let out = assert_cmd_success(&mut cmd);
        assert!(out.contains("Daemon started"));

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "status", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let out = assert_cmd_success(&mut cmd);
        assert!(out.contains("running") && out.contains("poll"));

        assert!(!ws.join(".ralph-stub-pid").exists(), "Ralph should NOT be running before any GH event");
    }
}

fn daemon_poll_launches_member_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let create_output = Command::new("gh")
            .args(["issue", "create", "-R", &ctx.repo_full_name,
                "--title", "Trigger daemon member launch", "--body", "E2E test trigger"])
            .env("GH_TOKEN", &gh_token).output().expect("Failed to create issue");
        assert!(create_output.status.success());

        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        let stub_pid_file = ws.join(".ralph-stub-pid");
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while !stub_pid_file.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(500));
        }
        assert!(stub_pid_file.exists(), "Daemon did not launch member within 30s");
        let stub_pid: u32 = fs::read_to_string(&stub_pid_file).unwrap().trim().parse().unwrap();
        assert!(is_alive(stub_pid));

        let daemon_log = ctx.home.join(format!(".botminter/logs/daemon-{}.log", TEAM_NAME));
        assert!(daemon_log.exists());
    }
}

fn daemon_stop_poll_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let pid_file = ctx.home.join(format!(".botminter/daemon-{}.pid", TEAM_NAME));
        let daemon_pid: u32 = fs::read_to_string(&pid_file).expect("daemon PID file").trim().parse().unwrap();
        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        let stub_pid: Option<u32> = fs::read_to_string(ws.join(".ralph-stub-pid")).ok().and_then(|s| s.trim().parse().ok());

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "stop", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let out = assert_cmd_success(&mut cmd);
        assert!(out.contains("Daemon stopped"));

        super::super::helpers::wait_for_exit(daemon_pid, Duration::from_secs(10));
        assert!(!is_alive(daemon_pid));
        if let Some(pid) = stub_pid {
            super::super::helpers::wait_for_exit(pid, Duration::from_secs(10));
            assert!(!is_alive(pid));
        }
        let _ = fs::remove_file(ws.join(".ralph-stub-pid"));
        let _ = fs::remove_file(ws.join(".ralph-stub-env"));
        let _ = fs::remove_file(ws.join(".ralph-stub-tg-response"));
    }
}

fn daemon_start_webhook_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "start", "--mode", "webhook", "--port", "19500", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let out = assert_cmd_success(&mut cmd);
        assert!(out.contains("Daemon started"));
        std::thread::sleep(Duration::from_millis(500));
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "status", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let out = assert_cmd_success(&mut cmd);
        assert!(out.contains("running") && out.contains("webhook"));
        std::mem::forget(DaemonGuard::new(&ctx.home, TEAM_NAME, Some(&gh_token)));
    }
}

fn daemon_stop_webhook_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "stop", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);
        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        let _ = fs::remove_file(ws.join(".ralph-stub-pid"));
        let _ = fs::remove_file(ws.join(".ralph-stub-env"));
    }
}

fn daemon_sigkill_escalation_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let ws = ctx.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        let ignore_file = ws.join(".ralph-stub-ignore-sigterm");
        fs::write(&ignore_file, "").unwrap();
        let sigterm_log = ws.join(".ralph-stub-sigterm.log");
        let _ = fs::remove_file(&sigterm_log);
        let _ = fs::remove_file(ws.join(".ralph-stub-pid"));

        let _guard = DaemonGuard::new(&ctx.home, TEAM_NAME, Some(&gh_token));
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "start", "--mode", "poll", "--interval", "2", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);

        let create_output = Command::new("gh")
            .args(["issue", "create", "-R", &ctx.repo_full_name,
                "--title", "Trigger SIGKILL test", "--body", "E2E"])
            .env("GH_TOKEN", &gh_token).output().expect("Failed to create issue");
        assert!(create_output.status.success());

        let stub_pid_file = ws.join(".ralph-stub-pid");
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while !stub_pid_file.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(500));
        }
        assert!(stub_pid_file.exists(), "Daemon should have launched ralph");
        let ralph_pid: u32 = fs::read_to_string(&stub_pid_file).unwrap().trim().parse().unwrap();
        assert!(is_alive(ralph_pid));
        assert!(sigterm_log.exists(), "Ralph should have logged SIGTERM trap setup");

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "stop", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);

        super::super::helpers::wait_for_exit(ralph_pid, Duration::from_secs(10));
        assert!(!is_alive(ralph_pid));
        let log_content = fs::read_to_string(&sigterm_log).unwrap();
        assert!(log_content.contains("SIGTERM received and ignored"));

        let _ = fs::remove_file(&ignore_file);
        let _ = fs::remove_file(&sigterm_log);
        let _ = fs::remove_file(ws.join(".ralph-stub-pid"));
        let _ = fs::remove_file(ws.join(".ralph-stub-env"));
    }
}

fn daemon_stale_pid_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let _guard = DaemonGuard::new(&ctx.home, TEAM_NAME, Some(&gh_token));
        let pid_dir = ctx.home.join(".botminter");
        fs::create_dir_all(&pid_dir).unwrap();
        fs::write(pid_dir.join(format!("daemon-{}.pid", TEAM_NAME)), "99999").unwrap();

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "start", "--mode", "poll", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let out = assert_cmd_success(&mut cmd);
        assert!(out.contains("Daemon started"), "Should start despite stale PID: {}", out);

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "stop", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);
    }
}

fn daemon_already_running_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let _guard = DaemonGuard::new(&ctx.home, TEAM_NAME, Some(&gh_token));
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "start", "--mode", "poll", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "start", "--mode", "poll", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let output = cmd.output().expect("failed to run second start");
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("already running"));
    }
}

fn daemon_crashed_detection_fn(gh_token: String) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "start", "--mode", "poll", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        assert_cmd_success(&mut cmd);

        let pid_file = ctx.home.join(format!(".botminter/daemon-{}.pid", TEAM_NAME));
        let daemon_pid: u32 = fs::read_to_string(&pid_file).unwrap().trim().parse().unwrap();
        force_kill(daemon_pid);
        super::super::helpers::wait_for_exit(daemon_pid, Duration::from_secs(5));

        let mut cmd = bm_cmd();
        cmd.args(["daemon", "status", "-t", TEAM_NAME])
            .env("HOME", &ctx.home).env("PATH", path_with_stub(&ctx.home)).env("GH_TOKEN", &gh_token);
        let out = assert_cmd_success(&mut cmd);
        assert!(out.contains("not running") || out.contains("stale"));
        assert!(!pid_file.exists(), "Stale PID file should be cleaned up");
    }
}

// ── Scenario construction ────────────────────────────────────────────

fn build_suite(gh_org: String, gh_token: String) -> GithubSuite {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let repo_full_name = format!("{}/bm-e2e-{}", gh_org, timestamp);

    let suite = GithubSuite::new_self_managed("scenario_operator_journey", &repo_full_name)
        .setup({
            move |ctx| {
                install_stub_ralph(&ctx.home);
                bootstrap_profiles_to_tmp(&ctx.home);
                setup_git_auth(&ctx.home);

                let tg_id_file = ctx.home.join(".tg-mock-container-id");
                let tg_url_file = ctx.home.join(".tg-mock-url");
                if tg_id_file.exists() && tg_url_file.exists() {
                    let cid = fs::read_to_string(&tg_id_file).unwrap().trim().to_string();
                    let port: u16 = fs::read_to_string(&tg_url_file).unwrap().trim()
                        .rsplit(':').next().unwrap().parse().unwrap();
                    let mock = telegram::TgMock::from_existing(cid, port);
                    if mock.is_running() {
                        eprintln!("tg-mock already running, reusing");
                        std::mem::forget(mock);
                        return;
                    }
                    drop(mock);
                }
                if telegram::podman_available() {
                    let mock = telegram::TgMock::start();
                    fs::write(&tg_url_file, mock.api_url()).unwrap();
                    let (container_id, _) = mock.into_parts();
                    fs::write(&tg_id_file, &container_id).unwrap();
                } else {
                    eprintln!("SKIP tg-mock: podman not available");
                }
            }
        })
        // ── First pass: fresh start ──────────────────────────────────
        .case("init_with_bridge_fresh", init_with_bridge_fn(gh_org.clone(), gh_token.clone()))
        .case("hire_member_fresh", hire_member_fn(gh_token.clone()))
        .case("projects_add_fresh", projects_add_fn(gh_org.clone(), gh_token.clone()))
        .case("teams_show_fresh", teams_show_fn())
        .case("bridge_identity_add_fresh", bridge_identity_add_fn(gh_token.clone()))
        .case("bridge_identity_list_fresh", bridge_identity_list_fn())
        .case("sync_bridge_and_repos_fresh", sync_bridge_and_repos_fn(gh_token.clone()))
        .case("sync_idempotent_fresh", sync_idempotent_fn(gh_token.clone()))
        .case("projects_sync_fresh", projects_sync_fn(gh_org.clone(), gh_token.clone()))
        .case("start_without_ralph_errors_fresh", start_without_ralph_errors_fn())
        .case("start_status_healthy_fresh", start_status_healthy_fn(gh_token.clone()))
        .case("bridge_functional_fresh", bridge_functional_fn())
        .case("stop_clean_shutdown_fresh", stop_clean_shutdown_fn())
        .case("stop_force_kills_fresh", stop_force_kills_fn(gh_token.clone()))
        .case("status_detects_crashed_fresh", status_detects_crashed_fn(gh_token.clone()))
        .case("members_list_fresh", members_list_fn())
        .case("teams_list_fresh", teams_list_fn())
        .case("daemon_start_poll_fresh", daemon_start_poll_fn(gh_token.clone()))
        .case("daemon_poll_launches_member_fresh", daemon_poll_launches_member_fn(gh_token.clone()))
        .case("daemon_stop_poll_fresh", daemon_stop_poll_fn(gh_token.clone()))
        .case("daemon_start_webhook_fresh", daemon_start_webhook_fn(gh_token.clone()))
        .case("daemon_stop_webhook_fresh", daemon_stop_webhook_fn(gh_token.clone()))
        .case("daemon_sigkill_escalation_fresh", daemon_sigkill_escalation_fn(gh_token.clone()))
        .case("daemon_stale_pid_fresh", daemon_stale_pid_fn(gh_token.clone()))
        .case("daemon_already_running_fresh", daemon_already_running_fn(gh_token.clone()))
        .case("daemon_crashed_detection_fresh", daemon_crashed_detection_fn(gh_token.clone()))
        // ── Reset HOME ───────────────────────────────────────────────
        .case("reset_home", |ctx| {
            eprintln!("Wiping HOME for second pass...");
            let tg_id = fs::read_to_string(ctx.home.join(".tg-mock-container-id")).ok();
            let tg_url = fs::read_to_string(ctx.home.join(".tg-mock-url")).ok();
            fs::remove_dir_all(&ctx.home).unwrap();
            fs::create_dir_all(&ctx.home).unwrap();
            install_stub_ralph(&ctx.home);
            bootstrap_profiles_to_tmp(&ctx.home);
            setup_git_auth(&ctx.home);
            reset_keyring();
            if let Some(id) = tg_id { fs::write(ctx.home.join(".tg-mock-container-id"), id).unwrap(); }
            if let Some(url) = tg_url { fs::write(ctx.home.join(".tg-mock-url"), url).unwrap(); }
        })
        // ── Second pass: existing repo ───────────────────────────────
        .case("init_with_bridge_existing", init_with_bridge_fn(gh_org.clone(), gh_token.clone()))
        .case_expect_error("hire_member_existing", hire_member_fn(gh_token.clone()),
            |err| err.contains("already exists"))
        .case_expect_error("projects_add_existing", projects_add_fn(gh_org.clone(), gh_token.clone()),
            |err| err.contains("already exists"))
        .case("teams_show_existing", teams_show_fn())
        .case("bridge_identity_add_existing", bridge_identity_add_fn(gh_token.clone()))
        .case("bridge_identity_list_existing", bridge_identity_list_fn())
        .case("sync_bridge_and_repos_existing", sync_bridge_and_repos_fn(gh_token.clone()))
        .case("sync_idempotent_existing", sync_idempotent_fn(gh_token.clone()))
        .case("projects_sync_existing", projects_sync_fn(gh_org.clone(), gh_token.clone()))
        .case("start_without_ralph_errors_existing", start_without_ralph_errors_fn())
        .case("start_status_healthy_existing", start_status_healthy_fn(gh_token.clone()))
        .case("bridge_functional_existing", bridge_functional_fn())
        .case("stop_clean_shutdown_existing", stop_clean_shutdown_fn())
        .case("stop_force_kills_existing", stop_force_kills_fn(gh_token.clone()))
        .case("status_detects_crashed_existing", status_detects_crashed_fn(gh_token.clone()))
        .case("members_list_existing", members_list_fn())
        .case("teams_list_existing", teams_list_fn())
        .case("daemon_start_poll_existing", daemon_start_poll_fn(gh_token.clone()))
        .case("daemon_poll_launches_member_existing", daemon_poll_launches_member_fn(gh_token.clone()))
        .case("daemon_stop_poll_existing", daemon_stop_poll_fn(gh_token.clone()))
        .case("daemon_start_webhook_existing", daemon_start_webhook_fn(gh_token.clone()))
        .case("daemon_stop_webhook_existing", daemon_stop_webhook_fn(gh_token.clone()))
        .case("daemon_sigkill_escalation_existing", daemon_sigkill_escalation_fn(gh_token.clone()))
        .case("daemon_stale_pid_existing", daemon_stale_pid_fn(gh_token.clone()))
        .case("daemon_already_running_existing", daemon_already_running_fn(gh_token.clone()))
        .case("daemon_crashed_detection_existing", daemon_crashed_detection_fn(gh_token.clone()))
        // ── Cleanup ──────────────────────────────────────────────────
        .case("cleanup", {
            let gh_org_c = gh_org.clone();
            let gh_token_c = gh_token.clone();
            move |ctx| {
                eprintln!("Final cleanup...");
                // Delete workspace repo
                let ws_repo = format!("{}/{}-{}", gh_org_c, TEAM_NAME, MEMBER_DIR);
                let _ = Command::new("gh").args(["repo", "delete", &ws_repo, "--yes"])
                    .env("GH_TOKEN", &gh_token_c).output();
                // Delete project repo (read URL from team repo manifest)
                let manifest_path = ctx.home.join("workspaces").join(TEAM_NAME).join("team/botminter.yml");
                if let Ok(contents) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_yml::from_str::<serde_yml::Value>(&contents) {
                        if let Some(projects) = manifest["projects"].as_sequence() {
                            for proj in projects {
                                if let Some(url) = proj["fork_url"].as_str() {
                                    // Convert https://github.com/org/repo.git → org/repo
                                    let repo_name = url.trim_start_matches("https://github.com/")
                                        .trim_end_matches(".git");
                                    let _ = Command::new("gh").args(["repo", "delete", repo_name, "--yes"])
                                        .env("GH_TOKEN", &gh_token_c).output();
                                }
                            }
                        }
                    }
                }
                // Delete team repo
                let _ = Command::new("gh").args(["repo", "delete", &ctx.repo_full_name, "--yes"]).output();
                let tg_id_file = ctx.home.join(".tg-mock-container-id");
                if let Ok(cid) = fs::read_to_string(&tg_id_file) {
                    let cid = cid.trim();
                    let _ = Command::new("podman").args(["stop", "-t", "2", cid]).output();
                    let _ = Command::new("podman").args(["rm", "-f", cid]).output();
                }
                cleanup_project_boards(&gh_org_c, &gh_token_c, TEAM_NAME);
            }
        });

    // Groups: start→bridge→stop in both passes, webhook start→stop in both
    // First pass: cases 10-12, 20-21
    // Second pass offset = 27 (26 journey + 1 reset)
    suite
        .group(10, 12).group(20, 21)
        .group(10 + 27, 12 + 27).group(20 + 27, 21 + 27)
}

pub fn scenario(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone()).build(config)
}

pub fn scenario_progressive(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone()).build_progressive(config)
}
