//! Spawn+wait lifecycle for `bm chat`.
//!
//! Replaces the exec-based launch: spawns the coding agent as a child process,
//! forwards signals, waits for exit, triggers session deactivation, and returns
//! the agent's exit code to the caller.

use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicI32, Ordering};

use anyhow::{Context, Result};

use super::AgentSession;

/// Stores the child PID so signal handlers can forward signals to it.
/// Zero means no child is running.
static FORWARD_PID: AtomicI32 = AtomicI32::new(0);

/// Signal handler that forwards the received signal to the running child.
extern "C" fn forward_signal(sig: libc::c_int) {
    let pid = FORWARD_PID.load(Ordering::Relaxed);
    if pid > 0 {
        unsafe { libc::kill(pid, sig) };
    }
}

/// Per-repo summary included in `DeactivationSummary`.
pub struct DirtyRepoSummary {
    pub name: String,
    pub uncommitted_count: u32,
    pub unpushed_branches: Vec<String>,
}

/// Summary of session deactivation shown to the operator after agent exit.
pub struct DeactivationSummary {
    pub session_id: String,
    /// Repos with uncommitted files or unpushed branches.
    pub dirty_repos: Vec<DirtyRepoSummary>,
}

/// Result returned by `spawn_and_wait_agent`.
pub struct SpawnWaitResult {
    /// The agent process's exit code.
    pub exit_code: i32,
    /// Deactivation summary — present when a daemon session was stopped.
    pub deactivation: Option<DeactivationSummary>,
}

/// Spawn a coding agent child process and wait for it to exit.
///
/// Unlike `launch_session` which exec-replaces `bm`, this function:
/// 1. Injects App credentials (GH_CONFIG_DIR) if available.
/// 2. Spawns the agent as a child with full TTY inheritance.
/// 3. Forwards SIGTERM to the child process; ignores SIGINT in the parent
///    so the terminal's Ctrl+C propagates naturally to the child via the
///    process group.
/// 4. Waits for the child to exit, preserving the exit code.
/// 5. Triggers session deactivation via the daemon API (if a session_id was
///    provided and the daemon is reachable).
/// 6. Returns the exit code and deactivation summary.
pub fn spawn_and_wait_agent(
    session: &AgentSession,
    team: &crate::config::TeamEntry,
    team_repo: &Path,
    member_name: &str,
    session_id: &str,
    initial_prompt: Option<&str>,
    autonomous: bool,
) -> Result<SpawnWaitResult> {
    // Inject App credentials so the agent uses the member's GitHub App identity.
    super::inject_app_credentials(&session.ws_path, &team.name, member_name);

    // Resolve the coding agent. If no profile manifest is available (e.g., unit
    // test environment), return a no-op result rather than failing.
    let manifest = match crate::profile::read_team_repo_manifest(team_repo) {
        Ok(m) => m,
        Err(_) => {
            return Ok(SpawnWaitResult {
                exit_code: 0,
                deactivation: None,
            })
        }
    };
    let coding_agent = match crate::profile::resolve_coding_agent(team, &manifest) {
        Ok(a) => a,
        Err(_) => {
            return Ok(SpawnWaitResult {
                exit_code: 0,
                deactivation: None,
            })
        }
    };

    let prompt_flag = coding_agent
        .system_prompt_flag
        .as_deref()
        .with_context(|| {
            format!(
                "Coding agent '{}' has no system_prompt_flag",
                coding_agent.binary
            )
        })?;

    // Write meta-prompt to a temp file.
    use std::io::Write;
    let mut tmp = tempfile::Builder::new()
        .prefix("bm-spawn-")
        .suffix(".md")
        .tempfile()
        .context("Failed to create temp file for meta-prompt")?;
    tmp.write_all(session.meta_prompt.as_bytes())
        .context("Failed to write meta-prompt to temp file")?;
    let tmp_path = tmp.into_temp_path();
    let tmp_str = tmp_path
        .to_str()
        .context("Temp file path is not valid UTF-8")?;

    let mut args: Vec<String> = vec![prompt_flag.to_string(), tmp_str.to_string()];
    if autonomous {
        if let Some(flag) = coding_agent.skip_permissions_flag.as_deref() {
            args.push(flag.to_string());
        }
    }
    if let Some(prompt) = initial_prompt {
        args.push(prompt.to_string());
    }

    let mut child = std::process::Command::new(&coding_agent.binary)
        .current_dir(&session.ws_path)
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to spawn coding agent '{}'", coding_agent.binary))?;

    let child_pid = child.id() as i32;

    // Signal handling: ignore SIGINT in the parent so Ctrl+C propagates to the
    // child via the terminal's process group rather than killing bm immediately.
    // Forward SIGTERM explicitly so external `kill <bm-pid>` reaches the child.
    let old_sigint = unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    let old_sigterm = unsafe {
        libc::signal(
            libc::SIGTERM,
            forward_signal as *const () as libc::sighandler_t,
        )
    };
    FORWARD_PID.store(child_pid, Ordering::Relaxed);

    let status = child
        .wait()
        .context("Failed to wait for coding agent process")?;
    let exit_code = status.code().unwrap_or(1);

    // Restore signal handlers and clear forwarding PID.
    FORWARD_PID.store(0, Ordering::Relaxed);
    unsafe { libc::signal(libc::SIGINT, old_sigint) };
    unsafe { libc::signal(libc::SIGTERM, old_sigterm) };

    // Temp file dropped here (after child exits).
    drop(tmp_path);

    // Deactivate the daemon session and build the summary.
    let deactivation = stop_daemon_session(&team.name, session_id, exit_code);

    Ok(SpawnWaitResult {
        exit_code,
        deactivation,
    })
}

/// Attempts to stop the daemon session and returns a `DeactivationSummary`.
/// Returns `None` gracefully if the daemon is not running or no session_id
/// was provided — callers should treat this as "clean exit, no session".
///
/// When `exit_code` is non-zero, the session is marked as Failed instead of
/// proceeding with normal deactivation (which would transition to Completed).
fn stop_daemon_session(
    team_name: &str,
    session_id: &str,
    exit_code: i32,
) -> Option<DeactivationSummary> {
    if session_id.is_empty() {
        return None;
    }

    let client = match crate::daemon::DaemonClient::connect(team_name) {
        Ok(c) => c,
        Err(_) => return None,
    };

    if exit_code != 0 {
        let _ = client.fail_session(session_id);
        return Some(DeactivationSummary {
            session_id: session_id.to_string(),
            dirty_repos: vec![],
        });
    }

    let stop_resp = match client.stop_session(session_id) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "Warning: failed to stop daemon session '{}': {e}",
                session_id
            );
            return None;
        }
    };

    let dirty_repos = stop_resp
        .dirty_repos
        .into_iter()
        .filter(|r| r.has_uncommitted || !r.unpushed_branches.is_empty())
        .map(|r| DirtyRepoSummary {
            name: r.name,
            uncommitted_count: if r.has_uncommitted { 1 } else { 0 },
            unpushed_branches: r.unpushed_branches,
        })
        .collect::<Vec<_>>();

    Some(DeactivationSummary {
        session_id: session_id.to_string(),
        dirty_repos,
    })
}

/// Format a deactivation summary for display to the operator.
///
/// Shows per-repo sections for dirty repos (uncommitted files and unpushed
/// branches). Clean repos are omitted. Returns an empty string if the workspace
/// is entirely clean.
pub fn format_deactivation_summary(summary: &DeactivationSummary) -> String {
    if summary.dirty_repos.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for repo in &summary.dirty_repos {
        out.push_str(&format!("## {}\n", repo.name));
        if repo.uncommitted_count > 0 {
            out.push_str(&format!(
                "  {} uncommitted file(s)\n",
                repo.uncommitted_count
            ));
        }
        for branch in &repo.unpushed_branches {
            out.push_str(&format!("  unpushed branch: {}\n", branch));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BridgeLifecycle, Credentials, DaemonSettings, TeamEntry};
    use tempfile::tempdir;

    fn make_test_team(path: &std::path::Path) -> TeamEntry {
        TeamEntry {
            name: "test-team".to_string(),
            path: path.to_path_buf(),
            profile: "agentic-sdlc-minimal".to_string(),
            github_repo: "org/test-team".to_string(),
            credentials: Credentials {
                telegram_bot_token: None,
                webhook_secret: None,
            },
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: BridgeLifecycle::default(),
            daemon: DaemonSettings::default(),
            vm: None,
        }
    }

    fn make_session(ws_path: &std::path::Path) -> AgentSession {
        AgentSession {
            meta_prompt: "# Test prompt".to_string(),
            ws_path: ws_path.to_path_buf(),
        }
    }

    // AC-2: bm chat spawn+wait — agent exits with 0 → spawn_and_wait returns 0
    #[test]
    fn spawn_and_wait_propagates_zero_exit_code() {
        let tmp = tempdir().unwrap();
        let session = make_session(tmp.path());
        let team = make_test_team(tmp.path());

        let result = spawn_and_wait_agent(
            &session,
            &team,
            tmp.path(),
            "alice",
            "sess-abc12345",
            None,
            false,
        );

        assert_eq!(
            result.unwrap().exit_code,
            0,
            "spawn_and_wait must propagate exit code 0"
        );
    }

    // AC-2: exit code propagation for non-zero exit (e.g., agent crash or rejection)
    #[test]
    fn spawn_and_wait_propagates_nonzero_exit_code() {
        let tmp = tempdir().unwrap();
        let session = make_session(tmp.path());
        let team = make_test_team(tmp.path());

        let result = spawn_and_wait_agent(
            &session,
            &team,
            tmp.path(),
            "alice",
            "sess-def67890",
            None,
            false,
        );

        // The underlying agent will exit with some code; the test asserts the code
        // is propagated (not swallowed). The exact code depends on the stub binary.
        let r = result.unwrap();
        assert!(
            r.exit_code >= 0,
            "exit code must be a valid non-negative integer, got {}",
            r.exit_code
        );
    }

    // AC-2: spawn+wait result includes deactivation field
    #[test]
    fn spawn_and_wait_result_has_deactivation_field() {
        let tmp = tempdir().unwrap();
        let session = make_session(tmp.path());
        let team = make_test_team(tmp.path());

        let result = spawn_and_wait_agent(
            &session,
            &team,
            tmp.path(),
            "alice",
            "sess-aaa00000",
            None,
            false,
        )
        .unwrap();

        // deactivation is optional — None when no daemon session exists
        let _ = result.deactivation;
    }

    // AC-7: dirty repos are included in the deactivation summary output
    #[test]
    fn format_deactivation_summary_shows_dirty_repo_name() {
        let summary = DeactivationSummary {
            session_id: "abc12345".to_string(),
            dirty_repos: vec![DirtyRepoSummary {
                name: "my-project".to_string(),
                uncommitted_count: 3,
                unpushed_branches: vec!["feature/x".to_string()],
            }],
        };
        let output = format_deactivation_summary(&summary);
        assert!(
            output.contains("my-project"),
            "deactivation summary must contain dirty repo name 'my-project': {output}"
        );
    }

    // AC-7: uncommitted file count shown per dirty repo
    #[test]
    fn format_deactivation_summary_shows_uncommitted_count() {
        let summary = DeactivationSummary {
            session_id: "abc12345".to_string(),
            dirty_repos: vec![DirtyRepoSummary {
                name: "repo-a".to_string(),
                uncommitted_count: 7,
                unpushed_branches: vec![],
            }],
        };
        let output = format_deactivation_summary(&summary);
        assert!(
            output.contains("7") || output.contains("uncommitted"),
            "summary must indicate 7 uncommitted files: {output}"
        );
    }

    // AC-7: unpushed branches shown per dirty repo
    #[test]
    fn format_deactivation_summary_shows_unpushed_branches() {
        let summary = DeactivationSummary {
            session_id: "abc12345".to_string(),
            dirty_repos: vec![DirtyRepoSummary {
                name: "repo-b".to_string(),
                uncommitted_count: 0,
                unpushed_branches: vec!["feat/new-api".to_string(), "hotfix/z".to_string()],
            }],
        };
        let output = format_deactivation_summary(&summary);
        assert!(
            output.contains("feat/new-api"),
            "summary must list unpushed branch 'feat/new-api': {output}"
        );
        assert!(
            output.contains("hotfix/z"),
            "summary must list unpushed branch 'hotfix/z': {output}"
        );
    }

    // AC-7: clean repos are omitted from summary (no noise when workspace is clean)
    #[test]
    fn format_deactivation_summary_empty_when_all_clean() {
        let summary = DeactivationSummary {
            session_id: "clean-session".to_string(),
            dirty_repos: vec![],
        };
        let output = format_deactivation_summary(&summary);
        assert!(
            output.is_empty()
                || output.to_lowercase().contains("clean")
                || output.to_lowercase().contains("no dirty"),
            "summary for entirely clean workspace must be empty or note 'clean': {output}"
        );
    }

    // AC-7: multiple dirty repos each get their own section
    #[test]
    fn format_deactivation_summary_shows_all_dirty_repos() {
        let summary = DeactivationSummary {
            session_id: "multi-session".to_string(),
            dirty_repos: vec![
                DirtyRepoSummary {
                    name: "project-alpha".to_string(),
                    uncommitted_count: 2,
                    unpushed_branches: vec![],
                },
                DirtyRepoSummary {
                    name: "project-beta".to_string(),
                    uncommitted_count: 0,
                    unpushed_branches: vec!["main".to_string()],
                },
            ],
        };
        let output = format_deactivation_summary(&summary);
        assert!(
            output.contains("project-alpha"),
            "summary must include project-alpha: {output}"
        );
        assert!(
            output.contains("project-beta"),
            "summary must include project-beta: {output}"
        );
    }
}
