# External Integrations

**Analysis Date:** 2026-03-10

## APIs & External Services

**GitHub API (via `gh` CLI):**
- Primary coordination fabric for all team operations
- Used for: repo creation, label management, GitHub Projects (v2), issue tracking, milestone management, PR operations, user/org listing, webhook event polling
- Client: `gh` CLI (shelled out via `std::process::Command`)
- Auth: `GH_TOKEN` env var or `gh auth token` (stored in `~/.botminter/config.yml` as `credentials.gh_token`)
- Key files:
  - `crates/bm/src/commands/init.rs` - Repo creation, label bootstrap, project board setup
  - `crates/bm/src/workspace.rs` - Repo view/create for workspace provisioning
  - `crates/bm/src/commands/daemon.rs` - Event polling via `gh api`

**GitHub Webhooks (Incoming):**
- Daemon webhook mode listens on configurable port (default 8484) via `tiny_http`
- Accepts GitHub webhook POST requests at `/webhook`
- Verifies HMAC-SHA256 signatures using `webhook_secret` from config
- Relevant events: `issues`, `issue_comment`, `pull_request`
- Files: `crates/bm/src/commands/daemon.rs` (lines 354-430)

**GitHub Event Polling (Alternative):**
- Daemon poll mode uses `gh api` to poll for events at configurable intervals
- Stores last event ID in `~/.botminter/daemon-<team>-poll.json`
- Files: `crates/bm/src/commands/daemon.rs`

**Telegram Bot API:**
- Bridge plugin for team communication
- Managed via bridge manifest system in profiles (`profiles/scrum/bridges/telegram/`, `profiles/scrum-compact/bridges/telegram/`)
- Identity management: `bm bridge identity add/rotate/remove`
- Room management: `bm bridge room create/list`
- Bot tokens stored in system keyring (not config files)
- Files: `crates/bm/src/bridge.rs` (bridge abstraction), `crates/bm/src/commands/bridge.rs`

**Claude Code (Coding Agent):**
- Launched as subprocess for interactive chat sessions (`bm chat`)
- Binary name: `claude`
- Uses `--append-system-prompt-file` flag with temp file
- Files: `crates/bm/src/session.rs` (lines 10-66)

**Ralph Orchestrator (Process Manager):**
- Launched as subprocess for autonomous member operation (`bm start`)
- Binary name: `ralph`
- Uses `ralph run -p <prompt_path>` command
- Env var `CLAUDECODE` is explicitly removed to avoid nested-Claude issues
- Files: `crates/bm/src/session.rs` (lines 68-122)

## Data Storage

**Databases:**
- None. All state is file-based.

**File Storage (Local Filesystem):**
- `~/.botminter/config.yml` - Global configuration (YAML, 0600 permissions)
- `~/.botminter/state.json` - Runtime state (member PIDs, workspaces)
- `~/.botminter/daemon-<team>.json` - Daemon config per team
- `~/.botminter/daemon-<team>-poll.json` - Poll mode state
- `<workzone>/<team>/team/` - Team repo (git repository, control plane)
- `<workzone>/<team>/<member>/` - Member workspaces (provisioned by `bm teams sync`)

**Git Repositories:**
- Team repo: GitHub-hosted, cloned locally to `<workzone>/<team>/team/`
- Project repos: Added as git submodules in member workspaces
- All coordination state (issues, labels, milestones, PRs) lives on GitHub

**Caching:**
- None

## Authentication & Identity

**GitHub Auth:**
- Token auto-detected from `GH_TOKEN` env var or `gh auth token` during `bm init`
- Validated via `gh api user` before proceeding
- Stored in `~/.botminter/config.yml` with 0600 permissions
- Passed to all `gh` CLI calls and Ralph instances at runtime
- Files: `crates/bm/src/commands/init.rs` (lines 1012-1018)

**Bridge Credentials (Keyring):**
- System keyring via `keyring` crate with `sync-secret-service` feature
- Service name pattern: `botminter-<team>-telegram`
- Supports custom keyring collection via `keyring_collection` config field
- Falls back to `dbus-secret-service` crate for direct D-Bus access when custom collection specified
- Linux requires: D-Bus session bus + Secret Service provider (gnome-keyring) with initialized `login` collection
- Files: `crates/bm/src/bridge.rs` (LocalCredentialStore, lines 76-340)

**Webhook Verification:**
- HMAC-SHA256 signature verification for incoming GitHub webhooks
- Secret stored as `credentials.webhook_secret` in config
- Files: `crates/bm/src/commands/daemon.rs` (lines 829-830)

## Monitoring & Observability

**Error Tracking:**
- None (no external service)

**Logs:**
- Daemon logs to `~/.botminter/daemon-<team>.log` with 10MB rotation (`crates/bm/src/commands/daemon.rs`)
- Member output: stdout/stderr inherited from Ralph subprocess
- CLI output: `eprintln!` for warnings, `println!` for info
- No structured logging framework

## CI/CD & Deployment

**Hosting:**
- GitHub Releases - Binary distribution (tar.gz per platform)
- GitHub Pages - Documentation site

**CI Pipeline:**
- GitHub Actions
- `.github/workflows/release.yml` - Tag-triggered release builds (4 platform matrix: linux x86_64, linux arm64, macos x86_64, macos arm64)
- `.github/workflows/docs.yml` - Push-to-main triggered docs deployment via Zensical/MkDocs

**Release Process:**
- `just release <version> <notes_file>` - Version bump, tag, push, GitHub release creation
- CI builds binaries and attaches to the release
- `just release-build-local <tag>` - Fallback for manual binary attachment

## Environment Configuration

**Required env vars (runtime):**
- `GH_TOKEN` (or `gh auth token` in PATH) - GitHub authentication

**Required env vars (E2E testing):**
- `TESTS_GH_TOKEN` - GitHub token for test operations
- `TESTS_GH_ORG` - GitHub org for test repo creation

**Optional env vars:**
- `CLAUDECODE` - Must be unset when launching nested Claude Code instances

**Secrets location:**
- GitHub token: `~/.botminter/config.yml` (file-based, 0600 permissions)
- Bridge tokens: System keyring (gnome-keyring / macOS Keychain)
- Webhook secret: `~/.botminter/config.yml`

## Webhooks & Callbacks

**Incoming:**
- `/webhook` - GitHub webhook endpoint (daemon webhook mode, `tiny_http` on configurable port)
- Accepts: `issues`, `issue_comment`, `pull_request` events
- Verification: HMAC-SHA256 via `X-Hub-Signature-256` header

**Outgoing:**
- Telegram Bot API calls (via bridge plugin, delegated to `just` recipes in bridge directory)
- GitHub API calls via `gh` CLI (issue creation, label management, project board updates)

## External Tool Dependencies

**Required at Runtime:**
- `gh` (GitHub CLI) - All GitHub operations, must be in PATH
- `git` - Repository operations (clone, submodule, push)
- `claude` (Claude Code) - Interactive chat sessions (`bm chat`, `bm minty`)
- `ralph` (Ralph Orchestrator) - Autonomous member operation (`bm start`)

**Optional at Runtime:**
- `just` - Bridge lifecycle management (start/stop/health recipes)
- `podman` - Telegram mock server in E2E tests

**Build-time Only:**
- `cargo` - Rust build system
- `cross` - Cross-compilation (CI only)
- `python3` + `zensical` - Documentation site

---

*Integration audit: 2026-03-10*
