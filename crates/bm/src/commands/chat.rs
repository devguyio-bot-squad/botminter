use std::io::Write;

use anyhow::{bail, Context, Result};

use crate::bridge;
use crate::chat;
use crate::config;
use crate::formation;
use crate::profile;
use crate::state;

/// Handles `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt] [-a]`.
pub fn run(
    member: &str,
    team_flag: Option<&str>,
    hat: Option<&str>,
    render_system_prompt: bool,
    autonomous: bool,
) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Check if this member is running in brain mode
    let runtime_state = state::load()?;
    let state_key = format!("{}/{}", team.name, member);
    if let Some(rt) = runtime_state.members.get(&state_key) {
        if rt.brain_mode && state::is_alive(rt.pid) {
            println!(
                "Member '{}' is running in brain mode (PID {}).",
                member, rt.pid
            );

            // Try to show the DM room info
            if let Ok(Some(bridge_dir)) = bridge::discover(&team_repo, &team.name) {
                let state_path = bridge::state_path(&cfg.workzone, &team.name);
                if let Ok(b) = bridge::Bridge::new(bridge_dir, state_path, team.name.clone()) {
                    if let Some(room_id) = b.room_for_member(member) {
                        println!("Chat via Matrix DM room: {}", room_id);
                    } else if let Some(room_id) = b.default_room_id() {
                        println!("Chat via Matrix room: {}", room_id);
                    } else {
                        println!("Chat via the bridge (no room ID available).");
                    }
                }
            } else {
                println!("Chat via the bridge.");
            }
            return Ok(());
        }
    }

    // Prepare session (validates member, workspace, hat, builds meta-prompt)
    let session = chat::prepare_chat_session(
        &team_repo,
        &team.name,
        &team.path,
        member,
        hat,
    )?;

    if render_system_prompt {
        println!("{}", session.meta_prompt);
        return Ok(());
    }

    // Resolve coding agent for launch
    let manifest = profile::read_team_repo_manifest(&team_repo)?;
    let coding_agent = profile::resolve_coding_agent(team, &manifest)?;

    // Write meta-prompt to temp file
    let mut tmp_file = tempfile::Builder::new()
        .prefix("bm-chat-")
        .suffix(".md")
        .tempfile()
        .context("Failed to create temp file for meta-prompt")?;
    tmp_file
        .write_all(session.meta_prompt.as_bytes())
        .context("Failed to write meta-prompt to temp file")?;
    let tmp_path = tmp_file.into_temp_path();

    // Build command arguments for the coding agent
    let tmp_path_str = tmp_path.to_str().context("Temp path is not valid UTF-8")?;
    let mut args: Vec<&str> = vec!["--append-system-prompt-file", tmp_path_str];
    if autonomous {
        args.push("--dangerously-skip-permissions");
    }

    // Resolve formation: v2 teams delegate through formation.exec_in(),
    // v1 teams (no formations dir) use direct process exec for backward compat.
    let resolved_formation = formation::resolve_formation(&team_repo, None)?;

    if resolved_formation.is_some() {
        // v2 team — delegate to formation.exec_in()
        let local_formation = formation::create_local_formation(&team.name)?;
        let mut cmd_parts: Vec<&str> = vec![&coding_agent.binary];
        cmd_parts.extend(&args);
        local_formation.exec_in(&session.ws_path, &cmd_parts)?;
        Ok(())
    } else {
        // v1 team (no formations dir) — legacy path: exec() replaces this process
        use std::os::unix::process::CommandExt;
        let mut cmd = std::process::Command::new(&coding_agent.binary);
        cmd.current_dir(&session.ws_path)
            .arg("--append-system-prompt-file")
            .arg(&tmp_path);
        if autonomous {
            cmd.arg("--dangerously-skip-permissions");
        }
        let err = cmd.exec();
        bail!("Failed to launch {}: {}", coding_agent.binary, err);
    }
}
