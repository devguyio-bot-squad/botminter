# Sprint 3: GitHub App Identity + bm fire + Credentials Export

## Checklist

- [x] Add `jsonwebtoken` dependency with `aws_lc_rs` + `use_pem` features
- [x] Implement `git/app_auth.rs` (JWT signing, installation token exchange)
- [ ] Implement manifest flow in `bm hire` (axum callback, browser, URL fallback, 5-min timeout) — *deferred: only --reuse-app path implemented*
- [x] Store App credentials via `formation.credential_store(GitHubApp)`
- [x] Add `--reuse-app`, `--app-id`, `--client-id`, `--private-key-file`, `--installation-id`, `--save-credentials` flags
- [x] Handle `bm hire` idempotency (existing member: replace App or reconnect)
- [x] Install App on team repo + project repos
- [ ] Wire App creation into `bm init` wizard (require org, no token prompt) — *import via --credentials-file works; interactive creation deferred*
- [x] Implement daemon token lifecycle (cache credentials, JWT→token, refresh loop)
- [x] Implement `formation.setup_token_delivery()` (create `.config/gh/`, git credential helper)
- [x] Implement `formation.refresh_token()` (atomic `hosts.yml` write)
- [x] Remove `gh_token` from `TeamEntry`/`Credentials`
- [x] Remove `require_gh_token()`
- [x] Remove `gh_token: Option<&str>` from `git/github.rs` functions
- [x] Update `bm projects add` to install member Apps on new repos
- [x] Implement `bm fire` with `--keep-app` flag
- [x] Implement `bm credentials export -o <file>`
- [x] Implement `bm init --credentials-file <file>`
- [x] Migrate E2E tests (hybrid: one manifest test, rest pre-provisioned) — *pre-provisioned only, no manifest flow test*
- [ ] Migrate exploratory tests to per-member App auth
- [x] Update profile docs and user-facing docs — *partial: CLI reference updated*
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

### 2. App Credential Storage in `bm hire`

**Objective:** `bm hire` stores GitHub App credentials and ensures the App has access to team repos.

**Implementation (current — `--reuse-app` path):**
- CLI flags: `--reuse-app`, `--app-id`, `--client-id`, `--private-key-file`, `--installation-id`, `--save-credentials`
- Reads PEM file, validates all 4 required flags, stores via `formation.credential_store(GitHubApp)`
- After storing: calls `ensure_app_on_repos()` which checks each team + project repo via `GET /repos/{owner}/{repo}/installation` and adds missing repos via `PUT /user/installations/{id}/repositories/{repo_id}` (operator PAT auth)
- `--save-credentials <path>` writes credentials to YAML file with 0600 permissions
- Skips App setup entirely when `github_repo` is empty (test teams, v1 teams)
- **Sequencing:** App credential storage happens AFTER `hire_member()` completes (placeholder rendering must finish first)

**Implementation (deferred — browser manifest flow):**
- The manifest flow code exists in `git/manifest_flow.rs` (axum server, form POST, code exchange, installation redirect)
- Not the focus of this sprint — will be validated after the auth pipeline is proven end-to-end

**Tests:** Unit tests for manifest JSON construction, credential key conventions, credential storage with `InMemoryCredentialStore`, file permissions on saved credentials.

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
