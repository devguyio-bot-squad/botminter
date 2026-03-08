# Technology Stack

**Analysis Date:** 2026-03-04

## Languages

**Primary:**
- Rust (2021 edition) - CLI binary (`crates/bm/`), all core logic

**Secondary:**
- Python 3.x - Documentation site tooling only (`docs/`)
- YAML - Configuration files (ralph.yml, botminter.yml, formation.yml, profiles)
- Markdown - Documentation content, knowledge files, invariants
- HTML/CSS/JS - Docs site overrides and custom landing page (`docs/overrides/`, `docs/content/stylesheets/`, `docs/content/js/`)

## Runtime

**Environment:**
- Native binary (compiled Rust, no runtime dependency)
- Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`

**Package Manager:**
- Cargo (Rust) - workspace with `resolver = "2"`
- Lockfile: `Cargo.lock` present and committed
- pip (Python) - docs dependencies only (`docs/requirements.txt`)

## Frameworks

**Core:**
- clap 4 (derive mode) - CLI argument parsing (`crates/bm/src/cli.rs`)
- clap_complete 4 (unstable-dynamic) - Shell completions (`crates/bm/src/completions.rs`)
- serde 1 (derive) - Serialization for all config/state types
- serde_yml 0.0.12 - YAML config parsing (botminter.yml, ralph.yml, formation.yml)
- serde_json 1 - JSON state files (daemon config, topology, poll state)
- anyhow 1 - Error handling throughout

**CLI UX:**
- cliclack 0.3 - Interactive wizard prompts (`crates/bm/src/commands/init.rs`)
- console 0.15 - Terminal styling and colors
- indicatif 0.17 - Progress bars/spinners
- comfy-table 7 - Tabular output formatting

**Testing:**
- cargo test (built-in) - Unit and integration tests
- reqwest 0.12 (dev-dependency, blocking+json) - HTTP client for E2E tests
- tempfile 3 - Temporary directories in tests (also runtime dependency)
- filetime 0.2 (dev-dependency) - File timestamp manipulation in tests

**Build/Dev:**
- just (Justfile) - Task runner (`Justfile` at project root)
- cargo clippy - Linting (warnings treated as errors: `-D warnings`)
- include_dir 0.7 - Compile-time embedding of `profiles/` directory into binary
- cross (via `cross-rs/cross`) - Cross-compilation for Linux ARM64 in CI

**Documentation:**
- Zensical - MkDocs-compatible static site generator (`docs/requirements.txt`)
- Material for MkDocs theme - Docs theme (`docs/mkdocs.yml` theme.name: material)
- pymdownx extensions - Enhanced markdown (superfences, tabbed, highlight, details)
- Mermaid 11 - Diagrams via CDN (`https://unpkg.com/mermaid@11/dist/mermaid.min.js`)

## Key Dependencies

**Critical:**
- clap 4 - Entire CLI surface is built on clap derive macros
- serde + serde_yml - All configuration parsing depends on these
- include_dir 0.7 - Profiles are embedded at compile time; changes to `profiles/` require rebuild
- anyhow 1 - Error propagation strategy for the entire codebase

**Infrastructure:**
- dirs 5 - Home directory resolution for `~/.botminter/` config
- which 7 - Runtime binary detection (claude, ralph, gh)
- libc 0.2 - Unix process management (signals, PIDs for daemon/start/stop)
- chrono 0.4 (serde feature) - Timestamp formatting in state files
- tiny_http 0.12 - Webhook HTTP server in daemon mode (`crates/bm/src/commands/daemon.rs`)
- hmac 0.12 + sha2 0.10 + hex 0.4 - GitHub webhook signature verification

**External Tool Dependencies (runtime, not Cargo):**
- `gh` CLI - GitHub API operations (issues, projects, repos, labels)
- `claude` CLI (Claude Code) - Coding agent sessions (`crates/bm/src/session.rs`)
- `ralph` (Ralph Orchestrator) - Member orchestration (`crates/bm/src/commands/start.rs`)
- `git` - Repository operations (clone, submodule, push)

## Configuration

**Environment:**
- `~/.botminter/config.yml` - Global config (workzone path, registered teams, credentials)
- Config file permissions: `0o600` (restricted read/write)
- `GH_TOKEN` env var - GitHub authentication (detected during `bm init`, stored in config)
- Telegram bot token - Optional, stored in team credentials
- Webhook secret - Optional, for daemon webhook verification

**Build:**
- `Cargo.toml` (workspace root) - Workspace member declaration
- `crates/bm/Cargo.toml` - Binary crate config, version `0.1.0-pre-alpha`
- `Justfile` - Build/test/docs/release tasks
- Feature flags: `e2e` - Enables E2E test compilation (`crates/bm/tests/e2e/`)

**Docs:**
- `docs/mkdocs.yml` - Site config (nav, theme, extensions, custom CSS/JS)
- `docs/requirements.txt` - Python dependency (`zensical`)
- `docs/overrides/home.html` - Custom landing page (375 lines)
- `docs/overrides/main.html` - Base template extension (palette JS injection)
- `docs/content/stylesheets/home.css` - Landing page styles (809 lines)
- `docs/content/js/mermaid-init.js` - Mermaid diagram initialization
- `docs/content/js/palette-version.js` - Theme palette version script

## Platform Requirements

**Development:**
- Rust stable toolchain (2021 edition)
- `just` task runner
- `gh` CLI authenticated
- Python 3.x (for docs only)
- `cargo clippy` must pass with `-D warnings`

**Production:**
- Linux (x86_64 or ARM64) or macOS (x86_64 or ARM64)
- `gh` CLI installed and authenticated
- `claude` CLI (Claude Code) installed
- `ralph` (Ralph Orchestrator) installed
- `git` available in PATH

**CI/CD:**
- GitHub Actions
- Two workflows: `docs.yml` (deploy docs) and `release.yml` (build binaries)

---

*Stack analysis: 2026-03-04*
