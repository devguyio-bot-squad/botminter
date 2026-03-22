use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config;
use crate::workspace;

use super::config::DaemonPaths;
use super::log::daemon_log;

/// Waits for a child process to exit, checking the shutdown flag every 500ms.
///
/// If the shutdown flag is set while the child is still running, sends SIGTERM
/// to the child, waits up to 5 seconds, then escalates to SIGKILL.
///
/// Returns `Some(status)` if the child exited normally, or `None` if it was
/// terminated due to shutdown.
pub fn wait_interruptible(
    child: &mut std::process::Child,
    shutdown: &Arc<AtomicBool>,
) -> Option<std::process::ExitStatus> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if shutdown.load(Ordering::SeqCst) {
                    let pid = child.id();
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                    // Wait up to 5 seconds for child to exit
                    for _ in 0..10 {
                        thread::sleep(Duration::from_millis(500));
                        if let Ok(Some(_)) = child.try_wait() {
                            return None;
                        }
                    }
                    // Escalate to SIGKILL
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(Duration::from_millis(500));
            }
            Err(_) => return None,
        }
    }
}

/// Launches all team members one-shot and waits for them to exit.
/// Returns the number of members launched.
pub fn launch_members_oneshot(
    team_name: &str,
    paths: &DaemonPaths,
    shutdown: &Arc<AtomicBool>,
) -> Result<u32> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, Some(team_name))?;
    let team_repo = team.path.join("team");

    let members_dir = team_repo.join("members");
    if !members_dir.is_dir() {
        daemon_log(paths, "WARN", "No members directory found");
        return Ok(0);
    }

    let member_dirs = workspace::list_member_dirs(&members_dir)?;
    if member_dirs.is_empty() {
        daemon_log(paths, "WARN", "No members found");
        return Ok(0);
    }

    let gh_token = team.credentials.gh_token.as_deref().unwrap_or("");
    let member_token: Option<&str> = None;

    let workzone = &cfg.workzone;
    let team_ws_base = workzone.join(team_name);

    let mut children: Vec<(String, std::process::Child)> = Vec::new();

    for member_dir_name in &member_dirs {
        let ws = workspace::find_workspace(&team_ws_base, member_dir_name);
        let ws = match ws {
            Some(ws) => ws,
            None => {
                daemon_log(
                    paths,
                    "WARN",
                    &format!("{}: no workspace found, skipping", member_dir_name),
                );
                continue;
            }
        };

        match launch_ralph_oneshot(
            &ws,
            gh_token,
            member_token,
            None,
            None,
            paths,
            member_dir_name,
        ) {
            Ok(child) => {
                daemon_log(
                    paths,
                    "INFO",
                    &format!("{}: launched (PID {})", member_dir_name, child.id()),
                );
                children.push((member_dir_name.clone(), child));
            }
            Err(e) => {
                daemon_log(
                    paths,
                    "ERROR",
                    &format!("{}: failed to launch — {}", member_dir_name, e),
                );
            }
        }
    }

    let launched = children.len() as u32;

    for (name, mut child) in children {
        match wait_interruptible(&mut child, shutdown) {
            Some(status) => {
                daemon_log(
                    paths,
                    "INFO",
                    &format!("{}: exited ({})", name, status),
                );
            }
            None => {
                daemon_log(
                    paths,
                    "INFO",
                    &format!("{}: terminated due to shutdown", name),
                );
            }
        }
    }

    Ok(launched)
}

/// Launches members one-shot with logging.
pub fn handle_member_launch(
    team_name: &str,
    paths: &DaemonPaths,
    shutdown: &Arc<AtomicBool>,
) {
    match launch_members_oneshot(team_name, paths, shutdown) {
        Ok(count) => {
            daemon_log(
                paths,
                "INFO",
                &format!("One-shot run complete: {} member(s) processed", count),
            );
        }
        Err(e) => {
            daemon_log(
                paths,
                "ERROR",
                &format!("Member launch failed: {}", e),
            );
        }
    }
}

/// Launches ralph one-shot for a single member.
fn launch_ralph_oneshot(
    workspace: &Path,
    gh_token: &str,
    member_token: Option<&str>,
    bridge_type: Option<&str>,
    service_url: Option<&str>,
    paths: &DaemonPaths,
    member_name: &str,
) -> Result<std::process::Child> {
    let mut cmd = Command::new("ralph");
    cmd.args(["run", "-p", "PROMPT.md"])
        .current_dir(workspace)
        .env("GH_TOKEN", gh_token)
        .env_remove("CLAUDECODE");

    if let Some(token) = member_token {
        match bridge_type {
            Some("rocketchat") => {
                cmd.env("RALPH_ROCKETCHAT_AUTH_TOKEN", token);
                if let Some(url) = service_url {
                    cmd.env("RALPH_ROCKETCHAT_SERVER_URL", url);
                }
            }
            Some("tuwunel") => {
                cmd.env("RALPH_MATRIX_ACCESS_TOKEN", token);
                if let Some(url) = service_url {
                    cmd.env("RALPH_MATRIX_HOMESERVER_URL", url);
                }
            }
            _ => {
                cmd.env("RALPH_TELEGRAM_BOT_TOKEN", token);
            }
        }
    }

    cmd.stdin(std::process::Stdio::null());

    let log_file_path = paths.member_log(member_name)?;
    daemon_log(
        paths,
        "INFO",
        &format!("{}: log file at {}", member_name, log_file_path.display()),
    );
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .with_context(|| {
            format!(
                "Failed to open member log file at {}",
                log_file_path.display()
            )
        })?;
    let log_file_err = log_file
        .try_clone()
        .context("Failed to clone member log file handle")?;
    cmd.stdout(log_file).stderr(log_file_err);

    let child = cmd.spawn().with_context(|| {
        format!("Failed to spawn ralph in {}", workspace.display())
    })?;

    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_interruptible_returns_on_child_exit() {
        let mut child = Command::new("sleep").arg("0.1").spawn().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));

        let result = wait_interruptible(&mut child, &shutdown);
        assert!(result.is_some(), "Should return Some(status) on normal exit");
        assert!(result.unwrap().success(), "sleep 0.1 should exit 0");
    }

    #[test]
    fn wait_interruptible_terminates_on_shutdown() {
        let mut child = Command::new("sleep").arg("999").spawn().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_clone = Arc::clone(&shutdown);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            shutdown_clone.store(true, Ordering::SeqCst);
        });

        let result = wait_interruptible(&mut child, &shutdown);
        assert!(
            result.is_none(),
            "Should return None when shutdown triggered"
        );
        assert!(
            child.try_wait().unwrap().is_some(),
            "Child should be dead after shutdown"
        );
    }
}
