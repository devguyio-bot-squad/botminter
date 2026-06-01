//! Session Journey
//!
//! Exercises the session lifecycle model end-to-end: creation, status tracking,
//! chat lifecycle, stop variants (graceful/force/bare), concurrent sessions,
//! daemon recovery, and failure retention.
//!
//! Tests require a running daemon and verify session state via `bm status --json`.
//! The coding agent stub (stub-agent.sh) is installed as `claude` in PATH by TestEnv.
//! No bridge is configured — this suite focuses purely on session mechanics.

use std::fs;
use std::time::Duration;

use libtest_mimic::Trial;

use super::super::helpers::{
    find_free_port, force_kill, is_alive, read_pid_from_state, wait_for_exit, E2eConfig,
    GithubSuite, ProcessGuard,
};
use super::super::test_env::TestEnv;

// ── Helpers ──────────────────────────────────────────────────────────

/// Fetches `bm status --json` and returns the parsed sessions array.
fn get_sessions_json(env: &TestEnv) -> Vec<serde_json::Value> {
    let output = env
        .command("bm")
        .args(["status", "--json", "-t", TEAM_NAME])
        .output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    json["sessions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

// ── Constants ─────────────────────────────────────────────────────────

const SUITE_NAME: &str = "scenario_session_journey";
const TEAM_NAME: &str = "e2e-session";
const PROFILE: &str = "agentic-sdlc-minimal";
const ROLE: &str = "engineer";
const MEMBER_NAME: &str = "alice";
const MEMBER_DIR: &str = "engineer-alice";

// ── Setup cases ───────────────────────────────────────────────────────

fn init_team_fn(
    gh_org: String,
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let workzone = env.home.join("workspaces");
        let repo_name = env
            .repo_full_name
            .split('/')
            .next_back()
            .unwrap()
            .to_string();
        let board_title = format!("{} Board", TEAM_NAME);

        let output = env
            .command("bm")
            .args([
                "init",
                "--non-interactive",
                "--profile",
                PROFILE,
                "--team-name",
                TEAM_NAME,
                "--org",
                &gh_org,
                "--repo",
                &repo_name,
                "--github-project-board",
                &board_title,
                "--workzone",
                &workzone.to_string_lossy(),
            ])
            .output();
        assert!(
            output.status.success(),
            "bm init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let team_dir = workzone.join(TEAM_NAME).join("team");
        assert!(team_dir.is_dir(), "team dir should exist after init");
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
        env.command("bm")
            .args([
                "hire",
                ROLE,
                "--name",
                MEMBER_NAME,
                "--reuse-app",
                "--app-id",
                &app_id,
                "--client-id",
                &app_client_id,
                "--installation-id",
                &app_installation_id,
                "--private-key-file",
                &app_private_key_file,
                "-t",
                TEAM_NAME,
            ])
            .run();

        let team_dir = env.home.join("workspaces").join(TEAM_NAME).join("team");
        assert!(
            team_dir.join("members").join(MEMBER_DIR).is_dir(),
            "member dir should exist after hire"
        );
    }
}

// ── Scenario 1: session_create_isolated ───────────────────────────────

fn session_create_isolated_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let mut guard = ProcessGuard::new(env, TEAM_NAME);
        let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);

        let stdout = env.command("bm").args(["start", "-t", TEAM_NAME]).run();
        assert!(
            stdout.contains("Started 1 member"),
            "bm start should create session, got: {}",
            stdout
        );

        if let Some(pid) = read_pid_from_state(&env.home) {
            guard.set_pid(pid);
        }

        // Workspace structure
        assert!(ws.exists(), "session workspace should exist");
        assert!(ws.join("team").is_dir(), "workspace should have team/ dir");
        for file in ["PROMPT.md", "CLAUDE.md", "ralph.yml"] {
            assert!(ws.join(file).exists(), "{} missing from workspace", file);
        }

        // Workspace is clean — no stale artifacts
        assert!(
            !ws.join(".stub-agent-pid").exists(),
            "fresh session workspace should not have .stub-agent-pid"
        );

        // Stub-agent wired: claude binary resolves to stub-bin/claude
        let which_output = env.command("which").args(["claude"]).output();
        assert!(
            which_output.status.success(),
            "stub-agent should be available as 'claude' in PATH"
        );
        let agent_path = String::from_utf8_lossy(&which_output.stdout);
        assert!(
            agent_path.contains("stub-bin/claude"),
            "claude should resolve to stub-bin/claude, got: {}",
            agent_path.trim()
        );

        // Session tracked by daemon — bm status --json should show Active session
        let sessions = get_sessions_json(env);
        assert!(
            !sessions.is_empty(),
            "sessions array should contain the started session"
        );
        let session = &sessions[0];
        assert_eq!(
            session["owning_member"].as_str(),
            Some(MEMBER_DIR),
            "session should be owned by {}",
            MEMBER_DIR
        );
        assert_eq!(
            session["current_state"].as_str(),
            Some("Active"),
            "session state should be Active"
        );

        // Clean up
        let _ = env
            .command("bm")
            .args(["stop", "--force", "-t", TEAM_NAME])
            .output();
        if let Some(pid) = guard.pid {
            wait_for_exit(pid, Duration::from_secs(5));
        }
        std::mem::forget(guard);
    }
}

// ── Scenario 2: session_chat_lifecycle ────────────────────────────────

fn session_chat_lifecycle_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);

        // Ensure workspace exists from previous start
        if !ws.exists() {
            let _ = env.command("bm").args(["start", "-t", TEAM_NAME]).output();
            let _ = env
                .command("bm")
                .args(["stop", "--force", "-t", TEAM_NAME])
                .output();
            std::thread::sleep(Duration::from_secs(1));
        }

        // Clean stale artifacts
        for f in [".stub-agent-pid", ".stub-agent-env", ".stub-agent-sigint.log"] {
            let _ = fs::remove_file(ws.join(f));
        }

        // bm chat execs the coding agent (stub-agent.sh). The stub exits immediately
        // (no BM_STUB_AGENT_INTERACTIVE), recording PID and env in the workspace.
        let output = env
            .command("bm")
            .args(["chat", MEMBER_NAME, "-t", TEAM_NAME])
            .output();
        assert!(
            output.status.success(),
            "bm chat should succeed (stub-agent exits cleanly), stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Stub-agent recorded its PID
        assert!(
            ws.join(".stub-agent-pid").exists(),
            "stub-agent should record PID to .stub-agent-pid in workspace"
        );

        // Stub-agent received environment
        let agent_env = fs::read_to_string(ws.join(".stub-agent-env"))
            .expect(".stub-agent-env should exist");
        assert!(
            agent_env.contains("GH_CONFIG_DIR") || agent_env.contains("GH_TOKEN"),
            "stub-agent should receive GitHub credentials"
        );

        // After chat exits, session deactivates → Completed
        let sessions = get_sessions_json(env);
        let has_completed = sessions.iter().any(|s| {
            s["current_state"].as_str() == Some("Completed")
                && s["session_type"].as_str() == Some("interactive")
        });
        assert!(
            has_completed,
            "after chat exit, a Completed interactive session should exist in: {:?}",
            sessions
        );
    }
}

// ── Scenario 3: session_concurrent ────────────────────────────────────

fn session_concurrent_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let mut guard = ProcessGuard::new(env, TEAM_NAME);

        // First session
        let stdout1 = env.command("bm").args(["start", "-t", TEAM_NAME]).run();
        assert!(
            stdout1.contains("Started 1 member"),
            "first start should succeed, got: {}",
            stdout1
        );
        if let Some(pid) = read_pid_from_state(&env.home) {
            guard.set_pid(pid);
        }

        // Second concurrent session for same member
        let output2 = env
            .command("bm")
            .args(["start", MEMBER_NAME, "-t", TEAM_NAME])
            .output();
        let stdout2 = String::from_utf8_lossy(&output2.stdout);
        assert!(
            !stdout2.contains("already running"),
            "second start should create concurrent session, not skip. Got: {}",
            stdout2
        );

        // Verify concurrent_count >= 2 via status --json
        let sessions = get_sessions_json(env);
        let active_count = sessions
            .iter()
            .filter(|s| {
                s["owning_member"].as_str() == Some(MEMBER_DIR)
                    && s["current_state"].as_str() == Some("Active")
            })
            .count();
        assert!(
            active_count >= 2,
            "should have >= 2 active sessions for {}, got {} in: {:?}",
            MEMBER_DIR,
            active_count,
            sessions
        );

        // Clean up
        let _ = env
            .command("bm")
            .args(["stop", "--force", "-t", TEAM_NAME])
            .output();
        if let Some(pid) = guard.pid {
            wait_for_exit(pid, Duration::from_secs(5));
        }
        std::mem::forget(guard);
    }
}

// ── Scenario 4: session_stop_graceful ─────────────────────────────────

fn session_stop_graceful_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let mut guard = ProcessGuard::new(env, TEAM_NAME);
        let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);

        // Start member
        env.command("bm").args(["start", "-t", TEAM_NAME]).run();
        if let Some(pid) = read_pid_from_state(&env.home) {
            guard.set_pid(pid);
        }

        // Make workspace dirty — create an uncommitted file
        let dirty_file = ws.join("dirty-test-file.txt");
        fs::write(&dirty_file, "uncommitted changes for finalization test").unwrap();

        // Graceful stop — should detect dirty repos and trigger finalization
        let output = env
            .command("bm")
            .args(["stop", MEMBER_NAME, "-t", TEAM_NAME])
            .output();
        // Verify stop returned immediately (deactivation is async)
        assert!(
            output.status.success(),
            "bm stop should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Wait for deactivation to complete
        if let Some(pid) = guard.pid {
            wait_for_exit(pid, Duration::from_secs(10));
        }

        // Session should enter Finalizing state (dirty repos detected)
        let sessions = get_sessions_json(env);
        let has_finalizing_or_completed = sessions.iter().any(|s| {
            let state = s["current_state"].as_str().unwrap_or("");
            state == "Finalizing" || state == "Completed"
        });
        assert!(
            has_finalizing_or_completed,
            "after graceful stop with dirty workspace, session should be Finalizing or Completed, got: {:?}",
            sessions
        );

        // Workspace should still exist (retained for finalization)
        assert!(
            ws.exists(),
            "workspace should be retained after graceful stop"
        );

        // Clean up dirty file
        let _ = fs::remove_file(&dirty_file);
        std::mem::forget(guard);
    }
}

// ── Scenario 5: session_force_stop ────────────────────────────────────

fn session_force_stop_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let mut guard = ProcessGuard::new(env, TEAM_NAME);
        let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);

        // Start member
        env.command("bm").args(["start", "-t", TEAM_NAME]).run();
        if let Some(pid) = read_pid_from_state(&env.home) {
            guard.set_pid(pid);
        }

        // Force stop — kill immediately, no finalization
        let output = env
            .command("bm")
            .args(["stop", "--force", "-t", TEAM_NAME])
            .output();
        assert!(
            output.status.success(),
            "bm stop --force should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Process should be dead
        if let Some(pid) = guard.pid {
            wait_for_exit(pid, Duration::from_secs(5));
            assert!(!is_alive(pid), "process should be dead after force stop");
        }

        // Session state should be Killed (not Completed)
        let sessions = get_sessions_json(env);
        let has_killed = sessions
            .iter()
            .any(|s| s["current_state"].as_str() == Some("Killed"));
        assert!(
            has_killed,
            "after force stop, session should be in Killed state, got: {:?}",
            sessions
        );

        // Workspace should be retained (re-trigger finalization later)
        assert!(
            ws.exists(),
            "workspace should be retained after force stop"
        );

        std::mem::forget(guard);
    }
}

// ── Scenario 6: session_status_dashboard ──────────────────────────────

fn session_status_dashboard_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let mut guard = ProcessGuard::new(env, TEAM_NAME);

        // Start member to create an active session
        env.command("bm").args(["start", "-t", TEAM_NAME]).run();
        if let Some(pid) = read_pid_from_state(&env.home) {
            guard.set_pid(pid);
        }

        // Text output — should contain session table with expected columns
        let text_output = env
            .command("bm")
            .args(["status", "-t", TEAM_NAME])
            .run();
        assert!(
            text_output.contains("Session ID"),
            "bm status should display Session ID column, got:\n{}",
            text_output
        );
        assert!(
            text_output.contains("State"),
            "bm status should display State column, got:\n{}",
            text_output
        );
        assert!(
            text_output.contains("Active"),
            "bm status should show Active session, got:\n{}",
            text_output
        );
        assert!(
            text_output.contains(MEMBER_DIR),
            "bm status should show member name {}, got:\n{}",
            MEMBER_DIR,
            text_output
        );

        // JSON output — verify structure
        let sessions = get_sessions_json(env);
        assert!(!sessions.is_empty(), "sessions array should not be empty");

        let session = &sessions[0];
        assert!(
            session["session_id"].as_str().is_some(),
            "session should have session_id"
        );
        assert!(
            session["start_time"].as_str().is_some(),
            "session should have start_time"
        );
        assert!(
            session["state_transitioned_at"].as_str().is_some(),
            "session should have state_transitioned_at"
        );
        assert!(
            session.get("concurrent_count").is_some(),
            "session should have concurrent_count field"
        );

        // Elapsed time in text should be human-readable (not raw timestamp)
        assert!(
            text_output.contains("Elapsed"),
            "bm status should display Elapsed column, got:\n{}",
            text_output
        );

        // Clean up
        let _ = env
            .command("bm")
            .args(["stop", "--force", "-t", TEAM_NAME])
            .output();
        if let Some(pid) = guard.pid {
            wait_for_exit(pid, Duration::from_secs(5));
        }
        std::mem::forget(guard);
    }
}

// ── Scenario 7: session_daemon_restart_recovery ───────────────────────

fn session_daemon_restart_recovery_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let mut guard = ProcessGuard::new(env, TEAM_NAME);
        let port = find_free_port();
        let port_str = port.to_string();

        // Start daemon in poll mode
        let daemon_output = env
            .command("bm")
            .args([
                "daemon", "start", "--mode", "poll", "--port", &port_str, "--interval", "60", "-t",
                TEAM_NAME,
            ])
            .output();
        assert!(
            daemon_output.status.success(),
            "bm daemon start should succeed, stderr: {}",
            String::from_utf8_lossy(&daemon_output.stderr)
        );

        // Start member to create an active session
        env.command("bm").args(["start", "-t", TEAM_NAME]).run();
        if let Some(pid) = read_pid_from_state(&env.home) {
            guard.set_pid(pid);
        }

        // Read daemon PID from pid file
        let daemon_pid_file = env
            .home
            .join(format!(".botminter/daemon-{}.pid", TEAM_NAME));
        let daemon_pid: u32 = fs::read_to_string(&daemon_pid_file)
            .expect("daemon pid file should exist")
            .trim()
            .parse()
            .expect("daemon pid should be a number");

        // Kill daemon process (simulate crash)
        force_kill(daemon_pid);
        wait_for_exit(daemon_pid, Duration::from_secs(5));

        // Also kill the member process (simulating stale session)
        if let Some(pid) = guard.pid {
            force_kill(pid);
            wait_for_exit(pid, Duration::from_secs(5));
        }

        // Restart daemon
        let restart_output = env
            .command("bm")
            .args([
                "daemon", "start", "--mode", "poll", "--port", &port_str, "--interval", "60", "-t",
                TEAM_NAME,
            ])
            .output();
        assert!(
            restart_output.status.success(),
            "daemon restart should succeed, stderr: {}",
            String::from_utf8_lossy(&restart_output.stderr)
        );

        // Poll for stale sessions to be marked as Failed (recovery runs on daemon start)
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let sessions = get_sessions_json(env);
            if sessions
                .iter()
                .any(|s| s["current_state"].as_str() == Some("Failed"))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for stale session to be marked Failed, got: {:?}",
                sessions
            );
            std::thread::sleep(Duration::from_millis(500));
        }

        // Stop daemon
        let _ = env
            .command("bm")
            .args(["daemon", "stop", "-t", TEAM_NAME])
            .output();
        std::mem::forget(guard);
    }
}

// ── Scenario 8: session_failed_interactive_retention ──────────────────

fn session_failed_interactive_retention_fn(
    _gh_token: String,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let ws = env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR);

        // Ensure workspace exists
        if !ws.exists() {
            let _ = env.command("bm").args(["start", "-t", TEAM_NAME]).output();
            let _ = env
                .command("bm")
                .args(["stop", "--force", "-t", TEAM_NAME])
                .output();
            std::thread::sleep(Duration::from_secs(1));
        }

        // Ensure a daemon is running so the interactive session is tracked.
        // Previous scenarios may have stopped the daemon.
        let port = find_free_port();
        let port_str = port.to_string();
        let daemon_out = env
            .command("bm")
            .args([
                "daemon", "start", "--mode", "poll", "--port", &port_str, "--interval", "60", "-t",
                TEAM_NAME,
            ])
            .output();
        if !daemon_out.status.success() {
            let stderr = String::from_utf8_lossy(&daemon_out.stderr);
            if !stderr.contains("already running") {
                panic!("Failed to start daemon for retention test: {}", stderr);
            }
        }

        // Clean stale artifacts
        for f in [".stub-agent-pid", ".stub-agent-env"] {
            let _ = fs::remove_file(ws.join(f));
        }

        // Run bm chat with failing agent (exit code 1)
        let output = env
            .command("bm")
            .args(["chat", MEMBER_NAME, "-t", TEAM_NAME])
            .env("BM_STUB_AGENT_EXIT_CODE", "1")
            .output();

        // Agent failed — bm chat should propagate the non-zero exit
        assert!(
            !output.status.success(),
            "bm chat with failing agent should propagate non-zero exit"
        );

        // Stub-agent still recorded its PID before failing
        assert!(
            ws.join(".stub-agent-pid").exists(),
            "failing stub-agent should still record PID"
        );

        // Session should be Failed (not cleaned up)
        let sessions = get_sessions_json(env);
        let has_failed_interactive = sessions.iter().any(|s| {
            s["current_state"].as_str() == Some("Failed")
                && s["session_type"].as_str() == Some("interactive")
        });
        assert!(
            has_failed_interactive,
            "failed interactive session should remain in Failed state for inspection, got: {:?}",
            sessions
        );

        // Workspace should be retained for inspection
        assert!(
            ws.exists(),
            "workspace should be retained after failed interactive session"
        );
    }
}

// ── Suite construction ────────────────────────────────────────────────

fn build_suite(gh_org: String, gh_token: String, config: &E2eConfig) -> GithubSuite {
    let app_id = config.app_id.clone();
    let app_client_id = config.app_client_id.clone();
    let app_installation_id = config.app_installation_id.clone();
    let app_private_key_file = config.app_private_key_file.clone();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let repo_full_name = format!("{}/bm-e2e-session-{}", gh_org, timestamp);

    GithubSuite::new_self_managed(SUITE_NAME, &repo_full_name)
        .setup(move |_env: &mut TestEnv| {
            eprintln!("  Session journey: testing session lifecycle model");
        })
        // ── Setup ──────────────────────────────────────────────────
        .case("init_team", init_team_fn(gh_org.clone(), gh_token.clone()))
        .case(
            "hire_member",
            hire_member_fn(
                gh_token.clone(),
                app_id,
                app_client_id,
                app_installation_id,
                app_private_key_file,
            ),
        )
        // ── Session scenarios ──────────────────────────────────────
        .case(
            "session_create_isolated",
            session_create_isolated_fn(gh_token.clone()),
        )
        .case(
            "session_chat_lifecycle",
            session_chat_lifecycle_fn(gh_token.clone()),
        )
        .case(
            "session_concurrent",
            session_concurrent_fn(gh_token.clone()),
        )
        .case(
            "session_stop_graceful",
            session_stop_graceful_fn(gh_token.clone()),
        )
        .case(
            "session_force_stop",
            session_force_stop_fn(gh_token.clone()),
        )
        .case(
            "session_status_dashboard",
            session_status_dashboard_fn(gh_token.clone()),
        )
        .case(
            "session_daemon_restart_recovery",
            session_daemon_restart_recovery_fn(gh_token.clone()),
        )
        .case(
            "session_failed_interactive_retention",
            session_failed_interactive_retention_fn(gh_token.clone()),
        )
        // Groups for progressive mode
        .group(2, 3)  // session_create_isolated + session_chat_lifecycle
        .group(4, 6)  // session_concurrent + session_stop_graceful + session_force_stop
        .group(7, 9)  // session_status_dashboard + daemon_recovery + failed_retention
}

pub fn scenario(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone(), config).build(config)
}

pub fn scenario_progressive(config: &E2eConfig) -> Trial {
    build_suite(config.gh_org.clone(), config.gh_token.clone(), config).build_progressive(config)
}
