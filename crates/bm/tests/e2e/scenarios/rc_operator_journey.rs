//! Rocket.Chat Operator Journey
//!
//! Exercises the full operator lifecycle with a Rocket.Chat bridge:
//! init -> hire -> bridge start -> identity add -> room create -> sync -> health -> stop
//!
//! Requires Podman to be available. The suite is skipped if Podman is not installed.

use std::fs;
use std::process::Command;

use libtest_mimic::Trial;

use super::super::helpers::{
    assert_cmd_success, bm_cmd, bootstrap_profiles_to_tmp,
    install_stub_ralph, path_with_stub, setup_git_auth,
    E2eConfig, GithubSuite, SuiteCtx,
};
use super::super::rocketchat::RcPodGuard;
use super::super::telegram;

// ── Constants ─────────────────────────────────────────────────────────

const TEAM_NAME: &str = "e2e-rc";
const PROFILE: &str = "scrum-compact";
const ROLE: &str = "superman";
const MEMBER_NAME: &str = "bot-alice";
const MEMBER_DIR: &str = "superman-bot-alice";

// ── Helpers ──────────────────────────────────────────────────────────

/// Apply the real (pre-keyring-isolation) D-Bus and XDG_RUNTIME_DIR env vars
/// to a command so podman can talk to systemd for cgroup management.
fn apply_real_dbus_env(cmd: &mut Command, home: &std::path::Path) {
    if let Ok(addr) = fs::read_to_string(home.join(".rc-real-dbus-addr")) {
        cmd.env("DBUS_SESSION_BUS_ADDRESS", addr.trim());
    }
    if let Ok(xdg) = fs::read_to_string(home.join(".rc-real-xdg-runtime")) {
        cmd.env("XDG_RUNTIME_DIR", xdg.trim());
    }
}

// ── Reusable case functions ───────────────────────────────────────────

fn init_with_rc_bridge_fn(
    gh_org: String,
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let workzone = ctx.home.join("workspaces");
        let repo_name = ctx.repo_full_name.split('/').next_back().unwrap();

        let mut cmd = bm_cmd();
        cmd.args([
            "init",
            "--non-interactive",
            "--profile",
            PROFILE,
            "--team-name",
            TEAM_NAME,
            "--org",
            &gh_org,
            "--repo",
            repo_name,
            "--bridge",
            "rocketchat",
            "--workzone",
            &workzone.to_string_lossy(),
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
        assert!(
            team_repo.join(".git").is_dir(),
            "team repo should have .git"
        );
        assert!(
            team_repo.join("botminter.yml").exists(),
            "should have botminter.yml"
        );

        // Verify bridge directory exists in team repo
        assert!(
            team_repo.join("bridges/rocketchat/bridge.yml").exists(),
            "should have rocketchat bridge.yml"
        );
        assert!(
            team_repo.join("bridges/rocketchat/Justfile").exists(),
            "should have rocketchat Justfile"
        );

        // Verify botminter.yml has bridge: rocketchat
        let manifest = fs::read_to_string(team_repo.join("botminter.yml")).unwrap();
        assert!(
            manifest.contains("bridge: rocketchat"),
            "botminter.yml should declare rocketchat bridge"
        );

        setup_git_auth(&ctx.home);
    }
}

fn hire_member_fn(
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
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
        assert!(
            stdout.contains(MEMBER_DIR) || stdout.contains(MEMBER_NAME),
            "hire output should mention member"
        );
    }
}

fn bridge_start_fn(
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        // Find a free port for RC
        let port = super::super::helpers::find_free_port();
        eprintln!("RC bridge will use port {}", port);

        // Save port for subsequent cases
        fs::write(ctx.home.join(".rc-port"), port.to_string()).unwrap();

        let mut cmd = bm_cmd();
        cmd.args(["bridge", "start", "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token)
            .env("RC_PORT", port.to_string());
        apply_real_dbus_env(&mut cmd, &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("bridge start: {}", stdout.trim());

        // Verify bridge-state.json
        let bstate_path = ctx
            .home
            .join("workspaces")
            .join(TEAM_NAME)
            .join("bridge-state.json");
        assert!(
            bstate_path.exists(),
            "bridge-state.json should exist after bridge start"
        );
        let bstate_contents = fs::read_to_string(&bstate_path).unwrap();
        let bstate: serde_json::Value = serde_json::from_str(&bstate_contents).unwrap();
        assert_eq!(
            bstate["status"].as_str(),
            Some("running"),
            "bridge-state.json should show status running"
        );

        // Create RcPodGuard as panic safety net
        let pod_name = format!("bm-rc-{}", TEAM_NAME);
        let guard = RcPodGuard::new(pod_name.clone(), port);

        // Save guard info to files for progressive mode and subsequent cases
        fs::write(ctx.home.join(".rc-pod-name"), &pod_name).unwrap();

        // Forget guard -- it will be consumed in the stop case
        // For now, save parts so we can recreate on resume
        let (name, p) = guard.into_parts();
        fs::write(ctx.home.join(".rc-guard-name"), &name).unwrap();
        fs::write(ctx.home.join(".rc-guard-port"), p.to_string()).unwrap();
    }
}

fn identity_add_fn(
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let port = fs::read_to_string(ctx.home.join(".rc-port"))
            .unwrap()
            .trim()
            .to_string();

        let mut cmd = bm_cmd();
        cmd.args(["bridge", "identity", "add", MEMBER_DIR, "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token)
            .env("RC_PORT", &port);
        apply_real_dbus_env(&mut cmd, &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("identity add: {}", stdout.trim());
        assert!(
            stdout.contains(MEMBER_DIR),
            "identity add output should mention member"
        );

        // Verify bridge-state.json has identity
        let bstate_path = ctx
            .home
            .join("workspaces")
            .join(TEAM_NAME)
            .join("bridge-state.json");
        let bstate_contents = fs::read_to_string(&bstate_path).unwrap();
        let bstate: serde_json::Value = serde_json::from_str(&bstate_contents).unwrap();
        let identity = &bstate["identities"][MEMBER_DIR];
        assert!(
            !identity["user_id"].as_str().unwrap_or("").is_empty(),
            "identity should have user_id after onboard"
        );
    }
}

fn room_create_fn(
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let port = fs::read_to_string(ctx.home.join(".rc-port"))
            .unwrap()
            .trim()
            .to_string();

        let mut cmd = bm_cmd();
        cmd.args([
            "bridge",
            "room",
            "create",
            "e2e-team",
            "-t",
            TEAM_NAME,
        ])
        .env("HOME", &ctx.home)
        .env("GH_TOKEN", &gh_token)
        .env("RC_PORT", &port);
        apply_real_dbus_env(&mut cmd, &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("room create: {}", stdout.trim());

        // Verify bridge-state.json has room
        let bstate_path = ctx
            .home
            .join("workspaces")
            .join(TEAM_NAME)
            .join("bridge-state.json");
        let bstate_contents = fs::read_to_string(&bstate_path).unwrap();
        let bstate: serde_json::Value = serde_json::from_str(&bstate_contents).unwrap();
        let rooms = bstate["rooms"].as_array().expect("rooms should be array");
        assert!(
            !rooms.is_empty(),
            "should have at least one room after room create"
        );
    }
}

fn sync_bridge_fn(
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let port = fs::read_to_string(ctx.home.join(".rc-port"))
            .unwrap()
            .trim()
            .to_string();

        let mut cmd = bm_cmd();
        cmd.args(["teams", "sync", "--bridge", "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token)
            .env("RC_PORT", &port)
            // Ensure credential is resolved via env var rather than keyring.
            // The keyring may not be accessible when using the real D-Bus
            // (needed for podman) instead of the isolated test D-Bus.
            .env("BM_BRIDGE_TOKEN_SUPERMAN_BOT_ALICE", "rc-e2e-token");
        apply_real_dbus_env(&mut cmd, &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("sync bridge: {}", stdout.trim());

        // Verify workspace was created and ralph.yml has RObot.rocketchat config
        let ws = ctx
            .home
            .join("workspaces")
            .join(TEAM_NAME)
            .join(MEMBER_DIR);
        assert!(
            ws.join(".botminter.workspace").exists(),
            "workspace should have marker file"
        );

        let ralph_yml_path = ws.join("ralph.yml");
        assert!(ralph_yml_path.exists(), "ralph.yml should exist");

        let ralph_contents = fs::read_to_string(&ralph_yml_path).unwrap();
        let ralph_doc: serde_yml::Value =
            serde_yml::from_str(&ralph_contents).unwrap();

        assert_eq!(
            ralph_doc["RObot"]["enabled"].as_bool(),
            Some(true),
            "RObot.enabled should be true"
        );
        assert!(
            ralph_doc["RObot"]["rocketchat"]["bot_user_id"]
                .as_str()
                .is_some(),
            "RObot.rocketchat.bot_user_id should be set"
        );
        assert!(
            ralph_doc["RObot"]["rocketchat"]["room_id"]
                .as_str()
                .is_some(),
            "RObot.rocketchat.room_id should be set"
        );
        assert!(
            ralph_doc["RObot"]["rocketchat"]["server_url"]
                .as_str()
                .is_some(),
            "RObot.rocketchat.server_url should be set"
        );

        // Verify NO auth_token in ralph.yml (secrets stay as env vars)
        assert!(
            !ralph_contents.contains("auth_token"),
            "ralph.yml must NOT contain auth_token"
        );
    }
}

fn bridge_health_fn(
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let mut cmd = bm_cmd();
        cmd.args(["bridge", "status", "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token);
        apply_real_dbus_env(&mut cmd, &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("bridge status: {}", stdout.trim());
        assert!(
            stdout.contains("running") || stdout.contains("healthy"),
            "bridge status should show running or healthy"
        );
    }
}

fn bridge_stop_fn(
    gh_token: String,
) -> impl Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |ctx| {
        let port = fs::read_to_string(ctx.home.join(".rc-port"))
            .unwrap()
            .trim()
            .to_string();

        let mut cmd = bm_cmd();
        cmd.args(["bridge", "stop", "-t", TEAM_NAME])
            .env("HOME", &ctx.home)
            .env("GH_TOKEN", &gh_token)
            .env("RC_PORT", &port);
        apply_real_dbus_env(&mut cmd, &ctx.home);
        let stdout = assert_cmd_success(&mut cmd);
        eprintln!("bridge stop: {}", stdout.trim());

        // Recreate guard from saved state, then consume it to prevent Drop cleanup
        let guard_name = fs::read_to_string(ctx.home.join(".rc-guard-name"))
            .unwrap()
            .trim()
            .to_string();
        let guard_port: u16 = fs::read_to_string(ctx.home.join(".rc-guard-port"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let guard = RcPodGuard::from_existing(guard_name, guard_port);
        // Consume the guard -- pod already stopped by bm bridge stop
        let _ = guard.into_parts();

        // Verify bridge-state.json shows stopped
        let bstate_path = ctx
            .home
            .join("workspaces")
            .join(TEAM_NAME)
            .join("bridge-state.json");
        let bstate_contents = fs::read_to_string(&bstate_path).unwrap();
        let bstate: serde_json::Value = serde_json::from_str(&bstate_contents).unwrap();
        assert_eq!(
            bstate["status"].as_str(),
            Some("stopped"),
            "bridge-state.json should show status stopped after bridge stop"
        );
    }
}

// ── Scenario construction ────────────────────────────────────────────

fn build_suite(gh_org: String, gh_token: String) -> GithubSuite {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let repo_full_name = format!("{}/bm-e2e-rc-{}", gh_org, timestamp);

    GithubSuite::new_self_managed("scenario_rc_operator_journey", &repo_full_name)
        .setup({
            move |ctx| {
                install_stub_ralph(&ctx.home);
                bootstrap_profiles_to_tmp(&ctx.home);
                setup_git_auth(&ctx.home);

                // Verify podman is available
                if !telegram::podman_available() {
                    panic!("SKIP: podman not available -- RC bridge requires Podman");
                }

                // Save real D-Bus/XDG env vars BEFORE keyring isolation replaces them.
                // Podman needs the real systemd D-Bus to manage cgroups.
                if let Ok(addr) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
                    fs::write(ctx.home.join(".rc-real-dbus-addr"), &addr).unwrap();
                } else {
                    // Compute from XDG_RUNTIME_DIR or /run/user/{uid}
                    let uid = unsafe { libc::getuid() };
                    let addr = format!("unix:path=/run/user/{}/bus", uid);
                    fs::write(ctx.home.join(".rc-real-dbus-addr"), &addr).unwrap();
                }
                if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
                    fs::write(ctx.home.join(".rc-real-xdg-runtime"), &xdg).unwrap();
                } else {
                    let uid = unsafe { libc::getuid() };
                    let xdg = format!("/run/user/{}", uid);
                    fs::write(ctx.home.join(".rc-real-xdg-runtime"), &xdg).unwrap();
                }
            }
        })
        .case(
            "01_init_with_rc_bridge",
            init_with_rc_bridge_fn(gh_org.clone(), gh_token.clone()),
        )
        .case("02_hire_member", hire_member_fn(gh_token.clone()))
        .case("03_bridge_start", bridge_start_fn(gh_token.clone()))
        .case("04_identity_add", identity_add_fn(gh_token.clone()))
        .case("05_room_create", room_create_fn(gh_token.clone()))
        .case("06_sync_bridge", sync_bridge_fn(gh_token.clone()))
        .case("07_bridge_health", bridge_health_fn(gh_token.clone()))
        .case("08_bridge_stop", bridge_stop_fn(gh_token.clone()))
        // ── Cleanup ──────────────────────────────────────────────────
        .case("cleanup", {
            let gh_org_c = gh_org.clone();
            let gh_token_c = gh_token.clone();
            move |ctx| {
                eprintln!("RC journey cleanup...");

                // Force-remove RC pod if it's still around
                let pod_name = format!("bm-rc-{}", TEAM_NAME);
                let _ = Command::new("podman")
                    .args(["pod", "rm", "-f", &pod_name])
                    .output();

                // Delete team repo
                let _ = Command::new("gh")
                    .args(["repo", "delete", &ctx.repo_full_name, "--yes"])
                    .env("GH_TOKEN", &gh_token_c)
                    .output();

                // Delete workspace repo
                let ws_repo = format!("{}/{}-{}", gh_org_c, TEAM_NAME, MEMBER_DIR);
                let _ = Command::new("gh")
                    .args(["repo", "delete", &ws_repo, "--yes"])
                    .env("GH_TOKEN", &gh_token_c)
                    .output();

                // Clean up project boards
                super::super::helpers::cleanup_project_boards(
                    &gh_org_c,
                    &gh_token_c,
                    TEAM_NAME,
                );
            }
        })
        // Group bridge start through stop as atomic (cases 2-7, 0-indexed)
        .group(2, 7)
}

pub fn scenario(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone()).build(config)
}

pub fn scenario_progressive(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone()).build_progressive(config)
}
