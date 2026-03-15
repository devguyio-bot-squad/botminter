use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Result};

use crate::bridge;
use crate::config;
use crate::state;
use crate::topology;

/// Maximum seconds to wait for graceful stop before giving up per member.
const GRACEFUL_TIMEOUT_SECS: u64 = 60;

/// Handles `bm stop [member] [-t team] [--force] [--bridge]`.
pub fn run(team_flag: Option<&str>, force: bool, member_filter: Option<&str>, bridge_flag: bool) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_name = &team.name;

    let mut runtime_state = state::load()?;

    // Find running members for this team
    let team_prefix = format!("{}/", team_name);
    let all_running: Vec<(String, u32, std::path::PathBuf)> = runtime_state
        .members
        .iter()
        .filter(|(key, _)| key.starts_with(&team_prefix))
        .map(|(key, rt)| (key.clone(), rt.pid, rt.workspace.clone()))
        .collect();

    // Filter to a single member if requested
    let running: Vec<_> = if let Some(target) = member_filter {
        let target_key = format!("{}/{}", team_name, target);
        let filtered: Vec<_> = all_running.into_iter().filter(|(k, _, _)| *k == target_key).collect();
        if filtered.is_empty() {
            println!("Member '{}' is not running for team '{}'.", target, team_name);
            return Ok(());
        }
        filtered
    } else {
        all_running
    };

    if !running.is_empty() {
        let mut stopped = 0u32;
        let mut errors = 0u32;

        for (key, pid, workspace) in &running {
            let member_name = key.strip_prefix(&team_prefix).unwrap_or(key);

            if !state::is_alive(*pid) {
                eprint!("{}... already exited", member_name);
                eprintln!();
                runtime_state.members.remove(key);
                state::save(&runtime_state)?;
                stopped += 1;
                continue;
            }

            if force {
                eprint!("Stopping {} (force)... ", member_name);
                force_stop(*pid);
                runtime_state.members.remove(key);
                state::save(&runtime_state)?;
                eprintln!("done");
                stopped += 1;
            } else {
                eprint!("Stopping {}... ", member_name);
                match graceful_stop(workspace, *pid) {
                    Ok(()) => {
                        runtime_state.members.remove(key);
                        state::save(&runtime_state)?;
                        eprintln!("done");
                        stopped += 1;
                    }
                    Err(e) => {
                        eprintln!("failed: {}", e);
                        eprintln!(
                            "  Hint: try `bm stop -f` to force-kill, or check workspace at {}",
                            workspace.display()
                        );
                        errors += 1;
                    }
                }
            }
        }

        println!(
            "\nStopped {} member(s), {} error(s).",
            stopped, errors
        );

        if errors > 0 {
            bail!(
                "Some members could not be stopped gracefully. \
                 Use `bm stop -f` to force-kill."
            );
        }
    } else {
        println!("No members running for team '{}'.", team_name);
    }

    // Bridge lifecycle: stop only if explicitly requested or configured
    // Skip when stopping a single member
    if member_filter.is_some() {
        return Ok(());
    }
    let should_stop_bridge = bridge_flag || team.bridge_lifecycle.stop_on_down;
    let team_repo = team.path.join("team");
    if let Some(bridge_dir) = bridge::discover(&team_repo, team_name)? {
        let state_path = bridge::state_path(&cfg.workzone, team_name);
        let mut b = bridge::Bridge::new(bridge_dir, state_path, team_name.to_string())?;
        if should_stop_bridge {
            if b.is_local() && b.is_running() && which::which("just").is_ok() {
                b.stop()?;
                b.save()?;
            }
        } else if b.is_local() && b.is_running() {
            println!(
                "Bridge '{}' left running. Use `bm stop --bridge` to stop it.",
                b.bridge_name()
            );
        }
    }

    // Remove topology file after all members stopped
    let topo_path = topology::topology_path(&cfg.workzone, team_name);
    if topo_path.exists() {
        topology::remove(&topo_path)?;
    }

    Ok(())
}

/// Graceful stop: run `ralph loops stop` in the workspace, then poll for exit.
fn graceful_stop(workspace: &std::path::Path, pid: u32) -> Result<()> {
    // Try ralph loops stop
    let output = Command::new("ralph")
        .args(["loops", "stop"])
        .current_dir(workspace)
        .output();

    match output {
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            bail!("ralph loops stop failed: {}", stderr.trim());
        }
        Err(e) => {
            bail!("Failed to run ralph loops stop: {}", e);
        }
        Ok(_) => {}
    }

    // Poll for process exit
    for _ in 0..GRACEFUL_TIMEOUT_SECS {
        if !state::is_alive(pid) {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
    }

    bail!(
        "Process {} did not exit after {}s. Use `bm stop -f` to force-kill.",
        pid,
        GRACEFUL_TIMEOUT_SECS
    );
}

/// Force stop: send SIGTERM to the process.
fn force_stop(pid: u32) {
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }
    // Brief wait for cleanup
    thread::sleep(Duration::from_millis(500));
}
