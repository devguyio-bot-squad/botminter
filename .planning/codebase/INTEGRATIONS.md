# External Integrations

**Analysis Date:** 2026-03-04

## APIs & External Services

**GitHub API (via `gh` CLI):**
- Used for all team coordination: issues, labels, milestones, PRs, project boards
- SDK/Client: `gh` CLI (shelled out via `std::process::Command`)
- Auth: `GH_TOKEN` env var or `gh auth token` (auto-detected during `bm init`)
- Operations: repo creation, label bootstrapping, project board setup (v2 with Status field), issue CRUD, org listing, user validation
- Key files: `crates/bm/src/commands/init.rs`, `crates/bm/src/commands/daemon.rs`

**GitHub Webhooks:**
- Daemon mode receives webhook events from GitHub
- Server: `tiny_http` 0.12 (`crates/bm/src/commands/daemon.rs`)
- Verification: HMAC-SHA256 signature validation (`hmac`, `sha2`, `hex` crates)
- Relevant events: `issues`, `issue_comment`, `pull_request`
- Webhook secret stored in `~/.botminter/config.yml` under team credentials

**GitHub Events API (polling):**
- Alternative to webhooks: daemon poll mode queries GitHub events API
- Uses `gh api` CLI for polling
- Poll state persisted at `~/.botminter/daemon-{team}-poll.json`

**Claude Code (Anthropic):**
- Coding agent for interactive sessions and member orchestration
- Binary: `claude` CLI (detected via `which::which`)
- Invocation: `crates/bm/src/session.rs` - spawns `claude --print` with skill content as prompt
- Used by: `bm chat`, `bm minty` commands
- No direct API calls - all interaction via CLI binary

**Ralph Orchestrator:**
- Member orchestration runtime - each team member runs as a Ralph instance
- Binary: `ralph` CLI (detected via `which::which`)
- Invocation: `crates/bm/src/commands/start.rs` - spawns ralph processes per member
- Config: `ralph.yml` per member workspace
- Source: https://github.com/mikeyobrien/ralph-orchestrator/
- Local checkout at `/opt/workspace/ralph-orchestrator` (development dependency)

**Telegram Bot API (optional):**
- Optional notification integration for team members
- Token stored in credentials: `telegram_bot_token` in `~/.botminter/config.yml`
- Profile variant: `profiles/scrum-compact-telegram/` for Telegram-enabled teams
- E2E tests include Telegram mock server: `crates/bm/tests/e2e/telegram.rs`

## Data Storage

**Databases:**
- None - all state is file-based

**File Storage (local filesystem):**
- Global config: `~/.botminter/config.yml` (YAML, 0o600 permissions)
- Daemon state: `~/.botminter/daemon-{team}.json`, `~/.botminter/daemon-{team}.pid`
- Poll state: `~/.botminter/daemon-{team}-poll.json`
- Runtime state: `{workzone}/{team}/runtime-state.json` (`crates/bm/src/state.rs`)
- Topology: `{workzone}/{team}/topology.json` (`crates/bm/src/topology.rs`)
- Logs: `~/.botminter/logs/daemon-{team}.log`, `~/.botminter/logs/member-{team}-{member}.log`
- Log rotation: 10 MB max per log file

**Team Repos (git):**
- Team coordination data lives in git repos on GitHub
- Local clones at `{workzone}/{team}/team/`
- Member workspaces at `{workzone}/{team}/{member}/`
- Project forks as git submodules within member workspaces

**Caching:**
- None

## Authentication & Identity

**GitHub Auth:**
- Primary auth mechanism for all team operations
- Token sources (checked in order during `bm init`):
  1. `GH_TOKEN` environment variable
  2. `gh auth token` CLI output
- Validated via `gh api user` before proceeding
- Stored in `~/.botminter/config.yml` per team under `credentials.gh_token`
- Passed to member processes as `GH_TOKEN` env var at runtime

**Webhook Auth:**
- Optional `webhook_secret` per team in credentials
- HMAC-SHA256 verification of GitHub webhook payloads
- Implementation: `hmac` + `sha2` + `hex` crates in `crates/bm/src/commands/daemon.rs`

**No user auth system:**
- Single-user CLI tool - no login/session/user management
- Relies entirely on GitHub token for authorization

## Monitoring & Observability

**Error Tracking:**
- None (no external error tracking service)

**Logs:**
- File-based logging to `~/.botminter/logs/`
- Per-daemon logs: `daemon-{team}.log`
- Per-member logs: `member-{team}-{member}.log`
- Log rotation at 10 MB
- No structured logging framework - uses direct file writes

**Status:**
- `bm status` command provides runtime dashboard
- Reads from `runtime-state.json` and `topology.json`

## CI/CD & Deployment

**Hosting:**
- GitHub Pages - documentation site
- GitHub Releases - binary distribution

**CI Pipeline (GitHub Actions):**

**Documentation (`docs.yml`):**
- Trigger: push to `master` or `main`
- Steps: checkout, Python setup, install zensical, build site, deploy to GitHub Pages
- Uses: `actions/configure-pages@v5`, `actions/upload-pages-artifact@v3`, `actions/deploy-pages@v4`
- Permissions: `contents: read`, `pages: write`, `id-token: write`

**Release (`release.yml`):**
- Trigger: push tag `v*` or `workflow_dispatch`
- Matrix build: 4 targets (Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64)
- Cross-compilation: `cross-rs/cross` for Linux ARM64
- Artifact: `bm-{target}.tar.gz` attached to GitHub Release
- Uses: `dtolnay/rust-toolchain@stable`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`

**Local Release (`just release`):**
- Justfile recipe for creating GitHub releases with signed tags
- Updates `Cargo.toml` version, creates signed git tag, pushes, creates release via `gh release create`
- Fallback: `just release-build-local` for manual binary attachment

## Environment Configuration

**Required env vars (runtime):**
- `GH_TOKEN` - GitHub authentication (auto-detected or stored in config)

**Optional env vars:**
- `CLAUDECODE` - Must be unset when launching nested Claude Code sessions (Claude-inside-Claude safety)
- Telegram bot token - stored in config, not env var

**Config file locations:**
- `~/.botminter/config.yml` - Global config with teams, credentials, workzone path
- `{workspace}/ralph.yml` - Per-member Ralph orchestrator config
- `{workspace}/PROMPT.md` - Per-member system prompt
- `{workspace}/CLAUDE.md` - Per-member Claude Code instructions

**Secrets location:**
- `~/.botminter/config.yml` - Contains `GH_TOKEN`, optional `telegram_bot_token`, optional `webhook_secret`
- File permissions: `0o600` (owner read/write only)
- Never committed to git

## Webhooks & Callbacks

**Incoming:**
- GitHub webhook receiver in daemon webhook mode
- Server: `tiny_http` on configurable port
- Endpoint: listens for `issues`, `issue_comment`, `pull_request` events
- Verification: HMAC-SHA256 signature check against stored webhook secret
- Handler: triggers member launches based on event type (`crates/bm/src/commands/daemon.rs`)

**Outgoing:**
- None directly from `bm` CLI
- Members may interact with GitHub via `gh` CLI during their Ralph sessions

## Kubernetes (Future/Partial)

**Formation Support:**
- K8s formation type defined in `crates/bm/src/formation.rs`
- Config model: context, image, namespace_prefix
- Topology model supports K8s endpoints: namespace, pod, container, context (`crates/bm/src/topology.rs`)
- Non-local formations delegate to a formation manager (Ralph session with deployment skills)
- Status: data models present, full implementation in progress

---

*Integration audit: 2026-03-04*
