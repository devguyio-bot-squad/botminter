use std::ffi::OsString;

use anyhow::{bail, Result};

use crate::chat;
use crate::config;
use crate::profile::Meeting;

/// Leak a String into a &'static str. Used for dynamic Clap subcommand names
/// which require 'static lifetimes. Acceptable for a CLI process that runs once.
fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

// These pub functions are consumed by main.rs and completions.rs for dynamic
// subcommand injection — not cross-command coupling (ADR-0007 exception).

/// Convert a profile Meeting into a Clap subcommand for dynamic injection.
pub fn build_meeting_subcommand(meeting: &Meeting) -> clap::Command {
    let mut cmd = clap::Command::new(leak(&meeting.name))
        .about(leak(&meeting.description))
        .arg(
            clap::Arg::new("team")
                .short('t')
                .long("team")
                .help("Team name (defaults to default team)"),
        )
        .arg(
            clap::Arg::new("autonomous")
                .short('a')
                .long("autonomous")
                .action(clap::ArgAction::SetTrue)
                .help("Run with --dangerously-skip-permissions"),
        );
    for arg_def in &meeting.args {
        let mut arg = clap::Arg::new(leak(&arg_def.name));
        if arg_def.positional {
            arg = arg.required(arg_def.required);
        } else {
            let long_name = arg_def.long.as_deref().unwrap_or(&arg_def.name);
            arg = arg.long(leak(long_name)).required(arg_def.required);
        }
        arg = arg.help(leak(&arg_def.description));
        cmd = cmd.arg(arg);
    }
    cmd
}

/// Dispatch a matched meeting subcommand.
pub fn run_meeting(meeting: &Meeting, matches: &clap::ArgMatches) -> Result<()> {
    let team_flag = matches.get_one::<String>("team").map(|s| s.as_str());
    let autonomous = matches.get_flag("autonomous");

    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    let member = chat::resolve_member_by_role(&team_repo, &meeting.member)?;
    let user_args = extract_user_args(meeting, matches)?;
    let initial_prompt = chat::build_meeting_prompt(
        meeting.prompt.as_deref(),
        user_args.as_deref(),
    );
    let session = chat::prepare_chat_session(
        &team_repo,
        &team.name,
        &team.path,
        &member,
        Some(&meeting.hat),
    )?;

    chat::launch_session(&session, team, &team_repo, initial_prompt.as_deref(), autonomous)
}

/// Handle `External(Vec<OsString>)` — unknown subcommands.
pub fn run_external(args: Vec<OsString>) -> Result<()> {
    let cmd_name = args
        .first()
        .and_then(|s| s.to_str())
        .unwrap_or("<unknown>");

    bail!(
        "Unknown command '{}'.\n\
         Run `bm --help` to see available commands.",
        cmd_name
    );
}

/// Extract user-provided args from ArgMatches (excluding --team and -a).
fn extract_user_args(meeting: &Meeting, matches: &clap::ArgMatches) -> Result<Option<String>> {
    let mut parts = Vec::new();

    for arg_def in &meeting.args {
        if let Some(val) = matches.get_one::<String>(&arg_def.name) {
            if arg_def.positional {
                parts.push(val.clone());
            } else {
                let label = arg_def.long.as_deref().unwrap_or(&arg_def.name);
                parts.push(format!("--{} {}", label, val));
            }
        }
    }

    if parts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parts.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Meeting, MeetingArg};

    fn planning_meeting() -> Meeting {
        Meeting {
            name: "planning".into(),
            description: "Collaborative planning session".into(),
            member: "engineer".into(),
            hat: "lead_plan-create".into(),
            prompt: Some("/pdd".into()),
            args: vec![
                MeetingArg {
                    name: "idea".into(),
                    positional: true,
                    long: None,
                    arg_type: None,
                    required: false,
                    description: "Rough idea to plan".into(),
                },
                MeetingArg {
                    name: "epic".into(),
                    positional: false,
                    long: Some("epic".into()),
                    arg_type: Some("int".into()),
                    required: false,
                    description: "Epic issue number to load as input".into(),
                },
            ],
        }
    }

    fn verification_meeting() -> Meeting {
        Meeting {
            name: "verification".into(),
            description: "Verify acceptance criteria for completed work".into(),
            member: "engineer".into(),
            hat: "qe_verify".into(),
            prompt: Some("/verification".into()),
            args: vec![MeetingArg {
                name: "work-item".into(),
                positional: true,
                long: None,
                arg_type: None,
                required: true,
                description: "Issue number to verify".into(),
            }],
        }
    }

    #[test]
    fn build_meeting_subcommand_creates_valid_clap_command() {
        let m = planning_meeting();
        let cmd = build_meeting_subcommand(&m);
        assert_eq!(cmd.get_name(), "planning");
    }

    #[test]
    fn build_meeting_subcommand_has_positional_named_and_team_args() {
        let m = planning_meeting();
        let cmd = build_meeting_subcommand(&m);
        let arg_names: Vec<&str> = cmd.get_arguments().map(|a| a.get_id().as_str()).collect();
        assert!(arg_names.contains(&"idea"));
        assert!(arg_names.contains(&"epic"));
        assert!(arg_names.contains(&"team"));
    }

    #[test]
    fn build_meeting_subcommand_required_positional() {
        let m = verification_meeting();
        let cmd = build_meeting_subcommand(&m);
        let arg = cmd
            .get_arguments()
            .find(|a| a.get_id().as_str() == "work-item")
            .unwrap();
        assert!(arg.is_required_set());
    }

    #[test]
    fn extract_user_args_with_positional_arg() {
        let m = planning_meeting();
        let cmd = build_meeting_subcommand(&m);
        let matches = cmd
            .try_get_matches_from(vec!["planning", "Add OAuth support"])
            .unwrap();
        let prompt = extract_user_args(&m, &matches).unwrap();
        assert_eq!(prompt, Some("Add OAuth support".into()));
    }

    #[test]
    fn extract_user_args_with_named_arg() {
        let m = planning_meeting();
        let cmd = build_meeting_subcommand(&m);
        let matches = cmd
            .try_get_matches_from(vec!["planning", "--epic", "42"])
            .unwrap();
        let prompt = extract_user_args(&m, &matches).unwrap();
        assert_eq!(prompt, Some("--epic 42".into()));
    }

    #[test]
    fn extract_user_args_no_args_returns_none() {
        let m = planning_meeting();
        let cmd = build_meeting_subcommand(&m);
        let matches = cmd.try_get_matches_from(vec!["planning"]).unwrap();
        let prompt = extract_user_args(&m, &matches).unwrap();
        assert!(prompt.is_none());
    }

    #[test]
    fn extract_user_args_with_both_args() {
        let m = planning_meeting();
        let cmd = build_meeting_subcommand(&m);
        let matches = cmd
            .try_get_matches_from(vec!["planning", "OAuth support", "--epic", "42"])
            .unwrap();
        let prompt = extract_user_args(&m, &matches).unwrap();
        assert_eq!(prompt, Some("OAuth support --epic 42".into()));
    }

    #[test]
    fn extract_user_args_ignores_team_flag() {
        let m = planning_meeting();
        let cmd = build_meeting_subcommand(&m);
        let matches = cmd
            .try_get_matches_from(vec!["planning", "-t", "my-team", "OAuth support"])
            .unwrap();
        let prompt = extract_user_args(&m, &matches).unwrap();
        assert_eq!(prompt, Some("OAuth support".into()));
    }
}
