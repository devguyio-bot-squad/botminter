pub mod attach;
pub mod bootstrap;
pub mod brain_run;
pub mod bridge;
pub mod chat;
pub mod completions;
pub mod credentials;
pub mod daemon;
pub mod debug;
pub mod disable;
pub mod enable;
pub mod env;
pub mod fire;
pub mod hire;
pub mod init;
pub mod knowledge;
pub mod meeting;
pub mod members;
pub mod minty;
pub mod profiles;
pub mod profiles_init;
pub mod projects;
pub mod roles;
pub mod session;
pub mod start;
pub mod status;
pub mod stop;
pub mod teams;

use std::path::Path;

use anyhow::Result;

use crate::profile::{self, ProfileInitResult};

/// Ensures profiles are initialized, displaying appropriate messages.
/// Used as a guard at the top of commands that require profiles.
///
/// Returns `Ok(())` if profiles are available (existing, newly initialized, or updated).
/// Returns `Err` if the user declined setup or an error occurred.
pub(crate) fn ensure_profiles(config_warning: bool) -> Result<()> {
    let result = profile::ensure_profiles_initialized()?;
    match result {
        ProfileInitResult::AlreadyCurrent => {}
        ProfileInitResult::Initialized { count, path } => {
            eprintln!("Initialized {} profiles in {}", count, path.display());
        }
        ProfileInitResult::Updated { count, path, .. } => {
            eprintln!("Updated {} profiles in {}", count, path.display());
        }
        ProfileInitResult::Declined => {
            eprintln!("Keeping existing profiles");
        }
        ProfileInitResult::SetupDeclined => {
            eprintln!("Run `bm profiles init` to set up profiles.");
            std::process::exit(0);
        }
    }

    // Check config file permissions (not related to daemon)
    if config_warning {
        if let Ok(path) = crate::config::config_path() {
            if let Some(warning) = crate::config::check_permissions_warning(&path) {
                eprintln!("Warning: {}", warning);
            }
        }
    }

    Ok(())
}

/// Ensures a daemon is running for the given team, starting one if needed.
///
/// Used by `bm start` and `bm chat` to guarantee session lifecycle tracking.
/// Non-fatal: returns Ok(()) even if the daemon cannot be started (e.g., schema
/// mismatch), allowing the command to proceed without session tracking.
pub(crate) fn ensure_daemon_running(team_name: &str, team_repo: &Path) -> Result<()> {
    match crate::daemon::query_status(team_name) {
        Ok(crate::daemon::DaemonStatusInfo::Running { .. }) => Ok(()),
        _ => {
            crate::daemon::start_daemon(team_name, team_repo, "poll", 0, 60, "127.0.0.1")?;
            Ok(())
        }
    }
}
