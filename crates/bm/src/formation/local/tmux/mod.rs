pub mod config;

use std::path::PathBuf;

use anyhow::{bail, Result};

use config::TmuxConfig;

pub struct TmuxVersion {
    pub major: u32,
    pub minor: u32,
}

pub struct TmuxWindow {
    pub index: u32,
    pub name: String,
    pub pane_pid: u32,
    pub dead: bool,
}

pub struct SessionInfo {
    pub session_name: String,
    pub socket_name: String,
    pub windows: Vec<TmuxWindow>,
    pub attach_command: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TmuxSession {
    socket_name: String,
    session_name: String,
    config_path: PathBuf,
}

impl TmuxSession {
    pub fn new(team_name: &str) -> Result<Self> {
        if team_name.is_empty()
            || !team_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!("Invalid team name '{team_name}': must match [a-zA-Z0-9_-]+");
        }

        let config_path = TmuxConfig::path()?;

        Ok(Self {
            socket_name: "botminter".to_string(),
            session_name: format!("bm-{team_name}"),
            config_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AC: Name Validation — Valid Names ─────────────────────────────

    #[test]
    fn new_accepts_hyphenated_name() {
        let session = TmuxSession::new("my-team")
            .expect("'my-team' is a valid team name");
        assert_eq!(session.session_name, "bm-my-team");
        assert_eq!(session.socket_name, "botminter");
    }

    #[test]
    fn new_accepts_underscored_name() {
        let session = TmuxSession::new("team_1")
            .expect("'team_1' is a valid team name");
        assert_eq!(session.session_name, "bm-team_1");
        assert_eq!(session.socket_name, "botminter");
    }

    #[test]
    fn new_accepts_mixed_case_alphanumeric() {
        let session = TmuxSession::new("MyTeam123")
            .expect("'MyTeam123' is a valid team name");
        assert_eq!(session.session_name, "bm-MyTeam123");
        assert_eq!(session.socket_name, "botminter");
    }

    // ── AC: Name Validation — Invalid Names ──────────────────────────

    #[test]
    fn new_rejects_semicolon_in_name() {
        let result = TmuxSession::new("my;team");
        assert!(result.is_err(), "name with semicolon must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("invalid") || err.contains("Invalid"),
            "error must mention invalid characters, got: {err}"
        );
    }

    #[test]
    fn new_rejects_colon_in_name() {
        let result = TmuxSession::new("team:name");
        assert!(result.is_err(), "name with colon must be rejected");
    }

    #[test]
    fn new_rejects_space_in_name() {
        let result = TmuxSession::new("team name");
        assert!(result.is_err(), "name with space must be rejected");
    }

    #[test]
    fn new_rejects_slash_in_name() {
        let result = TmuxSession::new("team/path");
        assert!(result.is_err(), "name with slash must be rejected");
    }

    #[test]
    fn new_rejects_empty_name() {
        let result = TmuxSession::new("");
        assert!(result.is_err(), "empty name must be rejected");
    }

    // ── AC: Config Path resolved via TmuxSession ─────────────────────

    #[test]
    fn new_sets_config_path_from_tmux_config() {
        let session = TmuxSession::new("my-team")
            .expect("valid name");
        let expected = TmuxConfig::path().expect("path should resolve");
        assert_eq!(
            session.config_path, expected,
            "config_path must match TmuxConfig::path()"
        );
    }
}
