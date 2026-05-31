//! `bm session` subcommand — inspect and manually clean up retained sessions.

// Implementation comes in CT-89-02 GREEN. This file anchors the module and
// holds the RED-phase tests that will compile only after GREEN adds the
// SessionCommand variants, inspect_session_handler, cleanup_session_handler,
// and SessionManager::inspect_session / cleanup_session / cleanup_sessions.

// ── AC-18 RED tests ──────────────────────────────────────────────────────────
// These tests reference types and functions that don't exist yet.
// They produce compile errors until GREEN implements them.
#[cfg(test)]
mod session_cli_tests {
    // SessionCommand doesn't exist yet in cli.rs → E0432
    use crate::cli::SessionCommand;

    // AC-18: bm session inspect <id> subcommand variant must exist.
    #[test]
    fn session_inspect_subcommand_variant_exists() {
        // Constructing SessionCommand::Inspect forces a compile error until the
        // variant is declared in cli.rs.
        let _cmd = SessionCommand::Inspect {
            session_id: "abc12345".to_string(),
            team: None,
        };
    }

    // AC-18: bm session cleanup <id> subcommand variant must exist.
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

    // AC-18: bm session cleanup --all variant must exist.
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

    // AC-18: bm session cleanup --member alice variant.
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

    // AC-18: bm session cleanup --older-than 48h variant.
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
