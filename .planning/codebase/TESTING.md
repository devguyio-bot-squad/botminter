# Testing Patterns

**Analysis Date:** 2026-03-10

## Test Framework

**Runner:**
- Rust's built-in test framework for unit and integration tests
- `libtest-mimic` 0.8 for E2E tests (custom harness with custom CLI args)
- Config: `[[test]]` entry in `crates/bm/Cargo.toml` with `harness = false` for E2E

**Assertion Library:**
- Standard `assert!()`, `assert_eq!()`, `assert!(result.is_err())` macros
- `assert!(matches!(...))` for enum variant checks
- Custom `assert_cmd_success()` / `assert_cmd_fails()` helpers for CLI subprocess tests

**Run Commands:**
```bash
just unit          # Unit + integration tests (no GitHub token needed)
just conformance   # Bridge spec conformance tests only
just e2e           # E2E tests (requires TESTS_GH_TOKEN + TESTS_GH_ORG)
just e2e-step      # Progressive E2E — one case at a time
just e2e-reset     # Clean up progressive E2E state
just test          # All tests: unit + conformance + e2e
just clippy        # Lint check (not tests, but part of quality gate)
```

**Running a single test:**
```bash
# Unit/integration test by name
cargo test -p bm <test_name>

# Single E2E scenario
cargo test -p bm --features e2e --test e2e -- \
    --gh-token "$TESTS_GH_TOKEN" --gh-org "$TESTS_GH_ORG" \
    <scenario_name> --test-threads=1
```

## Test File Organization

**Location:** Tests are split across three locations:

1. **Co-located unit tests** in `#[cfg(test)] mod tests` blocks within source files
2. **Integration tests** in `crates/bm/tests/` (separate binaries, can use `bm` as library)
3. **E2E tests** in `crates/bm/tests/e2e/` (custom harness, real GitHub)

**Naming:**
- Unit test modules: `mod tests` inside each source file
- Integration test files: descriptive names (`integration.rs`, `cli_parsing.rs`, `conformance.rs`, `bridge_sync.rs`, `profile_roundtrip.rs`)
- E2E files: `main.rs` (harness entry), `helpers.rs`, `github.rs`, `telegram.rs`, `isolated.rs`, `scenarios/`

**Structure:**
```
crates/bm/
  src/
    config.rs        # Contains #[cfg(test)] mod tests { ... }
    topology.rs      # Contains #[cfg(test)] mod tests { ... }
    state.rs         # Contains #[cfg(test)] mod tests { ... }
    commands/hire.rs  # Contains #[cfg(test)] mod tests { ... }
    profile.rs       # Contains #[cfg(test)] mod tests { ... }
    ...
  tests/
    integration.rs      # 3412 lines — multi-command workflow tests
    cli_parsing.rs      # 1295 lines — clap argument validation
    conformance.rs      # 437 lines — bridge spec compliance
    bridge_sync.rs      # 370 lines — bridge sync logic
    profile_roundtrip.rs # 61 lines — profile extract/read cycle
    e2e/
      main.rs           # Custom harness entry point
      helpers.rs        # Shared helpers (GithubSuite, KeyringGuard, etc.)
      github.rs         # TempRepo, TempProject with RAII cleanup
      telegram.rs       # TgMock Podman container helper
      isolated.rs       # Standalone smoke tests
      scenarios/
        mod.rs           # Suite dispatch
        operator_journey.rs  # Main happy path scenario
```

## Test Types

### Unit Tests (co-located)

Scope: Test individual functions and data structures in isolation. Located inside source files.

Pattern: Each source module has a `#[cfg(test)] mod tests` block with focused tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = test_config_path(tmp.path());

        let config = BotminterConfig { /* ... */ };
        save_to(&path, &config).unwrap();
        let loaded = load_from(&path).unwrap();

        assert_eq!(loaded.default_team, Some("my-team".to_string()));
    }

    #[test]
    fn load_missing_config_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let path = test_config_path(tmp.path());
        let result = load_from(&path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("bm init"));
    }
}
```

Common unit test patterns:
- **Round-trip tests:** serialize then deserialize, assert equality
- **Error path tests:** verify specific error messages using `.to_string()` + `assert!(err.contains(...))`
- **Permission tests:** verify files get correct Unix permissions (0o600)
- **Atomic write tests:** verify temp files are cleaned up after rename
- **Edge case tests:** missing files return `None`/empty, gap-filling algorithms, etc.

### Integration Tests (`crates/bm/tests/`)

Scope: Multi-command workflows against temporary directories. Use `bm` as a library (in-process) and as a subprocess.

**Key pattern — `setup_team()` helper** (`crates/bm/tests/integration.rs`):
```rust
fn setup_team(tmp: &Path, team_name: &str, profile_name: &str) -> PathBuf {
    let profiles_path = profile::profiles_dir_for(tmp);
    fs::create_dir_all(&profiles_path).unwrap();
    profile::extract_embedded_to_disk(&profiles_path).unwrap();

    let team_repo = workzone.join(team_name).join("team");
    fs::create_dir_all(&team_repo).unwrap();
    git(&team_repo, &["init", "-b", "main"]);
    // ... extract profile, create dirs, initial commit, save config
    team_repo
}
```

**Subprocess testing pattern** (`crates/bm/tests/cli_parsing.rs`):
```rust
fn bm(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bm"));
    cmd.env("HOME", home);
    cmd.env("XDG_CONFIG_HOME", home.join(".config"));
    cmd
}

#[test]
fn start_and_up_are_aliases() {
    let tmp = tempfile::tempdir().unwrap();
    let start = bm(tmp.path()).args(["start", "--help"]).output().unwrap();
    let up = bm(tmp.path()).args(["up", "--help"]).output().unwrap();
    assert_eq!(start_text, up_text);
}
```

### Conformance Tests (`crates/bm/tests/conformance.rs`)

Scope: Validate bridge spec artifacts (YAML/JSON schema compliance). No command execution.

```rust
#[test]
fn stub_bridge_yml_has_required_fields() {
    let path = stub_dir().join("bridge.yml");
    let val = read_yaml(&path);
    assert_eq!(val["apiVersion"].as_str(), Some("botminter.dev/v1alpha1"));
    assert_eq!(val["kind"].as_str(), Some("Bridge"));
    // ...
}
```

### E2E Tests (`crates/bm/tests/e2e/`)

Scope: Full operator journeys against real GitHub. Require `TESTS_GH_TOKEN` and `TESTS_GH_ORG`.

**Custom harness:** Uses `libtest-mimic` instead of `#[test]` macros. Key differences:
- Standard `--nocapture` does NOT work. Use `eprintln!()` for debug output.
- Custom args: `--gh-token`, `--gh-org`, `--progressive`, `--progressive-reset`
- Feature-gated: requires `--features e2e`
- Tests run against real GitHub (create/delete repos)
- MUST run single-threaded: `--test-threads=1`

## E2E Test Architecture

### GithubSuite Pattern

The core E2E abstraction is `GithubSuite` in `crates/bm/tests/e2e/helpers.rs`. It provides:
- Shared context (`SuiteCtx`) with GitHub token, org, repo name, and HOME dir
- Sequential case execution with independent pass/fail tracking
- Progressive mode (step through one case at a time across runs)
- RAII cleanup of GitHub repos via `TempRepo`
- Isolated keyring via `KeyringGuard` (D-Bus + gnome-keyring-daemon)

**Building a scenario:**
```rust
pub fn scenario(config: &E2eConfig) -> Trial {
    let repo_name = format!("{}/bm-e2e-journey-{}", config.gh_org, timestamp());

    GithubSuite::new_self_managed("scenario_operator_journey", &repo_name)
        .setup(|ctx| {
            // Setup runs BEFORE keyring isolation (podman needs real D-Bus)
            install_stub_ralph(&ctx.home);
            // Optional: start tg-mock container
        })
        .case("init_with_bridge", |ctx| {
            let mut cmd = bm_cmd();
            cmd.args(["init", "--non-interactive", ...])
               .env("HOME", &ctx.home)
               .env("GH_TOKEN", &ctx.gh_token);
            let stdout = assert_cmd_success(&mut cmd);
            // Assert expected state
        })
        .case("hire_member", |ctx| { /* ... */ })
        .case("teams_sync", |ctx| { /* ... */ })
        .case("start_member", |ctx| { /* ... */ })
        .case("stop_member", |ctx| { /* ... */ })
        .case("cleanup", |ctx| { /* ... */ })
        .build(config)
}
```

**Progressive mode:**
```rust
pub fn scenario_progressive(config: &E2eConfig) -> Trial {
    // Same suite definition
    GithubSuite::new_self_managed(...)
        .setup(...)
        .case(...)
        .build_progressive(config)  // <-- only difference
}
```

### SuiteCtx (Shared Context)

```rust
pub struct SuiteCtx {
    pub gh_token: String,
    pub gh_org: String,
    pub repo_full_name: String,
    pub home: PathBuf,
}
```

All cases in a suite share this context. The `home` directory is isolated per suite.

### Case Groups

Cases can be grouped so they run together in progressive mode:
```rust
.case("start_and_verify", |ctx| { /* ... */ })
.case("check_status", |ctx| { /* ... */ })
.group(3, 4)  // These two cases always run together
```

### Expected Errors

Cases can declare expected panics:
```rust
.case_expect_error("duplicate_hire_fails", |ctx| {
    // This should panic
    assert_cmd_success(&mut cmd);
}, |msg| msg.contains("already exists"))
```

## RAII Test Resources

### TempRepo (`crates/bm/tests/e2e/github.rs`)

Creates a private GitHub repo on construction, deletes on drop:
```rust
let repo = TempRepo::new_in_org("bm-e2e-smoke", &org)?;
// ... use repo.full_name ...
// Repo auto-deleted when `repo` goes out of scope
```

### TempProject (`crates/bm/tests/e2e/github.rs`)

Creates a GitHub Project (v2), deletes on drop.

### KeyringGuard (`crates/bm/tests/e2e/helpers.rs`)

Creates an isolated D-Bus session + gnome-keyring-daemon. Restores original env vars on drop:
```rust
let _guard = KeyringGuard::new();
// All keyring operations go to isolated instance
// Cleaned up when _guard drops
```

### ProcessGuard / DaemonGuard (`crates/bm/tests/e2e/helpers.rs`)

RAII guards that stop processes on drop:
```rust
let mut guard = ProcessGuard::new(&home, TEAM_NAME);
// ... start process ...
guard.set_pid(pid);
// Process killed on drop
```

## Test Helpers

**Command helpers** (`crates/bm/tests/e2e/helpers.rs`):
```rust
// Get a Command for the bm binary with isolated D-Bus
pub fn bm_cmd() -> Command

// Run command and assert success, return stdout
pub fn assert_cmd_success(cmd: &mut Command) -> String

// Run command and assert failure, return stderr
pub fn assert_cmd_fails(cmd: &mut Command) -> String
```

**Setup helpers:**
```rust
// Write .gitconfig for test git operations
pub fn setup_git_auth(home: &Path)

// Install stub-ralph.sh as "ralph" in PATH
pub fn install_stub_ralph(home: &Path)

// Get PATH with stub ralph prepended
pub fn path_with_stub(home: &Path) -> String

// Extract embedded profiles to temp dir
pub fn bootstrap_profiles_to_tmp(tmp: &Path) -> PathBuf
```

**Integration test helpers** (`crates/bm/tests/integration.rs`):
```rust
// Set up a team repo programmatically (bypasses interactive wizard)
fn setup_team(tmp: &Path, team_name: &str, profile_name: &str) -> PathBuf

// Run a git command in a directory
fn git(dir: &Path, args: &[&str])
```

## Stub Ralph

E2E tests use a stub shell script (`crates/bm/tests/e2e/stub-ralph.sh`) instead of real Ralph to avoid Claude API calls. The stub:
- Writes received environment variables to `.ralph-stub-env` for assertion
- Sleeps indefinitely (simulating a running Ralph process)
- Responds to SIGTERM for clean shutdown

## Telegram Mock

E2E tests use `tg-mock` (a Podman container) to mock the Telegram Bot API:
```rust
if telegram::podman_available() {
    let mock = telegram::TgMock::start();
    mock.inject_message(token, "hello", chat_id);
    let requests = mock.get_requests(token, "sendMessage");
}
```

Tests gracefully skip Telegram verification if Podman is not available.

## Mocking Patterns

**No mock framework.** The project uses these strategies instead:

1. **Trait-based injection** for credential stores:
   ```rust
   pub trait CredentialStore {
       fn store(&self, member_name: &str, token: &str) -> Result<()>;
       fn retrieve(&self, member_name: &str) -> Result<Option<String>>;
       // ...
   }

   // Production: LocalCredentialStore (system keyring)
   // Tests: InMemoryCredentialStore
   pub struct InMemoryCredentialStore {
       tokens: std::sync::Mutex<HashMap<String, String>>,
   }
   ```

2. **Closure injection** for testability:
   ```rust
   // Production
   pub fn interactive_claude_session(...) -> Result<()> {
       interactive_claude_session_with_check(..., |name| which::which(name).map(|_| ()))
   }

   // Internal: accepts check function
   fn interactive_claude_session_with_check<F>(..., check_binary: F) -> Result<()>
   where F: FnOnce(&str) -> Result<(), which::Error> { ... }
   ```

3. **Stub binaries** for external processes (stub-ralph.sh)

4. **Real external services** for E2E (GitHub API, tg-mock container)

**What to mock:**
- System keyring (use `InMemoryCredentialStore` or `KeyringGuard` for isolation)
- Ralph orchestrator binary (use stub-ralph.sh)
- Telegram Bot API (use tg-mock Podman container)

**What NOT to mock:**
- GitHub API in E2E tests (invariant: `invariants/gh-api-e2e.md`)
- File system operations (use real temp directories)
- Git operations (use real git with temp repos)

## Test Isolation

**Critical invariant** (`invariants/test-path-isolation.md`): Tests MUST NOT use real user directories.

**Patterns:**
- Unit tests: `tempfile::tempdir()` for file system operations, explicit path APIs (`save_to()`, `load_from()`)
- Integration tests: `tempfile::tempdir()` + subprocess `HOME` override
- E2E tests: `tempfile::tempdir()` for HOME, isolated D-Bus for keyring

```rust
// Unit test — explicit path, no env var mutation
let tmp = tempfile::tempdir().unwrap();
let path = tmp.path().join(".botminter").join("config.yml");
save_to(&path, &config).unwrap();
let loaded = load_from(&path).unwrap();

// Integration test — subprocess with HOME override
let tmp = tempfile::tempdir().unwrap();
let mut cmd = Command::new(env!("CARGO_BIN_EXE_bm"));
cmd.env("HOME", tmp.path());

// E2E test — full HOME isolation + keyring isolation
let home_td = tempfile::tempdir().unwrap();
let _keyring_guard = KeyringGuard::new();
bm_cmd().env("HOME", &home).args([...]);
```

## Coverage

**Requirements:** None enforced programmatically. Coverage targets are implicit through the invariant system:
- Every happy path feature must have E2E scenario coverage (`invariants/e2e-scenario-coverage.md`)
- Unit tests cover internal logic (round trips, edge cases, error paths)
- Integration tests cover multi-command workflows

## Test Counts

Approximate counts (as of 2026-03-10):
- ~576 unit + integration tests (run via `just unit`)
- ~20 E2E tests (run via `just e2e`)
- Conformance tests for bridge spec compliance

## Common Patterns

**Round-trip testing:**
```rust
#[test]
fn save_and_load_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("file.json");
    let data = create_sample_data();
    save(&path, &data).unwrap();
    let loaded = load(&path).unwrap().unwrap();
    assert_eq!(loaded.field, data.field);
}
```

**Error message testing:**
```rust
#[test]
fn missing_config_gives_helpful_error() {
    let result = load_from(&nonexistent_path);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("bm init"));
}
```

**Subprocess CLI testing:**
```rust
#[test]
fn command_fails_with_useful_error() {
    let tmp = tempfile::tempdir().unwrap();
    let output = bm(tmp.path())
        .args(["hire", "architect"])
        .output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No teams configured"));
}
```

**Idempotency testing (E2E invariant):**
```rust
// Run the same command twice — second run must not error
assert_cmd_success(&mut bm_cmd().args(["init", ...]));
assert_cmd_success(&mut bm_cmd().args(["init", ...]));  // Must succeed again
```

---

*Testing analysis: 2026-03-10*
