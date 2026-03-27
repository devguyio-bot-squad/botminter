# Implementation Plan: BotMinter Console Web UI

## Checklist

- [ ] Step 1: Daemon rewrite (tiny_http -> Axum)
- [ ] Step 2: Console API skeleton + teams endpoint
- [ ] Step 3: Frontend scaffold (Svelte 5 + SvelteKit + Tailwind) + dev workflow
- [ ] Step 4: Team overview API + page
- [ ] Step 5: Create workflow DOT files for profiles
- [ ] Step 6: Process API + pipeline visualization page
- [ ] Step 7: Members list + member detail API + pages
- [ ] Step 8: File read/write API + CodeMirror editor + git commit
- [ ] Step 9: Knowledge, Invariants, and Settings pages + sync action
- [ ] Step 10: Asset embedding + production build pipeline
- [ ] Step 11: Daemon integration + smoke tests

---

## Step 1: Daemon rewrite (tiny_http -> Axum)

**Objective:** Rewrite the daemon's HTTP server from synchronous `tiny_http` to async `axum`, converting the event loop to tokio. This is a prerequisite for hosting the console API. The webhook functionality must remain identical.

**Implementation guidance:**

- Add dependencies to `Cargo.toml`: `axum`, `tower-http` (cors, trace), `serde_json` (already present)
- Remove `tiny_http` dependency
- Rewrite `daemon/run.rs`:
  - Replace the `loop { server.recv() }` pattern with an Axum router + `axum::serve`
  - `POST /webhook` handler: port the webhook validation logic from `event.rs` (HMAC-SHA256 signature check, event type parsing) into an Axum handler
  - `GET /health` handler: return `{ ok: true, version: "..." }`
  - Convert poll mode loop to `tokio::spawn` background task with `tokio::time::interval`
  - Replace `libc::signal` handlers with `tokio::signal::ctrl_c()` + `tokio::signal::unix::signal(SIGTERM)` for graceful shutdown
  - Use `axum::serve(...).with_graceful_shutdown(shutdown_signal())` pattern
- Update `daemon/event.rs`:
  - Convert `handle_webhook_request()` from `tiny_http::Request` to Axum extractors (`axum::body::Bytes`, `HeaderMap`)
  - Keep HMAC-SHA256 validation logic unchanged
- Update `daemon/process.rs`:
  - Wrap `launch_members_oneshot()` in `tokio::task::spawn_blocking` since it uses synchronous process management (`std::process::Command`, `waitpid`)
  - Keep signal flag checking for interruption during member launches
- Keep `daemon/lifecycle.rs` UNCHANGED (start/stop/query use PID files and are called from the CLI, not from within the daemon)
- Keep `daemon/config.rs` UNCHANGED (pure data structs + file I/O)
- Keep `daemon/log.rs` UNCHANGED (log rotation)

**Test requirements:**
- Unit test: webhook handler accepts valid GitHub payloads and rejects invalid signatures (port existing test if any)
- Unit test: health endpoint returns correct version
- Integration test: start daemon on random port, POST a webhook payload, verify it's accepted
- Integration test: poll mode triggers member launch on new events (mock `gh api`)
- Regression: existing `bm daemon start` / `bm daemon stop` / `bm daemon status` CLI commands work unchanged

**Integration with previous work:** This is a **refactor** of existing code, not new functionality. The daemon's external behavior (webhook handling, poll mode, PID file lifecycle) is unchanged. Only the internal HTTP server and event loop are replaced.

**Demo:** Run `bm daemon start --team my-team --mode webhook --port 8484`. The daemon starts with Axum instead of tiny_http. `curl http://localhost:8484/health` returns `{"ok":true,"version":"0.7.0"}`. POST a test webhook payload -- same behavior as before. `bm daemon stop` cleanly shuts it down.

---

## Step 2: Console API skeleton + teams endpoint

**Objective:** Add the console API routes to the daemon's Axum router. Start with `GET /api/teams` which reads `~/.botminter/config.yml`.

**Implementation guidance:**

- Create `crates/bm/src/web/` module (named `web` to avoid collision with `console` crate dependency):
  - `mod.rs` -- `web_router() -> Router` function that returns all console API routes
  - `state.rs` -- `WebState` struct (holds config path, resolved team paths). Shared via Axum state
  - `teams.rs` -- `GET /api/teams` handler: reads `BotminterConfig`, returns team list
- In `daemon/run.rs`, merge the console router into the daemon router:
  ```rust
  let app = Router::new()
      .route("/webhook", post(webhook_handler))
      .route("/health", get(health_handler))
      .merge(web_router(web_state))  // NEW
  ```
- Add `tower-http::cors::CorsLayer` allowing `http://localhost:*` origins
- Error responses: return JSON `{ error: string }` with appropriate HTTP status codes

**Test requirements:**
- Unit test: `GET /api/teams` returns correct list (use `fixture-gen/fixtures/config/config.yml` as config template, `fixture-gen/fixtures/team-repo/` as team repo — copy both into a tempdir and adjust paths)
- Unit test: returns `[]` when no teams registered
- Unit test: returns `500` when config file is missing/corrupt
- Integration test: start daemon, `curl /api/teams`, verify JSON response

**Integration with previous work:** Extends Step 1's Axum router with the console routes. Reuses `BotminterConfig` from `config/mod.rs`.

**Demo:** Start the daemon. `curl http://localhost:8484/api/teams` returns `[{"name":"my-team","profile":"scrum-compact","github_repo":"myorg/my-team","path":"/path/to/team"}]`.

---

## Step 3: Frontend scaffold (Svelte 5 + SvelteKit + Tailwind) + dev workflow

**Objective:** Create the Svelte 5 frontend with the shell layout, team selector, sidebar navigation, and developer workflow recipes.

**Implementation guidance:**

- Initialize SvelteKit project in `console/`:
  ```
  npx sv create console --template minimal --types ts
  ```
- Install dependencies: `tailwindcss`, `@tailwindcss/vite`, `svelte-codemirror-editor` (or plan custom wrapper)
- Configure `vite.config.ts`:
  - Proxy `/api` and `/health` to `http://localhost:8484` (daemon port)
  - Add `@tailwindcss/vite` plugin
- Configure `adapter-static` for SPA build output (needed for rust-embed in Step 10)
- Set up Tailwind v4 dark theme in `src/app.css` (surface colors, role colors from mockups)
- Implement shell layout:
  - `src/routes/+layout.svelte` -- root layout, fetches team list
  - `src/lib/components/TeamSelector.svelte` -- dropdown listing teams from `GET /api/teams`, "Create teams with `bm init`" hint at bottom
  - `src/lib/components/Sidebar.svelte` -- navigation links scoped to `[team]` param
  - `src/routes/+page.svelte` -- redirect to `/teams/:defaultTeam/overview`
  - `src/routes/teams/[team]/+layout.svelte` -- team-scoped shell
  - Placeholder `+page.svelte` for each route: overview, process, members, knowledge, invariants, settings
- Create `src/lib/api.ts` -- typed fetch wrapper:
  ```typescript
  export async function fetchTeams(): Promise<TeamSummary[]>
  export async function fetchOverview(team: string): Promise<TeamOverview>
  // etc.
  ```
- Create `src/lib/types.ts` -- TypeScript interfaces matching Rust API responses
- Add developer workflow to root `Justfile`:
  ```
  just console-dev    # npm run dev in console/
  just dev            # starts daemon + console-dev concurrently
  ```
  Use `concurrently` npm package or a shell background job to run both

**Test requirements:**
- Vitest config with `@testing-library/svelte` + jsdom
- Component test: TeamSelector renders team list
- Component test: Sidebar highlights active route
- Build test: `npm run build` succeeds and outputs to `console/build/`

**Integration with previous work:** Proxies API calls to Step 2's daemon. `just dev` runs both processes.

**Demo:** Run `just dev`. Open `http://localhost:5173`. See the sidebar with team selector, navigation links. Click between teams -- URL changes. All content areas show placeholder text. Hot reload works -- edit a `.svelte` file, browser updates instantly.

---

## Step 4: Team overview API + page

**Objective:** Implement the team overview endpoint and the landing page matching the mockup.

**Implementation guidance:**

- Backend: Add `GET /api/teams/:team/overview` in `web/overview.rs`
  - Resolve team repo path from config
  - Read + parse `botminter.yml` (`ProfileManifest` -- reuse `profile::manifest`)
  - Scan `members/` directory: for each, read member `botminter.yml` (role, emoji), parse `ralph.yml` to count hats and list skills
  - Scan `knowledge/` and `invariants/` for file lists
  - Return `TeamOverview` JSON struct
- Frontend: Implement `src/routes/teams/[team]/overview/+page.svelte`
  - Fetch overview data via `api.fetchOverview(team)`
  - Render card grid matching `team-page.html` mockup:
    - Profile info card (name, profile badge, description, GitHub repo, coding agent)
    - Roles card (list with colored dots and descriptions)
    - Members card (clickable rows with emoji, role badge, hat/skill/knowledge counts)
    - Process summary card (status counts by workflow, mini pipeline preview, "View full process" link)
    - Projects card
    - Bridge card
    - Knowledge and Invariants summary cards

**Test requirements:**
- Backend: copy `fixture-gen/fixtures/team-repo/` into a tempdir, test overview handler returns correct counts (3 members, 2 roles, 4 knowledge files, 2 invariants)
- Frontend: component test with mock data renders all cards

**Integration with previous work:** Route under Step 3's team-scoped layout. Member cards link to member detail (Step 7).

**Demo:** Navigate to `/teams/my-team/overview`. See the complete team dashboard matching the mockup with real data from the team repo.

---

## Step 5: Create workflow DOT files for profiles

**Objective:** Create Graphviz DOT files that define the process state machines for each profile. These are the structured source of truth for the process pipeline visualization and must exist before the console's process page can render anything.

**Implementation guidance:**

- Create `profiles/scrum/workflows/` directory with:
  - `epic.dot` -- Epic workflow: 14 statuses, 3 human gates (po:design-review, po:plan-review, po:accept), rejection loops from each gate back to the previous work phase
  - `story.dot` -- Story workflow: 8 statuses (dev:ready -> qe:test-design -> dev:implement -> dev:code-review -> qe:verify -> arch:sign-off -> po:merge -> done)
  - `specialist.dot` -- SRE (sre:infra-setup) and Content Writing (cw:write -> cw:review)
  - `manager.dot` -- Manager workflow (mgr:todo -> mgr:in-progress -> mgr:done)
- Create `profiles/scrum-compact/workflows/` with the same files (scrum-compact uses the same process)
- Each DOT file must:
  - Use `rankdir=LR` for left-to-right flow
  - Color nodes by role prefix (amber for PO, indigo for ARCH, green for DEV, cyan for QE, etc.)
  - Use `shape=octagon` for human gate statuses
  - Use `shape=doublecircle` for terminal states (done)
  - Use `style=dashed, color=red` for rejection edges
  - Include `label="approved"` / `label="rejected"` on gate transitions
- Derive the state machine from `profiles/scrum/PROCESS.md` prose AND `botminter.yml` statuses. Note: three `lead:*` statuses (`lead:design-review`, `lead:plan-review`, `lead:breakdown-review`) exist in `botminter.yml` but are not documented in PROCESS.md. These are intermediate review gates between architect work and PO human gates. The transition pattern is: `arch:<phase> → lead:<phase>-review → po:<phase>-review`. The DOT files are the canonical source of truth for transitions — update PROCESS.md to match if needed
- Validate DOT files render correctly: `dot -Tsvg epic.dot -o epic.svg` (requires graphviz installed, or use viz.js online playground)
- Verify `bm init` extracts `workflows/` directory during team bootstrapping (it should work automatically since `include_dir` captures the entire profile tree, but verify)
- Update profile `schema_version` comment to note the addition (no breaking change -- `workflows/` is optional, process page gracefully degrades without it)

**Test requirements:**
- Each DOT file parses without errors (`dot -Tsvg` exits 0)
- DOT files cover ALL statuses from `botminter.yml` (no missing statuses, no extra statuses). If PROCESS.md is ambiguous about a status, the DOT file defines the canonical transition and PROCESS.md should be updated to match
- `bm init` creates a team repo that includes `workflows/` directory with DOT files
- Unit test: process API handler returns workflow content from team repos that have DOT files
- Unit test: process API handler returns empty workflows array for team repos that lack `workflows/` (graceful degradation)

**Integration with previous work:** These files become the data source for Step 6's process page. They live in the profile alongside `PROCESS.md` and `botminter.yml`.

**Demo:** Run `dot -Tsvg profiles/scrum/workflows/epic.dot -o /tmp/epic.svg && open /tmp/epic.svg`. See the complete epic state machine with role-colored nodes, gate badges, and rejection loops. Run `bm init` to bootstrap a new team -- verify `workflows/` appears in the team repo.

---

## Step 6: Process API + pipeline visualization page

**Objective:** Implement the process page with Graphviz-rendered workflow diagrams, role responsibilities, and status/label tables.

**Implementation guidance:**

- Backend: Add `GET /api/teams/:team/process` in `web/process.rs`
  - Read `PROCESS.md` as raw markdown string
  - Read `workflows/*.dot` files from team repo, return each as `{ name, dot }`
  - Read `botminter.yml` for statuses, labels, views
  - Return `ProcessInfo` JSON (markdown, workflows, statuses, labels, views)
- Frontend: Implement `src/routes/teams/[team]/process/+page.svelte`
  - Tab bar: Pipeline | Statuses | Labels | Views | PROCESS.md
  - **Pipeline tab** -- `src/lib/components/ProcessPipeline.svelte`:
    - Install `@viz-js/viz` (Graphviz compiled to WASM, ~2MB)
    - For each workflow from the API, render the `.dot` content to SVG using viz.js
    - Display each workflow as a labeled section (Epic, Story, etc.) with the rendered SVG
    - SVG is interactive: hover nodes for status details
  - **Role Responsibilities section**: group all statuses by role prefix, show owned statuses per role
  - **Human Gates section**: highlight supervised-mode checkpoints (gates are identifiable from DOT files by `shape=octagon`)
  - **PROCESS.md tab**: render markdown with a lightweight parser (marked or markdown-it)
  - **Statuses/Labels/Views tabs**: simple table views
- Create `src/lib/components/MarkdownRenderer.svelte`: wrapper around markdown parser
**Test requirements:**
- Backend: process handler returns correct workflow DOT content (use `fixture-gen/fixtures/team-repo/` — after Step 5 adds `workflows/*.dot` files, update the fixtures to include them)
- Backend: process handler returns empty workflows array when `workflows/` dir is missing (graceful degradation)
- Backend: process handler returns correct status/label counts
- Frontend: ProcessPipeline renders SVG from DOT input (use Playwright for real DOM test since viz.js needs WASM)
- Frontend: statuses group correctly by role prefix in Role Responsibilities section
- Frontend: process page shows "No workflow diagrams available" when workflows array is empty

**Integration with previous work:** Uses Step 5's DOT files as data source. Linked from overview page "View full process" action.

**Demo:** Navigate to `/teams/my-team/process`. See rendered SVG state machine diagrams for Epic, Story, Specialist, Manager workflows with role-colored nodes, rejection loops (dashed red edges), and GATE nodes (octagonal shape). Click tabs to see statuses table, labels table, rendered PROCESS.md.

---

## Step 7: Members list + member detail API + pages

**Objective:** Implement members listing and member detail with YAML viewer. Combined into one step since members list is small and the detail page is the substantial work.

**Implementation guidance:**

- Backend: Add `GET /api/teams/:team/members` in `web/members.rs`
  - Scan `members/` dir, read each member's `botminter.yml` + count hats from `ralph.yml`
  - Return `Vec<MemberSummary>` JSON
- Backend: Add `GET /api/teams/:team/members/:name` in same module
  - Read member's `botminter.yml`, `ralph.yml`, `CLAUDE.md`, `PROMPT.md`
  - Parse `ralph.yml` YAML to extract hat summaries (hat name, description, triggers, publishes from the `hats` map)
  - List knowledge files, invariant files, skill directories
  - Return `MemberDetail` JSON
- Frontend: Members list page (`src/routes/teams/[team]/members/+page.svelte`)
  - Fetch + render member cards (role badge, hat count, skill count)
  - Click navigates to detail
- Frontend: Member detail page (`src/routes/teams/[team]/members/[name]/+page.svelte`)
  - Two-panel layout: tab list + content area
  - Tabs: Ralph YAML | CLAUDE.md | PROMPT.md | Hats | Knowledge | Invariants
  - `src/lib/components/YamlViewer.svelte`: CodeMirror 6 read-only with YAML mode, folding, search
    - Note: if `svelte-codemirror-editor` doesn't support Svelte 5, create a custom wrapper using CodeMirror's imperative `EditorView` API via Svelte's `use:action` directive
  - `MarkdownRenderer.svelte` (from Step 6) for CLAUDE.md and PROMPT.md
  - Hats tab: list parsed hat summaries, click a hat name to scroll to that section in YAML tab

**Test requirements:**
- Backend: member detail handler parses hat summaries correctly (use `fixture-gen/fixtures/team-repo/members/superman-alice/ralph.yml` — has 14 hats)
- Frontend: YamlViewer renders YAML with syntax highlighting (use Playwright for real DOM test since CodeMirror needs it)
- Frontend: tabs switch content correctly

**Integration with previous work:** Linked from overview page member cards and members list.

**Demo:** Navigate to `/teams/my-team/members`. See member cards. Click `superman-01`. See ralph.yml with syntax highlighting and folding. Switch tabs to see rendered CLAUDE.md, PROMPT.md, hat summary list.

---

## Step 8: File read/write API + CodeMirror editor + git commit

**Objective:** Implement generic file operations with path traversal security, the editable CodeMirror component, and automatic git commits on save.

**Implementation guidance:**

- Backend: Add `web/files.rs`:
  - `GET /api/teams/:team/files/*path`: read file, detect content_type by extension, return `FileContent` JSON
  - `PUT /api/teams/:team/files/*path`: write file, git add + commit, return `{ ok, path, commit_sha }`
  - `GET /api/teams/:team/tree?path=...`: list directory entries as `Vec<TreeEntry>`
  - **Path security** (critical):
    - Reject paths containing `..` segments BEFORE any filesystem access
    - Reject absolute paths
    - Canonicalize both the team repo root AND the resolved path, verify resolved starts with root
    - Reject symlinks that resolve outside the repo root
    - Return `403 Forbidden` for all violations
  - **Git commit on write**:
    - After writing file: `git -C <repo> add <path> && git -C <repo> commit -m "console: update <path>"`
    - Use `std::process::Command` (or `tokio::process::Command`) for git operations
    - Return the commit SHA in the response
    - Do NOT push (operator pushes when ready)
- Frontend: `src/lib/components/FileEditor.svelte`:
  - CodeMirror 6 with language detection (YAML: `.yml`/`.yaml`, Markdown: `.md`, JSON: `.json`)
  - Syntax highlighting, line numbers, search (Ctrl+F), code folding
  - Save button: `PUT /api/teams/:team/files/*path`, show toast with commit SHA
  - "Save & Sync" button: PUT then `POST /api/teams/:team/sync`
  - Unsaved changes indicator
  - `beforeunload` warning
- Upgrade `YamlViewer.svelte` to include an "Edit" toggle that switches to `FileEditor`

**Test requirements:**
- Backend: path traversal rejection for `../../../etc/passwd`, `%2e%2e/etc/passwd`, `/etc/passwd`, symlink escape
- Backend: file write + git commit roundtrip (verify commit exists in git log)
- Backend: file read returns correct content_type
- Frontend: FileEditor save triggers PUT and shows toast

**Integration with previous work:** FileEditor is used by Step 7's YamlViewer (edit mode), Step 9's knowledge/invariant/settings pages.

**Demo:** On member detail page, click "Edit" on ralph.yml. Make a change, click Save. See toast: "Saved. Commit: abc1234". Run `git log` in the team repo -- see the commit. Reload the page -- change persists.

---

## Step 9: Knowledge, Invariants, and Settings pages + sync action

**Objective:** Complete the remaining pages and the sync action. Combined since these are all similar file-browsing patterns.

**Implementation guidance:**

- Backend: Add `POST /api/teams/:team/sync` in `web/sync.rs`
  - Call `workspace::sync` logic via `spawn_blocking` (it's synchronous and can take time)
  - Return `{ ok, message, changed_files }` JSON
  - Set a reasonable timeout (60s) and return error if exceeded
- Frontend: Knowledge page (`src/routes/teams/[team]/knowledge/+page.svelte`)
  - Fetch tree for `knowledge/`, `projects/*/knowledge/`, `members/*/knowledge/`
  - Group files by scope level with headers (Team, Project, Member)
  - Click file -> view with MarkdownRenderer, "Edit" toggle to FileEditor
- Frontend: Invariants page (`src/routes/teams/[team]/invariants/+page.svelte`)
  - Same pattern as knowledge for `invariants/` at team/project/member levels
- Frontend: Settings page (`src/routes/teams/[team]/settings/+page.svelte`)
  - Load `botminter.yml` in FileEditor
  - "Sync to workspaces" button calling `POST /api/teams/:team/sync`
  - Show sync results in a toast or expandable result panel
- Add sync button to the top header bar (available on all pages)
- Add toast notification component (`src/lib/components/Toast.svelte`)

**Test requirements:**
- Backend: sync handler calls workspace sync and returns results
- Backend: sync timeout returns error after 60s
- Frontend: knowledge page groups files by scope
- Frontend: sync button shows results
- Integration: edit a file, sync, verify workspace files updated

**Integration with previous work:** Uses Step 8's file API, FileEditor, and git commit. Sync button also appears in Step 3's header.

**Demo:** Navigate through Knowledge, Invariants, Settings pages. Edit a knowledge file, save (git committed). Click "Sync" in the header -- see toast with sync results. Verify the workspace's files were updated.

---

## Step 10: Asset embedding + production build pipeline

**Objective:** Embed the Svelte frontend in the `bm` binary for production use.

**Implementation guidance:**

- Add `rust-embed` dependency (feature-gated: `console` feature)
- Configure SvelteKit's `adapter-static` to output to `console/build/`
- Create `web/assets.rs`:
  - `#[derive(RustEmbed)] #[folder = "../../console/build/"]` struct (path relative to `crates/bm/Cargo.toml`)
  - Axum handler that serves embedded files for non-`/api` routes
  - SPA fallback: serve `index.html` for any path that doesn't match a static file (SvelteKit client-side routing handles the rest)
  - Set correct `Content-Type` headers based on file extension
  - Cache headers: `Cache-Control: public, max-age=31536000` for hashed assets, `no-cache` for `index.html`
- Add to daemon router (behind `#[cfg(feature = "console")]`):
  ```rust
  #[cfg(feature = "console")]
  let app = app.fallback(serve_embedded_assets);
  ```
- Add Justfile recipes:
  - `just console-build` -- `cd console && npm run build`
  - `just build-full` -- `just console-build && cargo build -p bm --features console`
- Handle the build-order problem: create an empty `console/build/` placeholder at the workspace root so `cargo build` doesn't fail when the frontend hasn't been built yet. Use `rust-embed`'s `#[allow_empty = true]` attribute or a cargo build script.
- Add `console` feature to CI matrix

**Test requirements:**
- Build: `just build-full` succeeds
- Smoke: start daemon from the full build, `curl http://localhost:8484/` returns HTML
- Smoke: `curl http://localhost:8484/api/teams` still returns JSON (API routes take priority over asset fallback)
- Smoke: `curl http://localhost:8484/teams/my-team/overview` returns `index.html` (SPA fallback)

**Integration with previous work:** All previous steps work via Vite dev proxy. This step adds the production path. No code changes to frontend or API.

**Demo:** Run `just build-full`. Start daemon. Open `http://localhost:8484` in browser -- the full console loads from the embedded binary. Navigate all pages. No Vite dev server needed.

---

## Step 11: Daemon integration + smoke tests

**Objective:** Wire everything together, update CLI output, and add end-to-end tests.

**Implementation guidance:**

- Update daemon startup in `daemon/run.rs` to log: `Console available at http://localhost:{port}`
- Update `bm daemon start` output to show console URL
- Update `bm status` (`state/dashboard.rs`) to show console URL when daemon is running
- Add `--no-console` flag to daemon to disable console routes (for minimal webhook-only mode)
- Write comprehensive smoke tests:
  - Start daemon, verify `/health` responds
  - Verify `/api/teams` returns team list
  - Verify `/api/teams/:team/overview` returns overview data
  - Verify `PUT /api/teams/:team/files/*` writes and git commits
  - Verify SPA fallback serves `index.html` for frontend routes
  - Verify `POST /webhook` still works (regression)
- Add Playwright E2E test (if time permits):
  - Start daemon with full build
  - Load console in headless Chrome
  - Navigate pages, verify content renders
  - Edit a file, save, verify commit

**Test requirements:**
- Integration: daemon starts with console, `/health` responds
- Integration: daemon stops cleanly (graceful shutdown)
- Integration: webhook still works alongside console routes
- Regression: `bm daemon start` / `bm daemon stop` / `bm daemon status` unchanged
- E2E (stretch): headless Chrome navigates console pages

**Integration with previous work:** Final step connecting all previous work. No new features -- just wiring and verification.

**Demo:** Run `bm daemon start`. See `Console available at http://localhost:8484` in output. Run `bm status` -- see console URL in the dashboard. Open browser, navigate all pages. Edit a file, save and sync. POST a webhook -- members still launch correctly. `bm daemon stop` -- clean shutdown, console stops.
