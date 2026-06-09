use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::Result;

use crate::config;
use crate::state;
use crate::workspace;

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
    _shutdown: &Arc<AtomicBool>,
    sessions: Option<&SessionsApiState>,
) -> Result<u32> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, Some(team_name))?;
    let team_repo = team.path.join("team");

    let runtime_state = state::load()?;
    let enabled = state::enabled_members(&runtime_state, team_name);
    if enabled.is_empty() {
        tracing::debug!("No enabled members — skipping launch");
        return Ok(0);
    }

    let members_dir = team_repo.join("members");
    let all_members = workspace::list_member_dirs(&members_dir)?;
    let enabled_members: Vec<&str> = all_members.iter()
        .filter(|m| enabled.contains(&format!("{}/{}", team_name, m)))
        .map(|m| m.as_str())
        .collect();

    if enabled_members.is_empty() {
        tracing::debug!("No enabled members match discovered members");
        return Ok(0);
    }

    let mut total_launched = 0u32;

    if let Some(sessions_state) = sessions {
        for member in &enabled_members {
            match sessions_state.start_loop_session_blocking(member) {
                Ok(session_id) => {
                    tracing::info!(member = %member, session_id = %session_id, "Session started");
                    total_launched += 1;
                }
                Err(e) => {
                    if e.contains("already has a live autonomous session") {
                        tracing::debug!(member = %member, "Already running — skipping");
                    } else {
                        tracing::error!(member = %member, error = %e, "Session start failed");
                    }
                }
            }
        }
    } else {
        for member in &enabled_members {
            let result = crate::formation::start_local_members(
                team,
                &cfg,
                &team_repo,
                Some(member),
                true,
                None,
            )?;

            for m in &result.launched {
                tracing::info!(member = %m.name, pid = m.pid, "Launched");
            }
            for m in &result.skipped {
                tracing::info!(member = %m.name, pid = m.pid, "Already running");
            }
            for m in &result.errors {
                tracing::error!(member = %m.name, error = %m.error, "Launch failed");
            }
            total_launched += result.launched.len() as u32;
        }
    }

    Ok(total_launched)
}

/// Launches members one-shot with logging.
pub fn handle_member_launch(
    team_name: &str,
    shutdown: &Arc<AtomicBool>,
    sessions: Option<SessionsApiState>,
) {
    match launch_members_oneshot(team_name, shutdown, sessions.as_ref()) {
        Ok(count) => {
            tracing::info!(count = count, "One-shot run complete");
        }
        Err(e) => {
            tracing::error!(error = %e, "Member launch failed");
        }
    }
}
