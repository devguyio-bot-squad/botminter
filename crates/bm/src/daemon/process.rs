use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::Result;

use crate::config;

use super::config::DaemonPaths;
use super::log::daemon_log;

/// Launches all team members using the single entry point (`start_local_members`).
///
/// This is called by the daemon poll loop and webhook handler. It delegates to
/// `formation::start_local_members()` which handles App credential resolution,
/// bridge tokens, brain mode detection, and state tracking.
///
/// Returns the number of members launched.
pub fn launch_members_oneshot(
    team_name: &str,
    paths: &DaemonPaths,
    _shutdown: &Arc<AtomicBool>,
) -> Result<u32> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, Some(team_name))?;
    let team_repo = team.path.join("team");

    // The daemon IS the formation's internal runtime — it calls start_local_members
    // directly, not formation.start_members() (which would HTTP-call back into this
    // daemon, creating a circular loop).
    let result = crate::formation::start_local_members(
        team,
        &cfg,
        &team_repo,
        None,   // all members
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

    Ok(result.launched.len() as u32)
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
