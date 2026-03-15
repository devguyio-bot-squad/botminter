# Technology Stack

**Analysis Date:** 2026-03-10

## Languages

**Primary:**
- Rust (2021 edition) - CLI binary (`crates/bm/`), all core logic

**Secondary:**
- Bash - Justfile recipes, stub scripts (`crates/bm/tests/e2e/stub-ralph.sh`), CI workflows
- Python - Documentation site tooling only (`docs/requirements.txt`)
- YAML - Configuration files (`ralph.yml`, `botminter.yml`, `formation.yml`, `config.yml`)
- Markdown - Knowledge files, invariants, profiles, documentation

## Runtime

**Environment:**
- Rust stable (rustc 1.93.1)
- Compiles to native binary (`bm`)
- Unix-only: uses `std::os::unix::fs::PermissionsExt`, `libc` for process management

**Package Manager:**
- Cargo 1.93.1
- Lockfile: `Cargo.lock` present and committed
- Workspace layout: `Cargo.toml` at root, single member `crates/bm/`

## Frameworks

**Core:**
- clap 4.5.60 (`derive` feature) - CLI argument parsing and subcommand dispatch (`crates/bm/src/cli.rs`)
- clap_complete 4.x (`unstable-dynamic` feature) - Dynamic shell completions

**Testing:**
- Built-in Rust test framework - Unit and integration tests (`cargo test -p bm`)
- libtest-mimic 0.8.1 - Custom E2E test harness (`crates/bm/tests/e2e/main.rs`), non-standard `#[test]` macros
- reqwest 0.12.28 (`blocking`, `json` features) - HTTP client for E2E tests only (dev-dependency)

**Build/Dev:**
- just (Justfile) - Task runner for build, test, docs, release (`Justfile`)
- include_dir 0.7.4 - Embeds `profiles/` directory into binary at compile time (`crates/bm/src/profile.rs`)
- cross - Cross-compilation for ARM64 Linux in CI (`.github/workflows/release.yml`)

**Documentation:**
- Zensical (MkDocs fork) - Documentation site generator (`docs/mkdocs.yml`)
- Material for MkDocs theme - Docs styling and features
- GitHub Pages - Docs hosting (`.github/workflows/docs.yml`)

## Key Dependencies

**Critical:**
- serde 1.0.228 + serde_json + serde_yml 0.0.12 - All config/state serialization (YAML and JSON)
- anyhow 1.x - Error handling throughout the codebase
- keyring 3.6.3 (`sync-secret-service` feature) - System keyring for bridge credential storage (`crates/bm/src/bridge.rs`)
- dbus-secret-service 4.1.0 - Direct D-Bus Secret Service access for custom keyring collections (`crates/bm/src/bridge.rs`)
- include_dir 0.7.4 - Compile-time profile embedding (`crates/bm/src/profile.rs`)

**Infrastructure:**
- tiny_http 0.12.0 - Lightweight HTTP server for daemon webhook mode (`crates/bm/src/commands/daemon.rs`)
- hmac 0.12 + sha2 0.10 + hex 0.4 - GitHub webhook signature verification (HMAC-SHA256) (`crates/bm/src/commands/daemon.rs`)
- dirs 5.x - Home directory resolution for `~/.botminter/` (`crates/bm/src/config.rs`)
- which 7.x - Binary existence checks (`claude`, `ralph`, `just`) (`crates/bm/src/session.rs`)
- chrono 0.4 (`serde` feature) - Timestamp generation for state tracking
- tempfile 3.x - Temp files for session prompts and atomic state writes
- libc 0.2 - Unix process management (signal sending, PID checks)

**UI/UX:**
- cliclack 0.3 - Interactive terminal prompts (wizard flow in `crates/bm/src/commands/init.rs`)
- console 0.15 - Terminal styling and colors
- indicatif 0.17 - Progress bars/spinners
- comfy-table 7.x - Tabular output formatting

## Configuration

**Environment:**
- `~/.botminter/config.yml` - Global config (workzone path, teams, credentials), 0600 permissions enforced (`crates/bm/src/config.rs`)
- `~/.botminter/state.json` - Runtime state (member PIDs) (`crates/bm/src/state.rs`)
- `~/.botminter/daemon-<team>.json` - Daemon config per team (`crates/bm/src/commands/daemon.rs`)
- `GH_TOKEN` env var or `gh auth token` - GitHub authentication (auto-detected during `bm init`)
- `TESTS_GH_TOKEN` + `TESTS_GH_ORG` - E2E test credentials
- `.env` file existence noted but contents never read by the tool

**Build:**
- `Cargo.toml` (root) - Workspace definition
- `crates/bm/Cargo.toml` - Binary crate config, feature flags (`e2e`)
- `Justfile` - Development task definitions
- `profiles/` - Embedded at compile time into binary

## Feature Flags

- `e2e` - Gates E2E test compilation (`crates/bm/Cargo.toml` line 34). Without this flag, E2E tests are not built.

## Platform Requirements

**Development:**
- Rust stable toolchain (2021 edition)
- `just` task runner
- `gh` CLI (GitHub CLI) for all GitHub operations
- `claude` binary (Claude Code) for interactive sessions
- `ralph` binary (Ralph Orchestrator) for member launches
- Linux: D-Bus + Secret Service provider (gnome-keyring) for credential storage
- Python 3 for docs development only

**Production (Runtime):**
- Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64 (CI build matrix)
- `gh` CLI must be installed and authenticated
- `claude` and/or `ralph` binaries for member operations
- System keyring (Linux: gnome-keyring with login collection) for bridge credentials

**CI/CD:**
- GitHub Actions (`.github/workflows/release.yml`, `.github/workflows/docs.yml`)
- Release workflow: tag-triggered, builds for 4 targets, uploads tarballs to GitHub Releases

---

*Stack analysis: 2026-03-10*
