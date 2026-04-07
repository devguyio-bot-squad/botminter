# Implementation Plan: Team Runtime Architecture + Per-Member GitHub App Identity

## Sprint Index

- [x] **Sprint 1: Formation Trait + CredentialStore + Team API Boundary** (complete)
  - Foundation: Formation trait, key-value CredentialStore, Team as API boundary. Auth model unchanged — `gh_token` signatures preserved.
  - [sprint-1/plan.md](sprint-1/plan.md) | [sprint-1/PROMPT.md](sprint-1/PROMPT.md)

- [x] **Sprint 2: Daemon Supervisor + CLI Through Team + Brain Model** (complete, 1 minor gap: `bm status` bypasses Team struct)
  - Daemon as formation-internal supervisor with RESTful HTTP API (OpenAPI schema). `bm start/stop/status` through Team. `bm env create/delete`. Brain delegates loops to daemon. Auth model unchanged.
  - [sprint-2/plan.md](sprint-2/plan.md) | [sprint-2/PROMPT.md](sprint-2/PROMPT.md)

- [ ] **Sprint 3: GitHub App Identity + bm fire + Credentials Export** (mostly complete, 4 open items)
  - JWT auth module, manifest flow in `bm hire`, token lifecycle, `GH_CONFIG_DIR` + `hosts.yml`, remove `gh_token`, `bm fire`, `bm credentials export/import`. Auth model swapped to per-member GitHub App.
  - **Open:** interactive manifest flow (deferred), exploratory test migration, `TESTS_GH_TOKEN` removal, `bm init` interactive App creation
  - [sprint-3/plan.md](sprint-3/plan.md) | [sprint-3/PROMPT.md](sprint-3/PROMPT.md)

Each sprint includes its own unit tests, integration tests, E2E updates, and doc updates.

## Sprint Dependency Chain

```
Sprint 1 (formation refactor — same auth)
    └── Sprint 2 (daemon + CLI migration — same auth)
         └── Sprint 3 (swap auth model + fire + export)
```

Sprints 1-2 are pure refactoring — existing `gh_token` model preserved, existing tests pass throughout. Sprint 3 is the single point where auth changes.

## Review Findings Incorporated

- **C1 (gh_token signatures):** Sprint 1 does NOT change `git/github.rs` function signatures. The Formation trait wraps existing functions — `gh_token` parameters stay until Sprint 3 when the auth model actually changes.
- **C2 (Sprint 2 scope):** Sprint 2 is large but work is sequential: daemon HTTP API → state ownership → CLI migration → `bm env` → Brain model. Each step builds on the previous.
- **C3 (init wizard UX):** Sprint 3 acknowledges the UX change — each hire opens a browser for one click. Acceptable for 1-3 members during init.
- **I4 (CredentialStore/bridge-state.json):** Key-value `CredentialStore` is pure keyring operations. Bridge-state.json identity metadata stays in the bridge module — not a credential store concern.
- **I5 (tests during refactor):** Extract-and-delegate approach — new code alongside old, old delegates to new, old removed once delegation works. Tests never break.
- **I7 (daemon credential loading):** Daemon loads member App credentials on-demand when `POST /members/start` is called, not at daemon startup. Caches in memory for refresh loops.
- **R7 (daemon API style):** RESTful HTTP with OpenAPI schema on existing axum server.

## Sprint Summaries

### Sprint 1: Formation Trait + CredentialStore + Team API Boundary

Establishes the architectural foundation without changing the auth model or any function signatures.

**What gets built:**
- `Formation` trait with all method signatures (per ADR-0008)
- Key-value `CredentialStore` trait (`store/retrieve/remove/list_keys`)
- `CredentialDomain` enum (Bridge + GitHubApp)
- Team struct as API boundary — holds formation internally
- `LinuxLocalFormation` — wraps existing formation free functions behind the trait
- `MacosLocalFormation` — stub ("not yet supported")
- Module restructure: `formation/local/linux/`, `formation/local/macos/`

**What gets moved:**
- `LocalCredentialStore` from `bridge/credential.rs` → `formation/local/linux/credential.rs`
- Generalized to key-value interface
- Bridge module calls credential store for keyring ops, manages bridge-state.json itself

**What does NOT change:**
- `gh_token` stays in `TeamEntry`/`Credentials`
- `gh_token: Option<&str>` parameters stay in `git/github.rs` functions
- `require_gh_token()` stays
- Member launch still uses `GH_TOKEN` env var
- All existing code paths work through formation delegation

**Approach:** Extract-and-delegate. New Formation code wraps existing functions. Existing entry points delegate. Old standalone paths removed once delegation is verified.

**Demo:** All existing commands work exactly as before. `bm start/stop` go through `team.start()`/`team.stop()` → formation → existing code. Bridge credentials work through key-value `CredentialStore`. All existing tests pass.

### Sprint 2: Daemon Supervisor + CLI Through Team + Brain Model

Transforms the daemon into the formation's internal member supervisor. Work is sequential:

**Step 1 — Daemon RESTful HTTP API:**
- Add OpenAPI-documented endpoints to existing axum server
- `POST /api/members/start` — launch member(s)
- `POST /api/members/stop` — stop member(s)
- `GET /api/members` — member status
- `GET /api/health` — daemon health (exists, enhance)
- Daemon calls existing `start_local_members()`/`stop_local_members()` internally

**Step 2 — State ownership:**
- Daemon owns `state.json` — all mutations through daemon
- CLI reads `state.json` for display only
- State file path and PID + port written to daemon state file

**Step 3 — CLI migration:**
- `bm start` → ensures daemon running → `POST /api/members/start`
- `bm stop` → `POST /api/members/stop` (daemon keeps running)
- `bm stop --all` → stop members + stop daemon
- `bm status` → `GET /api/members` from daemon
- `bm chat` → `formation.exec_in()`
- `bm attach` → `formation.shell()`

**Step 4 — `bm env`:**
- `bm env create` → `formation.setup()` (replaces `bm runtime create`)
- `bm env delete` → teardown (replaces `bm runtime delete`)

**Step 5 — Brain model:**
- Brain delegates loop spawning to daemon via HTTP API
- Brain system prompt updated
- `bm-agent` gains daemon communication for loop management

**Auth model unchanged:** Daemon reads `gh_token` from config, injects as `GH_TOKEN` env var — same as today.

**Demo:** `bm start` → daemon starts → members launched via HTTP API. `bm stop` → members stopped, daemon keeps running. Brain delegates loops to daemon. `bm env create` replaces `bm runtime create`. All existing tests pass.

### Sprint 3: GitHub App Identity + bm fire + Credentials Export

The auth model swap. Work is sequential:

**Step 1 — JWT auth module:**
- `git/app_auth.rs`: JWT signing (`jsonwebtoken` crate), installation token exchange
- Unit tests with mock HTTP

**Step 2 — Manifest flow in `bm hire`:**
- Axum callback server on `127.0.0.1:{port}`, auto-submitting HTML form
- Browser open + URL fallback for headless, 5-minute timeout
- Code exchange via `POST /app-manifests/{code}/conversions`
- Store App credentials via `formation.credential_store(GitHubApp)`
- Install App on team repo + project repos
- Flags: `--reuse-app`, `--app-id`, `--private-key-file`, `--installation-id`, `--save-credentials`
- `bm hire` idempotency for existing members (replace App or reconnect)
- Wire into `bm init` wizard (require org, no token prompt)

**Step 3 — Token lifecycle in daemon:**
- On `POST /api/members/start`: read App credentials from keyring, cache in memory
- Sign JWT → exchange for installation token
- `formation.setup_token_delivery()` (one-time: create `.config/gh/`, git credential helper in `.git/config`)
- `formation.refresh_token()` (atomic `hosts.yml` write)
- 50-minute refresh loop per member
- Exponential backoff on failure

**Step 4 — Remove `gh_token`:**
- Remove `gh_token` from `TeamEntry`/`Credentials`
- Remove `require_gh_token()`
- Remove `gh_token: Option<&str>` from all `git/github.rs` functions — operator functions use `detect_token()`, member functions use daemon-managed tokens
- Update `bm init` to validate `gh auth` session, not prompt for token
- `bm projects add` installs member Apps on new repos

**Step 5 — `bm fire`:**
- Stop member via team → formation → daemon
- Uninstall App via `DELETE /app/installations/{id}` (JWT auth)
- Remove credentials from credential store
- Remove member directory + workspace
- `--keep-app` flag
- Print manual App deletion instructions

**Step 6 — Credentials export/import:**
- `bm credentials export -o <file>` — reads all member credentials from keyring via formation
- `bm init --credentials-file <file>` — imports during init on new machine
- File format: YAML with GitHub App + bridge credentials per member
- 0600 permissions, security warning

**Step 7 — Test migration:**
- E2E: one manifest flow test, rest use pre-provisioned App via `--reuse-app`
- Exploratory tests updated for per-member App auth
- Remove `TESTS_GH_TOKEN` from test infrastructure

**Demo:** Full operator journey: `bm init` (org, no token prompt) → `bm hire` (App created, bot identity) → `bm start` (per-member tokens via `hosts.yml`) → `gh issue list` works with bot identity → token refreshes at 50 min → `bm fire` (App uninstalled) → `bm credentials export` + `bm init --credentials-file` on fresh system. All tests green.
