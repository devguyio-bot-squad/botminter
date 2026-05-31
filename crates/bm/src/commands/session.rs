//! `bm session` subcommand — inspect and manually clean up retained sessions.

use anyhow::Result;

use crate::cli::SessionCommand;

pub fn run(command: SessionCommand) -> Result<()> {
    match command {
        SessionCommand::Inspect {
            session_id,
            team: _,
        } => {
            eprintln!("Session inspect not yet connected to daemon (session: {session_id})");
            Ok(())
        }
        SessionCommand::Cleanup {
            session_id,
            all: _,
            member: _,
            older_than: _,
            team: _,
        } => {
            if let Some(id) = session_id {
                eprintln!("Session cleanup not yet connected to daemon (session: {id})");
            } else {
                eprintln!("Bulk session cleanup not yet connected to daemon");
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
