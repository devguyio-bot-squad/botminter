//! Shared helpers for E2E tests.

use std::fs;
use std::net::TcpStream;
use std::ops::Range;
use std::os::unix::fs::PermissionsExt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use libtest_mimic::Trial;

// ── E2eConfig + Progressive Mode ────────────────────────────────────

/// Configuration for E2E tests, parsed from CLI arguments.
#[derive(Clone)]
pub struct E2eConfig {
    pub gh_token: String,
    pub gh_org: String,
    pub progressive: Option<ProgressiveMode>,
}

#[derive(Clone, Debug)]
pub enum ProgressiveMode {
    Step(Option<String>),
    Reset(Option<String>),
}

/// Persisted state for a progressive suite run.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProgressState {
    pub suite_name: String,
    pub repo_full_name: String,
    pub home_dir: String,
    pub next_case: usize,
    pub total_cases: usize,
    pub setup_done: bool,
    pub tg_mock_container_id: Option<String>,
    pub tg_mock_port: Option<u16>,
    #[serde(default)]
    pub rc_pod_name: Option<String>,
    #[serde(default)]
    pub rc_pod_port: Option<u16>,
}

impl ProgressState {
    pub fn progress_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/e2e-progress")
    }

    pub fn state_dir(suite_name: &str) -> PathBuf {
        Self::progress_base().join(suite_name)
    }

    fn state_file(suite_name: &str) -> PathBuf {
        Self::progress_base().join(format!("{}.json", suite_name))
    }

    pub fn load(suite_name: &str) -> Option<Self> {
        let content = fs::read_to_string(Self::state_file(suite_name)).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) {
        fs::create_dir_all(Self::progress_base()).unwrap();
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(Self::state_file(&self.suite_name), json).unwrap();
    }

    pub fn delete(suite_name: &str) {
        let _ = fs::remove_file(Self::state_file(suite_name));
        let _ = fs::remove_dir_all(Self::state_dir(suite_name));
    }

    pub fn list_all() -> Vec<String> {
        let Ok(entries) = fs::read_dir(Self::progress_base()) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_suffix(".json").map(|s| s.to_string())
            })
            .collect()
    }
}

// ── Command helpers ─────────────────────────────────────────────────

pub fn run_test<F: FnOnce() + std::panic::UnwindSafe>(f: F) -> Result<(), libtest_mimic::Failed> {
    match std::panic::catch_unwind(f) {
        Ok(()) => Ok(()),
        Err(e) => Err(panic_to_string(e).into()),
    }
}

// Isolated D-Bus address set by `KeyringGuard`. Thread-local because
// tests run single-threaded (`--test-threads=1`).
std::thread_local! {
    static ISOLATED_DBUS_ADDR: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

pub fn bm_cmd() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bm"));
    ISOLATED_DBUS_ADDR.with(|addr| {
        if let Some(ref a) = *addr.borrow() {
            cmd.env("DBUS_SESSION_BUS_ADDRESS", a);
        }
    });
    cmd
}

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

pub fn assert_cmd_fails(cmd: &mut Command) -> String {
    let output = cmd.output().expect("failed to run command");
    assert!(
        !output.status.success(),
        "command succeeded unexpectedly: stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Finds a free TCP port by binding to port 0.
pub fn find_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("failed to bind to a free port")
        .local_addr()
        .expect("failed to get local address")
        .port()
}

pub fn wait_for_port(port: u16, timeout: Duration) {
    let start = Instant::now();
    let addr = format!("127.0.0.1:{}", port);
    while start.elapsed() < timeout {
        if TcpStream::connect(&addr).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("timeout waiting for port {} after {:?}", port, timeout);
}

// ── Git auth ────────────────────────────────────────────────────────

pub fn setup_git_auth(home: &Path) {
    fs::write(
        home.join(".gitconfig"),
        "[user]\n\
         \temail = e2e@botminter.test\n\
         \tname = BM E2E\n\
         [credential]\n\
         \thelper = !gh auth git-credential\n",
    )
    .expect("failed to write .gitconfig");
}

/// Starts gnome-keyring-daemon with `--replace --unlock` and a newline password.
/// Verifies the login collection is unlocked before returning.
fn start_keyring_daemon() {
    use std::io::Write;

    let mut gkd = Command::new("gnome-keyring-daemon")
        .args(["--replace", "--unlock", "--components=secrets,pkcs11", "--daemonize"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start gnome-keyring-daemon");
    if let Some(mut stdin) = gkd.stdin.take() {
        // A newline is the minimum valid password — empty (0 bytes) is treated as NULL
        stdin.write_all(b"\n").unwrap();
    }
    let _ = gkd.wait();
    std::thread::sleep(Duration::from_secs(1));

    let check = Command::new("busctl")
        .args(["--user", "get-property", "org.freedesktop.secrets",
               "/org/freedesktop/secrets/collection/login",
               "org.freedesktop.Secret.Collection", "Locked"])
        .output();
    if let Ok(out) = check {
        let result = String::from_utf8_lossy(&out.stdout);
        assert!(result.trim() == "b false",
            "Keyring collection is not unlocked: {}", result.trim());
    } else {
        panic!("Failed to verify keyring — busctl not available?");
    }
}

/// Wipes keyring data from `XDG_DATA_HOME/keyrings/`.
fn wipe_keyring_data() {
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        let keyrings_dir = PathBuf::from(data_home).join("keyrings");
        if keyrings_dir.exists() {
            fs::remove_dir_all(&keyrings_dir).unwrap();
        }
    }
}

/// Tears down and restarts the keyring with fresh data.
/// Call during `reset_home` to ensure the second pass has no leaked credentials.
pub fn reset_keyring() {
    wipe_keyring_data();
    start_keyring_daemon();
    eprintln!("  Keyring reset (fresh data)");
}

/// RAII guard that creates a fully isolated gnome-keyring-daemon for e2e tests.
///
/// Sets process-wide env vars (`DBUS_SESSION_BUS_ADDRESS`, `XDG_RUNTIME_DIR`,
/// `XDG_DATA_HOME`) so all subprocess `Command`s inherit them. Restores the
/// original values on drop. Requires `--test-threads=1`.
pub struct KeyringGuard {
    dbus_pid: u32,
    tmpdir: PathBuf,
    original_dbus_addr: Option<String>,
    original_xdg_runtime: Option<String>,
    original_xdg_data: Option<String>,
}

impl KeyringGuard {
    pub fn new() -> Self {
        let tmpdir = tempfile::tempdir().unwrap().keep();
        let runtime_dir = tmpdir.join("runtime");
        let data_dir = tmpdir.join("data");
        fs::create_dir_all(&runtime_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();

        // Save original env vars
        let original_dbus_addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        let original_xdg_runtime = std::env::var("XDG_RUNTIME_DIR").ok();
        let original_xdg_data = std::env::var("XDG_DATA_HOME").ok();

        // Set isolated env vars
        std::env::set_var("XDG_RUNTIME_DIR", &runtime_dir);
        std::env::set_var("XDG_DATA_HOME", &data_dir);
        std::env::remove_var("GNOME_KEYRING_CONTROL");

        // Start isolated D-Bus session
        let dbus_output = Command::new("dbus-daemon")
            .args(["--session", "--fork", "--print-address", "--print-pid"])
            .output()
            .expect("failed to start dbus-daemon — is dbus installed?");
        assert!(dbus_output.status.success(), "dbus-daemon failed to start");

        let stdout = String::from_utf8_lossy(&dbus_output.stdout);
        let lines: Vec<&str> = stdout.trim().lines().collect();
        assert!(lines.len() >= 2, "dbus-daemon output missing address/pid: {:?}", lines);
        let dbus_addr = lines[0].trim();
        let dbus_pid: u32 = lines[1].trim().parse().expect("invalid dbus-daemon PID");

        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", dbus_addr);
        ISOLATED_DBUS_ADDR.with(|addr| {
            *addr.borrow_mut() = Some(dbus_addr.to_string());
        });

        // Start gnome-keyring-daemon (reuses shared function)
        start_keyring_daemon();

        eprintln!("  Isolated keyring ready (dbus pid={}, addr={})", dbus_pid, dbus_addr);

        KeyringGuard {
            dbus_pid,
            tmpdir,
            original_dbus_addr,
            original_xdg_runtime,
            original_xdg_data,
        }
    }
}

impl Drop for KeyringGuard {
    fn drop(&mut self) {
        // Kill the isolated D-Bus daemon (gnome-keyring-daemon dies with it)
        unsafe { libc::kill(self.dbus_pid as i32, libc::SIGTERM); }

        // Clear thread-local
        ISOLATED_DBUS_ADDR.with(|addr| { *addr.borrow_mut() = None; });

        // Restore original env vars
        match &self.original_dbus_addr {
            Some(v) => std::env::set_var("DBUS_SESSION_BUS_ADDRESS", v),
            None => std::env::remove_var("DBUS_SESSION_BUS_ADDRESS"),
        }
        match &self.original_xdg_runtime {
            Some(v) => std::env::set_var("XDG_RUNTIME_DIR", v),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
        match &self.original_xdg_data {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }

        let _ = fs::remove_dir_all(&self.tmpdir);
        eprintln!("  Isolated keyring cleaned up");
    }
}

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

pub fn bootstrap_profiles_to_tmp(tmp: &Path) -> PathBuf {
    let profiles_base = tmp.join(".config").join("botminter").join("profiles");
    fs::create_dir_all(&profiles_base).unwrap();
    bm::profile::extract_embedded_to_disk(&profiles_base)
        .expect("Failed to extract embedded profiles to temp dir");
    profiles_base
}

// ── Process helpers ─────────────────────────────────────────────────

pub fn is_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

pub fn force_kill(pid: u32) {
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
}

pub fn wait_for_exit(pid: u32, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !is_alive(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("Process {} did not exit within {:?}", pid, timeout);
}

// ── Stub Ralph ──────────────────────────────────────────────────────

/// Path to the stub ralph script relative to CARGO_MANIFEST_DIR.
const STUB_RALPH_SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/e2e/stub-ralph.sh");

pub fn install_stub_ralph(home: &Path) {
    let stub_dir = home.join("stub-bin");
    fs::create_dir_all(&stub_dir).unwrap();
    let stub_path = stub_dir.join("ralph");
    fs::copy(STUB_RALPH_SCRIPT, &stub_path).expect("failed to copy stub-ralph.sh");
    fs::set_permissions(&stub_path, fs::Permissions::from_mode(0o755)).unwrap();
}

pub fn path_with_stub(home: &Path) -> String {
    format!(
        "{}:{}",
        home.join("stub-bin").display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

// ── Process state helpers ───────────────────────────────────────────

pub fn read_pid_from_state(home: &Path) -> Option<u32> {
    let state_path = home.join(".botminter").join("state.json");
    let contents = fs::read_to_string(&state_path).ok()?;
    let state: bm::state::RuntimeState = serde_json::from_str(&contents).ok()?;
    state.members.values().next().map(|rt| rt.pid)
}

pub struct ProcessGuard {
    pub pid: Option<u32>,
    home: PathBuf,
    team_name: String,
}

impl ProcessGuard {
    pub fn new(home: &Path, team_name: &str) -> Self {
        ProcessGuard {
            pid: None,
            home: home.to_path_buf(),
            team_name: team_name.to_string(),
        }
    }

    pub fn set_pid(&mut self, pid: u32) {
        self.pid = Some(pid);
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = bm_cmd()
            .args(["stop", "--force", "-t", &self.team_name])
            .env("HOME", &self.home)
            .env("PATH", path_with_stub(&self.home))
            .output();
        if let Some(pid) = self.pid {
            if is_alive(pid) {
                force_kill(pid);
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

// ── DaemonGuard ─────────────────────────────────────────────────────

pub struct DaemonGuard {
    team_name: String,
    home: PathBuf,
    gh_token: Option<String>,
}

impl DaemonGuard {
    pub fn new(home: &Path, team_name: &str, gh_token: Option<&str>) -> Self {
        DaemonGuard {
            team_name: team_name.to_string(),
            home: home.to_path_buf(),
            gh_token: gh_token.map(|s| s.to_string()),
        }
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let mut cmd = bm_cmd();
        cmd.args(["daemon", "stop", "-t", &self.team_name])
            .env("HOME", &self.home)
            .env("PATH", path_with_stub(&self.home));
        if let Some(ref token) = self.gh_token {
            cmd.env("GH_TOKEN", token);
        }
        let _ = cmd.output();

        let pid_file = self.home.join(format!(".botminter/daemon-{}.pid", self.team_name));
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_alive(pid) {
                    force_kill(pid);
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }
        for suffix in ["pid", "json"] {
            let _ = fs::remove_file(self.home.join(format!(".botminter/daemon-{}.{}", self.team_name, suffix)));
        }
        let _ = fs::remove_file(self.home.join(format!(".botminter/daemon-{}-poll.json", self.team_name)));
    }
}

// ── Cleanup helpers ─────────────────────────────────────────────────

pub fn cleanup_project_boards(gh_org: &str, gh_token: &str, title_match: &str) {
    let output = Command::new("gh")
        .args(["project", "list", "--owner", gh_org, "--format", "json", "--limit", "100"])
        .env("GH_TOKEN", gh_token)
        .output()
        .expect("failed to list projects");

    if output.status.success() {
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
        if let Some(projects) = json["projects"].as_array() {
            for project in projects {
                let title = project["title"].as_str().unwrap_or("");
                if title.contains(title_match) {
                    if let Some(number) = project["number"].as_u64() {
                        eprintln!("Cleaning up project board #{}: {}", number, title);
                        let _ = Command::new("gh")
                            .args(["project", "delete", "--owner", gh_org, &number.to_string(), "--format", "json"])
                            .env("GH_TOKEN", gh_token)
                            .output();
                    }
                }
            }
        }
    }
}

/// Reads the GitHub repo name from a bm config file.
pub fn repo_from_config(home: &Path) -> String {
    let config_path = home.join(".botminter").join("config.yml");
    let config = bm::config::load_from(&config_path)
        .expect("failed to load bm config from home");
    config.teams[0].github_repo.clone()
}

// ── GithubSuite ─────────────────────────────────────────────────────

/// Shared context available to all cases in a GithubSuite.
#[allow(dead_code)]
pub struct SuiteCtx {
    pub gh_token: String,
    pub gh_org: String,
    pub repo_full_name: String,
    pub home: PathBuf,
}

type CaseFn = Box<dyn Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe>;
type ErrorVerifier = Box<dyn Fn(&str) -> bool + Send + Sync>;

struct CaseEntry {
    name: String,
    func: CaseFn,
    /// When Some, the case is expected to panic. The verifier receives the panic message
    /// and returns true if the error is the expected one.
    expect_error: Option<ErrorVerifier>,
}

pub struct GithubSuite {
    name: String,
    /// When Some, GithubSuite creates a TempRepo with this prefix. When None, self-managed.
    repo_prefix: Option<String>,
    /// For self-managed mode: the full repo name (org/repo) to use.
    repo_full_name: Option<String>,
    setup_fn: Option<CaseFn>,
    cases: Vec<CaseEntry>,
    groups: Vec<Range<usize>>,
}

impl GithubSuite {
    /// Create a suite that auto-creates a TempRepo (legacy mode for isolated tests).
    #[allow(dead_code)]
    pub fn new(name: &str, repo_prefix: &str) -> Self {
        GithubSuite {
            name: name.to_string(),
            repo_prefix: Some(repo_prefix.to_string()),
            repo_full_name: None,
            setup_fn: None,
            cases: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// Create a suite where the test manages its own repo lifecycle.
    /// No TempRepo is pre-created. The suite's cleanup case handles deletion.
    pub fn new_self_managed(name: &str, repo_full_name: &str) -> Self {
        GithubSuite {
            name: name.to_string(),
            repo_prefix: None,
            repo_full_name: Some(repo_full_name.to_string()),
            setup_fn: None,
            cases: Vec::new(),
            groups: Vec::new(),
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
        self.cases.push(CaseEntry {
            name: name.to_string(),
            func: Box::new(f),
            expect_error: None,
        });
        self
    }

    /// Register a case that is expected to fail (panic). The verifier receives the
    /// panic message and should return true if the error is the expected one.
    pub fn case_expect_error<F, V>(mut self, name: &str, f: F, verifier: V) -> Self
    where
        F: Fn(&SuiteCtx) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static,
        V: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.cases.push(CaseEntry {
            name: name.to_string(),
            func: Box::new(f),
            expect_error: Some(Box::new(verifier)),
        });
        self
    }

    pub fn group(mut self, start: usize, end: usize) -> Self {
        self.groups.push(start..end + 1);
        self
    }

    fn case_range_at(&self, cursor: usize) -> Range<usize> {
        for g in &self.groups {
            if g.contains(&cursor) {
                return g.clone();
            }
        }
        cursor..cursor + 1
    }

    /// Build a progressive-mode Trial.
    pub fn build_progressive(self, config: &E2eConfig) -> Trial {
        let cfg = config.clone();
        let name = self.name.clone();
        Trial::test(name, move || {
            let suite_name = &self.name;
            let total = self.cases.len();
            let is_self_managed = self.repo_prefix.is_none();

            let mut state = if let Some(s) = ProgressState::load(suite_name) {
                eprintln!("  [{}] resuming from case {}/{}", suite_name, s.next_case + 1, s.total_cases);
                s
            } else {
                let home_dir = ProgressState::state_dir(suite_name).join("home");
                fs::create_dir_all(&home_dir).unwrap();

                let repo_full_name = if is_self_managed {
                    self.repo_full_name.as_ref().unwrap().clone()
                } else {
                    let repo = super::github::TempRepo::new_in_org(
                        self.repo_prefix.as_ref().unwrap(), &cfg.gh_org,
                    ).map_err(libtest_mimic::Failed::from)?;
                    let name = repo.full_name.clone();
                    std::mem::forget(repo);
                    name
                };

                if !is_self_managed {
                    bootstrap_profiles_to_tmp(&home_dir);
                }

                let s = ProgressState {
                    suite_name: suite_name.clone(),
                    repo_full_name,
                    home_dir: home_dir.to_string_lossy().to_string(),
                    next_case: 0,
                    total_cases: total,
                    setup_done: false,
                    tg_mock_container_id: None,
                    tg_mock_port: None,
                    rc_pod_name: None,
                    rc_pod_port: None,
                };
                s.save();
                eprintln!("  [{}] created home", suite_name);
                s
            };

            let home_dir = PathBuf::from(&state.home_dir);

            let ctx = SuiteCtx {
                gh_token: cfg.gh_token.clone(),
                gh_org: cfg.gh_org.clone(),
                repo_full_name: state.repo_full_name.clone(),
                home: home_dir,
            };

            if !state.setup_done {
                if let Some(ref setup) = self.setup_fn {
                    if let Err(e) = catch_unwind(AssertUnwindSafe(|| setup(&ctx))) {
                        let msg = panic_to_string(e);
                        eprintln!("  SETUP FAILED: {}", msg);
                        return Err(format!("suite setup failed: {}", msg).into());
                    }
                }
                state.setup_done = true;

                // Pick up tg-mock info
                let tg_id_file = ctx.home.join(".tg-mock-container-id");
                let tg_url_file = ctx.home.join(".tg-mock-url");
                if tg_id_file.exists() && tg_url_file.exists() {
                    let cid = fs::read_to_string(&tg_id_file).unwrap().trim().to_string();
                    let url = fs::read_to_string(&tg_url_file).unwrap().trim().to_string();
                    if let Some(port) = url.rsplit(':').next().and_then(|s| s.parse::<u16>().ok()) {
                        state.tg_mock_container_id = Some(cid);
                        state.tg_mock_port = Some(port);
                    }
                }

                state.save();
                eprintln!("  setup complete");
            }

            // Isolated keyring AFTER setup (podman needs real D-Bus/XDG)
            let _keyring_guard = KeyringGuard::new();

            let range = self.case_range_at(state.next_case);
            let range_end = range.end;
            let is_final_step = range_end >= total;

            let mut failures = Vec::new();
            for i in range {
                let entry = &self.cases[i];
                let next_name = if i + 1 < total {
                    format!(", next: {}", self.cases[i + 1].name)
                } else {
                    ", suite complete".to_string()
                };
                match catch_unwind(AssertUnwindSafe(|| (entry.func)(&ctx))) {
                    Ok(()) => {
                        if entry.expect_error.is_some() {
                            eprintln!("  FAILED {} (expected error but succeeded)", entry.name);
                            failures.push((entry.name.clone(), "expected error but case succeeded".to_string()));
                        } else {
                            eprintln!("  ok {}  ({}/{} done{})", entry.name, i + 1, total, next_name);
                        }
                    }
                    Err(e) => {
                        let msg = panic_to_string(e);
                        if let Some(ref verifier) = entry.expect_error {
                            if verifier(&msg) {
                                eprintln!("  ok {} (expected error verified)  ({}/{} done{})", entry.name, i + 1, total, next_name);
                            } else {
                                eprintln!("  FAILED {} (wrong error): {}", entry.name, msg);
                                failures.push((entry.name.clone(), format!("wrong error: {}", msg)));
                            }
                        } else {
                            eprintln!("  FAILED {}: {}", entry.name, msg);
                            failures.push((entry.name.clone(), msg));
                        }
                    }
                }
            }

            if !failures.is_empty() {
                eprintln!("  Case(s) failed, will retry on next run (cursor stays at {})", state.next_case);
                state.save();
                let msgs: Vec<String> = failures.iter().map(|(n, m)| format!("  {}: {}", n, m)).collect();
                return Err(format!("{} case(s) failed:\n{}", failures.len(), msgs.join("\n")).into());
            }

            state.next_case = range_end;

            if is_final_step {
                eprintln!("  [{}] suite complete — cleaning up", suite_name);

                if !is_self_managed {
                    // Auto-managed: delete the repo
                    drop(super::github::TempRepo::from_existing(&state.repo_full_name));
                }

                if let Some(cid) = &state.tg_mock_container_id {
                    eprintln!("  Stopping tg-mock container {}", &cid[..12.min(cid.len())]);
                    let _ = Command::new("podman").args(["stop", "-t", "2", cid]).output();
                    let _ = Command::new("podman").args(["rm", "-f", cid]).output();
                }

                ProgressState::delete(suite_name);
            } else {
                state.save();
            }

            Ok(())
        })
    }

    /// Build into a single libtest-mimic Trial (normal mode — runs all cases).
    pub fn build(self, config: &E2eConfig) -> Trial {
        let cfg = config.clone();
        let name = self.name.clone();
        Trial::test(name, move || {
            let is_self_managed = self.repo_prefix.is_none();
            let home_td = tempfile::tempdir().unwrap();
            let home = home_td.path().to_path_buf();

            let repo_full_name;
            let _temp_repo; // holds TempRepo for auto-cleanup when not self-managed

            if is_self_managed {
                repo_full_name = self.repo_full_name.as_ref().unwrap().clone();
                _temp_repo = None;
            } else {
                let repo = super::github::TempRepo::new_in_org(
                    self.repo_prefix.as_ref().unwrap(), &cfg.gh_org,
                ).map_err(libtest_mimic::Failed::from)?;
                repo_full_name = repo.full_name.clone();
                bootstrap_profiles_to_tmp(&home);
                _temp_repo = Some(repo);
            }

            let ctx = SuiteCtx {
                gh_token: cfg.gh_token.clone(),
                gh_org: cfg.gh_org.clone(),
                repo_full_name,
                home,
            };

            // Setup runs BEFORE keyring isolation (podman needs real D-Bus/XDG)
            if let Some(ref setup) = self.setup_fn {
                if let Err(e) = catch_unwind(AssertUnwindSafe(|| setup(&ctx))) {
                    let msg = panic_to_string(e);
                    eprintln!("  SETUP FAILED: {}", msg);
                    return Err(format!("suite setup failed: {}", msg).into());
                }
                eprintln!("  setup complete");
            }

            // Isolated keyring AFTER setup (lives for all cases)
            let _keyring_guard = KeyringGuard::new();

            let total = self.cases.len();
            let mut failures = Vec::new();

            for entry in &self.cases {
                match catch_unwind(AssertUnwindSafe(|| (entry.func)(&ctx))) {
                    Ok(()) => {
                        if entry.expect_error.is_some() {
                            eprintln!("  FAILED {} (expected error but succeeded)", entry.name);
                            failures.push((entry.name.clone(), "expected error but case succeeded".to_string()));
                        } else {
                            eprintln!("  ok {}", entry.name);
                        }
                    }
                    Err(e) => {
                        let msg = panic_to_string(e);
                        if let Some(ref verifier) = entry.expect_error {
                            if verifier(&msg) {
                                eprintln!("  ok {} (expected error verified)", entry.name);
                            } else {
                                eprintln!("  FAILED {} (wrong error): {}", entry.name, msg);
                                failures.push((entry.name.clone(), format!("wrong error: {}", msg)));
                            }
                        } else {
                            eprintln!("  FAILED {}: {}", entry.name, msg);
                            failures.push((entry.name.clone(), msg));
                        }
                    }
                }
            }

            let passed = total - failures.len();
            eprintln!("  ({}/{} passed)", passed, total);

            if failures.is_empty() {
                Ok(())
            } else {
                let msgs: Vec<String> = failures.iter().map(|(n, m)| format!("  {}: {}", n, m)).collect();
                Err(format!("{} case(s) failed:\n{}", failures.len(), msgs.join("\n")).into())
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
