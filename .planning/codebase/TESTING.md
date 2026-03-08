# Testing Patterns

**Analysis Date:** 2026-03-04

## Test Framework

**Runner:**
- Rust's built-in `cargo test` via `#[test]` attribute
- No external test runner (no `nextest`)
- Config: `Cargo.toml` at `crates/bm/Cargo.toml`

**Assertion Library:**
- Standard `assert!()`, `assert_eq!()`, `assert_ne!()` macros
- No external assertion library

**Run Commands:**
```bash
just test          # cargo test -p bm (unit + integration, excludes e2e)
just e2e           # cargo test -p bm --features e2e -- --test-threads=1
just e2e-verbose   # same with --nocapture for visible output
just clippy        # cargo clippy -p bm -- -D warnings
```

## Test File Organization

**Location:** Mixed — unit tests are co-located within source files, integration and CLI tests are in `crates/bm/tests/`.

**Naming:**
- Unit tests: `#[cfg(test)] mod tests` block at the bottom of each source file
- Integration tests: `crates/bm/tests/integration.rs`
- CLI parsing tests: `crates/bm/tests/cli_parsing.rs`
- E2E tests: `crates/bm/tests/e2e/` (feature-gated, multi-file module)

**Structure:**
```
crates/bm/
  src/
    config.rs          # Contains #[cfg(test)] mod tests { ... }
    state.rs           # Contains #[cfg(test)] mod tests { ... }
    formation.rs       # Contains #[cfg(test)] mod tests { ... }
    commands/
      hire.rs          # Contains #[cfg(test)] mod tests { ... }
  tests/
    cli_parsing.rs     # CLI arg parsing tests (subprocess-based)
    integration.rs     # Multi-command workflow tests (~2000 lines)
    README.md          # Test architecture documentation
    e2e/
      main.rs          # E2E module root with smoke test
      helpers.rs       # Shared E2E helpers (bm_cmd, DaemonGuard, etc.)
      github.rs        # GitHub API helpers (TempRepo, TempProject RAII)
      telegram.rs      # Telegram mock helpers (podman-based)
      init_to_sync.rs  # Full lifecycle E2E tests
      daemon_lifecycle.rs  # Daemon start/stop E2E tests
      start_to_stop.rs     # Member launch E2E tests
```

## Test Structure

**Unit test pattern** (co-located in source files):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptive_test_name() {
        let tmp = tempfile::tempdir().unwrap();
        // Setup
        let result = function_under_test(tmp.path());
        // Assert
        assert_eq!(result, expected);
    }
}
```

**Section separators in test files** — tests are grouped by concern with Unicode line separators:
```rust
// ── Command aliases (2 tests) ────────────────────────────────────────

#[test]
fn start_and_up_are_aliases() { ... }

// ── Flag parsing (5 tests) ───────────────────────────────────────────

#[test]
fn team_flag_short_and_long() { ... }
```

**Integration test pattern** (subprocess-based):
```rust
fn bm() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bm"))
}

#[test]
fn some_cli_behavior() {
    let output = bm().args(["some", "command"]).output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    assert_ne!(code, CLAP_PARSE_ERROR_CODE,
        "`bm some command` should not be a parse error, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
```

## Test Isolation Model

**Critical principle:** No `env::set_var()` — tests never mutate process-global environment variables.

**Filesystem isolation:**
- Every test creates a `tempfile::tempdir()` for its own filesystem tree
- Config files written to `{tmp}/.botminter/config.yml`
- Profiles extracted to `{tmp}/.config/botminter/profiles/`

**Subprocess isolation:**
- Commands that resolve config via `dirs::home_dir()` run as subprocesses
- HOME is overridden per-test via `.env("HOME", tmp.path())`
- XDG_CONFIG_HOME overridden similarly for profile tests

**Port isolation:**
- Webhook/daemon tests use OS-assigned free ports via `get_free_port()`
- `wait_for_port(port, timeout)` polls TCP readiness instead of `thread::sleep()`

**Documented in:** `crates/bm/tests/README.md`

## Test Helpers

**Integration test helpers** (in `crates/bm/tests/integration.rs`):

| Helper | Purpose |
|--------|---------|
| `setup_team(tmp, name, profile)` | Creates team repo + config in a temp directory |
| `bm_cmd(home)` | Creates a `Command` with HOME set for isolation |
| `bm_run(home, args)` | Runs `bm` and asserts success |
| `bm_run_fail(home, args)` | Runs `bm` and asserts failure, returns stderr |
| `bm_hire(home, role, name, team)` | Hires a member via subprocess |
| `bm_sync(home, team)` | Runs `bm teams sync` via subprocess |
| `get_free_port()` | Returns an OS-assigned free port |
| `wait_for_port(port, timeout)` | Polls until TCP port accepts connections |
| `claude_code_agent()` | Returns a `CodingAgentDef` for tests |
| `git(dir, args)` | Runs a git command, asserts success |

**E2E test helpers** (in `crates/bm/tests/e2e/helpers.rs`):

| Helper | Purpose |
|--------|---------|
| `bm_cmd()` | Creates a `Command` for the `bm` binary |
| `assert_cmd_success(cmd)` | Asserts exit 0, returns stdout |
| `assert_cmd_fails(cmd)` | Asserts non-zero exit, returns stderr |
| `is_alive(pid)` | Checks process liveness via `kill(pid, 0)` |
| `force_kill(pid)` | Sends SIGKILL to a process |
| `wait_for_exit(pid, timeout)` | Polls until process exits |
| `DaemonGuard` | RAII guard that stops/cleans up daemon on drop |

**E2E GitHub helpers** (in `crates/bm/tests/e2e/github.rs`):

| Helper | Purpose |
|--------|---------|
| `TempRepo::new_in_org(name, org)` | Creates a temp GitHub repo (RAII, deletes on drop) |
| `TempProject::new(org, title)` | Creates a temp GitHub Project (RAII, deletes on drop) |
| `gh_auth_ok()` | Checks if `gh auth status` succeeds |
| `list_labels(repo)` | Queries labels on a GitHub repo |
| `clean_persistent_repo()` | Resets the persistent E2E test repo |

## Mocking

**Framework:** No mocking framework. Tests use real filesystem and real external services.

**Patterns:**
- **Filesystem mocking:** `tempfile::tempdir()` creates isolated temp directories — all tests write to temp dirs
- **External service mocking:** Telegram mock server via `podman` container (`crates/bm/tests/e2e/telegram.rs`)
- **No in-process mocks:** Functions are tested via their public API with real file I/O

**What to mock:**
- Telegram API: use `tg-mock` podman container for E2E tests
- HOME directory: override via `.env("HOME", tmp_dir)` on subprocess `Command`

**What NOT to mock:**
- GitHub API: E2E tests use real GitHub repos (feature-gated behind `--features e2e`)
- Filesystem: use real temp directories, never mock `fs` operations
- Git operations: use real git repos initialized in temp directories

## Fixtures and Factories

**Test Data:**
```rust
// Config factory pattern (inline construction)
let config = BotminterConfig {
    workzone: PathBuf::from("/tmp/workspaces"),
    default_team: Some("my-team".to_string()),
    teams: vec![TeamEntry {
        name: "my-team".to_string(),
        path: PathBuf::from("/tmp/workspaces/my-team"),
        profile: "scrum".to_string(),
        github_repo: "org/my-team".to_string(),
        credentials: Credentials::default(),
        coding_agent: None,
    }],
};
```

**Team repo setup factory** (integration tests):
```rust
fn setup_team(tmp: &Path, team_name: &str, profile_name: &str) -> PathBuf {
    let profiles_path = profile::profiles_dir_for(tmp);
    fs::create_dir_all(&profiles_path).unwrap();
    profile::extract_embedded_to_disk(&profiles_path).unwrap();
    // ... git init, extract profile, write config ...
    team_repo
}
```

**Location:** Test helpers are defined at the top of each test file, not in shared fixtures directories.

## Coverage

**Requirements:** None enforced. No coverage threshold configured.

**View Coverage:** Not configured. No coverage tool in CI or Justfile.

## Test Types

**Unit Tests:**
- Co-located `#[cfg(test)] mod tests` in source files
- Test individual functions with isolated temp directories
- Examples: round-trip serialization, auto-suffix computation, formation resolution, state management
- Files with unit tests: `crates/bm/src/config.rs`, `crates/bm/src/state.rs`, `crates/bm/src/formation.rs`, `crates/bm/src/commands/hire.rs`

**CLI Parsing Tests** (`crates/bm/tests/cli_parsing.rs`):
- Verify clap argument definitions: aliases, flags, required args, error messages
- Run the `bm` binary as a subprocess
- Check exit codes: `2` = clap parse error, `1` = runtime error (parsing succeeded)
- Fast, parallel execution, no filesystem setup needed

**Integration Tests** (`crates/bm/tests/integration.rs`):
- Multi-command workflows against temp directories
- Test sequences: setup_team -> hire -> sync -> verify workspace structure
- Full filesystem isolation with per-test HOME
- Exercise `bm` library API and subprocess invocations
- ~2000 lines, extensive coverage of workspace sync, multi-team, idempotency

**E2E Tests** (`crates/bm/tests/e2e/`):
- Feature-gated: `--features e2e`
- Run serially: `--test-threads=1` (shared GitHub resources)
- Prerequisites: `gh auth status` must pass, `podman` for Telegram tests
- RAII cleanup: `TempRepo`, `TempProject`, `DaemonGuard` all clean up on drop
- Skip pattern: `require_gh_auth!()` macro skips tests when auth unavailable
- Test organization: `devguyio-bot-squad` org for GitHub operations

## Common Patterns

**Assertion with context message:**
```rust
assert!(
    stdout.contains("expected text"),
    "bm command should show expected text, output:\n{}",
    stdout
);
```

**Exit code checking (CLI parsing tests):**
```rust
const CLAP_PARSE_ERROR_CODE: i32 = 2;

let output = bm().args(["some", "command"]).output().unwrap();
let code = output.status.code().unwrap_or(-1);
assert_ne!(
    code, CLAP_PARSE_ERROR_CODE,
    "`bm some command` should not be a parse error, stderr: {}",
    String::from_utf8_lossy(&output.stderr)
);
```

**Error message assertion:**
```rust
let result = function_under_test();
assert!(result.is_err());
let err = result.unwrap_err().to_string();
assert!(err.contains("expected substring"));
```

**RAII test cleanup (E2E):**
```rust
pub struct DaemonGuard {
    team_name: String,
    home: PathBuf,
    stub_dir: Option<PathBuf>,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        // Graceful stop, then force-kill, then cleanup files
    }
}
```

**Idempotency verification:**
```rust
// Run operation twice, verify same result
let out1 = assert_cmd_success(&mut cmd1);
let out2 = assert_cmd_success(&mut cmd2);
// Verify structure unchanged after double operation
```

## Documentation Testing

**No automated docs testing.** The docs site (`docs/`) has no link checker, spell checker, or build-on-CI validation.

**Manual quality tracking:**
- `docs/review-report.md` tracks accuracy issues found by review agents
- Issues are tracked with IDs (e.g., D5.1) and statuses (Open/Fixed)
- Three independent review agents evaluated 17 pages

**Docs build commands:**
```bash
just docs-build    # Build static docs site to docs/site/
just docs-serve    # Start live-reload dev server at localhost:8000
```

**CI for docs:**
- `.github/workflows/docs.yml` exists (contents not examined, but workflow is present)
- `.github/workflows/release.yml` exists for binary releases

---

*Testing analysis: 2026-03-04*
