# Architecture

**Analysis Date:** 2026-03-04

## Pattern Overview

**Overall:** CLI-driven GitOps team management with compile-time embedded profiles and a two-layer runtime model (inner: Ralph Orchestrator instances, outer: GitHub Issues coordination).

**Key Characteristics:**
- Single Rust binary (`bm`) with profiles compiled in via `include_dir`
- Profile-based generation: profiles stamp out team repos; team repos are the runtime control plane
- Two-layer runtime: Ralph Orchestrator instances (inner loop) coordinate through GitHub Issues (outer loop)
- Workspace model uses git submodules to link team repo and project forks into member workspaces
- Event-driven daemon supports webhook and poll modes for automated member launches
- Coding-agent-agnostic design via agent tag filtering (`+agent:NAME` / `-agent` blocks)

## Layers

**CLI Layer (clap):**
- Purpose: Parse commands, dispatch to command handlers
- Location: `crates/bm/src/cli.rs`
- Contains: `Cli` struct with `Command` enum and nested subcommand enums (`TeamsCommand`, `MembersCommand`, `ProfilesCommand`, `ProjectsCommand`, `KnowledgeCommand`, `DaemonCommand`)
- Depends on: Nothing (pure definitions)
- Used by: `crates/bm/src/main.rs`

**Command Handlers:**
- Purpose: Implement each CLI command's business logic
- Location: `crates/bm/src/commands/`
- Contains: One module per command group
- Key files:
  - `crates/bm/src/commands/init.rs` — Interactive wizard (cliclack TUI) for team creation
  - `crates/bm/src/commands/hire.rs` — Add members to team from role definitions
  - `crates/bm/src/commands/teams.rs` — List, show, sync teams
  - `crates/bm/src/commands/start.rs` — Launch Ralph instances for all members
  - `crates/bm/src/commands/stop.rs` — Stop running Ralph instances
  - `crates/bm/src/commands/status.rs` — Status dashboard
  - `crates/bm/src/commands/daemon.rs` — Event-driven daemon (webhook/poll)
  - `crates/bm/src/commands/chat.rs` — Interactive session with a member
  - `crates/bm/src/commands/minty.rs` — Launch Minty assistant
  - `crates/bm/src/commands/knowledge.rs` — Knowledge/invariant management
  - `crates/bm/src/commands/profiles.rs` — List/describe profiles
  - `crates/bm/src/commands/profiles_init.rs` — Extract embedded profiles to disk
  - `crates/bm/src/commands/projects.rs` — Project management
  - `crates/bm/src/commands/members.rs` — Member listing/details
  - `crates/bm/src/commands/roles.rs` — Role listing
  - `crates/bm/src/commands/completions.rs` — Shell completion generation
- Depends on: config, profile, state, formation, topology, workspace, chat, session, agent_tags
- Used by: `main.rs`

**Configuration Layer:**
- Purpose: Persistent config at `~/.botminter/config.yml`
- Location: `crates/bm/src/config.rs`
- Contains: `BotminterConfig`, `TeamEntry`, `Credentials` structs; load/save/resolve functions
- Key data: workzone path, default team, team entries (name, path, profile, github_repo, credentials, coding_agent)
- Depends on: dirs, serde_yml
- Used by: All command handlers

**Profile Layer:**
- Purpose: Embed, extract, parse, and filter methodology profiles
- Location: `crates/bm/src/profile.rs` (largest file, ~69K)
- Contains: Profile parsing (botminter.yml), role/member definitions, schema version checking, profile extraction with agent tag filtering, coding agent resolution
- Key abstraction: Profiles are embedded at compile time via `include_dir!("$CARGO_MANIFEST_DIR/../../profiles")`
- Depends on: agent_tags, serde, include_dir
- Used by: init, hire, start, teams

**Agent Tags Layer:**
- Purpose: Filter profile content for the resolved coding agent
- Location: `crates/bm/src/agent_tags.rs`
- Contains: Line-based `+agent:NAME` / `-agent` tag parser supporting HTML (`<!-- -->`) and hash (`#`) comment syntax
- Depends on: Nothing (pure functions)
- Used by: profile

**Workspace Layer:**
- Purpose: Create and manage member workspace repos with submodules
- Location: `crates/bm/src/workspace.rs` (largest file, ~71K)
- Contains: Workspace creation (local or GitHub-backed), submodule management, file surfacing (copying/symlinking from team repo to workspace root), gitignore management
- Depends on: profile (CodingAgentDef)
- Used by: teams sync

**State Layer:**
- Purpose: Track running Ralph process PIDs
- Location: `crates/bm/src/state.rs`
- Contains: `RuntimeState`, `MemberRuntime` structs; stored at `~/.botminter/state.json`
- Depends on: config
- Used by: start, stop, status

**Formation Layer:**
- Purpose: Define deployment topologies (local processes vs k8s)
- Location: `crates/bm/src/formation.rs`
- Contains: `FormationConfig`, `K8sConfig`, `ManagerConfig`; loaded from `formations/<name>/formation.yml` in team repo
- Depends on: Nothing external
- Used by: start

**Topology Layer:**
- Purpose: Track where members are running across formations
- Location: `crates/bm/src/topology.rs`
- Contains: `Topology`, `MemberTopology`, `Endpoint` (Local or K8s variants); stored at `{workzone}/{team}/topology.json`
- Depends on: Nothing external
- Used by: start, stop, status

**Chat/Session Layer:**
- Purpose: Build meta-prompts and launch interactive Claude Code sessions
- Location: `crates/bm/src/chat.rs`, `crates/bm/src/session.rs`
- Contains: Meta-prompt assembly (role identity, hat instructions, guardrails, references), Claude Code process spawning
- Depends on: which
- Used by: chat, minty commands

**Completions Layer:**
- Purpose: Dynamic shell completions with real team/member/role data
- Location: `crates/bm/src/completions.rs`
- Contains: clap_complete integration with runtime data lookups
- Depends on: clap_complete, config
- Used by: completions command, main.rs (CompleteEnv)

## Data Flow

**Team Creation (bm init):**

1. Ensure profiles exist on disk (`~/.config/botminter/profiles/`)
2. Interactive wizard collects: workzone, team name, profile, GitHub org/repo, credentials
3. Create GitHub repo via `gh repo create`
4. Extract profile content to team repo, filtering agent tags for resolved coding agent
5. Bootstrap GitHub labels and Project board from profile's PROCESS.md
6. Register team in `~/.botminter/config.yml`

**Member Launch (bm start):**

1. Load config, resolve team, verify schema version
2. Resolve formation (local default or named formation)
3. For each hired member: find workspace, build Ralph launch command
4. Spawn `ralph` process per member with appropriate env vars (GH_TOKEN, etc.)
5. Record PIDs in `~/.botminter/state.json` and topology in `topology.json`

**Workspace Sync (bm teams sync):**

1. Load team config, scan `members/` dir in team repo
2. For each member: create workspace repo if missing, update submodules, surface files (PROMPT.md, CLAUDE.md, ralph.yml, agent dirs/skills)
3. Optionally push team repo to GitHub first

**Event-Driven Daemon (bm daemon start):**

1. Fork a background process (`bm daemon-run`)
2. In webhook mode: listen on HTTP port for GitHub webhook events
3. In poll mode: periodically query GitHub API for new events
4. On relevant events (issues, issue_comment, pull_request): launch affected members' Ralph instances

**State Management:**
- Config: `~/.botminter/config.yml` (YAML, 0600 permissions)
- Runtime PIDs: `~/.botminter/state.json` (JSON)
- Topology: `{workzone}/{team}/topology.json` (JSON, 0600 permissions)
- Daemon state: `~/.botminter/daemon-{team}.json` and `daemon-{team}-poll.json`

## Key Abstractions

**Profile:**
- Purpose: Methodology template (scrum, scrum-compact) containing process, roles, knowledge, invariants, formations
- Examples: `profiles/scrum/`, `profiles/scrum-compact/`
- Pattern: Embedded at compile time, extracted to `~/.config/botminter/profiles/` on first run, used by `bm init` to stamp out team repos

**Team Repo:**
- Purpose: Git-backed control plane for a team instance
- Location: `{workzone}/{team}/team/` (git repo, pushed to GitHub)
- Pattern: Contains botminter.yml manifest, PROCESS.md, member configs, knowledge, invariants, formations

**Workspace:**
- Purpose: Working directory for a single member's Ralph instance
- Location: `{workzone}/{team}/{member}/`
- Pattern: Git repo with `team/` and `projects/<name>/` as submodules; runtime files surfaced to root

**Formation:**
- Purpose: Deployment strategy (local processes or k8s pods)
- Examples: `profiles/scrum-compact/formations/local/`, `profiles/scrum-compact/formations/k8s/`
- Pattern: `formation.yml` config with optional manager (Ralph session for non-local deployments)

**Knowledge/Invariant Scoping:**
- Purpose: Four-level additive knowledge resolution
- Pattern: team > project > member > member+project. More specific overrides general. Invariants are constitutional (hard constraints).

## Entry Points

**Binary Entry:**
- Location: `crates/bm/src/main.rs`
- Triggers: User runs `bm <command>`
- Responsibilities: Parse CLI via clap, dispatch to command handler, return result

**Daemon Entry (hidden):**
- Location: `crates/bm/src/commands/daemon.rs` (`run_daemon` function)
- Triggers: `bm daemon-run` (spawned by `bm daemon start`)
- Responsibilities: Run event loop (webhook HTTP server or poll loop), launch Ralph instances on events

**Library Entry:**
- Location: `crates/bm/src/lib.rs`
- Exposes: All modules as public for integration testing

## Error Handling

**Strategy:** anyhow for error propagation with context chains

**Patterns:**
- All command handlers return `Result<()>` using `anyhow::Result`
- Context added via `.context("message")` and `.with_context(|| format!(...))`
- User-facing errors use `bail!()` with actionable guidance (e.g., showing manual `gh` commands on failure)
- Prerequisite checks fail early with clear messages (missing `claude`, `ralph`, `gh` binaries)

## Cross-Cutting Concerns

**Logging:** No structured logging framework. User-facing output via `println!`, `eprintln!`, and cliclack TUI (spinners, progress). Daemon logs to file with rotation at 10MB.

**Validation:** Input validation in wizard via cliclack validators. Schema version checks before operations. URL validation for project forks (HTTPS-only).

**Authentication:** `GH_TOKEN` resolved from environment or `gh auth token`, stored in `~/.botminter/config.yml` (0600 permissions). Passed to all `gh` CLI calls and Ralph instances at runtime. Webhook secret for daemon webhook verification stored similarly.

**Agent Tag Filtering:** Profile files contain `+agent:NAME` / `-agent` blocks. Content is filtered at extraction time for the resolved coding agent (default: claude-code). Supports HTML and hash comment syntax.

## Docs Site Architecture

**Framework:** MkDocs with Material for MkDocs theme
- Config: `docs/mkdocs.yml`
- Content source: `docs/content/`
- Build output: `docs/site/` (generated, not committed)
- Python venv: `docs/.venv/` (managed by `just docs-setup`)

**Custom Landing Page:**
- Template override: `docs/overrides/home.html` — full custom HTML landing page extending `main.html`
- Custom CSS: `docs/content/stylesheets/home.css` — brand colors, dark theme, landing page layout
- Base override: `docs/overrides/main.html` — injects palette version script
- Index trigger: `docs/content/index.md` — frontmatter-only file (`template: home.html`, hides navigation/toc)

**Mermaid Diagrams:**
- External mermaid.js loaded from CDN (`unpkg.com/mermaid@11`)
- Init script: `docs/content/js/mermaid-init.js`
- Used in workflow and concept pages for architecture diagrams

**Palette Management:**
- Version script: `docs/content/js/palette-version.js` — clears stale localStorage when palette config changes
- Dark/light toggle with auto-detection via `prefers-color-scheme`

**Navigation Structure:**
```
Home (custom landing page)
├── Getting Started/
│   ├── Overview (index.md)
│   ├── Prerequisites
│   ├── Bootstrap Your Team
│   └── Your First Journey
├── The Agentic Workflow (workflow.md)
├── Concepts/
│   ├── Architecture
│   ├── Workspace Model
│   ├── Knowledge & Invariants
│   ├── Coordination Model
│   └── Profiles
├── How-To Guides/
│   ├── Generate a Team Repo
│   ├── Manage Members
│   ├── Launch Members
│   └── Manage Knowledge
├── Reference/
│   ├── CLI Commands
│   ├── Daemon Operations
│   ├── Process Conventions
│   ├── Configuration Files
│   ├── Member Roles
│   └── Design Principles
├── Roadmap
└── FAQ
```

**Markdown Extensions:** admonition, tables, pymdownx (details, highlight with line numbers, superfences with mermaid fences, tabbed), attr_list, md_in_html

**Dev Workflow:**
- `just docs-setup` — Create venv, install dependencies
- `just docs-serve` — Live-reload dev server at localhost:8000

---

*Architecture analysis: 2026-03-04*
