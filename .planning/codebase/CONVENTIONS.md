# Coding Conventions

**Analysis Date:** 2026-03-04

## Naming Patterns

**Files:**
- Rust source files: `snake_case.rs` (e.g., `agent_tags.rs`, `profiles_init.rs`)
- Command modules: one file per CLI command in `crates/bm/src/commands/` (e.g., `hire.rs`, `teams.rs`, `daemon.rs`)
- Test files: `snake_case.rs` in `crates/bm/tests/` (e.g., `cli_parsing.rs`, `integration.rs`)
- Documentation: `kebab-case.md` in `docs/content/` (e.g., `bootstrap-your-team.md`, `workspace-model.md`)

**Functions:**
- `snake_case` for all functions (Rust standard)
- Public entry points for commands: `pub fn run(...)` or `pub fn list(...)`, `pub fn show(...)`
- Test helpers: descriptive `snake_case` (e.g., `setup_team()`, `claude_code_agent()`, `assert_cmd_success()`)
- Boolean queries: `is_` prefix (e.g., `is_alive()`, `is_local()`)

**Variables:**
- `snake_case` for all variables
- Constants: `UPPER_SNAKE_CASE` (e.g., `CONFIG_PERMISSIONS`, `MAX_LOG_SIZE`, `RELEVANT_EVENTS`, `BM_GITIGNORE_STATIC`)

**Types:**
- `PascalCase` for structs, enums, traits (Rust standard)
- Structs: descriptive nouns (e.g., `BotminterConfig`, `TeamEntry`, `MemberRuntime`, `FormationConfig`)
- Enums: `PascalCase` variants (e.g., `Command::Init`, `TeamsCommand::Sync`)

**Modules:**
- One module per CLI subcommand group in `crates/bm/src/commands/`
- Core modules at `crates/bm/src/` for cross-cutting concerns (`config.rs`, `state.rs`, `profile.rs`, `workspace.rs`)

## Code Style

**Formatting:**
- `rustfmt` (default settings, no `.rustfmt.toml` override)
- No `.editorconfig` file

**Linting:**
- `clippy` with warnings-as-errors: `cargo clippy -p bm -- -D warnings`
- Run via `just clippy`
- Zero TODO/FIXME/HACK/XXX markers in the source tree — the codebase is clean

## Import Organization

**Order:**
1. `std::` imports (grouped by module)
2. External crate imports (`anyhow`, `clap`, `serde`, etc.)
3. Internal crate imports (`crate::config`, `crate::profile`, etc.)
4. `super::` imports (in submodules)

**Example** (from `crates/bm/src/commands/hire.rs`):
```rust
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::config;
use crate::profile;

use super::init::{finalize_member_manifest, run_git};
```

**Path Aliases:**
- None. All imports use explicit `crate::` or `super::` paths.

## Error Handling

**Framework:** `anyhow` for all error propagation.

**Patterns:**
- Use `Result<()>` (from `anyhow`) as the return type for all command functions
- Use `bail!()` for early-return errors with user-facing messages
- Use `.context("descriptive message")?` for wrapping lower-level errors
- Use `.with_context(|| format!("message with {}", var))?` when context needs formatting
- Error messages are user-actionable: include what failed and what to do next

**Example** (from `crates/bm/src/config.rs`):
```rust
pub fn load_from(path: &Path) -> Result<BotminterConfig> {
    if !path.exists() {
        bail!("No teams configured. Run `bm init` first.");
    }
    let contents = fs::read_to_string(path)
        .context("Failed to read config file")?;
    let config: BotminterConfig = serde_yml::from_str(&contents)
        .context("Failed to parse config file")?;
    Ok(config)
}
```

**User-facing error messages:**
- Include the value that failed (e.g., `"Team '{}' not found"`)
- Suggest the fix (e.g., `"Run \`bm init\` first."`)
- List alternatives when applicable (e.g., `"Available formations: {}"`)

## Logging

**Framework:** Direct `println!()` / `eprintln!()` — no logging framework.

**Patterns:**
- `println!()` for normal output (tables, success messages)
- `eprintln!()` for warnings (e.g., config permissions warning in `crates/bm/src/config.rs`)
- No structured logging, no log levels
- Tables use `comfy-table` with UTF-8 rounded corners preset

## Comments

**When to Comment:**
- Doc comments (`///`) on all public functions and structs
- Module-level doc comments (`//!`) at the top of test files explaining scope and constraints
- Section separator comments using Unicode box-drawing characters: `// -- Section Name --` pattern

**Section separators** (used consistently in test files):
```rust
// ── Test helpers ──────────────────────────────────────────────────────
// ── Command aliases (2 tests) ────────────────────────────────────────
```

**Doc comments:**
- Use `///` for public items with a single-line summary
- Multi-line doc comments for complex functions (e.g., `workspace.rs`)
- No `#[doc(hidden)]` usage

## Function Design

**Size:** Functions are generally under 100 lines. The largest files (`workspace.rs` at 1912 lines, `profile.rs` at 1796 lines) contain many small functions, not a few large ones.

**Parameters:**
- Use `Option<&str>` for optional string flags (team name, member name)
- Use `&str` for required string parameters
- Use `&Path` for filesystem paths
- Resolve optional team flags via `config::resolve_team(&cfg, flag)?`

**Return Values:**
- `Result<()>` for commands (success = side effects happened)
- `Result<T>` for data-returning functions
- `Result<Vec<String>>` for list operations

## Module Design

**Exports:**
- `pub mod` declarations in `crates/bm/src/lib.rs` — flat list, one per module
- `pub mod` declarations in `crates/bm/src/commands/mod.rs` — one per command
- No re-exports or barrel files

**Visibility:**
- `pub fn` for functions called from `main.rs` or tests
- `pub(crate)` for internal-only items (e.g., `pub(crate) mod embedded` in `profile.rs`)
- Private functions for module-internal helpers

## CLI Design Patterns

**Clap derive macros:**
- All CLI types use `#[derive(Parser)]` or `#[derive(Subcommand)]`
- Subcommand nesting: `Command::Teams { command: TeamsCommand }` pattern
- Optional team flag: `#[arg(short, long)] team: Option<String>` on every relevant command
- Hidden commands: `#[command(hide = true)]` for internal-only commands like `DaemonRun`
- Aliases: `#[command(alias = "up")]` for `Start`

## Commit Convention

**Format:** `<type>(<scope>): <subject>` — conventional commits.

**Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

**Scope:** Optional. The area of the codebase affected.

**Subject:** Imperative mood, lowercase, no period.

**Reference:** `Ref: #<issue-number>` in commit body when applicable.

**Defined in:** `profiles/scrum/knowledge/commit-convention.md`

## Documentation Writing Conventions

The docs site uses MkDocs Material and lives in `docs/`. Content is in `docs/content/`.

**Build tool:** `zensical` (installed via `docs/requirements.txt`), a MkDocs-compatible tool.

**Serving:** `just docs-serve` (live reload at localhost:8000), `just docs-build` (static build to `docs/site/`).

**Information architecture** (Diataxis):
- `docs/content/getting-started/` — Tutorials (step-by-step walkthroughs)
- `docs/content/concepts/` — Explanation (architecture, models, design rationale)
- `docs/content/how-to/` — How-to guides (task-oriented procedures)
- `docs/content/reference/` — Reference (CLI commands, configuration, process)

**Page structure:**
- H1 title at top (single `#` heading per page)
- No skipped heading levels (H1 -> H2 -> H3)
- First paragraph answers "what is this page about?" immediately
- Cross-references at the bottom in a "Related topics" or "Next steps" section with relative links

**Markdown extensions used:**
- Admonitions: `!!! note`, `!!! warning`, `!!! tip` — for callouts
- Collapsible sections: `???+ example "Title"` or `??? note "Title"` — for optional detail
- Code blocks: triple backticks with language specifier (`bash`, `yaml`)
- Mermaid diagrams: fenced `mermaid` blocks for flowcharts
- Tabbed content: `=== "Tab Name"` for multi-option content (e.g., shell completions)
- Tables: pipe tables for structured data (parameters, comparisons)

**Writing style:**
- Second person ("you") for tutorials and how-to guides
- Present tense throughout
- Active voice preferred
- No emojis in prose (checkmarks allowed in tables)
- Document length target: under 3000 words per page (longest is ~1042 words)
- Concise sentences — avoid filler phrases

**CLI reference format** (in `docs/content/reference/cli.md`):
- Each command gets an H3 heading with inline code: `### \`bm hire\``
- Usage shown as a bash code block
- Parameters in a pipe table with Required/Description columns
- **Behavior:** section with bullet list of what the command does
- Examples with realistic values

**Landing page:**
- `docs/content/index.md` uses a custom template (`home.html` in `docs/overrides/`)
- Custom CSS in `docs/content/stylesheets/home.css`

**Docs quality tracking:**
- `docs/review-report.md` tracks accuracy issues with IDs (e.g., D5.1), status, and resolution

**Config reference:**
- `docs/mkdocs.yml` defines nav structure, theme, and extensions
- Logo: `docs/content/assets/logo-icon-only.png`
- Favicon: `docs/content/assets/fav-icon.png`

---

*Convention analysis: 2026-03-04*
