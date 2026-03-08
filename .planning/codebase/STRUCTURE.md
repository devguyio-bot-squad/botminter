# Codebase Structure

**Analysis Date:** 2026-03-04

## Directory Layout

```
botminter/
├── crates/
│   └── bm/                    # Main Rust binary crate
│       ├── src/               # Source code
│       │   ├── commands/      # Command handler modules
│       │   ├── main.rs        # Entry point
│       │   ├── cli.rs         # Clap CLI definitions
│       │   ├── lib.rs         # Library exports
│       │   ├── config.rs      # Config loading/saving
│       │   ├── profile.rs     # Profile parsing & extraction (~69K)
│       │   ├── workspace.rs   # Workspace creation & sync (~71K)
│       │   ├── agent_tags.rs  # Agent tag filtering (~16K)
│       │   ├── chat.rs        # Meta-prompt building (~12K)
│       │   ├── completions.rs # Dynamic shell completions (~19K)
│       │   ├── formation.rs   # Formation config parsing
│       │   ├── session.rs     # Claude Code session launch
│       │   ├── state.rs       # Runtime PID state
│       │   └── topology.rs    # Member topology tracking
│       ├── tests/
│       │   ├── integration.rs # Unit/integration tests (~87K)
│       │   ├── cli_parsing.rs # CLI parsing tests (~31K)
│       │   └── e2e/           # End-to-end tests (feature-gated)
│       │       ├── main.rs
│       │       ├── init_to_sync.rs
│       │       ├── start_to_stop.rs
│       │       ├── daemon_lifecycle.rs
│       │       ├── github.rs
│       │       ├── telegram.rs
│       │       └── helpers.rs
│       └── Cargo.toml         # Crate manifest
├── profiles/
│   ├── scrum/                 # Multi-agent scrum profile
│   │   ├── botminter.yml      # Profile manifest
│   │   └── coding-agent/      # Agent-specific files (skills, agents)
│   └── scrum-compact/         # Single-agent compact profile
│       ├── botminter.yml      # Profile manifest
│       ├── context.md         # Becomes CLAUDE.md in workspace
│       ├── PROCESS.md         # Workflow/label definitions
│       ├── coding-agent/      # Agent skills and references
│       │   ├── agents/
│       │   └── skills/
│       │       ├── board-scanner/
│       │       ├── gh/
│       │       │   ├── SKILL.md
│       │       │   ├── scripts/
│       │       │   └── references/
│       │       └── status-workflow/
│       ├── formations/
│       │   ├── local/         # Local process formation
│       │   │   └── formation.yml
│       │   └── k8s/           # Kubernetes formation
│       │       ├── formation.yml
│       │       ├── ralph.yml
│       │       ├── PROMPT.md
│       │       └── hats/
│       ├── knowledge/         # Team-level knowledge
│       │   ├── commit-convention.md
│       │   ├── communication-protocols.md
│       │   └── pr-standards.md
│       ├── invariants/        # Team-level invariants
│       │   ├── code-review-required.md
│       │   └── test-coverage.md
│       └── ralph-prompts/     # Ralph Orchestrator prompt templates
│           ├── guardrails.md
│           ├── hat-template.md
│           ├── orientation.md
│           └── reference/
├── docs/
│   ├── mkdocs.yml             # MkDocs config (nav, theme, extensions)
│   ├── content/               # Markdown source files
│   │   ├── index.md           # Landing page trigger (frontmatter only)
│   │   ├── workflow.md        # "The Agentic Workflow" page
│   │   ├── faq.md             # FAQ page
│   │   ├── roadmap.md         # Roadmap page
│   │   ├── getting-started/
│   │   │   ├── index.md       # Getting Started overview
│   │   │   ├── prerequisites.md
│   │   │   ├── bootstrap-your-team.md
│   │   │   └── first-journey.md
│   │   ├── concepts/
│   │   │   ├── architecture.md
│   │   │   ├── workspace-model.md
│   │   │   ├── knowledge-invariants.md
│   │   │   ├── coordination-model.md
│   │   │   └── profiles.md
│   │   ├── how-to/
│   │   │   ├── generate-team-repo.md
│   │   │   ├── manage-members.md
│   │   │   ├── launch-members.md
│   │   │   └── manage-knowledge.md
│   │   ├── reference/
│   │   │   ├── cli.md
│   │   │   ├── daemon-operations.md
│   │   │   ├── process.md
│   │   │   ├── configuration.md
│   │   │   ├── member-roles.md
│   │   │   └── design-principles.md
│   │   ├── assets/            # Images (logos, favicon, og-preview)
│   │   ├── stylesheets/
│   │   │   └── home.css       # Brand colors, dark theme, landing styles
│   │   └── js/
│   │       ├── mermaid-init.js     # Mermaid diagram initialization
│   │       └── palette-version.js  # Palette cache busting
│   ├── overrides/             # MkDocs Material template overrides
│   │   ├── home.html          # Custom landing page template (~15K)
│   │   └── main.html          # Base override (palette version script)
│   ├── site/                  # Generated output (not committed)
│   └── .venv/                 # Python virtualenv for MkDocs
├── specs/
│   ├── master-plan/           # Top-level design documents
│   │   ├── rough-idea.md
│   │   ├── requirements.md
│   │   ├── design.md
│   │   ├── plan.md
│   │   ├── summary.md
│   │   └── research/         # Research artifacts
│   ├── milestones/
│   │   ├── completed/        # Past milestone planning
│   │   └── [active]/         # Current milestone artifacts
│   ├── tasks/                 # Standalone task batches
│   ├── prompts/               # Reusable planning prompts
│   └── design-principles.md
├── knowledge/                 # Dev workflow knowledge (this repo)
├── invariants/                # Dev workflow invariants (this repo)
├── minty/                     # Minty assistant config
│   ├── config.yml
│   ├── prompt.md
│   └── skills/
├── assets/                    # Project assets (branding, etc.)
├── .claude/                   # Claude Code config for this repo
│   ├── agents/
│   └── skills/                # ~20 Claude Code skills
├── .planning/                 # GSD planning artifacts
├── .github/
│   └── workflows/             # CI workflows
├── Cargo.toml                 # Workspace manifest
├── Cargo.lock
├── Justfile                   # Development task runner
├── CLAUDE.md                  # Claude Code instructions for this repo
├── ralph.yml                  # Ralph Orchestrator config for this repo
├── PROMPT.md                  # Symlink to current milestone PROMPT
├── README.md
├── RELEASE_NOTES.md
├── LICENSE                    # Apache-2.0
└── .gitignore
```

## Directory Purposes

**`crates/bm/src/`:**
- Purpose: All Rust source code for the `bm` CLI binary
- Contains: Library modules and command handlers
- Key files: `profile.rs` and `workspace.rs` are the largest (~69K and ~71K respectively) containing core profile extraction and workspace management logic

**`crates/bm/src/commands/`:**
- Purpose: One module per CLI command group
- Contains: 16 command modules matching CLI subcommands
- Key files: `init.rs` (wizard, ~44K), `daemon.rs` (~42K), `start.rs` (~19K)

**`crates/bm/tests/`:**
- Purpose: All test code
- Contains: `integration.rs` (unit/integration tests, ~87K), `cli_parsing.rs` (~31K), `e2e/` directory (feature-gated end-to-end tests)

**`profiles/`:**
- Purpose: Methodology profile templates embedded into the binary at compile time
- Contains: Two profiles — `scrum` (multi-agent) and `scrum-compact` (single agent)
- Key files: `botminter.yml` in each profile defines manifest; `PROCESS.md` defines workflow

**`docs/content/`:**
- Purpose: MkDocs documentation source files
- Contains: Markdown pages organized by Diataxis framework (tutorials, how-to, concepts, reference)
- Key files: `index.md` (landing page trigger), `workflow.md` (core messaging page)

**`docs/overrides/`:**
- Purpose: MkDocs Material theme template overrides
- Contains: Custom landing page HTML (`home.html`), base template injection (`main.html`)

**`specs/`:**
- Purpose: Design-first planning artifacts produced before implementation
- Contains: Master plan, per-milestone requirements/design/plan documents, reusable planning prompts

**`knowledge/`:**
- Purpose: Development knowledge for this repo's Ralph workflow
- Contains: Guides for testing patterns, Ralph integration, skill development, process safety

**`invariants/`:**
- Purpose: Hard constraints for this repo's development workflow
- Contains: Testing invariants (flaky tests, path isolation, e2e patterns), profile update rules

**`minty/`:**
- Purpose: Configuration for the Minty interactive assistant
- Contains: `config.yml`, `prompt.md`, skills directory

**`.claude/skills/`:**
- Purpose: Claude Code skills for developing botminter itself
- Contains: ~20 skills (code-assist, test-driven-development, release-bump, pr-demo, etc.)

## Key File Locations

**Entry Points:**
- `crates/bm/src/main.rs`: Binary entry point, CLI dispatch
- `crates/bm/src/lib.rs`: Library entry, module re-exports

**Configuration:**
- `Cargo.toml`: Workspace manifest (members = ["crates/*"])
- `crates/bm/Cargo.toml`: Crate manifest with dependencies and e2e feature flag
- `docs/mkdocs.yml`: Documentation site config
- `Justfile`: Development task runner (build, test, clippy, docs-serve, docs-build)
- `ralph.yml`: Ralph Orchestrator config for developing this repo

**Core Logic:**
- `crates/bm/src/profile.rs`: Profile parsing, extraction, schema versioning, agent tag processing
- `crates/bm/src/workspace.rs`: Workspace creation, submodule management, file surfacing
- `crates/bm/src/config.rs`: Global config at `~/.botminter/config.yml`
- `crates/bm/src/commands/init.rs`: Team creation wizard
- `crates/bm/src/commands/daemon.rs`: Event-driven daemon (webhook + poll modes)
- `crates/bm/src/agent_tags.rs`: `+agent:NAME` / `-agent` content filtering

**Testing:**
- `crates/bm/tests/integration.rs`: Core integration tests (~87K)
- `crates/bm/tests/cli_parsing.rs`: CLI argument parsing tests (~31K)
- `crates/bm/tests/e2e/`: End-to-end tests requiring `--features e2e`

**Documentation:**
- `docs/content/`: All markdown source
- `docs/overrides/home.html`: Custom landing page
- `docs/content/stylesheets/home.css`: Brand styling
- `docs/content/workflow.md`: Core "Agentic Workflow" messaging

## Naming Conventions

**Files:**
- Rust modules: `snake_case.rs` (e.g., `agent_tags.rs`, `profiles_init.rs`)
- Commands: named after CLI subcommand (e.g., `init.rs`, `hire.rs`, `start.rs`)
- Docs: `kebab-case.md` (e.g., `bootstrap-your-team.md`, `daemon-operations.md`)
- Profile files: specific names (`botminter.yml`, `PROCESS.md`, `context.md`, `formation.yml`)
- Knowledge/invariants: `kebab-case.md`

**Directories:**
- Rust: `snake_case` (e.g., `commands/`)
- Docs: `kebab-case` (e.g., `getting-started/`, `how-to/`)
- Profiles: `kebab-case` (e.g., `scrum-compact/`, `coding-agent/`)

**Types:**
- Structs: `PascalCase` (e.g., `BotminterConfig`, `TeamEntry`, `FormationConfig`)
- Enums: `PascalCase` with `PascalCase` variants (e.g., `Command::Init`, `Endpoint::Local`)

## Where to Add New Code

**New CLI Command:**
- Add variant to `Command` enum in `crates/bm/src/cli.rs`
- Create handler module in `crates/bm/src/commands/<name>.rs`
- Register module in `crates/bm/src/commands/mod.rs`
- Add dispatch in `crates/bm/src/main.rs`
- Add CLI parsing tests in `crates/bm/tests/cli_parsing.rs`
- Add integration tests in `crates/bm/tests/integration.rs`
- Update docs at `docs/content/reference/cli.md`

**New Core Module:**
- Create `crates/bm/src/<name>.rs`
- Register in `crates/bm/src/lib.rs`

**New Profile:**
- Create `profiles/<profile-name>/` with `botminter.yml` manifest
- Add `PROCESS.md`, `context.md`, knowledge/, invariants/ as needed
- Profile is automatically embedded at compile time via `include_dir`

**New Documentation Page:**
- Add markdown file in appropriate `docs/content/<section>/` directory
- Register in `docs/mkdocs.yml` nav structure

**New Test:**
- Unit/integration: add to `crates/bm/tests/integration.rs`
- CLI parsing: add to `crates/bm/tests/cli_parsing.rs`
- E2E: add to appropriate file in `crates/bm/tests/e2e/` and register in `crates/bm/tests/e2e/main.rs`

**New Knowledge/Invariant (for this repo):**
- Knowledge: `knowledge/<name>.md`
- Invariant: `invariants/<name>.md`

**New Claude Code Skill (for this repo):**
- Create `.claude/skills/<skill-name>/` directory

## Special Directories

**`profiles/`:**
- Purpose: Methodology templates embedded in binary
- Generated: No (authored by profile developers)
- Committed: Yes
- Note: Changes here require recompilation (`cargo build`) to take effect

**`docs/site/`:**
- Purpose: MkDocs generated HTML output
- Generated: Yes (by `just docs-build`)
- Committed: No (in .gitignore)

**`docs/.venv/`:**
- Purpose: Python virtualenv for MkDocs dependencies
- Generated: Yes (by `just docs-setup`)
- Committed: No

**`target/`:**
- Purpose: Cargo build output
- Generated: Yes
- Committed: No

**`.planning/`:**
- Purpose: GSD planning artifacts for current work
- Generated: By planning tools
- Committed: Yes

**`specs/milestones/completed/`:**
- Purpose: Archive of past milestone planning artifacts
- Generated: No (moved manually after completion)
- Committed: Yes

---

*Structure analysis: 2026-03-04*
