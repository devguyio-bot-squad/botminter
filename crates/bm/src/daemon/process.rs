use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::Result;

use crate::config;
use crate::state;
use crate::workspace;

use super::config::DaemonPaths;
use super::log::daemon_log;
use super::sessions_api::SessionsApiState;

/// Launches enabled team members via the sessions API (when available) or the
/// legacy formation path. The sessions API path creates ephemeral session
/// workspaces under `~/.botminter/sessions/<team>/<member>/<id>/`.
///
/// Called by the daemon poll loop and webhook handler. Only members in the
/// `enabled` set (via `bm enable`) are eligible for event-driven launch.
/// `bm start` bypasses this check — it always starts regardless of enable state.
///
/// Returns the number of members launched.
pub fn launch_members_oneshot(
    team_name: &str,
    paths: &DaemonPaths,
    _shutdown: &Arc<AtomicBool>,
    sessions: Option<&SessionsApiState>,
) -> Result<u32> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, Some(team_name))?;
    let team_repo = team.path.join("team");

    // Only launch members that are in the enabled set.
    let runtime_state = state::load()?;
    let enabled = state::enabled_members(&runtime_state, team_name);
    if enabled.is_empty() {
        daemon_log(paths, "DEBUG", "No enabled members — skipping launch");
        return Ok(0);
    }

    // Discover all members, then filter to enabled ones.
    let members_dir = team_repo.join("members");
    let all_members = workspace::list_member_dirs(&members_dir)?;
    let enabled_members: Vec<&str> = all_members.iter()
        .filter(|m| enabled.contains(&format!("{}/{}", team_name, m)))
        .map(|m| m.as_str())
        .collect();

    if enabled_members.is_empty() {
        daemon_log(paths, "DEBUG", "No enabled members match discovered members");
        return Ok(0);
    }

    let mut total_launched = 0u32;

    if let Some(sessions_state) = sessions {
        // Sessions API path: creates an ephemeral session workspace for each member.
        for member in &enabled_members {
            match sessions_state.start_loop_session_blocking(member) {
                Ok(session_id) => {
                    daemon_log(
                        paths,
                        "INFO",
                        &format!("{}: session {} started", member, session_id),
                    );
                    total_launched += 1;
                }
                Err(e) => {
                    // "already running" is not an error — it is expected when the
                    // daemon fires multiple poll ticks while a session is live.
                    if e.contains("already has a live autonomous session") {
                        daemon_log(
                            paths,
                            "DEBUG",
                            &format!("{}: already running — skipping", member),
                        );
                    } else {
                        daemon_log(
                            paths,
                            "ERROR",
                            &format!("{}: session start failed: {}", member, e),
                        );
                    }
                }
            }
        }
    } else {
        // Legacy formation path (no session workspace created).
        for member in &enabled_members {
            let result = crate::formation::start_local_members(
                team,
                &cfg,
                &team_repo,
                Some(member),
                true,   // no_bridge — daemon doesn't manage bridge lifecycle
                None,   // no formation override
            )?;

            for m in &result.launched {
                daemon_log(paths, "INFO", &format!("{}: launched (PID {})", m.name, m.pid));
            }
            for m in &result.skipped {
                daemon_log(paths, "INFO", &format!("{}: already running (PID {})", m.name, m.pid));
            }
            for m in &result.errors {
                daemon_log(paths, "ERROR", &format!("{}: {}", m.name, m.error));
            }
            total_launched += result.launched.len() as u32;
        }
    }

    Ok(total_launched)
}

/// Launches members one-shot with logging.
pub fn handle_member_launch(
    team_name: &str,
    paths: &DaemonPaths,
    shutdown: &Arc<AtomicBool>,
    sessions: Option<SessionsApiState>,
) {
    match launch_members_oneshot(team_name, paths, shutdown, sessions.as_ref()) {
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
