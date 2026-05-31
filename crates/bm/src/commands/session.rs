//! `bm session` subcommand — inspect and manually clean up retained sessions.

use anyhow::Result;

use crate::cli::SessionCommand;
use crate::config;
use crate::daemon::DaemonClient;

pub fn run(command: SessionCommand) -> Result<()> {
    match command {
        SessionCommand::Inspect { session_id, team } => {
            let cfg = config::load()?;
            let team_entry = config::resolve_team(&cfg, team.as_deref())?;
            let client = DaemonClient::connect(&team_entry.name)?;
            let resp = client.inspect_session(&session_id)?;
            if !resp.ok {
                anyhow::bail!("inspect failed for session {session_id}");
            }
            println!("Session: {}", resp.session_id);
            println!("Member:  {}", resp.member_name);
            println!("Type:    {}", resp.session_type);
            println!("State:   {}", resp.current_state);
            if let Some(wp) = &resp.workspace_path {
                println!("Workspace: {wp}");
            }
            println!("Created: {}", resp.created_at);
            println!("Updated: {}", resp.state_transitioned_at);
            if let Some(fr) = &resp.finalization_results {
                println!("Finalization: {:?}", fr.exit_status);
                if !fr.committed_repos.is_empty() {
                    println!("  Committed repos: {}", fr.committed_repos.join(", "));
                }
                if !fr.pushed_branches.is_empty() {
                    println!("  Pushed branches: {}", fr.pushed_branches.join(", "));
                }
                if !fr.recovery_branches.is_empty() {
                    println!("  Recovery branches: {}", fr.recovery_branches.join(", "));
                }
                if !fr.issue_urls.is_empty() {
                    println!("  Issue URLs: {}", fr.issue_urls.join(", "));
                }
            }
            if let Some(gs) = &resp.git_state {
                println!("Git state:");
                println!("  Branches: {}", gs.branches.join(", "));
                println!("  Uncommitted changes: {}", gs.has_uncommitted);
                if !gs.unpushed_commits.is_empty() {
                    println!("  Unpushed commits: {}", gs.unpushed_commits.join(", "));
                }
            }
            Ok(())
        }
        SessionCommand::Cleanup {
            session_id,
            all,
            member,
            older_than,
            team,
        } => {
            let cfg = config::load()?;
            let team_entry = config::resolve_team(&cfg, team.as_deref())?;
            let client = DaemonClient::connect(&team_entry.name)?;

            if let Some(id) = session_id {
                let resp = client.cleanup_session(&id)?;
                if resp.ok {
                    println!("Session {id} cleaned up.");
                } else {
                    let err = resp.error.unwrap_or_else(|| "unknown error".to_string());
                    anyhow::bail!("cleanup failed for session {id}: {err}");
                }
            } else {
                let resp =
                    client.bulk_cleanup_sessions(all, member.as_deref(), older_than.as_deref())?;
                if resp.ok {
                    println!("Cleaned up {} session(s).", resp.removed);
                } else {
                    let err = resp.error.unwrap_or_else(|| "unknown error".to_string());
                    anyhow::bail!("bulk cleanup failed: {err}");
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod session_cli_tests {
    use crate::cli::SessionCommand;

    #[test]
    fn session_inspect_subcommand_variant_exists() {
        let _cmd = SessionCommand::Inspect {
            session_id: "abc12345".to_string(),
            team: None,
        };
    }

    #[test]
    fn session_cleanup_individual_subcommand_variant_exists() {
        let _cmd = SessionCommand::Cleanup {
            session_id: Some("abc12345".to_string()),
            all: false,
            member: None,
            older_than: None,
            team: None,
        };
    }

    #[test]
    fn session_cleanup_all_subcommand_variant_exists() {
        let _cmd = SessionCommand::Cleanup {
            session_id: None,
            all: true,
            member: None,
            older_than: None,
            team: None,
        };
    }

    #[test]
    fn session_cleanup_member_subcommand_variant_exists() {
        let _cmd = SessionCommand::Cleanup {
            session_id: None,
            all: false,
            member: Some("alice".to_string()),
            older_than: None,
            team: None,
        };
    }

    #[test]
    fn session_cleanup_older_than_subcommand_variant_exists() {
        let _cmd = SessionCommand::Cleanup {
            session_id: None,
            all: false,
            member: None,
            older_than: Some("48h".to_string()),
            team: None,
        };
    }
}
