use anyhow::{bail, Context, Result};

use crate::cli::SessionCommand;
use crate::config;
use crate::daemon::DaemonClient;
use crate::daemon::sessions_api::BulkCleanupRequest;

pub fn run(command: SessionCommand) -> Result<()> {
    match command {
        SessionCommand::Inspect { session_id, team } => run_inspect(&session_id, team.as_deref()),
        SessionCommand::Cleanup {
            session_id,
            all,
            member,
            older_than,
            team,
        } => run_cleanup(session_id.as_deref(), all, member.as_deref(), older_than.as_deref(), team.as_deref()),
    }
}

fn run_inspect(session_id: &str, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let client = DaemonClient::connect(&team.name)?;
    let resp = client.inspect_session(session_id)?;
    if !resp.ok {
        bail!("inspect failed: {}", resp.error.unwrap_or_default());
    }
    println!("Session ID:   {}", resp.session_id.as_deref().unwrap_or("—"));
    println!("Member:       {}", resp.member_name.as_deref().unwrap_or("—"));
    println!("Type:         {}", resp.session_type.as_deref().unwrap_or("—"));
    println!("State:        {}", resp.current_state.as_deref().unwrap_or("—"));
    println!("Workspace:    {}", resp.workspace_path.as_deref().unwrap_or("—"));
    if let Some(fr) = resp.finalization_results {
        let pretty = serde_json::to_string_pretty(&fr).unwrap_or_else(|_| fr.to_string());
        println!("Finalization:\n{pretty}");
    }
    if let Some(gs) = resp.git_state {
        let pretty = serde_json::to_string_pretty(&gs).unwrap_or_else(|_| gs.to_string());
        println!("Git State:\n{pretty}");
    }
    Ok(())
}

fn run_cleanup(
    session_id: Option<&str>,
    all: bool,
    member: Option<&str>,
    older_than: Option<&str>,
    team_flag: Option<&str>,
) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let client = DaemonClient::connect(&team.name)?;

    if let Some(id) = session_id {
        let resp = client.cleanup_session(id)?;
        if !resp.ok {
            bail!("cleanup failed: {}", resp.error.unwrap_or_default());
        }
        let sid = resp.session_id.as_deref().unwrap_or(id);
        println!("Cleaned session {sid}");
        println!("  workspace removed: {}", resp.workspace_removed);
        println!("  registry removed:  {}", resp.registry_removed);
    } else if all {
        let req = BulkCleanupRequest { filter: "all".to_string(), value: None };
        let resp = client.bulk_cleanup_sessions(&req)?;
        if !resp.ok {
            bail!("bulk cleanup failed: {}", resp.error.unwrap_or_default());
        }
        println!("Cleaned {} session(s).", resp.cleaned);
    } else if let Some(name) = member {
        let req = BulkCleanupRequest { filter: "member".to_string(), value: Some(name.to_string()) };
        let resp = client.bulk_cleanup_sessions(&req)?;
        if !resp.ok {
            bail!("bulk cleanup failed: {}", resp.error.unwrap_or_default());
        }
        println!("Cleaned {} session(s).", resp.cleaned);
    } else if let Some(duration_str) = older_than {
        let secs = parse_duration_secs(duration_str)?;
        let req = BulkCleanupRequest {
            filter: "older_than".to_string(),
            value: Some(secs.to_string()),
        };
        let resp = client.bulk_cleanup_sessions(&req)?;
        if !resp.ok {
            bail!("bulk cleanup failed: {}", resp.error.unwrap_or_default());
        }
        println!("Cleaned {} session(s).", resp.cleaned);
    } else {
        bail!("specify a session ID, --all, --member <name>, or --older-than <duration>");
    }

    Ok(())
}

fn parse_duration_secs(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        bail!("empty duration string");
    }
    if s.len() < 2 {
        bail!("invalid duration '{}': must be a number followed by a unit (s, m, h, d)", s);
    }
    let (num_part, unit) = s.split_at(s.len() - 1);
    let n: u64 = num_part
        .trim()
        .parse()
        .with_context(|| format!("invalid duration '{}': number part must be an integer", s))?;
    match unit {
        "s" => Ok(n),
        "m" => Ok(n * 60),
        "h" => Ok(n * 3_600),
        "d" => Ok(n * 86_400),
        other => bail!("invalid duration unit '{}' in '{}': use s, m, h, or d", other, s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_secs_hours() {
        assert_eq!(parse_duration_secs("48h").unwrap(), 48 * 3600);
    }

    #[test]
    fn parse_duration_secs_days() {
        assert_eq!(parse_duration_secs("7d").unwrap(), 7 * 86400);
    }

    #[test]
    fn parse_duration_secs_minutes() {
        assert_eq!(parse_duration_secs("30m").unwrap(), 30 * 60);
    }

    #[test]
    fn parse_duration_secs_seconds() {
        assert_eq!(parse_duration_secs("120s").unwrap(), 120);
    }

    #[test]
    fn parse_duration_secs_invalid_unit() {
        assert!(parse_duration_secs("5w").is_err());
    }

    #[test]
    fn parse_duration_secs_empty() {
        assert!(parse_duration_secs("").is_err());
    }

    #[test]
    fn parse_duration_secs_non_numeric() {
        assert!(parse_duration_secs("abch").is_err());
    }

    #[test]
    fn parse_duration_secs_single_char() {
        assert!(parse_duration_secs("h").is_err());
    }
}
