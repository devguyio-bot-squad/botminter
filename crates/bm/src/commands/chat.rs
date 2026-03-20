use std::io::Write;

use anyhow::{bail, Context, Result};

use crate::chat;
use crate::config;
use crate::profile;
use crate::state;

/// Handles `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt]`.
pub fn run(
    member: &str,
    team_flag: Option<&str>,
    hat: Option<&str>,
    render_system_prompt: bool,
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
            println!(
                "The brain is available through the bridge chat. \
                 Direct ACP chat will be available in a future release."
            );
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

    // Launch coding agent via exec (replaces this process)
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(&coding_agent.binary)
        .current_dir(&session.ws_path)
        .arg("--append-system-prompt-file")
        .arg(&tmp_path)
        .exec();

    bail!("Failed to launch {}: {}", coding_agent.binary, err);
}
