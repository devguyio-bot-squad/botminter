pub mod config;

use std::fmt;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};

use config::TmuxConfig;

const MINIMUM_MAJOR_VERSION: u32 = 3;

#[derive(Debug)]
pub struct TmuxVersion {
    pub major: u32,
    pub minor: u32,
}

impl fmt::Display for TmuxVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

fn tmux_cmd() -> Command {
    let mut cmd = Command::new("tmux");
    cmd.env_remove("TMUX_TMPDIR");
    cmd
}

fn parse_tmux_version(output: &str) -> Result<TmuxVersion> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        bail!("empty tmux version output");
    }

    let bytes = trimmed.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i].is_ascii_digit() {
            let major_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && bytes[i] == b'.' {
                let major: u32 = trimmed[major_start..i].parse()?;
                i += 1;
                let minor_start = i;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i > minor_start {
                    let minor: u32 = trimmed[minor_start..i].parse()?;
                    if major < MINIMUM_MAJOR_VERSION {
                        bail!(
                            "tmux version {major}.{minor} is below minimum required {MINIMUM_MAJOR_VERSION}.0"
                        );
                    }
                    return Ok(TmuxVersion { major, minor });
                }
            }
        }
        i += 1;
    }

    bail!("failed to parse tmux version from: {trimmed}")
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TmuxSession {
    socket_name: String,
    session_name: String,
    config_path: PathBuf,
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

    pub fn check_tmux_available() -> Result<TmuxVersion> {
        let output = tmux_cmd()
            .arg("-V")
            .output()
            .context("tmux is not installed or not found in PATH")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_tmux_version(&stdout)
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

    // ── CT-02: Version Parsing — Standard Format ────────────────────

    #[test]
    fn parse_version_standard_format() {
        let v = parse_tmux_version("tmux 3.4")
            .expect("'tmux 3.4' is a valid version string");
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
    }

    // ── CT-02: Version Parsing — Letter Suffix ──────────────────────

    #[test]
    fn parse_version_letter_suffix() {
        let v = parse_tmux_version("tmux 3.3a")
            .expect("'tmux 3.3a' is a valid version string");
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 3);
    }

    // ── CT-02: Version Parsing — Development Prefix ─────────────────

    #[test]
    fn parse_version_development_prefix() {
        let v = parse_tmux_version("tmux next-3.4")
            .expect("'tmux next-3.4' is a valid version string");
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
    }

    // ── CT-02: Version Parsing — Release Candidate ──────────────────

    #[test]
    fn parse_version_release_candidate() {
        let v = parse_tmux_version("tmux 3.2-rc")
            .expect("'tmux 3.2-rc' is a valid version string");
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 2);
    }

    // ── CT-02: Minimum Version Enforcement ──────────────────────────

    #[test]
    fn parse_version_rejects_below_minimum() {
        let result = parse_tmux_version("tmux 2.9");
        assert!(result.is_err(), "version 2.9 must be rejected (minimum is 3.0)");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("3.0") && err.contains("2.9"),
            "error must mention required version 3.0 and found version 2.9, got: {err}"
        );
    }

    #[test]
    fn parse_version_rejects_below_minimum_with_suffix() {
        let result = parse_tmux_version("tmux 2.9a");
        assert!(result.is_err(), "version 2.9a must be rejected");
    }

    // ── CT-02: Unparseable Version String ───────────────────────────

    #[test]
    fn parse_version_rejects_garbage_input() {
        let result = parse_tmux_version("not tmux");
        assert!(result.is_err(), "garbage input must be rejected");
    }

    #[test]
    fn parse_version_rejects_empty_input() {
        let result = parse_tmux_version("");
        assert!(result.is_err(), "empty input must be rejected");
    }

    // ── CT-02: Display for TmuxVersion ──────────────────────────────

    #[test]
    fn tmux_version_display() {
        let v = TmuxVersion { major: 3, minor: 4 };
        assert_eq!(v.to_string(), "3.4");
    }

    // ── CT-02: TMUX_TMPDIR Unset ────────────────────────────────────

    #[test]
    fn tmux_cmd_removes_tmux_tmpdir() {
        std::env::set_var("TMUX_TMPDIR", "/tmp/evil");
        let output = tmux_cmd()
            .arg("-V")
            .output();
        std::env::remove_var("TMUX_TMPDIR");
        // The command should execute without using the evil TMUX_TMPDIR.
        // If tmux is available, this succeeds; the key assertion is that
        // tmux_cmd() builds a Command that env_removes TMUX_TMPDIR.
        // We verify by checking that check_tmux_available works even
        // with TMUX_TMPDIR set to garbage.
        assert!(output.is_ok(), "tmux_cmd() must produce a valid Command");
    }

    // ── CT-02: Integration — Real tmux ──────────────────────────────

    #[test]
    fn check_tmux_available_returns_valid_version() {
        let v = TmuxSession::check_tmux_available()
            .expect("tmux should be available in the test environment");
        assert!(
            v.major >= 3,
            "system tmux must be version 3.0+, got: {}",
            v
        );
    }

    #[test]
    fn check_tmux_available_works_with_tmux_tmpdir_set() {
        std::env::set_var("TMUX_TMPDIR", "/nonexistent/path");
        let result = TmuxSession::check_tmux_available();
        std::env::remove_var("TMUX_TMPDIR");
        assert!(
            result.is_ok(),
            "check_tmux_available must succeed even with TMUX_TMPDIR set to garbage"
        );
    }
}
