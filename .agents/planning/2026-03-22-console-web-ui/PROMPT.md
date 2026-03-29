# BotMinter Console — Web Dashboard for Day-Two Operations

## Objective

Implement the BotMinter Console — a browser-accessible web dashboard for day-two operations of agentic teams. The console is a design-time introspection and editing tool served by the daemon process, allowing operators to visualize their team's process methodology, inspect member hat configurations, and edit profile files with changes propagating to members and GitHub.

## Spec Directory

All planning artifacts: `.agents/planning/2026-03-22-console-web-ui/`

- `design/detailed-design.md` — Authoritative design: architecture, API, components, data models, security
- `design/mockups/team-page.html` — HTML/Tailwind mockup of team overview page
- `design/mockups/process-page.html` — HTML/Tailwind mockup of process pipeline page
- `implementation/plan.md` — 11-step implementation plan with checklist
- `research/profile-data-model.md` — Team repo structure and data model
- `research/tech-stack.md` — Technology choices and comparisons

## Execution Order

Follow the checklist in `implementation/plan.md`. Steps build sequentially unless noted:

1. **Daemon rewrite** — Rewrite `daemon/run.rs` from `tiny_http` to `axum`. Convert poll loop to `tokio::spawn`, signal handling to `tokio::signal`. Remove `tiny_http` dep, add `axum` + `tower-http`. Clean up dead signal handler code in `process.rs` (`setup_signal_handlers`, `SHUTDOWN_FLAG`, `sigterm_handler`).
2. **Console API skeleton** — Create `crates/bm/src/web/` module (NOT `console/` — the `console` crate name is already taken by a dependency). `GET /api/teams` and `GET /health`. Merge into daemon router. Add CORS.
3. **Frontend scaffold** — Initialize SvelteKit project at `console/` (workspace root, sibling to `crates/`). Svelte 5, Tailwind v4, TypeScript. Sidebar, team selector, route structure. Add `just dev` recipe. Note: `rust-embed` `#[folder]` paths are relative to `Cargo.toml`, so use `#[folder = "../../console/build/"]`. *(Can parallelize with Steps 1-2)*
4. **Team overview** — `GET /api/teams/:team/overview` API + overview page matching `team-page.html` mockup.
5. **Workflow DOT files** — Create `workflows/*.dot` Graphviz files in both profiles defining the process state machines. Derive from PROCESS.md + `botminter.yml` statuses.
6. **Process page** — `GET /api/teams/:team/process` API + pipeline visualization using `@viz-js/viz` (Graphviz WASM). Render DOT files as interactive SVG.
7. **Members** — `GET /api/teams/:team/members` + `GET /api/teams/:team/members/:name` APIs. Members list + member detail with CodeMirror 6 YAML viewer.
8. **File editor** — `GET/PUT /api/teams/:team/files/*path` + `GET /api/teams/:team/tree` APIs. Path traversal security. CodeMirror editor. Auto git commit on save.
9. **Knowledge, Invariants, Settings** — Remaining pages using file API. `POST /api/teams/:team/sync` with `spawn_blocking` + 60s timeout. Toast notifications.
10. **Asset embedding** — `rust-embed` behind `console` feature flag. SvelteKit `adapter-static`. SPA fallback in Axum. `just build-full` recipe.
11. **Daemon integration** — Wire console into daemon startup. Update `bm status` to show console URL. Smoke tests.

## Key Design Decisions

### Daemon rewrite (not separate process)
The console lives inside the daemon process (`bm daemon start`), not as a standalone server. The existing daemon uses synchronous `tiny_http` which is rewritten to async `axum`. This avoids a new process model and reuses the daemon's lifecycle (PID file, signal handling, log rotation). `bm start`/`bm stop` are unaffected — they use a separate code path through the `formation` module.

### Svelte 5 (not React)
Lighter than React, simpler mental model, compiled output is small (~10-20KB vs ~60-100KB). BotMinter's console has different needs than Ralph's React dashboard — no component sharing needed. Notable adopters: NYT, IKEA, 1Password, Decathlon. See `research/tech-stack.md`.

### REST API (not JSON-RPC)
Simple, debuggable with curl, maps naturally to the file/resource model. WebSocket can be added later for live features (logs, chat). JSON-RPC was considered for MCP/LSP ecosystem alignment but REST is more pragmatic for MVP.

### Graphviz DOT files for process definition
The process pipeline is a state machine with transitions, rejection loops, and human gates. PROCESS.md describes this only as prose. DOT files in `workflows/` become the structured source of truth, rendered to SVG via `@viz-js/viz` (WASM). Future: PROCESS.md can be generated from DOT files.

### Teams as namespaces
The console is multi-team aware. A team selector in the sidebar scopes all views. All API endpoints are team-scoped: `/api/teams/:team/*`. Team creation is out of scope — operators use `bm init` CLI.

### Auto git commit on file save
`PUT /api/teams/:team/files/*path` writes the file, then runs `git add` + `git commit`. Changes are tracked in the team repo's git history and won't be lost. Does NOT push.

## Key Constraints

- **Read `design/detailed-design.md` FIRST** — it is the authoritative source for all API endpoints, data models, component specs, and architectural decisions
- The daemon rewrite MUST preserve identical webhook behavior — existing `bm daemon start/stop/status` commands MUST work unchanged
- The daemon binds to `0.0.0.0` (required for GitHub webhooks). A `--bind` flag allows restricting to `127.0.0.1`
- Path traversal security is critical — canonicalize paths, reject `..`, reject symlinks escaping repo root, return 403
- Process page MUST gracefully degrade when `workflows/` directory is missing
- Three `lead:*` statuses in `botminter.yml` are undocumented in PROCESS.md. The DOT files define their canonical transitions: `arch:<phase> -> lead:<phase>-review -> po:<phase>-review`
- `ProfileManifest` already derives `Serialize` — reuse directly for API responses
- tokio is already a dependency with `rt-multi-thread` + `signal` features. See `commands/brain_run.rs` for the pattern: `tokio::runtime::Runtime::new()` + `rt.block_on()` in a sync function (NOT in `main()` directly — in a subcommand handler called from `main()`)
- The `--bind` flag does NOT exist yet — it must be added to the `DaemonRun` CLI definition in Step 1 (default: `0.0.0.0`)
- Team creation wizard is OUT OF SCOPE
- All runtime features are OUT OF SCOPE (live status, logs, chat, start/stop controls)

## Acceptance Criteria

1. **(Regression)** All existing tests pass — `just unit`, `just clippy` are green
2. **Daemon webhook preserved**
   - Given a daemon started with `bm daemon start --mode webhook --port 8484`
   - When a valid GitHub webhook payload is POSTed to `/webhook`
   - Then the daemon accepts it and launches members (identical to pre-rewrite behavior)
3. **Health endpoint**
   - Given a running daemon
   - When `curl http://localhost:8484/health` is run
   - Then it returns `{"ok":true,"version":"..."}` with status 200
4. **Teams API**
   - Given a `~/.botminter/config.yml` with registered teams
   - When `curl http://localhost:8484/api/teams` is run
   - Then it returns a JSON array of team entries with name, profile, github_repo, and path
5. **Team overview page**
   - Given a team with hired members and configured projects
   - When navigating to `/teams/:team/overview` in the browser
   - Then the page shows profile info, roles, members with hat counts, process summary, projects, bridge, knowledge, and invariants
6. **Process pipeline rendering**
   - Given a team with `workflows/*.dot` files
   - When navigating to `/teams/:team/process`
   - Then SVG state machine diagrams render with role-colored nodes, GATE badges, and rejection loops
7. **Process graceful degradation**
   - Given a team WITHOUT a `workflows/` directory
   - When navigating to `/teams/:team/process`
   - Then the page shows statuses/labels/PROCESS.md tabs but no pipeline diagram (no crash)
8. **Member YAML viewer**
   - Given a team with a hired member
   - When navigating to `/teams/:team/members/:name`
   - Then ralph.yml displays with syntax highlighting, folding, and search in CodeMirror
9. **File edit and git commit**
   - Given a file viewed in the console editor
   - When the operator edits and clicks Save
   - Then the file is written to disk, `git add` + `git commit` runs, and the commit SHA is returned
10. **Path traversal blocked**
    - Given a request to `GET /api/teams/:team/files/../../../etc/passwd`
    - Then the server returns 403 Forbidden
11. **Sync action**
    - Given a modified file in the team repo
    - When the operator clicks "Sync"
    - Then `bm teams sync` logic runs and changed workspace files are reported
12. **Team selector**
    - Given multiple registered teams
    - When the operator switches teams in the sidebar dropdown
    - Then all views rescope to the selected team
13. **Dev workflow**
    - Given the source code checked out
    - When `just dev` is run
    - Then the daemon starts and Vite dev server starts with hot reload at `http://localhost:5173`
14. **Production build**
    - Given the source code
    - When `just build-full` is run
    - Then the binary includes embedded frontend assets and serves the console without a separate dev server
15. **Clean shutdown**
    - Given a running daemon with console
    - When `bm daemon stop` is run
    - Then the daemon shuts down gracefully (no orphaned processes)

## Key References

- Design: `.agents/planning/2026-03-22-console-web-ui/design/detailed-design.md`
- Implementation plan: `.agents/planning/2026-03-22-console-web-ui/implementation/plan.md`
- Mockups: `.agents/planning/2026-03-22-console-web-ui/design/mockups/`
- **Test fixtures**: `.agents/planning/2026-03-22-console-web-ui/fixture-gen/fixtures/` — real artifacts from `bm init` + `bm hire` + `bm teams sync` (see below)
- Daemon module: `crates/bm/src/daemon/` (run.rs, event.rs, process.rs, lifecycle.rs, config.rs)
- Config module: `crates/bm/src/config/mod.rs` (BotminterConfig, TeamEntry)
- Profile manifest: `crates/bm/src/profile/manifest.rs` (ProfileManifest — already Serialize)
- Console API module: `crates/bm/src/web/` (NEW — created in Step 2, named `web` to avoid collision with `console` crate dep)
- Brain runtime pattern: `crates/bm/src/commands/brain_run.rs` (tokio `Runtime::new()` + `block_on()` pattern)
- Frontend project: `console/` at workspace root (sibling to `crates/`, NOT inside `crates/bm/`)
- Workspace sync: `crates/bm/src/workspace/team_sync.rs`
- Profile data: `profiles/scrum/botminter.yml`, `profiles/scrum/PROCESS.md`
- Dependencies: `crates/bm/Cargo.toml`

## Test Fixtures

Real artifacts generated by running `bm init --profile scrum-compact` + `bm hire` (3 members) + `bm teams sync` on a test user. Located at `.agents/planning/2026-03-22-console-web-ui/fixture-gen/fixtures/`:

```
fixtures/
  config/config.yml              # Global config (~/.botminter/config.yml), GH token redacted
  team-repo/                     # Complete bootstrapped team repo (110 files, 488K)
    botminter.yml                # Profile manifest (statuses, labels, views, roles)
    PROCESS.md                   # Process definition
    CLAUDE.md                    # Team-level context
    knowledge/                   # 3 knowledge files (commit-convention, communication, pr-standards)
    invariants/                  # 2 invariant files (code-review-required, test-coverage)
    members/
      superman-alice/            # Superman role (ralph.yml with 14 hats, CLAUDE.md, PROMPT.md)
      superman-bob/              # Same role, different member
      chief-of-staff-mgr/          # Chief-of-staff role (ralph.yml with 1 hat + 6 skills)
    coding-agent/skills/         # Team-level skills (gh, board-scanner, status-workflow)
    bridges/                     # Bridge configs (telegram, tuwunel, rocketchat)
    formations/                  # Formation configs (local, k8s)
    brain/system-prompt.md       # Brain process template
  workspace-sample/              # Surfaced workspace (alice) with ralph.yml, CLAUDE.md, .claude/, brain-prompt.md
```

**Use these fixtures for backend unit tests** — copy into a tempdir to simulate a real team repo without needing GitHub access. The `team-repo/` directory is the exact structure the console API reads from disk. The `config/config.yml` shows the config format (adjust paths to point to the tempdir).

**Fixture generation is reproducible** — run `just -f .agents/planning/2026-03-22-console-web-ui/fixture-gen/Justfile generate` on the `bm-dashboard-test-user` SSH user to regenerate. Requires `bm` binary at `~/.local/bin/bm` and `gh` auth configured (copied from `bm-test-user`).

## Completion Condition

Done when all 11 steps pass their acceptance criteria, the console is accessible via `bm daemon start`, and the full operator flow works: open console, browse team overview, inspect process pipeline, view member hats, edit a file, save and sync.
