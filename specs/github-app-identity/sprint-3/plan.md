# Sprint 3: GitHub App Identity + bm fire + Credentials Export

## Checklist

- [ ] Add `jsonwebtoken` dependency with `aws_lc_rs` + `use_pem` features
- [ ] Implement `git/app_auth.rs` (JWT signing, installation token exchange)
- [ ] Implement manifest flow in `bm hire` (axum callback, browser, URL fallback, 5-min timeout)
- [ ] Store App credentials via `formation.credential_store(GitHubApp)`
- [ ] Add `--reuse-app`, `--app-id`, `--client-id`, `--private-key-file`, `--installation-id`, `--save-credentials` flags
- [ ] Handle `bm hire` idempotency (existing member: replace App or reconnect)
- [ ] Install App on team repo + project repos
- [ ] Wire App creation into `bm init` wizard (require org, no token prompt)
- [ ] Implement daemon token lifecycle (cache credentials, JWT→token, refresh loop)
- [ ] Implement `formation.setup_token_delivery()` (create `.config/gh/`, git credential helper)
- [ ] Implement `formation.refresh_token()` (atomic `hosts.yml` write)
- [ ] Remove `gh_token` from `TeamEntry`/`Credentials`
- [ ] Remove `require_gh_token()`
- [ ] Remove `gh_token: Option<&str>` from `git/github.rs` functions
- [ ] Update `bm projects add` to install member Apps on new repos
- [ ] Implement `bm fire` with `--keep-app` flag
- [ ] Implement `bm credentials export -o <file>`
- [ ] Implement `bm init --credentials-file <file>`
- [ ] Migrate E2E tests (hybrid: one manifest test, rest pre-provisioned)
- [ ] Migrate exploratory tests to per-member App auth
- [ ] Update profile docs and user-facing docs
- [ ] Remove `TESTS_GH_TOKEN` from test infrastructure

## Steps (Sequential)

### 1. JWT Auth Module

**Objective:** Build `git/app_auth.rs` for GitHub App authentication.

**Implementation:**
- Add `jsonwebtoken = { version = "10", features = ["aws_lc_rs", "use_pem"] }` to Cargo.toml
- `generate_jwt(client_id, private_key_pem) -> Result<String>` — RS256, `iss`=client_id, `iat`=now-60, `exp`=now+600
- `exchange_for_installation_token(jwt, installation_id) -> Result<InstallationToken>` — `POST /app/installations/{id}/access_tokens` via `gh api`
- `InstallationToken { token: String, expires_at: DateTime }` struct

**Tests:** Unit tests with mock private key and mocked HTTP responses for token exchange.

### 2. Manifest Flow in `bm hire`

**Objective:** `bm hire` creates a GitHub App per member via the manifest flow.

**Implementation:**
- Uses a scoped `tokio::runtime::Runtime::new().block_on()` for the axum server (same pattern as `daemon/run.rs`)
- Construct manifest JSON (name, permissions including `organization_projects:admin`, redirect_url, setup_url)
- Start temporary axum server on `127.0.0.1:{random_port}` with:
  - `GET /start` — serves auto-submitting HTML form with manifest JSON
  - `GET /callback` — receives `code` + `state`, validates state, exchanges code via `POST /app-manifests/{code}/conversions`, stores App ID + Client ID + PEM. Then **redirects browser to `{html_url}/installations/new`** to prompt App installation.
  - `GET /installed` — receives redirect after user installs the App on org. Signs JWT from the new PEM, queries `GET /app/installations` to get `installation_id`. Stores installation ID. Shows success page.
  - 5-minute timeout per step, clean shutdown on timeout
- Two browser clicks required: (1) "Create GitHub App", (2) "Install" on org
- Determine org via `gh api /repos/{owner}/{repo}` → `owner.type`
  - Org → `https://github.com/organizations/{org}/settings/apps/new`
  - Personal → block with error (org required)
- Open browser to `http://127.0.0.1:{port}/start` (or print URL for headless)
- For headless: print URLs at each step (create URL, then install URL after first callback)
- Name collision check: `GET https://github.com/apps/{slug}` before starting
- After installation ID obtained: add team repo + all existing project repos to the installation
- Flags: `--reuse-app` + `--app-id`/`--client-id`/`--private-key-file`/`--installation-id` bypass entire browser flow (client-id is required for JWT signing)
- `--save-credentials <path>` writes credentials to file
- Handle existing member dir: replace App (no --reuse-app) or reconnect (--reuse-app)
- **Sequencing with `hire_member()`:** The manifest flow and credential storage MUST happen AFTER `hire_member()` completes (which includes `finalize_member_manifest()` and `render_member_placeholders()`). The placeholder rendering is profile-level and must complete before App credentials are stored. The App creation step should be a new function called after `hire_member()` returns, not inserted into `hire_member()` itself.

See `research/manifest-flow-installation-gap.md` for the corrected two-step flow.

**Tests:** Unit tests for manifest JSON construction, name collision detection. Integration test with `InMemoryCredentialStore`.

### 3. Wire into `bm init`

**Objective:** `bm init` wizard uses `gh auth` session, requires org, creates Apps during hire.

**Implementation:**
- Remove token prompt from init wizard
- Validate `gh auth` session via `detect_token_non_interactive()`
- Block personal accounts — require org
- Hire step triggers App creation per member (manifest flow or pre-generated)
- Projects add step installs member Apps on project repos
- `bm init --credentials-file` imports credentials from file instead of creating Apps

**Tests:** E2E test for init flow with pre-provisioned App credentials.

### 4. Daemon Token Lifecycle

**Objective:** Daemon manages per-member installation tokens with automatic refresh.

**Implementation:**
- On `POST /api/members/start`: read member's App credentials from keyring via credential store, cache in memory
- Sign JWT → exchange for installation token
- Call `formation.setup_token_delivery(member, workspace, bot_user)`:
  - Create `{workspace}/.config/gh/` directory
  - Write initial `hosts.yml` with token + bot user + `git_protocol: https`
  - Write git credential helper to `{workspace}/.git/config` (NOT global)
- Call `formation.refresh_token(member, workspace, token)`:
  - Atomic write (temp file + rename) to `hosts.yml`
- Launch member process with `GH_CONFIG_DIR={workspace}/.config/gh/`
- Spawn background refresh task per member (50-minute interval)
- On refresh failure: log, exponential backoff, existing token valid until 1-hour expiry
- On daemon restart: re-read credentials from keyring for adopted members, refresh immediately

**Tests:** Integration test — daemon generates token, writes hosts.yml, member process can use `gh`.

### 5. Remove `gh_token`

**Objective:** Remove all PAT-based auth paths.

**Implementation:**
- Remove `gh_token` from `Credentials` struct in `config/mod.rs`
- Remove `require_gh_token()` from `config/mod.rs`
- Remove `gh_token: Option<&str>` parameter from all `git/github.rs` functions
- Operator-facing functions call `detect_token()` internally
- Member-facing code paths use daemon-managed tokens (no function parameter needed)
- Daemon no longer reads `team.credentials.gh_token` — reads App credentials from keyring
- Update all callers across the codebase

**Tests:** Verify no compilation errors. All tests pass with new auth model.

### 6. `bm fire`

**Objective:** New command to remove a team member with App cleanup.

**Implementation:**
- Stop member via `team.stop(member)`
- Sign JWT from member's App credentials
- Uninstall App: `DELETE /app/installations/{installation_id}` (JWT-authenticated)
- Remove credentials from credential store (`remove` all keys with member prefix)
- Remove member directory from team repo
- Remove member workspace
- Print manual App deletion instructions (no API exists)
- `--keep-app` flag: skip uninstallation, preserve for reuse

**Tests:** Integration test — fire removes credentials, directory, workspace. E2E test with real App uninstallation.

### 7. Credentials Export/Import

**Objective:** Machine migration support.

**Implementation:**
- `bm credentials export -o <file>` — iterate all members, read credentials from keyring via formation, write YAML file with 0600 permissions, print security warning
- `bm init --credentials-file <file>` — during init, import credentials into keyring via formation
- File format per design.md "Credentials Export Format" section
- Include both GitHub App and bridge credentials per member

**Tests:** Round-trip test — export from one credential store, import into another, verify all values match.

### 8. `bm projects add` Enhancement

**Objective:** Install member Apps on new project repos.

**Implementation:**
- After adding project repo to team, for each hired member with App credentials:
- Sign JWT from member's credentials
- Add repo to installation: `PUT /user/installations/{installation_id}/repositories/{repo_id}`

**Tests:** E2E test — add project, verify member App has access.

### 9. Test Migration

**Objective:** All tests use per-member App auth.

**Implementation:**
- One E2E test exercises manifest flow (URL fallback + automated code exchange, cleanup)
- All other E2E tests use pre-provisioned App via `--reuse-app` with CI secrets
- Exploratory tests updated for per-member App credentials
- Remove `TESTS_GH_TOKEN` from test infrastructure
- Update test helpers and fixtures

**Tests:** `just test` and `just exploratory-test` pass with new auth model.

### 10. Profile and Docs Update

**Objective:** Documentation reflects new auth model.

**Implementation:**
- Update profile `github-project` skill documentation (tokens are auto-managed, no PAT references). Note: the skill was renamed from `gh` to `github-project` on main — both `scrum` and `scrum-compact` profiles already use this name.
- Update `docs/content/getting-started/`, `docs/content/reference/cli.md`
- Update `docs/content/how-to/generate-team-repo.md`
- Document `bm fire`, `bm env`, `bm credentials export/import`
- Document org requirement
- Update CLAUDE.md with new auth model context
- Verify that `github-project` skill scripts (`create-issue.sh`, `query-issues.sh`, `subtask-ops.sh`) work with `GH_CONFIG_DIR`-based auth — these use `gh api graphql` which reads from `hosts.yml`, so they should work without changes.

**Tests:** `just docs-build` succeeds.

## Review Findings Incorporated

- **Installation ID gap:** Manifest flow is two browser clicks (create + install). Local server handles `/callback` and `/installed`. See `research/manifest-flow-installation-gap.md`.
- **Axum in sync CLI:** Uses scoped `Runtime::new().block_on()` for the manifest flow server (same pattern as daemon).
- **`gh_token` removal strategy:** Mechanical find-and-replace. Remove parameter from `git/github.rs`, fix all compilation errors. Rust compiler catches every call site.
- **Daemon API auth:** Shared secret token in daemon state file for localhost API (from Sprint 2 review).
- **`bm fire` partial failure:** Steps execute sequentially. On error, report what succeeded and what failed. No rollback — operator can re-run or manually clean up.
- **Credentials export security:** 0600 permissions, security warning printed. Refuse to write inside a git repo directory.

## Deviations from Design

None — this sprint implements the full design.
