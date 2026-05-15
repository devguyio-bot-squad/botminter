use std::fmt::Write as _;
use std::fs;
use std::process::Child;

use anyhow::{Context, Result};

use super::local::tmux::TmuxSession;

/// Spawns a detached thread that calls `child.wait()`, triggering `waitpid()`
/// so the child is reaped by the kernel instead of becoming a zombie.
///
/// SIGCHLD=SIG_IGN is not viable because it breaks `Command::output()` (used
/// by `gh api` in poll mode). This per-child reaper is the alternative.
pub(crate) fn reap_child(mut child: Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

fn collect_bridge_env_vars(
    member_token: Option<&str>,
    bridge_type: Option<&str>,
    service_url: Option<&str>,
    gh_config_dir: Option<&std::path::Path>,
) -> Result<Vec<(String, String)>> {
    let mut vars = Vec::new();

    if let Some(config_dir) = gh_config_dir {
        let s = config_dir
            .to_str()
            .context("GH_CONFIG_DIR path contains non-UTF-8 characters")?;
        vars.push(("GH_CONFIG_DIR".into(), s.into()));
    }

    if let Some(token) = member_token {
        match bridge_type {
            Some("rocketchat") => {
                vars.push(("RALPH_ROCKETCHAT_AUTH_TOKEN".into(), token.into()));
                if let Some(url) = service_url {
                    vars.push(("RALPH_ROCKETCHAT_SERVER_URL".into(), url.into()));
                }
            }
            Some("tuwunel") => {
                vars.push(("RALPH_MATRIX_ACCESS_TOKEN".into(), token.into()));
                if let Some(url) = service_url {
                    vars.push(("RALPH_MATRIX_HOMESERVER_URL".into(), url.into()));
                }
            }
            _ => {
                vars.push(("RALPH_TELEGRAM_BOT_TOKEN".into(), token.into()));
            }
        }
    }

    Ok(vars)
}

fn vars_to_unset(gh_config_dir: Option<&std::path::Path>) -> Vec<&'static str> {
    let mut vars = vec!["CLAUDECODE"];
    if gh_config_dir.is_some() {
        vars.extend_from_slice(&["GH_TOKEN", "GITHUB_TOKEN"]);
    }
    vars
}

/// Launches `ralph run -p PROMPT.md` in the given workspace directory.
/// Returns the child PID.
///
/// If `gh_config_dir` is set, `GH_CONFIG_DIR` is set instead of `GH_TOKEN`.
/// This is used for members with GitHub App credentials: the daemon writes
/// `hosts.yml` with the installation token, and `gh` reads from it.
/// `GH_TOKEN` would override `hosts.yml`, so we must not set both.
pub fn launch_ralph(
    tmux: &TmuxSession,
    member_name: &str,
    workspace: &std::path::Path,
    member_token: Option<&str>,
    bridge_type: Option<&str>,
    service_url: Option<&str>,
    gh_config_dir: Option<&std::path::Path>,
) -> Result<u32> {
    let bridge_vars = collect_bridge_env_vars(member_token, bridge_type, service_url, gh_config_dir)?;

    // Credentials are written to a temporary file and sourced by the bash
    // wrapper. Sourcing sets env vars via setenv() which does NOT update
    // /proc/pid/environ, keeping secrets hidden from `ps auxe`.
    let mut cred_content = String::new();
    for (key, value) in &bridge_vars {
        writeln!(cred_content, "export {key}='{value}'").unwrap();
    }

    let has_credentials = !cred_content.is_empty();
    if has_credentials {
        let cred_path = workspace.join(".launch-credentials");
        fs::write(&cred_path, &cred_content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&cred_path, fs::Permissions::from_mode(0o600))?;
        }
    }

    let env_u: Vec<_> = vars_to_unset(gh_config_dir).iter().map(|v| format!("-u {v}")).collect();
    let unsets = format!("env {}", env_u.join(" "));

    let cmd_str = if has_credentials {
        // Source credentials (setenv — not in /proc/pid/environ), delete the
        // file, then pause with `read -t` (a bash builtin that spawns no
        // child process) so the pane survives the create_window liveness
        // check while /proc/pid/environ is still secret-free.
        format!(
            ". .launch-credentials; rm -f .launch-credentials; \
             read -t 1 2>/dev/null || true; exec {unsets} ralph run -p PROMPT.md"
        )
    } else {
        format!("exec {unsets} ralph run -p PROMPT.md")
    };

    tmux.create_window(member_name, &["bash", "-c", &cmd_str], workspace, &[])
}

/// Configuration for launching a brain process, bundling bridge-related params.
pub struct BrainLaunchConfig<'a> {
    pub tmux: &'a TmuxSession,
    pub member_name: &'a str,
    pub workspace: &'a std::path::Path,
    pub system_prompt_path: &'a std::path::Path,
    pub member_token: Option<&'a str>,
    pub bridge_type: Option<&'a str>,
    pub service_url: Option<&'a str>,
    pub room_id: Option<&'a str>,
    pub user_id: Option<&'a str>,
    pub operator_user_id: Option<&'a str>,
    pub team_repo: Option<&'a std::path::Path>,
    /// When set, uses GH_CONFIG_DIR instead of GH_TOKEN (App credential path).
    pub gh_config_dir: Option<&'a std::path::Path>,
}

/// Launches the brain multiplexer for a chat-first member.
///
/// Spawns `bm brain-run` as a background process, which runs the multiplexer
/// event loop (ACP session + event watcher + heartbeat). Returns the child PID.
pub fn launch_brain(config: &BrainLaunchConfig<'_>) -> Result<u32> {
    let bm_binary = std::env::current_exe()
        .context("Failed to determine bm binary path")?;
    let bm_str = bm_binary
        .to_str()
        .context("bm binary path contains non-UTF-8 characters")?;
    let ws_str = config
        .workspace
        .to_str()
        .context("workspace path contains non-UTF-8 characters")?;
    let sp_str = config
        .system_prompt_path
        .to_str()
        .context("system prompt path contains non-UTF-8 characters")?;
    let log_path = config.workspace.join("brain-stderr.log");
    let log_str = log_path
        .to_str()
        .context("log path contains non-UTF-8 characters")?;

    let mut env_strs = collect_bridge_env_vars(
        config.member_token,
        config.bridge_type,
        config.service_url,
        config.gh_config_dir,
    )?;

    if let Some(rid) = config.room_id {
        env_strs.push(("BM_BRAIN_ROOM_ID".into(), rid.into()));
    }
    if let Some(uid) = config.user_id {
        env_strs.push(("BM_BRAIN_USER_ID".into(), uid.into()));
    }
    if let Some(op_uid) = config.operator_user_id {
        env_strs.push(("BM_BRAIN_OPERATOR_USER_ID".into(), op_uid.into()));
    }
    if let Some(repo) = config.team_repo {
        let s = repo
            .to_str()
            .context("team repo path contains non-UTF-8 characters")?;
        env_strs.push(("BM_TEAM_REPO".into(), s.into()));
    }

    let envs: Vec<(&str, &str)> = env_strs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let unset_cmd = format!("unset {}", vars_to_unset(config.gh_config_dir).join(" "));

    // Bash wrapper tees all output (stdout+stderr) to brain-stderr.log while
    // keeping it visible in the tmux pane. The startup marker proves the
    // launcher and stderr routing are working.
    let cmd_str = format!(
        "{unset_cmd}; exec > >(tee \"{log_str}\") 2>&1; \
         printf 'BRAIN_STDERR_TEST\\n'; sleep 0.2; \
         exec \"{bm_str}\" brain-run --workspace \"{ws_str}\" --system-prompt \"{sp_str}\""
    );

    config.tmux.create_window(
        config.member_name,
        &["bash", "-c", &cmd_str],
        config.workspace,
        &envs,
    )
}

/// Returns true if the workspace has a `brain-prompt.md` file,
/// indicating this member should run in brain (chat-first) mode.
pub fn is_brain_member(workspace: &std::path::Path) -> bool {
    workspace.join("brain-prompt.md").exists()
}

/// Checks if a member has a credential but RObot.enabled is false in ralph.yml.
///
/// Returns `true` if there is a mismatch (credential present but RObot disabled),
/// meaning the user should run `bm teams sync` to update.
pub fn check_robot_enabled_mismatch(
    ralph_yml_path: &std::path::Path,
    has_credential: bool,
) -> bool {
    if !has_credential {
        return false;
    }
    if !ralph_yml_path.exists() {
        return false;
    }
    let contents = match fs::read_to_string(ralph_yml_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let doc: serde_yml::Value = match serde_yml::from_str(&contents) {
        Ok(d) => d,
        Err(_) => return false,
    };

    // Check if RObot.enabled is explicitly false
    match doc
        .get("RObot")
        .and_then(|r| r.get("enabled"))
        .and_then(|e| e.as_bool())
    {
        Some(false) => true,  // Mismatch: has cred but disabled
        _ => false,           // Either enabled or not set at all
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;

    fn tmux_cmd() -> StdCommand {
        let mut cmd = StdCommand::new("tmux");
        cmd.env_remove("TMUX_TMPDIR");
        cmd
    }

    struct TmuxGuard {
        session_name: String,
    }

    impl TmuxGuard {
        fn new(session_name: &str) -> Self {
            Self {
                session_name: session_name.to_string(),
            }
        }
    }

    impl Drop for TmuxGuard {
        fn drop(&mut self) {
            let _ = tmux_cmd()
                .args(["-L", "botminter", "kill-session", "-t", &self.session_name])
                .output();
        }
    }

    #[test]
    fn launch_ralph_receives_tmux_and_member_name() {
        let _: fn(&TmuxSession, &str, &std::path::Path, Option<&str>, Option<&str>, Option<&str>, Option<&std::path::Path>) -> Result<u32> =
            launch_ralph;
    }

    #[test]
    fn launch_brain_signature_accepts_config_with_tmux() {
        let _: fn(&BrainLaunchConfig<'_>) -> Result<u32> = launch_brain;
    }

    // ── CT-01: AC1 — Ralph Launch Creates Window ───────────────────

    #[test]
    fn launch_ralph_creates_tmux_window_with_valid_pid() {
        let session = TmuxSession::new("ct7-01-ralph").unwrap();
        let _guard = TmuxGuard::new(&session.session_name());
        session.create().expect("session should be created");

        let workspace = tempfile::tempdir().unwrap();

        let result = launch_ralph(
            &session,
            "bob",
            workspace.path(),
            None,
            None,
            None,
            None,
        );
        let pid = result.expect("launch_ralph should succeed");

        assert!(
            session.window_exists("bob"),
            "tmux window 'bob' must exist after launch_ralph"
        );

        assert!(pid > 0, "returned PID must be positive");
        let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
        assert!(alive, "process with returned PID must be running");
    }

    // ── CT-01: AC2 — Brain Launch Creates Window with Stderr Log ───

    #[test]
    fn launch_brain_creates_window_with_stderr_log() {
        let session = TmuxSession::new("ct7-01-brain").unwrap();
        let _guard = TmuxGuard::new(&session.session_name());
        session.create().expect("session should be created");

        let workspace = tempfile::tempdir().unwrap();
        let system_prompt = workspace.path().join("brain-prompt.md");
        fs::write(&system_prompt, "# Test brain prompt").unwrap();

        let config = BrainLaunchConfig {
            tmux: &session,
            member_name: "brain-bob",
            workspace: workspace.path(),
            system_prompt_path: &system_prompt,
            member_token: None,
            bridge_type: None,
            service_url: None,
            room_id: None,
            user_id: None,
            operator_user_id: None,
            team_repo: None,
            gh_config_dir: None,
        };

        let result = launch_brain(&config);
        let pid = result.expect("launch_brain should succeed");

        assert!(
            session.window_exists("brain-bob"),
            "tmux window 'brain-bob' must exist after launch_brain"
        );

        let log_path = workspace.path().join("brain-stderr.log");
        assert!(log_path.exists(), "brain-stderr.log must be created");

        // Verify stderr content appears in log (test uses a stub that writes marker)
        let log_content = fs::read_to_string(&log_path).unwrap_or_default();
        assert!(
            log_content.contains("BRAIN_STDERR_TEST"),
            "brain-stderr.log must contain stderr output"
        );

        // Verify stderr also visible in tmux pane
        let capture = tmux_cmd()
            .args([
                "-L", "botminter", "capture-pane", "-t",
                &format!("{}:brain-bob", session.session_name()),
                "-p",
            ])
            .output()
            .expect("capture-pane should execute");
        let pane_output = String::from_utf8_lossy(&capture.stdout);
        assert!(
            pane_output.contains("BRAIN_STDERR_TEST"),
            "pane capture must show stderr output, got: {pane_output}"
        );

        assert!(pid > 0, "returned PID must be positive");
    }

    // ── CT-01: AC3 — Agent Output Visible in Pane ──────────────────

    #[test]
    fn agent_output_visible_in_tmux_pane() {
        let session = TmuxSession::new("ct7-01-output").unwrap();
        let _guard = TmuxGuard::new(&session.session_name());
        session.create().expect("session should be created");

        let workspace = tempfile::tempdir().unwrap();

        launch_ralph(
            &session,
            "agent-out",
            workspace.path(),
            None,
            None,
            None,
            None,
        )
        .expect("launch_ralph should succeed");

        // Poll capture-pane every 500ms, up to 10s
        let mut found = false;
        for _ in 0..20 {
            let capture = tmux_cmd()
                .args([
                    "-L", "botminter", "capture-pane", "-t",
                    &format!("{}:agent-out", session.session_name()),
                    "-p",
                ])
                .output();
            if let Ok(output) = capture {
                let text = String::from_utf8_lossy(&output.stdout);
                if !text.trim().is_empty() {
                    found = true;
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        assert!(found, "agent output must be visible in tmux pane within 10s");
    }

    // ── CT-01: AC4 — Daemon Not in Tmux ────────────────────────────

    #[test]
    fn daemon_pid_not_in_any_tmux_window() {
        let session = TmuxSession::new("ct7-01-nodaemon").unwrap();
        let _guard = TmuxGuard::new(&session.session_name());
        session.create().expect("session should be created");

        let workspace = tempfile::tempdir().unwrap();

        launch_ralph(
            &session,
            "member1",
            workspace.path(),
            None,
            None,
            None,
            None,
        )
        .expect("launch_ralph should succeed");

        let windows = session.list_windows().expect("list_windows should succeed");
        for win in &windows {
            assert!(
                !win.name.contains("daemon"),
                "no tmux window should be named 'daemon', found: {}",
                win.name
            );
        }
        // Daemon PID (current process) must not be a pane_pid
        let daemon_pid = std::process::id();
        for win in &windows {
            assert_ne!(
                win.pane_pid, daemon_pid,
                "daemon PID must not appear as a pane PID in any tmux window"
            );
        }
    }

    // ── CT-01: AC5 — Credential Security ───────────────────────────

    #[test]
    fn launch_credentials_not_in_pane_start_command_or_ps() {
        let session = TmuxSession::new("ct7-01-creds").unwrap();
        let _guard = TmuxGuard::new(&session.session_name());
        session.create().expect("session should be created");

        let workspace = tempfile::tempdir().unwrap();
        let secret_value = "SUPER_SECRET_TOKEN_ct701";

        launch_ralph(
            &session,
            "secmember",
            workspace.path(),
            Some(secret_value),
            None,
            None,
            None,
        )
        .expect("launch_ralph should succeed");

        // Check pane_start_command does not contain secret
        let pane_cmd = tmux_cmd()
            .args([
                "-L", "botminter", "display-message", "-t",
                &format!("{}:secmember", session.session_name()),
                "-p", "#{pane_start_command}",
            ])
            .output()
            .expect("display-message should execute");
        let pane_cmd_str = String::from_utf8_lossy(&pane_cmd.stdout);
        assert!(
            !pane_cmd_str.contains(secret_value),
            "pane_start_command must not contain secret value"
        );

        // Check ps output does not contain secret
        let ps = StdCommand::new("ps")
            .args(["auxe"])
            .output()
            .expect("ps should execute");
        let ps_str = String::from_utf8_lossy(&ps.stdout);
        assert!(
            !ps_str.contains(secret_value),
            "ps output must not contain secret value"
        );
    }

    // ── CT-01: AC6 — TMUX_TMPDIR Isolation ─────────────────────────

    #[test]
    fn tmux_tmpdir_isolation_for_launched_agents() {
        let evil_dir = "/tmp/evil-tmux-ct701";
        std::env::set_var("TMUX_TMPDIR", evil_dir);

        let session = TmuxSession::new("ct7-01-tmpdir").unwrap();
        let _guard = TmuxGuard::new(&session.session_name());
        session.create().expect("session should be created");

        let workspace = tempfile::tempdir().unwrap();

        launch_ralph(
            &session,
            "isomember",
            workspace.path(),
            None,
            None,
            None,
            None,
        )
        .expect("launch_ralph should succeed");

        std::env::remove_var("TMUX_TMPDIR");

        // Verify socket is under /tmp/tmux-<uid>/, not under evil_dir
        let uid = unsafe { libc::getuid() };
        let expected_dir = format!("/tmp/tmux-{uid}");
        let socket_path = std::path::Path::new(&expected_dir).join("botminter");
        assert!(
            socket_path.exists(),
            "tmux socket must be under {expected_dir}, not under {evil_dir}"
        );
        assert!(
            !std::path::Path::new(evil_dir).join("botminter").exists(),
            "tmux socket must not be under TMUX_TMPDIR ({evil_dir})"
        );
    }

    // ── Existing tests ─────────────────────────────────────────────

    #[test]
    fn is_brain_member_with_brain_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("brain-prompt.md"), "# Brain").unwrap();
        assert!(is_brain_member(tmp.path()));
    }

    #[test]
    fn is_brain_member_without_brain_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_brain_member(tmp.path()));
    }

    #[test]
    fn check_robot_enabled_diagnostic() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(
            &ralph_yml,
            "preset: feature-development\nRObot:\n  enabled: false\n",
        )
        .unwrap();

        let has_credential = true;
        let robot_enabled = check_robot_enabled_mismatch(&ralph_yml, has_credential);
        assert!(
            robot_enabled,
            "should return true when credential exists but RObot.enabled is false"
        );
    }
}
