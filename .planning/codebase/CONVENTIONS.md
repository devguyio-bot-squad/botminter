# Coding Conventions

**Analysis Date:** 2026-03-10

## Naming Patterns

**Files:**
- Source modules: `snake_case.rs` (e.g., `agent_tags.rs`, `profile.rs`, `workspace.rs`)
- Command modules: one file per CLI subcommand in `crates/bm/src/commands/` (e.g., `hire.rs`, `start.rs`, `teams.rs`)
- Test files: `snake_case.rs` in `crates/bm/tests/` (e.g., `integration.rs`, `cli_parsing.rs`, `conformance.rs`)
- E2E scenarios: descriptive names in `crates/bm/tests/e2e/scenarios/` (e.g., `operator_journey.rs`)

**Functions:**
- Use `snake_case` for all functions
- Public entry points for commands: `pub fn run(...)` or `pub fn list(...)` — verb-first naming
- Helper functions: descriptive verbs (`auto_suffix`, `count_members`, `read_projects`, `setup_git_auth`)
- Builder/factory patterns: `new()`, `new_in_org()`, `from_existing()`

**Variables:**
- Use `snake_case` for all variables and parameters
- Abbreviations kept lowercase: `gh_token`, `gh_org`, `tg_mock`
- Path variables descriptive: `team_repo`, `member_dir`, `workzone`, `config_path`

**Types:**
- Structs and enums: `PascalCase` (e.g., `BotminterConfig`, `TeamEntry`, `RuntimeState`)
- Enum variants: `PascalCase` (e.g., `Endpoint::Local`, `Endpoint::K8s`)
- Trait names: `PascalCase` describing behavior (e.g., `CredentialStore`)

**Constants:**
- Module-level: `SCREAMING_SNAKE_CASE` (e.g., `CONFIG_DIR`, `CONFIG_FILE`, `CONFIG_PERMISSIONS`)
- Test constants: `SCREAMING_SNAKE_CASE` (e.g., `TEAM_NAME`, `PROFILE`, `MEMBER_DIR`)

## Code Style

**Formatting:**
- Default `rustfmt` settings (no `.rustfmt.toml` present)
- 4-space indentation (Rust default)
- Run `cargo fmt` before committing

**Linting:**
- Clippy with warnings-as-errors: `cargo clippy -p bm -- -D warnings`
- Run via `just clippy`
- `#[allow(dead_code)]` used sparingly on test helper functions not yet in use

## Import Organization

**Order:**
1. `std` library imports (grouped by module: `std::fs`, `std::path`, `std::process`, etc.)
2. Blank line
3. External crate imports (`anyhow`, `serde`, `clap`, `comfy_table`, etc.)
4. Blank line
5. Internal crate imports (`crate::config`, `crate::profile`, `crate::workspace`)

**Example from `crates/bm/src/commands/hire.rs`:**
```rust
use std::fs;
use std::io::IsTerminal;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::bridge::{self, CredentialStore};
use crate::config;
use crate::profile;
```

**Path Aliases:**
- No custom path aliases. All imports use `crate::` for internal modules
- Re-exports via `pub mod` in `crates/bm/src/lib.rs` (flat module list)

## Error Handling

**Framework:** `anyhow` for all error propagation

**Patterns:**
- Use `Result<T>` (anyhow's `Result`) as the return type for all fallible functions
- Use `bail!()` for early-return errors with descriptive messages:
  ```rust
  bail!("Role '{}' not available in profile '{}'. Available roles: {}",
      role, team.profile, available_roles.join(", "));
  ```
- Use `.context()` and `.with_context()` to add context to upstream errors:
  ```rust
  fs::read_to_string(&manifest_path)
      .context("Failed to read team repo's botminter.yml")?;
  ```
- Use `.with_context()` for dynamic context strings:
  ```rust
  fs::create_dir_all(dir)
      .with_context(|| format!("Failed to create config directory at {}", dir.display()))?;
  ```
- Include actionable guidance in user-facing errors:
  ```rust
  bail!("No teams configured. Run `bm init` first.");
  bail!("No default team set. Use `-t <team>` or run `bm init` to create a team.");
  ```
- List available options when a lookup fails:
  ```rust
  format!("Team '{}' not found. Available teams: {}", team_name, available.join(", "))
  ```

**Non-fatal warnings:** Use `eprintln!()` for warnings that should not abort execution:
```rust
eprintln!("Warning: Config file {} has permissions {:04o} (expected {:04o}).", ...);
```

## Logging

**Framework:** No logging framework. Uses `println!()` for user output, `eprintln!()` for diagnostics/warnings.

**Patterns:**
- `println!()` for successful operation results shown to the user
- `eprintln!()` for warnings, progress info, and debug output in tests
- No structured logging (no log levels, no log crate)

## Comments

**When to Comment:**
- Module-level doc comments (`//!`) on test files and complex modules:
  ```rust
  //! Custom E2E test harness for the `bm` CLI.
  //!
  //! Uses libtest-mimic to accept custom CLI arguments.
  ```
- Doc comments (`///`) on all public structs, enums, traits, and functions
- Inline comments for non-obvious logic (e.g., `// Safety: kill with signal 0 only checks existence`)
- Section separators using `// -- Section Name --` pattern:
  ```rust
  // -- CredentialStore trait + implementations --
  ```

**Doc Comments:**
- Use `///` for public API documentation
- Include parameter descriptions inline in the doc text, not as separate `@param` tags
- Document what the function does, not how it does it

## Function Design

**Size:** Most functions are under 80 lines. Larger functions (100-200 lines) exist in command modules (`init.rs`, `start.rs`) for multi-step orchestration flows.

**Parameters:**
- Use `&str` / `&Path` for borrowed string/path parameters
- Use `Option<&str>` for optional flags: `fn run(team_flag: Option<&str>)`
- Group related parameters into structs for complex calls:
  ```rust
  pub struct WorkspaceRepoParams<'a> {
      pub team_repo_path: &'a Path,
      pub workspace_base: &'a Path,
      pub member_dir_name: &'a str,
      // ...
  }
  ```

**Return Values:**
- `Result<()>` for commands that succeed or fail
- `Result<T>` for functions returning data
- `Result<Option<T>>` for "might not exist" semantics (e.g., `load()` returning `None` for missing files)

## Module Design

**Exports:**
- `crates/bm/src/lib.rs` is a flat list of `pub mod` declarations (no re-exports, no barrel)
- `crates/bm/src/commands/mod.rs` is a flat list of `pub mod` declarations
- Modules expose public functions and types needed by other modules; internal helpers are private

**Barrel Files:**
- No barrel files. Modules are imported directly: `use crate::config;`

## CLI Design Patterns

**Clap derive macros:** All CLI types use `#[derive(Parser)]` / `#[derive(Subcommand)]`:
```rust
#[derive(Parser)]
#[command(name = "bm", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
```

**Team flag pattern:** All team-scoped commands accept `Option<String>` for `-t`/`--team` and resolve via `config::resolve_team()`.

**Consistent dispatch:** `main.rs` pattern-matches on command enum variants and delegates to `commands::<module>::run()`.

## Serialization

**YAML:** `serde_yml` for config files (`config.yml`, `botminter.yml`, `bridge.yml`, `formation.yml`)
**JSON:** `serde_json` for state files (`state.json`, `topology.json`, bridge state)
**Derive macros:** All serializable types use `#[derive(Serialize, Deserialize)]`
**Optional fields:** Use `#[serde(default, skip_serializing_if = "Option::is_none")]` for optional fields

## File I/O Patterns

**Atomic writes:** State files use temp-file-then-rename:
```rust
let tmp_path = path.with_extension("json.tmp");
fs::write(&tmp_path, contents)?;
fs::rename(&tmp_path, path)?;
```

**Permissions:** Config and state files get `0o600` permissions (owner read/write only):
```rust
let perms = fs::Permissions::from_mode(0o600);
fs::set_permissions(path, perms)?;
```

## Commit Convention

- Format: `<type>(<scope>): <subject>`
- Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- Include `Ref: #<issue-number>` when applicable
- Defined in `profiles/scrum/knowledge/commit-convention.md`

## Constitutional Invariants

All files in `invariants/` are hard constraints. Key invariants:
- `invariants/cli-idempotency.md` — all state-mutating commands MUST be idempotent
- `invariants/test-path-isolation.md` — tests MUST use temp directories, never real HOME
- `invariants/e2e-scenario-coverage.md` — E2E tests MUST be complete user journeys
- `invariants/flaky-tests.md` — flaky tests MUST be root-caused and fixed or tracked
- `invariants/no-hardcoded-profiles.md` — no hardcoded profile names in runtime code
- `invariants/gh-api-e2e.md` — E2E tests use real GitHub APIs

---

*Convention analysis: 2026-03-10*
