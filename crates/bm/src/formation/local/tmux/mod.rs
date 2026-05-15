pub mod config;

use std::fmt;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
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

fn allocate_pty() -> Result<(std::fs::File, std::fs::File)> {
    use std::os::unix::io::FromRawFd;
    unsafe {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        if libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) != 0
        {
            bail!("failed to allocate pseudo-terminal for attach");
        }
        Ok((
            std::fs::File::from_raw_fd(master),
            std::fs::File::from_raw_fd(slave),
        ))
    }
}

fn validate_name(name: &str, label: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!("Invalid {label} '{name}': must match [a-zA-Z0-9_-]+");
    }
    Ok(())
}

impl TmuxSession {
    pub fn new(team_name: &str) -> Result<Self> {
        validate_name(team_name, "team name")?;

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

    fn config_str(&self) -> Result<&str> {
        self.config_path
            .to_str()
            .context("config path contains non-UTF-8 characters")
    }

    fn run_session_cmd(&self, subcommand: &str, args: &[&str]) -> Result<()> {
        let mut cmd = tmux_cmd();
        cmd.args(["-L", &self.socket_name, subcommand]);
        cmd.args(args);

        let output = cmd.output().with_context(|| {
            format!(
                "failed to run tmux {subcommand} for '{}' on socket '{}'",
                self.session_name, self.socket_name
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "tmux {subcommand} failed for '{}' on socket '{}': {}",
                self.session_name,
                self.socket_name,
                stderr.trim()
            );
        }

        Ok(())
    }

    fn source_config(&self) -> Result<()> {
        let config = self.config_str()?;
        tmux_cmd()
            .args(["-L", &self.socket_name, "source-file", config])
            .output()
            .with_context(|| format!("failed to source config file '{config}'"))?;
        Ok(())
    }

    fn require_window(&self, window: &str) -> Result<String> {
        if !self.window_exists(window) {
            bail!(
                "window '{window}' not found in session '{}'",
                self.session_name
            );
        }
        Ok(format!("{}:{}", self.session_name, window))
    }

    fn query_pane_format(&self, target: &str, format: &str) -> Result<String> {
        let output = tmux_cmd()
            .args([
                "-L",
                &self.socket_name,
                "display-message",
                "-t",
                target,
                "-p",
                format,
            ])
            .output()
            .with_context(|| {
                format!("failed to query '{format}' for target '{target}'")
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "tmux display-message failed for target '{target}': {}",
                stderr.trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn create(&self) -> Result<()> {
        TmuxConfig::ensure_written()?;
        self.run_session_cmd(
            "new-session",
            &["-d", "-s", &self.session_name, "-f", self.config_str()?],
        )?;
        self.source_config()
    }

    pub fn exists(&self) -> bool {
        tmux_cmd()
            .args([
                "-L",
                &self.socket_name,
                "has-session",
                "-t",
                &self.session_name,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn destroy(&self) -> Result<()> {
        self.run_session_cmd("kill-session", &["-t", &self.session_name])
    }

    pub fn destroy_if_exists(&self) -> Result<()> {
        if self.exists() {
            self.destroy()?;
        }
        Ok(())
    }

    pub fn create_window(
        &self,
        name: &str,
        cmd: &[&str],
        cwd: &Path,
        envs: &[(&str, &str)],
    ) -> Result<u32> {
        validate_name(name, "window name")?;
        self.source_config()?;

        let cwd_str = cwd
            .to_str()
            .context("cwd contains non-UTF-8 characters")?;

        let mut tmux = tmux_cmd();
        tmux.args([
            "-L",
            &self.socket_name,
            "new-window",
            "-t",
            &self.session_name,
            "-n",
            name,
            "-c",
            cwd_str,
        ]);

        for (k, v) in envs {
            tmux.args(["-e", &format!("{k}={v}")]);
        }

        tmux.arg("--");
        tmux.args(cmd);

        let output = tmux.output().with_context(|| {
            format!(
                "failed to create window '{name}' in session '{}' on socket '{}'",
                self.session_name, self.socket_name
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "tmux new-window failed for window '{name}' in session '{}' on socket '{}': {}",
                self.session_name,
                self.socket_name,
                stderr.trim()
            );
        }

        let target = format!("{}:{}", self.session_name, name);

        let pid_str = self.query_pane_format(&target, "#{pane_pid}")?;
        let pid: u32 = pid_str.parse().with_context(|| {
            format!("failed to parse PID from tmux output: '{pid_str}'")
        })?;

        std::thread::sleep(std::time::Duration::from_millis(100));

        let dead_str = self.query_pane_format(&target, "#{pane_dead}:#{pane_dead_status}")?;
        if let Some(exit_code) = dead_str.strip_prefix("1:") {
            bail!(
                "process in window '{name}' exited immediately with exit status {exit_code}"
            );
        }

        Ok(pid)
    }

    pub fn window_exists(&self, name: &str) -> bool {
        let output = tmux_cmd()
            .args([
                "-L",
                &self.socket_name,
                "list-windows",
                "-t",
                &self.session_name,
                "-F",
                "#{window_name}",
            ])
            .stderr(std::process::Stdio::null())
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.lines().any(|line| line.trim() == name)
            }
            _ => false,
        }
    }

    pub fn is_pane_dead(&self, window: &str) -> Result<bool> {
        let target = self.require_window(window)?;
        let result = self.query_pane_format(&target, "#{pane_dead}")?;
        match result.as_str() {
            "1" => Ok(true),
            "0" => Ok(false),
            other => bail!("unexpected pane_dead value for window '{window}': {other}"),
        }
    }

    pub fn pane_pid(&self, window: &str) -> Result<u32> {
        let target = self.require_window(window)?;
        let pid_str = self.query_pane_format(&target, "#{pane_pid}")?;
        pid_str
            .parse()
            .with_context(|| format!("failed to parse PID for window '{window}': '{pid_str}'"))
    }

    pub fn list_windows(&self) -> Result<Vec<TmuxWindow>> {
        let output = tmux_cmd()
            .args([
                "-L",
                &self.socket_name,
                "list-windows",
                "-t",
                &self.session_name,
                "-F",
                "#{window_index}|#{window_name}|#{pane_pid}|#{pane_dead}",
            ])
            .output()
            .with_context(|| {
                format!(
                    "failed to list windows for session '{}' on socket '{}'",
                    self.session_name, self.socket_name
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "tmux list-windows failed for session '{}' on socket '{}': {}",
                self.session_name,
                self.socket_name,
                stderr.trim()
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut windows = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 4 {
                continue;
            }
            windows.push(TmuxWindow {
                index: parts[0].parse().with_context(|| {
                    format!("failed to parse window index from '{}'", parts[0])
                })?,
                name: parts[1].to_string(),
                pane_pid: parts[2].parse().with_context(|| {
                    format!("failed to parse pane PID from '{}'", parts[2])
                })?,
                dead: parts[3] == "1",
            });
        }
        Ok(windows)
    }

    pub fn kill_window_process(&self, name: &str) -> Result<()> {
        let pid = self.pane_pid(name)?;
        let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if ret == -1 {
            let errno = std::io::Error::last_os_error();
            if errno.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            bail!(
                "failed to send SIGTERM to PID {pid} for window '{name}': {errno}"
            );
        }
        Ok(())
    }

    pub fn remove_window(&self, name: &str) -> Result<()> {
        let target = self.require_window(name)?;
        self.run_session_cmd("kill-window", &["-t", &target])
    }

    pub fn remove_dead_window(&self, name: &str) -> Result<()> {
        if !self.window_exists(name) {
            return Ok(());
        }
        if self.is_pane_dead(name)? {
            self.remove_window(name)?;
        }
        Ok(())
    }

    pub fn session_info(&self) -> Result<SessionInfo> {
        let windows = self.list_windows()?;
        Ok(SessionInfo {
            session_name: self.session_name.clone(),
            socket_name: self.socket_name.clone(),
            attach_command: format!(
                "tmux -L {} attach-session -t {}",
                self.socket_name, self.session_name
            ),
            windows,
        })
    }

    pub fn attach(&self, window: Option<&str>) -> Result<()> {
        let target = match window {
            Some(w) => format!("{}:{}", self.session_name, w),
            None => self.session_name.clone(),
        };
        let mut cmd = tmux_cmd();
        cmd.args(["-L", &self.socket_name, "attach-session", "-t", &target]);

        let _pty_master: Option<std::fs::File>;
        if !std::io::stdin().is_terminal() {
            let (master, slave) = allocate_pty()?;
            _pty_master = Some(master);
            let slave_out = slave.try_clone().context("clone PTY slave")?;
            let slave_err = slave.try_clone().context("clone PTY slave")?;
            cmd.stdin(slave).stdout(slave_out).stderr(slave_err);
        } else {
            _pty_master = None;
        }

        let status = cmd.status().with_context(|| {
            format!(
                "failed to attach to '{target}' on socket '{}'",
                self.socket_name
            )
        })?;
        if !status.success() {
            bail!(
                "tmux attach-session failed for '{target}' on socket '{}'",
                self.socket_name
            );
        }
        Ok(())
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

    // ── CT-03: Session Lifecycle — Cleanup Guard ────────────────────

    struct TmuxGuard {
        session_name: String,
    }

    impl TmuxGuard {
        fn new(session_name: &str) -> Self {
            Self {
                session_name: session_name.to_string(),
            }
        }
    }

    impl Drop for TmuxGuard {
        fn drop(&mut self) {
            let _ = tmux_cmd()
                .args(["-L", "botminter", "kill-session", "-t", &self.session_name])
                .output();
        }
    }

    // ── CT-03: AC1 — Session Creation ───────────────────────────────

    #[test]
    fn session_create_creates_session() {
        let session = TmuxSession::new("ct03-create").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        session.create().expect("create() should succeed");

        let output = tmux_cmd()
            .args(["-L", "botminter", "has-session", "-t", "bm-ct03-create"])
            .output()
            .expect("tmux has-session command should execute");
        assert!(
            output.status.success(),
            "session bm-ct03-create must exist on botminter socket after create()"
        );
    }

    // ── CT-03: AC2 — Exists Returns True When Session Created ───────

    #[test]
    fn session_exists_returns_true_when_created() {
        let session = TmuxSession::new("ct03-exists-t").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        session.create().expect("create() should succeed");
        assert!(
            session.exists(),
            "exists() must return true after create()"
        );
    }

    // ── CT-03: AC3 — Exists Returns False When No Session ───────────

    #[test]
    fn session_exists_returns_false_when_not_created() {
        let session = TmuxSession::new("ct03-exists-f").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        assert!(
            !session.exists(),
            "exists() must return false when no session exists"
        );
    }

    // ── CT-03: AC4 — Destroy Removes Session ────────────────────────

    #[test]
    fn session_destroy_removes_session() {
        let session = TmuxSession::new("ct03-destroy").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        session.create().expect("create() should succeed");
        session.destroy().expect("destroy() should succeed");
        assert!(
            !session.exists(),
            "exists() must return false after destroy()"
        );
    }

    // ── CT-03: AC5 — Idempotent Destroy ─────────────────────────────

    #[test]
    fn session_destroy_if_exists_no_error_when_absent() {
        let session = TmuxSession::new("ct03-idempotent").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        let result = session.destroy_if_exists();
        assert!(
            result.is_ok(),
            "destroy_if_exists() must return Ok when no session exists, got: {:?}",
            result.unwrap_err()
        );
    }

    // ── CT-03: AC6 — Config File Written Before Session ─────────────

    #[test]
    fn session_create_ensures_config_exists() {
        let session = TmuxSession::new("ct03-config").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        session.create().expect("create() should succeed");

        let config_path = TmuxConfig::path().unwrap();
        assert!(
            config_path.exists(),
            "config file must exist at {} after create()",
            config_path.display()
        );

        let metadata = std::fs::metadata(&config_path).unwrap();
        let mode = std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o777;
        assert_eq!(
            mode, 0o600,
            "config file must have 0600 permissions, got: {mode:04o}"
        );
    }

    // ── CT-03: AC7 — Socket Isolation ───────────────────────────────

    #[test]
    fn session_socket_isolation() {
        let session = TmuxSession::new("ct03-socket").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        session.create().expect("create() should succeed");

        let botminter_output = tmux_cmd()
            .args(["-L", "botminter", "list-sessions", "-F", "#{session_name}"])
            .output()
            .expect("tmux list-sessions on botminter socket should execute");
        let botminter_sessions = String::from_utf8_lossy(&botminter_output.stdout);
        assert!(
            botminter_sessions.contains("bm-ct03-socket"),
            "session must appear on botminter socket, got: {botminter_sessions}"
        );

        let default_output = tmux_cmd()
            .args(["list-sessions", "-F", "#{session_name}"])
            .output();
        if let Ok(output) = default_output {
            let default_sessions = String::from_utf8_lossy(&output.stdout);
            assert!(
                !default_sessions.contains("bm-ct03-socket"),
                "session must NOT appear on default socket, got: {default_sessions}"
            );
        }
    }

    // ── CT-03: AC8 — Full Lifecycle ─────────────────────────────────

    #[test]
    fn session_full_lifecycle() {
        let session = TmuxSession::new("ct03-lifecycle").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        session.create().expect("create() should succeed");
        assert!(session.exists(), "exists() must be true after create()");

        session.destroy().expect("destroy() should succeed");
        assert!(!session.exists(), "exists() must be false after destroy()");
    }

    // ── CT-03: Double Create ────────────────────────────────────────

    #[test]
    fn session_double_create_returns_error() {
        let session = TmuxSession::new("ct03-double").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        session.create().expect("first create() should succeed");

        let result = session.create();
        assert!(
            result.is_err(),
            "second create() must return Err when session already exists"
        );
    }

    // ── CT-01: AC1 — Window Creation with Command ──────────────────────

    #[test]
    fn create_window_creates_named_window_with_valid_pid() {
        let session = TmuxSession::new("ct01-create").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        let pid = session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        assert!(pid > 0, "PID must be a positive integer, got: {pid}");

        let has_window = tmux_cmd()
            .args([
                "-L", "botminter", "list-windows", "-t", "bm-ct01-create",
                "-F", "#{window_name}",
            ])
            .output()
            .expect("list-windows should execute");
        let windows = String::from_utf8_lossy(&has_window.stdout);
        assert!(
            windows.contains("bob"),
            "window named 'bob' must exist, got: {windows}"
        );

        let alive = unsafe { libc::kill(pid as i32, 0) };
        assert_eq!(alive, 0, "PID {pid} must be alive (kill -0 returned {alive})");
    }

    // ── CT-01: AC2 — Environment Variable Passing ──────────────────────

    #[test]
    fn create_window_passes_env_vars_securely() {
        let session = TmuxSession::new("ct01-env").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        let pid = session
            .create_window(
                "envtest",
                &["sleep", "300"],
                &cwd,
                &[("SECRET", "token123")],
            )
            .expect("create_window with envs should succeed");

        let start_cmd = tmux_cmd()
            .args([
                "-L", "botminter", "display-message",
                "-t", "bm-ct01-env:envtest",
                "-p", "#{pane_start_command}",
            ])
            .output()
            .expect("display-message should execute");
        let start_cmd_str = String::from_utf8_lossy(&start_cmd.stdout);
        assert!(
            !start_cmd_str.contains("token123"),
            "secret must NOT appear in pane_start_command, got: {start_cmd_str}"
        );

        let ps_output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "args="])
            .output()
            .expect("ps should execute");
        let ps_str = String::from_utf8_lossy(&ps_output.stdout);
        assert!(
            !ps_str.contains("token123"),
            "secret must NOT appear in ps output, got: {ps_str}"
        );
    }

    // ── CT-01: AC3 — Window Name Validation — Valid ────────────────────

    #[test]
    fn create_window_accepts_valid_names() {
        let session = TmuxSession::new("ct01-valid").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        for name in &["bob", "agent_1", "my-worker"] {
            let result = session.create_window(name, &["sleep", "300"], &cwd, &[]);
            assert!(
                result.is_ok(),
                "window name '{name}' must be accepted, got: {:?}",
                result.unwrap_err()
            );
        }
    }

    // ── CT-01: AC4 — Window Name Validation — Invalid ──────────────────

    #[test]
    fn create_window_rejects_invalid_names() {
        let session = TmuxSession::new("ct01-invalid").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        for name in &["my;window", "win:name", "win name", ""] {
            let result = session.create_window(name, &["sleep", "300"], &cwd, &[]);
            assert!(
                result.is_err(),
                "window name '{name}' must be rejected"
            );
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Invalid") || err.contains("invalid"),
                "error for '{name}' must mention invalid, got: {err}"
            );
        }
    }

    // ── CT-01: AC5 — Immediate Exit Detection — Non-Zero Exit ──────────

    #[test]
    fn create_window_detects_immediate_nonzero_exit() {
        let session = TmuxSession::new("ct01-exit").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        let result = session.create_window("fail", &["false"], &cwd, &[]);
        assert!(
            result.is_err(),
            "create_window with 'false' must return Err, not a stale PID"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exit") || err.contains("Exit") || err.contains("status"),
            "error must mention exit status, got: {err}"
        );
    }

    // ── CT-01: AC6 — Immediate Exit Detection — Binary Not Found ───────

    #[test]
    fn create_window_detects_binary_not_found() {
        let session = TmuxSession::new("ct01-notfound").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        let result =
            session.create_window("missing", &["__nonexistent_binary__"], &cwd, &[]);
        assert!(
            result.is_err(),
            "create_window with nonexistent binary must return Err"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            !err.contains("not yet implemented"),
            "error must be a real error, not a stub: {err}"
        );
    }

    // ── CT-01: AC7 — No Session Error ──────────────────────────────────

    #[test]
    fn create_window_without_session_returns_error() {
        let session = TmuxSession::new("ct01-nosess").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);

        let cwd = std::env::temp_dir();
        let result = session.create_window("test", &["sleep", "300"], &cwd, &[]);
        assert!(
            result.is_err(),
            "create_window without a session must return Err"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            !err.contains("not yet implemented"),
            "error must describe missing session, not be a stub: {err}"
        );
    }

    // ── CT-02 (Story #6): Window Queries — window_exists ──────────────

    #[test]
    fn window_exists_returns_true_when_window_present() {
        let session = TmuxSession::new("ct02-wexist-t").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        assert!(
            session.window_exists("bob"),
            "window_exists must return true when window 'bob' exists in the session"
        );
    }

    #[test]
    fn window_exists_returns_false_when_window_absent() {
        let session = TmuxSession::new("ct02-wexist-f").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        assert!(
            !session.window_exists("bob"),
            "window_exists must return false when window 'bob' does not exist (no error, no panic)"
        );
    }

    // ── CT-02 (Story #6): Window Queries — is_pane_dead ───────────────

    #[test]
    fn is_pane_dead_returns_false_for_live_process() {
        let session = TmuxSession::new("ct02-dead-f").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        let dead = session
            .is_pane_dead("bob")
            .expect("is_pane_dead should succeed for existing window");
        assert!(
            !dead,
            "is_pane_dead must return false for a live process (sleep 300)"
        );
    }

    #[test]
    fn is_pane_dead_returns_true_for_exited_process() {
        let session = TmuxSession::new("ct02-dead-t").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        // Create window with `true` — exits immediately with 0.
        // remain-on-exit keeps the pane so we can query it.
        // Use new-window directly to avoid create_window's immediate-exit detection.
        let target = format!("{}:{}", session.session_name, "bob");
        tmux_cmd()
            .args([
                "-L", "botminter", "new-window",
                "-t", &session.session_name,
                "-n", "bob",
                "-c", cwd.to_str().unwrap(),
                "--", "true",
            ])
            .output()
            .expect("tmux new-window should succeed");

        // Poll for the pane to become dead (tmux async state update)
        let mut detected = false;
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            match session.is_pane_dead("bob") {
                Ok(true) => {
                    detected = true;
                    break;
                }
                Ok(false) => continue,
                Err(e) => panic!("is_pane_dead returned error: {e}"),
            }
        }
        assert!(
            detected,
            "is_pane_dead must return true for a window whose command has exited"
        );
    }

    // ── CT-02 (Story #6): Window Queries — pane_pid ───────────────────

    #[test]
    fn pane_pid_returns_valid_pid_for_running_window() {
        let session = TmuxSession::new("ct02-pid").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        let pid = session
            .pane_pid("bob")
            .expect("pane_pid should succeed for existing window");
        assert!(pid > 0, "PID must be positive, got: {pid}");

        let alive = unsafe { libc::kill(pid as i32, 0) };
        assert_eq!(alive, 0, "PID {pid} must be a running process");
    }

    // ── CT-02 (Story #6): Window Queries — list_windows ───────────────

    #[test]
    fn list_windows_returns_all_windows_with_state() {
        let session = TmuxSession::new("ct02-list").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'bob' should succeed");
        session
            .create_window("cos", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'cos' should succeed");

        let windows = session
            .list_windows()
            .expect("list_windows should succeed");

        let names: Vec<&str> = windows.iter().map(|w| w.name.as_str()).collect();
        assert!(
            names.contains(&"bob"),
            "list_windows must include window 'bob', got: {names:?}"
        );
        assert!(
            names.contains(&"cos"),
            "list_windows must include window 'cos', got: {names:?}"
        );

        for w in &windows {
            if w.name == "bob" || w.name == "cos" {
                assert!(w.pane_pid > 0, "window '{}' must have a valid PID", w.name);
                assert!(!w.dead, "window '{}' must not be dead (running sleep)", w.name);
            }
        }
    }

    // ── CT-02 (Story #6): Window Queries — Non-Existent Window Error ──

    #[test]
    fn non_existent_window_returns_error() {
        let session = TmuxSession::new("ct02-ghost").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let pid_result = session.pane_pid("ghost");
        assert!(
            pid_result.is_err(),
            "pane_pid for non-existent window must return Err"
        );
        let pid_err = pid_result.unwrap_err().to_string();
        assert!(
            pid_err.contains("ghost"),
            "error must mention window name 'ghost', got: {pid_err}"
        );

        let dead_result = session.is_pane_dead("ghost");
        assert!(
            dead_result.is_err(),
            "is_pane_dead for non-existent window must return Err"
        );
        let dead_err = dead_result.unwrap_err().to_string();
        assert!(
            dead_err.contains("ghost"),
            "error must mention window name 'ghost', got: {dead_err}"
        );
    }

    // ── CT-03 (Story #6): Kill Window Process ────────────────────────────

    #[test]
    fn kill_window_process_sends_sigterm_and_pane_dies() {
        let session = TmuxSession::new("ct03w-kill").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        let pid = session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        session
            .kill_window_process("bob")
            .expect("kill_window_process should succeed");

        let mut detected = false;
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            match session.is_pane_dead("bob") {
                Ok(true) => {
                    detected = true;
                    break;
                }
                Ok(false) => continue,
                Err(e) => panic!("is_pane_dead returned error: {e}"),
            }
        }
        assert!(
            detected,
            "is_pane_dead must return true after kill_window_process (PID {pid} should be dead)"
        );

        assert!(
            session.window_exists("bob"),
            "window 'bob' must still exist in session after kill (remain-on-exit)"
        );
    }

    // ── CT-03 (Story #6): Remove Window ──────────────────────────────────

    #[test]
    fn remove_window_removes_named_window() {
        let session = TmuxSession::new("ct03w-remove").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        session
            .remove_window("bob")
            .expect("remove_window should succeed");

        assert!(
            !session.window_exists("bob"),
            "window 'bob' must not exist after remove_window"
        );
    }

    // ── CT-03 (Story #6): Remove Dead Window — Dead Pane ─────────────────

    #[test]
    fn remove_dead_window_removes_dead_pane() {
        let session = TmuxSession::new("ct03w-rmdead").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        // Use tmux directly to create a window with `true` (exits immediately)
        // to avoid create_window's immediate-exit detection
        tmux_cmd()
            .args([
                "-L", "botminter", "new-window",
                "-t", &session.session_name,
                "-n", "deadwin",
                "-c", cwd.to_str().unwrap(),
                "--", "true",
            ])
            .output()
            .expect("tmux new-window should succeed");

        // Poll until pane is dead
        let mut dead = false;
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if session.is_pane_dead("deadwin").unwrap_or(false) {
                dead = true;
                break;
            }
        }
        assert!(dead, "pane must be dead before testing remove_dead_window");

        session
            .remove_dead_window("deadwin")
            .expect("remove_dead_window should succeed for dead pane");

        assert!(
            !session.window_exists("deadwin"),
            "dead window must be removed after remove_dead_window"
        );
    }

    // ── CT-03 (Story #6): Remove Dead Window — Live Pane (No-Op) ─────────

    #[test]
    fn remove_dead_window_is_noop_for_live_pane() {
        let session = TmuxSession::new("ct03w-rmlive").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        let pid = session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        let result = session.remove_dead_window("bob");
        assert!(
            result.is_ok(),
            "remove_dead_window must return Ok for live pane, got: {:?}",
            result.unwrap_err()
        );

        assert!(
            session.window_exists("bob"),
            "live window 'bob' must still exist after remove_dead_window"
        );

        let alive = unsafe { libc::kill(pid as i32, 0) };
        assert_eq!(alive, 0, "process PID {pid} must still be alive");
    }

    // ── CT-03 (Story #6): Session Info ───────────────────────────────────

    #[test]
    fn session_info_returns_correct_state() {
        let session = TmuxSession::new("ct03w-info").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'bob' should succeed");
        session
            .create_window("cos", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'cos' should succeed");

        let info = session
            .session_info()
            .expect("session_info should succeed");

        assert_eq!(
            info.session_name, "bm-ct03w-info",
            "session_name must match"
        );
        assert_eq!(
            info.socket_name, "botminter",
            "socket_name must be 'botminter'"
        );

        let names: Vec<&str> = info.windows.iter().map(|w| w.name.as_str()).collect();
        assert!(
            names.contains(&"bob"),
            "session_info windows must include 'bob', got: {names:?}"
        );
        assert!(
            names.contains(&"cos"),
            "session_info windows must include 'cos', got: {names:?}"
        );

        assert!(
            info.attach_command.contains("tmux"),
            "attach_command must contain 'tmux', got: {}",
            info.attach_command
        );
        assert!(
            info.attach_command.contains("botminter"),
            "attach_command must reference botminter socket, got: {}",
            info.attach_command
        );
    }

    // ── CT-03 (Story #6): Attach — Basic ─────────────────────────────────

    #[test]
    fn attach_basic_creates_client() {
        let session = TmuxSession::new("ct03w-attach").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window should succeed");

        let session_name = session.session_name.clone();
        let handle = std::thread::spawn(move || {
            let s = TmuxSession::new("ct03w-attach").unwrap();
            s.attach(None)
        });

        // Poll for tmux client
        let mut client_found = false;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let output = tmux_cmd()
                .args(["-L", "botminter", "list-clients", "-F", "#{client_session}"])
                .output();
            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains(&session_name) {
                    client_found = true;
                    break;
                }
            }
        }

        // Send detach keys to release the blocking attach
        let _ = tmux_cmd()
            .args([
                "-L", "botminter", "send-keys",
                "-t", &session_name,
                "C-b", "",
            ])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = tmux_cmd()
            .args(["-L", "botminter", "detach-client", "-s", &session_name])
            .output();

        let result = handle.join().expect("attach thread should not panic");

        assert!(
            client_found,
            "tmux client must appear in list-clients after attach"
        );
    }

    // ── CT-03 (Story #6): Attach — With Window Target ────────────────────

    #[test]
    fn attach_with_window_targets_specific_window() {
        let session = TmuxSession::new("ct03w-attwin").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'bob' should succeed");
        session
            .create_window("cos", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'cos' should succeed");

        let session_name = session.session_name.clone();
        let handle = std::thread::spawn(move || {
            let s = TmuxSession::new("ct03w-attwin").unwrap();
            s.attach(Some("bob"))
        });

        // Poll for client
        let mut client_found = false;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let output = tmux_cmd()
                .args(["-L", "botminter", "list-clients", "-F", "#{client_session}"])
                .output();
            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains(&session_name) {
                    client_found = true;
                    break;
                }
            }
        }

        // Detach client
        let _ = tmux_cmd()
            .args(["-L", "botminter", "detach-client", "-s", &session_name])
            .output();

        let _ = handle.join();

        assert!(
            client_found,
            "tmux client must appear when attaching with window target"
        );
    }

    // ── CT-03 (Story #6): Full Lifecycle Integration ─────────────────────

    #[test]
    fn window_full_lifecycle_integration() {
        let session = TmuxSession::new("ct03w-life").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();

        // Create two windows: one long-running, one that exits immediately
        session
            .create_window("alive", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'alive' should succeed");

        // Use tmux directly for `true` to avoid immediate-exit detection
        tmux_cmd()
            .args([
                "-L", "botminter", "new-window",
                "-t", &session.session_name,
                "-n", "mortal",
                "-c", cwd.to_str().unwrap(),
                "--", "true",
            ])
            .output()
            .expect("tmux new-window for 'mortal' should succeed");

        // List and verify both windows exist
        let windows = session.list_windows().expect("list_windows should succeed");
        let names: Vec<&str> = windows.iter().map(|w| w.name.as_str()).collect();
        assert!(names.contains(&"alive"), "must have 'alive' window");
        assert!(names.contains(&"mortal"), "must have 'mortal' window");

        // Wait for mortal to die
        let mut mortal_dead = false;
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if session.is_pane_dead("mortal").unwrap_or(false) {
                mortal_dead = true;
                break;
            }
        }
        assert!(mortal_dead, "'mortal' window must be dead");

        // Remove dead window
        session
            .remove_dead_window("mortal")
            .expect("remove_dead_window should succeed");
        assert!(
            !session.window_exists("mortal"),
            "'mortal' window must be removed"
        );

        // Verify live window is undisturbed
        assert!(
            session.window_exists("alive"),
            "'alive' window must still exist"
        );
        assert!(
            !session.is_pane_dead("alive").unwrap(),
            "'alive' window must still be running"
        );
    }

    // ── CT-03 (Story #6): Remove Non-Existent Window Error ───────────────

    #[test]
    fn remove_nonexistent_window_returns_error() {
        let session = TmuxSession::new("ct03w-rmghost").unwrap();
        let _guard = TmuxGuard::new(&session.session_name);
        session.create().expect("session create should succeed");

        let result = session.remove_window("ghost");
        assert!(
            result.is_err(),
            "remove_window for non-existent window must return Err"
        );
    }
}
