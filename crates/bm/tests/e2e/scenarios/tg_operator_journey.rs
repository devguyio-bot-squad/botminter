//! Telegram Operator Journey
//!
//! Exercises the Telegram (external) bridge flow as a lighter scenario:
//! init -> hire -> identity add -> sync -> start -> verify env vars -> stop -> cleanup
//!
//! No daemon tests, no per-member start/stop, no idempotency second pass.
//! Just the Telegram-specific bridge lifecycle.
//!
//! Requires tg-mock (Podman). Skipped if Podman is not available.

use std::fs;
use std::time::Duration;

use libtest_mimic::Trial;

use super::super::helpers::{
    cleanup_project_boards, read_pid_from_state,
    E2eConfig, GithubSuite, ProcessGuard,
};
use super::super::telegram;
use super::super::test_env::TestEnv;

// ── Constants ─────────────────────────────────────────────────────────

const TEAM_NAME: &str = "e2e-tg";
const PROFILE: &str = "agentic-sdlc-minimal";
const ROLE: &str = "engineer";
const MEMBER_NAME: &str = "tg-alice";
const MEMBER_DIR: &str = "engineer-tg-alice";
const BOT_TOKEN: &str = "123456789:ABCDEFGhijklmnopqrstuvwxyz-e2e";

// ── Reusable case functions ───────────────────────────────────────────

fn init_with_tg_bridge_fn(
    gh_org: String,
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let workzone = env.home.join("workspaces");
        let repo_name = env.repo_full_name.split('/').next_back().unwrap().to_string();
        let board_title = format!("{} Board", TEAM_NAME);

        let output = env.command("bm")
            .args([
                "init", "--non-interactive",
                "--profile", PROFILE,
                "--team-name", TEAM_NAME,
                "--org", &gh_org,
                "--repo", &repo_name,
                "--bridge", "telegram",
                "--github-project-board", &board_title,
                "--workzone", &workzone.to_string_lossy(),
            ])
            .output();
        assert!(output.status.success(), "bm init failed: {}", String::from_utf8_lossy(&output.stderr));

        let team_repo = workzone.join(TEAM_NAME).join("team");
        assert!(team_repo.join(".git").is_dir(), "team repo should have .git");
        assert!(team_repo.join("botminter.yml").exists(), "should have botminter.yml");
    }
}

fn hire_member_fn(
    _gh_token: String,
    app_id: String,
    app_client_id: String,
    app_installation_id: String,
    app_private_key_file: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let stdout = env.command("bm")
            .args([
                "hire", ROLE, "--name", MEMBER_NAME, "-t", TEAM_NAME,
                "--reuse-app",
                "--app-id", &app_id,
                "--client-id", &app_client_id,
                "--private-key-file", &app_private_key_file,
                "--installation-id", &app_installation_id,
            ])
            .run();
        assert!(stdout.contains(MEMBER_DIR) || stdout.contains(MEMBER_NAME));
    }
}

fn bridge_identity_add_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let stdout = env.command("bm")
            .args(["bridge", "identity", "add", MEMBER_DIR, "-t", TEAM_NAME])
            .env(
                &format!("BM_BRIDGE_TOKEN_{}", MEMBER_DIR.to_uppercase().replace('-', "_")),
                BOT_TOKEN,
            )
            .run();
        assert!(stdout.contains(MEMBER_DIR));

        // Verify token was stored
        let list_out = env.command("bm")
            .args(["bridge", "identity", "list", "-t", TEAM_NAME])
            .run();
        assert!(list_out.contains(MEMBER_DIR), "identity should appear in list after add");
    }
}

fn sync_bridge_and_repos_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let stdout = env.command("bm")
            .args(["teams", "sync", "--bridge", "--repos", "-t", TEAM_NAME])
            .run();
        assert!(!stdout.contains("No bridge configured"));

        let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        assert!(ws.join(".botminter.workspace").exists());
        assert!(ws.join("ralph.yml").exists());
    }
}

fn start_and_verify_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        // Remove brain-prompt.md so bm start uses ralph (the stub) instead of
        // bm brain-run. Brain mode is tested separately in exploratory tests.
        let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
        let _ = fs::remove_file(ws.join("brain-prompt.md"));

        let mut guard = ProcessGuard::new(env, TEAM_NAME);
        let mut cmd = env.command("bm");
        cmd.args(["start", "-t", TEAM_NAME]);
        if let Some(url) = env.get_export("tg_mock_url") {
            cmd.env("RALPH_TELEGRAM_API_URL", url)
                .env("RALPH_TELEGRAM_BOT_TOKEN", BOT_TOKEN);
        }
        let stdout = cmd.run();
        assert!(stdout.contains("Started 1 member"));

        if let Some(pid) = read_pid_from_state(&env.home) { guard.set_pid(pid); }

        // Verify Telegram env vars in stub ralph
        if env.get_export("tg_mock_url").is_some() {
            std::thread::sleep(Duration::from_secs(3));
            let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);
            let env_content = fs::read_to_string(ws.join(".ralph-stub-env")).unwrap();
            assert!(env_content.contains("RALPH_TELEGRAM_API_URL="),
                "stub env should contain RALPH_TELEGRAM_API_URL");
            assert!(env_content.contains(&format!("RALPH_TELEGRAM_BOT_TOKEN={}", BOT_TOKEN)),
                "stub env should contain RALPH_TELEGRAM_BOT_TOKEN");
            assert!(env_content.contains("GH_CONFIG_DIR="),
                "stub env should contain GH_CONFIG_DIR (App credential path)");
            let tg_response = fs::read_to_string(ws.join(".ralph-stub-tg-response")).unwrap();
            assert!(tg_response.contains("ok"),
                "stub should have received ok from tg-mock");
        }

        std::mem::forget(guard);
    }
}

fn stop_fn() -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let pid_before = read_pid_from_state(&env.home);
        let stdout = env.command("bm")
            .args(["stop", "-t", TEAM_NAME])
            .run();
        assert!(stdout.contains("Stopped 1 member"));
        if let Some(pid) = pid_before {
            super::super::helpers::wait_for_exit(pid, Duration::from_secs(5));
        }
    }
}

// ── Scenario construction ────────────────────────────────────────────

fn build_suite(gh_org: String, gh_token: String, config: &E2eConfig) -> GithubSuite {
    let app_id = config.app_id.clone();
    let app_client_id = config.app_client_id.clone();
    let app_installation_id = config.app_installation_id.clone();
    let app_private_key_file = config.app_private_key_file.clone();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let repo_full_name = format!("{}/bm-e2e-tg-{}", gh_org, timestamp);

    GithubSuite::new_self_managed("scenario_tg_operator_journey", &repo_full_name)
        .setup({
            move |env: &mut TestEnv| {
                // Start tg-mock if podman is available
                if let Some(url) = env.get_export("tg_mock_url") {
                    // Check if the container from a previous progressive run is still alive
                    if let Some(cid) = env.get_export("tg_mock_container_id") {
                        let mock = telegram::TgMock::from_existing(
                            cid.to_string(),
                            url.rsplit(':').next().unwrap().parse().unwrap(),
                        );
                        if mock.is_running() {
                            eprintln!("tg-mock already running, reusing");
                            std::mem::forget(mock);
                            return;
                        }
                        drop(mock);
                    }
                }
                if telegram::podman_available() {
                    let mock = telegram::TgMock::start();
                    env.export("tg_mock_url", &mock.api_url());
                    let (container_id, _) = mock.into_parts();
                    env.export("tg_mock_container_id", &container_id);
                } else {
                    eprintln!("SKIP tg-mock: podman not available");
                }
            }
        })
        .case("01_init_with_tg_bridge", init_with_tg_bridge_fn(gh_org.clone(), gh_token.clone()))
        .case("02_hire_member", hire_member_fn(gh_token.clone(), app_id.clone(), app_client_id.clone(), app_installation_id.clone(), app_private_key_file.clone()))
        .case("03_bridge_identity_add", bridge_identity_add_fn(gh_token.clone()))
        .case("04_sync_bridge_and_repos", sync_bridge_and_repos_fn(gh_token.clone()))
        .case("05_start_and_verify", start_and_verify_fn(gh_token.clone()))
        .case("06_stop", stop_fn())
        // ── Cleanup ──────────────────────────────────────────────────
        .case("cleanup", {
            let gh_org_c = gh_org.clone();
            let gh_token_c = gh_token.clone();
            move |env: &mut TestEnv| {
                eprintln!("TG journey cleanup...");
                // Stop tg-mock container
                if let Some(cid) = env.get_export("tg_mock_container_id") {
                    let _ = env.command("podman")
                        .args(["stop", "-t", "2", cid])
                        .output();
                    let _ = env.command("podman")
                        .args(["rm", "-f", cid])
                        .output();
                }
                // Delete workspace repo
                let ws_repo = format!("{}/{}-{}", gh_org_c, TEAM_NAME, MEMBER_DIR);
                let _ = env.command("gh")
                    .args(["repo", "delete", &ws_repo, "--yes"])
                    .output();
                // Delete team repo
                let _ = env.command("gh")
                    .args(["repo", "delete", &env.repo_full_name, "--yes"])
                    .output();
                cleanup_project_boards(&gh_org_c, &gh_token_c, TEAM_NAME);
            }
        })
        // Group start + stop (cases 4-5, 0-indexed)
        .group(4, 5)
}

pub fn scenario(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone(), config).build(config)
}

pub fn scenario_progressive(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone(), config).build_progressive(config)
}
