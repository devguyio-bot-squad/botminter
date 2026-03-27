# Sprint 3: GitHub App Identity + bm fire + Credentials Export

## Objective

Swap the auth model from shared PAT to per-member GitHub Apps. Build the manifest flow in `bm hire`, daemon-managed token lifecycle with `GH_CONFIG_DIR` + `hosts.yml` delivery, and remove `gh_token` from the codebase. Add `bm fire`, `bm credentials export/import`, and migrate all tests.

## Prerequisites

Sprint 2 delivered: daemon as formation-internal supervisor with HTTP API, `bm start/stop/status` through Team, `bm env`, Brain delegates loops to daemon. Auth still uses `gh_token`.

## Deviations from Design

1. **Browser manifest flow deferred.** The interactive browser flow (App creation via auto-submitting form + installation redirect) is implemented but not the focus of this sprint. Sprint 3 drives the entire auth pipeline through `--reuse-app` with a single pre-provisioned App. The browser flow will be validated as a follow-up.

2. **Single shared test App.** Instead of one App per E2E scenario, all scenarios share one pre-provisioned App (`bm-test-app`). The App is installed on "All repositories" in the test org, so `ensure_app_on_repos` is a no-op.

3. **`TESTS_GH_TOKEN` retained.** The operator PAT is still needed for test infrastructure (repo create/delete, `bm init`). Only member identity uses the App. Requirement 18 ("remove `TESTS_GH_TOKEN`") is deferred.

## Key References

- Design: `specs/github-app-identity/design.md`
- ADR-0011: `.planning/adrs/0011-github-app-per-member-identity.md` (GitHub App identity)
- ADR-0008: `.planning/adrs/0008-team-runtime-architecture.md` (Formation credential delivery)
- Research: `specs/github-app-identity/research/manifest-flow.md`
- Research: `specs/github-app-identity/research/jwt-and-app-lifecycle.md`
- Research: `specs/github-app-identity/research/token-delivery.md`
- Sprint plan: `specs/github-app-identity/sprint-3/plan.md`

## Requirements

1. `git/app_auth.rs` MUST provide JWT generation (RS256, `iss`=Client ID, `iat`=now-60, `exp`=now+600) and installation token exchange via `POST /app/installations/{id}/access_tokens`. MUST use `jsonwebtoken` v10 with `aws_lc_rs` + `use_pem` features. Ref: research/jwt-and-app-lifecycle.md.

2. `bm hire` MUST support three credential acquisition paths: pre-generated flags (highest priority), browser manifest flow, URL fallback (headless). Ref: ADR-0011 "Credential acquisition modes".

3. The manifest flow is a two-step browser flow: (1) App creation via auto-submitting form → `/callback` receives code, (2) App installation via redirect to `{html_url}/installations/new` → `/installed` receives installation event. The manifest JSON MUST include both `redirect_url` and `setup_url`. The local server MUST use a scoped `tokio::runtime::Runtime::new().block_on()` (same pattern as daemon). MUST timeout after 5 minutes per step. Ref: research/manifest-flow-installation-gap.md.

4. Manifest permissions MUST include `organization_projects: admin` (NOT `projects: admin`). Ref: research/manifest-flow.md correction.

5. `bm init` MUST require a GitHub organization — personal accounts MUST be blocked with clear error. Ref: requirements.md R5.

6. `bm hire` MUST be idempotent on the member directory: existing member without `--reuse-app` creates a new App (replacement), existing member with `--reuse-app` stores provided credentials (reconnect). Ref: design.md "bm hire Idempotency" table.

7. `bm hire --save-credentials <path>` MUST write App credentials to file during creation.

8. The daemon MUST load member App credentials on-demand when `POST /api/members/start` is received (NOT at daemon startup). Credentials MUST be cached in memory for the refresh loop. Ref: requirements.md I7.

9. `formation.setup_token_delivery()` MUST create `{workspace}/.config/gh/`, write initial `hosts.yml`, and configure git credential helper in `{workspace}/.git/config` (NOT global `~/.gitconfig`). Ref: requirements.md R6.

10. `formation.refresh_token()` MUST atomically write `hosts.yml` (temp file + rename). Ref: ADR-0011 "Token delivery".

11. The daemon MUST refresh installation tokens at the 50-minute mark per member. On failure, MUST retry with exponential backoff. Existing token remains valid until its 1-hour expiry. Ref: design.md "Token Lifecycle".

12. `gh_token` MUST be removed from `TeamEntry`, `Credentials` struct, and all `git/github.rs` function parameters. `require_gh_token()` MUST be removed. Operator functions MUST use `detect_token()` internally.

13. Installation tokens MUST NOT be validated via `/user` endpoint (returns 403). Trust the JWT exchange flow. Ref: requirements.md R4.

14. `bm fire <member>` MUST stop the member, uninstall the App (`DELETE /app/installations/{id}`), remove credentials from keyring, remove member directory and workspace. `--keep-app` MUST skip uninstallation. MUST print manual App deletion instructions. Ref: design.md "bm fire" section.

15. `bm credentials export -o <file>` MUST read all members' credentials (GitHub App + bridge) from keyring via formation and write YAML with 0600 permissions + security warning. Ref: design.md "Credentials Export Format".

16. `bm init --credentials-file <file>` MUST import credentials during init on new machine. Ref: design.md "Machine Migration".

17. `bm projects add` MUST install all hired members' Apps on the new project repo. Ref: design.md "bm projects add".

18. E2E tests MUST use pre-provisioned App via `--reuse-app` with credentials from `.envrc` env vars (`TESTS_APP_ID`, `TESTS_APP_CLIENT_ID`, `TESTS_APP_INSTALLATION_ID`, `TESTS_APP_PRIVATE_KEY_FILE`). `TESTS_GH_TOKEN` is retained for operator-level test infrastructure. Ref: requirements.md Q12.

19. All profile docs, CLI reference docs, and knowledge files referencing `GH_TOKEN` or PATs MUST be updated. Note: the `gh` skill was renamed to `github-project` on main — update `github-project` skill docs (not `gh`).

## Sprint 3 Build Notes

### `just test` excludes E2E during Sprint 3
The `just test` recipe was changed to `unit + conformance` only. E2E tests call `bm hire` which now triggers the manifest flow (browser open + 5-min timeout) for teams with real `github_repo` values. E2E tests cannot pass until Step 9 (Migrate E2E tests) adapts them to use `--reuse-app` with pre-provisioned credentials.

Run E2E separately with `just e2e` only after Step 9 is complete.

### Pre-provisioned GitHub App (DONE)

One GitHub App (`bm-test-app`) has been created in `devguyio-bot-squad` and installed on "All repositories". Credentials are in `.envrc` (gitignored):

- `TESTS_APP_ID=<REDACTED>`
- `TESTS_APP_CLIENT_ID=<REDACTED>`
- `TESTS_APP_INSTALLATION_ID=<REDACTED>`
- `TESTS_APP_PRIVATE_KEY_FILE=$HOME/.config/github-apps/bm-test-app.pem`

All E2E scenarios share this single App via `--reuse-app`. The operator PAT (`TESTS_GH_TOKEN`) is still used for test infrastructure (repo create/delete, init). The App provides member identity.

### App permissions

Repository: Contents (R/W), Issues (R/W), Pull requests (R/W), Administration (R/W).
Organization: Projects (Admin) — this is "Organization projects", NOT classic "Projects".

### `ensure_app_on_repos` behavior

After `bm hire --reuse-app` stores credentials, it checks each team + project repo via `GET /repos/{owner}/{repo}/installation`. If the App isn't installed on a repo (404), it adds it via `PUT /user/installations/{id}/repositories/{repo_id}` using the operator's PAT. If the App is installed on "All repositories", this is a no-op (always 200).

### Sprint approach change

The manifest flow (browser-based App creation) is deferred. Sprint 3 focuses on the `--reuse-app` path end-to-end: credential storage, token lifecycle, token delivery, `bm fire`, credentials export/import. The browser flow will be added as a follow-up once the auth pipeline is proven.

## Acceptance Criteria

1. **Given** `bm hire <role> --name superman`, **when** the operator completes the manifest flow, **then** a GitHub App named `{team}-superman` is created, credentials stored in keyring, App installed on team repo + project repos.

2. **Given** `bm hire --reuse-app --app-id 123 --client-id Iv1.abc --private-key-file key.pem --installation-id 456`, **then** credentials are stored without manifest flow.

3. **Given** a headless environment, **when** browser fails to open, **then** the localhost URL is printed.

4. **Given** no callback within 5 minutes, **then** server shuts down with error.

5. **Given** `bm hire` for an existing member (no `--reuse-app`), **then** new App created, old credentials replaced, member dir preserved.

6. **Given** `bm init` on a personal account, **then** it fails with clear "org required" error.

7. **Given** `bm start superman`, **when** daemon processes the request, **then** App credentials loaded from keyring, JWT signed, installation token exchanged, `hosts.yml` written, member launched with `GH_CONFIG_DIR`.

8. **Given** 50 minutes elapsed, **then** daemon refreshes token and updates `hosts.yml` atomically.

9. **Given** `GH_CONFIG_DIR` set on member process, **when** `gh issue list` runs, **then** uses token from `hosts.yml` with bot identity.

10. **Given** `git push` in member workspace, **then** git uses credential helper from `.git/config` (not global), which reads from `GH_CONFIG_DIR`.

11. **Given** `bm fire superman`, **then** App uninstalled, credentials removed, member dir removed, manual deletion instructions printed.

12. **Given** `bm fire --keep-app`, **then** credentials removed but App installation preserved.

13. **Given** `bm credentials export -o creds.yml`, **then** all credentials written with 0600 permissions.

14. **Given** `bm init --credentials-file creds.yml` on new machine, **then** credentials imported, `bm teams sync -a` produces working workspaces.

15. (Regression) **Given** `just test`, **when** run, **then** all tests pass with new auth model.

16. (Regression) **Given** `just exploratory-test`, **when** run, **then** all exploratory tests pass with per-member App auth.
