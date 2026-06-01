use anyhow::Result;

use crate::chat;
use crate::chat::spawn::{format_deactivation_summary, spawn_and_wait_agent};
use crate::config;
use crate::daemon::DaemonClient;
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

    // Resolve member name ("alice" → "engineer-alice")
    let resolved_member = chat::resolve_member_name(&team_repo, member)
        .unwrap_or_else(|_| member.to_string());

    let runtime_state = state::load()?;
    let state_key = format!("{}/{}", team.name, resolved_member);
    if let Some(rt) = runtime_state.members.get(&state_key) {
        if rt.brain_mode && state::is_alive(rt.pid) {
            eprintln!(
                "Note: member '{}' is also running in brain mode (PID {}). Starting independent chat session.",
                resolved_member, rt.pid
            );
        }
    }

    let session = chat::prepare_chat_session(&team_repo, &team.name, &team.path, member, hat)?;

    if render_system_prompt {
        println!("{}", session.meta_prompt);
        return Ok(());
    }

    // Ensure a daemon is running so session lifecycle is tracked.
    let _ = super::ensure_daemon_running(&team.name, &team_repo);

    // Register a daemon session so deactivation is tracked.
    // Failure to connect or start a session is non-fatal — proceed without one.
    let session_id = try_start_daemon_session(&team.name, &resolved_member);

    let result = spawn_and_wait_agent(
        &session,
        team,
        &team_repo,
        &resolved_member,
        session_id.as_deref().unwrap_or(""),
        None,
        autonomous,
    )?;

    if let Some(ref summary) = result.deactivation {
        let text = format_deactivation_summary(summary);
        if !text.is_empty() {
            eprintln!("\n--- Workspace state at session end ---");
            eprintln!("{text}");
        }
    }

    std::process::exit(result.exit_code);
}

/// Tries to connect to the daemon and register a session for the member.
/// Returns the session ID on success, None if the daemon is not running.
fn try_start_daemon_session(team_name: &str, member: &str) -> Option<String> {
    let client = DaemonClient::connect(team_name).ok()?;
    let resp = client.start_session(member, "interactive", None).ok()?;
    resp.session.map(|s| s.session_id)
}
