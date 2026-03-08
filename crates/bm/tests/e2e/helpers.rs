//! Shared helpers for E2E tests.

use std::fs;
use std::net::TcpStream;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use libtest_mimic::Trial;

/// Configuration for E2E tests, parsed from CLI arguments.
#[derive(Clone)]
pub struct E2eConfig {
    /// GitHub token for API access. Required.
    pub gh_token: String,
    /// GitHub org where test repos/projects are created. Required.
    pub gh_org: String,
}

/// Runs a test function, converting panics to libtest-mimic errors.
pub fn run_test<F: FnOnce() + std::panic::UnwindSafe>(f: F) -> Result<(), libtest_mimic::Failed> {
    match std::panic::catch_unwind(f) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "test panicked".to_string()
            };
            Err(msg.into())
        }
    }
}

/// Creates a `Command` for the `bm` binary.
pub fn bm_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bm"))
}

/// Polls a TCP port until it accepts connections or the timeout expires.
pub fn wait_for_port(port: u16, timeout: Duration) {
    let start = Instant::now();
    let addr = format!("127.0.0.1:{}", port);

    while start.elapsed() < timeout {
        if TcpStream::connect(&addr).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    panic!(
        "timeout waiting for port {} after {:?}",
        port, timeout
    );
}

/// Runs a command, asserts exit 0, returns stdout.
pub fn assert_cmd_success(cmd: &mut Command) -> String {
    let output = cmd.output().expect("failed to run command");
    assert!(
        output.status.success(),
        "command failed with exit {}: stderr={}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Runs a command, asserts non-zero exit, returns stderr.
#[allow(dead_code)] // Infrastructure for task-08 (start-to-stop lifecycle)
pub fn assert_cmd_fails(cmd: &mut Command) -> String {
    let output = cmd.output().expect("failed to run command");
    assert!(
        !output.status.success(),
        "command succeeded unexpectedly: stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

// ── Git auth helpers ────────────────────────────────────────────────

/// Writes a `.gitconfig` in the given HOME directory that sets up:
/// - User identity (for commits)
/// - Credential helper using `gh auth git-credential` (for pushes)
///
/// This ensures all git operations under this HOME — including those
/// run internally by `bm` commands — can authenticate with GitHub.
pub fn setup_git_auth(home: &Path) {
    let gitconfig = home.join(".gitconfig");
    fs::write(
        &gitconfig,
        "[user]\n\
         \temail = e2e@botminter.test\n\
         \tname = BM E2E\n\
         [credential]\n\
         \thelper = !gh auth git-credential\n",
    )
    .expect("failed to write .gitconfig");
}

/// Verifies that GitHub CLI auth and git credential flow work.
/// Call once at harness startup before registering any tests.
pub fn preflight_gh_auth() {
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .expect("failed to run `gh auth status` — is gh installed?");
    assert!(
        output.status.success(),
        "GitHub CLI is not authenticated. Run `gh auth login` first.\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── Profile helpers ─────────────────────────────────────────────────

/// Extracts all embedded profiles to a temp directory for use with _from variants.
/// Returns the profiles base path (e.g., tmp/.config/botminter/profiles/).
pub fn bootstrap_profiles_to_tmp(tmp: &Path) -> PathBuf {
    let profiles_base = tmp.join(".config").join("botminter").join("profiles");
    std::fs::create_dir_all(&profiles_base).unwrap();
    bm::profile::extract_embedded_to_disk(&profiles_base)
        .expect("Failed to extract embedded profiles to temp dir");
    profiles_base
}

// ── Process helpers ─────────────────────────────────────────────────

/// Checks if a process is alive using kill(pid, 0).
pub fn is_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Force-kills a process. Used for test cleanup.
pub fn force_kill(pid: u32) {
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
}

/// Waits until a process exits, with timeout.
pub fn wait_for_exit(pid: u32, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !is_alive(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "Process {} did not exit within {:?}",
        pid, timeout
    );
}

// ── DaemonGuard ─────────────────────────────────────────────────────

/// RAII guard that stops and cleans up a daemon process on drop.
///
/// Use this in E2E tests that start a daemon to ensure cleanup even if
/// the test panics.
pub struct DaemonGuard {
    team_name: String,
    home: PathBuf,
    stub_dir: Option<PathBuf>,
    gh_token: Option<String>,
}

impl DaemonGuard {
    /// Create a guard for a daemon with optional stub PATH override and GH_TOKEN.
    pub fn new(home: &Path, team_name: &str, stub_dir: Option<&Path>, gh_token: Option<&str>) -> Self {
        DaemonGuard {
            team_name: team_name.to_string(),
            home: home.to_path_buf(),
            stub_dir: stub_dir.map(|p| p.to_path_buf()),
            gh_token: gh_token.map(|s| s.to_string()),
        }
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        // Try graceful stop via bm daemon stop
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "stop", "-t", &self.team_name])
            .env("HOME", &self.home);
        if let Some(ref stub_dir) = self.stub_dir {
            cmd.env(
                "PATH",
                format!(
                    "{}:{}",
                    stub_dir.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            );
        }
        if let Some(ref token) = self.gh_token {
            cmd.env("GH_TOKEN", token);
        }
        let _ = cmd.output();

        // Force-kill via PID file if still alive
        let pid_file = self
            .home
            .join(format!(".botminter/daemon-{}.pid", self.team_name));
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_alive(pid) {
                    force_kill(pid);
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }

        // Clean up files
        let _ = fs::remove_file(
            self.home
                .join(format!(".botminter/daemon-{}.pid", self.team_name)),
        );
        let _ = fs::remove_file(
            self.home
                .join(format!(".botminter/daemon-{}.json", self.team_name)),
        );
        let _ = fs::remove_file(
            self.home
                .join(format!(".botminter/daemon-{}-poll.json", self.team_name)),
        );
    }
}

// ── GithubSuite ─────────────────────────────────────────────────────

/// Shared context available to all cases in a GithubSuite.
pub struct SuiteCtx {
    pub repo: super::github::TempRepo,
    pub gh_token: String,
    pub home: tempfile::TempDir,
    pub profiles_base: PathBuf,
}

/// A suite of E2E test cases sharing a single GitHub repo.
/// Creates the repo once, runs setup, executes cases sequentially,
/// reports per-case results, cleans up on drop.
pub struct GithubSuite {
    name: String,
    repo_prefix: String,
    setup_fn: Option<Box<dyn Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe>>,
    cases: Vec<(
        String,
        Box<dyn Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe>,
    )>,
}

impl GithubSuite {
    pub fn new(name: &str, repo_prefix: &str) -> Self {
        GithubSuite {
            name: name.to_string(),
            repo_prefix: repo_prefix.to_string(),
            setup_fn: None,
            cases: Vec::new(),
        }
    }

    pub fn setup<F>(mut self, f: F) -> Self
    where
        F: Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static,
    {
        self.setup_fn = Some(Box::new(f));
        self
    }

    pub fn case<F>(mut self, name: &str, f: F) -> Self
    where
        F: Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static,
    {
        self.cases.push((name.to_string(), Box::new(f)));
        self
    }

    /// Build into a single libtest-mimic Trial.
    pub fn build(self, config: &E2eConfig) -> Trial {
        let cfg = config.clone();
        let name = self.name.clone();
        Trial::test(name, move || {
            // Create shared resources
            let home = tempfile::tempdir().unwrap();
            let repo =
                super::github::TempRepo::new_in_org(&self.repo_prefix, &cfg.gh_org)
                    .map_err(|e| libtest_mimic::Failed::from(e))?;
            let profiles_base = bootstrap_profiles_to_tmp(home.path());

            let ctx = SuiteCtx {
                repo,
                gh_token: cfg.gh_token.clone(),
                home,
                profiles_base,
            };

            // Run setup if provided
            if let Some(ref setup) = self.setup_fn {
                if let Err(e) = catch_unwind(AssertUnwindSafe(|| setup(&ctx))) {
                    let msg = panic_to_string(e);
                    eprintln!("  SETUP FAILED: {}", msg);
                    return Err(format!("suite setup failed: {}", msg).into());
                }
                eprintln!("  setup complete");
            }

            // Run all cases, track failures
            let total = self.cases.len();
            let mut failures = Vec::new();

            for (case_name, case_fn) in &self.cases {
                match catch_unwind(AssertUnwindSafe(|| case_fn(&ctx))) {
                    Ok(()) => eprintln!("  ok {}", case_name),
                    Err(e) => {
                        let msg = panic_to_string(e);
                        eprintln!("  FAILED {}: {}", case_name, msg);
                        failures.push((case_name.clone(), msg));
                    }
                }
            }

            let passed = total - failures.len();
            eprintln!("  ({}/{} passed)", passed, total);

            if failures.is_empty() {
                Ok(())
            } else {
                let msgs: Vec<String> = failures
                    .iter()
                    .map(|(name, msg)| format!("  {}: {}", name, msg))
                    .collect();
                Err(
                    format!("{} case(s) failed:\n{}", failures.len(), msgs.join("\n"))
                        .into(),
                )
            }
        })
    }
}

fn panic_to_string(e: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}
