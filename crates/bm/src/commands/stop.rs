use anyhow::{bail, Result};

use crate::config;
use crate::formation;

/// Handles `bm stop [member] [-t team] [--force] [--bridge]`.
pub fn run(team_flag: Option<&str>, force: bool, member_filter: Option<&str>, bridge_flag: bool) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;

    let result = formation::stop_local_members(team, &cfg, member_filter, force, bridge_flag)?;

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

    // Display bridge status
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

    Ok(())
}
