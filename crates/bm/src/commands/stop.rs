use anyhow::{bail, Result};

use crate::config;
use crate::daemon;
use crate::formation;
use crate::team::Team;

/// Handles `bm stop [member] [-t team] [--force] [--bridge] [--all]`.
pub fn run(team_flag: Option<&str>, force: bool, member_filter: Option<&str>, bridge_flag: bool, stop_all: bool) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;

    // Resolve Team → Formation → stop_members
    let local_formation = formation::create_local_formation(&team.name)?;
    let team_api = Team::new(team, local_formation);

    // --all implies --bridge
    let effective_bridge_flag = bridge_flag || stop_all;

    let result = team_api.stop(&cfg, member_filter, force, effective_bridge_flag, stop_all)?;

    // Display: specific member not running
    if let (true, Some(target)) = (result.no_members_running, member_filter) {
        println!(
            "Member '{}' is not running for team '{}'.",
            target, team.name
        );
        return Ok(());
    }

    // Display: no members running at all
    if result.no_members_running {
        println!("No members running for team '{}'.", team.name);
    } else {
        // Display stop outcomes
        for m in &result.stopped {
            if m.already_exited {
                eprintln!("{}... already exited", m.name);
            } else if m.forced {
                eprintln!("Stopping {} (force)... done", m.name);
            } else {
                eprintln!("Stopping {}... done", m.name);
            }
        }
        for m in &result.errors {
            eprintln!("Stopping {}... failed: {}", m.name, m.error);
        }

        println!(
            "\nStopped {} member(s), {} error(s).",
            result.stopped.len(),
            result.errors.len()
        );

        if !result.errors.is_empty() {
            bail!(
                "Some members could not be stopped gracefully. \
                 Use `bm stop -f` to force-kill."
            );
        }
    }

    // Bridge lifecycle — command-layer concern per ADR-0008.
    // The daemon doesn't handle bridge, so we do it here.
    if member_filter.is_none() {
        let bridge_outcome = handle_bridge_stop(team, &cfg, effective_bridge_flag)?;
        match &bridge_outcome {
            Some(formation::BridgeStopOutcome::Stopped(name)) => {
                println!("Bridge '{}' stopped.", name);
            }
            Some(formation::BridgeStopOutcome::LeftRunning(name)) => {
                println!(
                    "Bridge '{}' left running. Use `bm stop --bridge` to stop it.",
                    name
                );
            }
            None => {}
        }
    }

    // Also display bridge from result (for legacy code paths)
    match &result.bridge {
        Some(formation::BridgeStopOutcome::Stopped(name)) => {
            println!("Bridge '{}' stopped.", name);
        }
        Some(formation::BridgeStopOutcome::LeftRunning(name)) => {
            println!(
                "Bridge '{}' left running. Use `bm stop --bridge` to stop it.",
                name
            );
        }
        None => {}
    }

    // If --all, also stop the daemon
    if stop_all {
        match daemon::query_status(&team.name)? {
            daemon::DaemonStatusInfo::Running { pid, .. } => {
                daemon::stop_daemon(&team.name)?;
                eprintln!("Daemon stopped (PID {}).", pid);
            }
            daemon::DaemonStatusInfo::NotRunning { .. } => {
                // Daemon not running — nothing to do
            }
        }
    }

    Ok(())
}

/// Handle bridge stop lifecycle. Extracted from stop_local_members to keep
/// bridge lifecycle at the command layer per ADR-0008.
fn handle_bridge_stop(
    team: &config::TeamEntry,
    cfg: &config::BotminterConfig,
    bridge_flag: bool,
) -> Result<Option<formation::BridgeStopOutcome>> {
    use crate::bridge::{self, BridgeStopResult};

    let should_stop = bridge_flag || team.bridge_lifecycle.stop_on_down;
    let team_repo = team.path.join("team");

    let bridge_dir = match bridge::discover(&team_repo, &team.name)? {
        Some(d) => d,
        None => return Ok(None),
    };

    let state_path = bridge::state_path(&cfg.workzone, &team.name);
    let mut b = bridge::Bridge::new(bridge_dir, state_path, team.name.clone())?;

    if should_stop {
        if b.is_local() && b.is_running() && which::which("just").is_ok() {
            let bridge_name = b.bridge_name().to_string();
            match b.stop()? {
                BridgeStopResult::Stopped => {
                    b.save()?;
                    return Ok(Some(formation::BridgeStopOutcome::Stopped(bridge_name)));
                }
                BridgeStopResult::External => {}
            }
        }
    } else if b.is_local() && b.is_running() {
        return Ok(Some(formation::BridgeStopOutcome::LeftRunning(
            b.bridge_name().to_string(),
        )));
    }

    Ok(None)
}
