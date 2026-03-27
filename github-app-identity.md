# Milestone: v0.08 — Per-Member GitHub App Identity

## Vision

Replace the shared PAT authentication model with per-member GitHub Apps. Each team member gets its own GitHub App (created during `bm hire` via the Manifest flow), its own bot identity on GitHub, and its own short-lived installation tokens managed autonomously at runtime. Operator commands use the operator's native `gh auth` session. No backward compatibility with PATs.

## ADR

See `.planning/adrs/0011-github-app-per-member-identity.md` for the full architectural decision.

## Requirements

### AUTH — Authentication Model

- **AUTH-01:** `bm hire` creates a GitHub App per member via the App Manifest flow (browser redirect + one-click confirmation)
- **AUTH-02:** App credentials (App ID, private key, installation ID) are stored in the system keyring via `CredentialStore`, keyed by `botminter/{team}/{member}/`
- **AUTH-03:** `credentials.gh_token` is removed from `TeamEntry` and `Credentials` struct; `config.yml` contains no GitHub secrets
- **AUTH-04:** Operator commands (`bm init`, `bm teams sync`, `bm hire`, `bm projects add`, etc.) use the operator's `gh auth` session at runtime — no stored operator token
- **AUTH-05:** `bm init` wizard no longer prompts for or stores a GitHub token; it validates the operator's existing `gh auth` session and fails with clear guidance if none exists

### TOKEN — Per-Member Token Lifecycle

- **TOKEN-01:** Each member process (Ralph or Brain) signs a JWT (RS256) from its App's private key on startup and exchanges it for a GitHub installation access token
- **TOKEN-02:** Installation tokens are refreshed automatically at the 50-minute mark via a background task within the member process
- **TOKEN-03:** The refreshed token is used for all subsequent `gh` CLI calls (`GH_TOKEN` env var)
- **TOKEN-04:** Token generation and refresh failures are logged and retried with exponential backoff; member operation continues with the existing token until expiry

### MANIFEST — App Manifest Flow

- **MANIFEST-01:** `bm hire` constructs a JSON manifest with App name `{team}-{member}`, required permissions (`issues:write`, `contents:write`, `pull_requests:write`, `projects:admin`), and a localhost callback URL
- **MANIFEST-02:** `bm hire` starts a temporary local HTTP server, opens the operator's browser to the GitHub manifest creation URL, and receives the callback with the temporary code
- **MANIFEST-03:** `bm hire` exchanges the code for App credentials via `POST /app-manifests/{code}/conversions` (using `gh api`)
- **MANIFEST-04:** `bm hire` installs the newly created App on the team repo and stores the installation ID
- **MANIFEST-05:** App name collisions are detected and reported with actionable guidance (suggest alternative names or manual cleanup)

### CLEANUP — PAT Removal

- **CLEANUP-01:** `detect_token()` and `detect_token_non_interactive()` in `git/github.rs` are repurposed for operator-only use — they detect the operator's `gh auth` session, never a stored token
- **CLEANUP-02:** All `gh_token: Option<&str>` parameters in `git/github.rs` functions are replaced with operator auth resolution (call `gh auth token` or expect `GH_TOKEN` env var)
- **CLEANUP-03:** `launch_ralph()` and `launch_brain()` no longer receive `gh_token` as a parameter — each member resolves its own token from keyring-stored App credentials
- **CLEANUP-04:** The `bm init` token prompt, `mask_token()`, and `validate_token()` functions are removed or repurposed
- **CLEANUP-05:** Profile `gh` skill documentation is updated to reflect App-based auth (tokens are auto-managed, not user-provided)

### INFRA — Infrastructure Changes

- **INFRA-01:** `jsonwebtoken` crate is added as a dependency for RS256 JWT signing
- **INFRA-02:** A new `git/app_auth.rs` module provides: JWT generation, installation token exchange, token caching with TTL, and background refresh task
- **INFRA-03:** `CredentialStore` trait is extended (or a new provider added) to store/retrieve App credentials (App ID, private key PEM, installation ID)

### TEST — Test Strategy

- **TEST-01:** E2E tests create a test GitHub App via the manifest flow (or use a pre-provisioned test App) instead of `TESTS_GH_TOKEN`
- **TEST-02:** Unit tests for JWT generation and token exchange use mocked HTTP responses
- **TEST-03:** Integration tests verify the full hire-to-start flow: App creation, keyring storage, token generation, `gh` CLI operation
- **TEST-04:** Exploratory tests are updated to use per-member App auth instead of shared PAT

## Phases

### Phase 11: App Auth Module & Operator Auth Migration

**Goal:** Build the `git/app_auth.rs` module (JWT signing, token exchange, caching) and migrate all operator commands to use `gh auth` session instead of stored `gh_token`.

**Requirements:** AUTH-03, AUTH-04, AUTH-05, CLEANUP-01, CLEANUP-02, CLEANUP-04, INFRA-01, INFRA-02

**Success Criteria:**
1. `git/app_auth.rs` can sign a JWT from an App ID + private key PEM string, exchange it for an installation token, and cache the token with TTL tracking
2. All operator commands work with `gh auth login` session — no `credentials.gh_token` in config
3. `bm init` no longer prompts for a GitHub token; fails clearly if `gh auth` is not configured
4. `credentials.gh_token` is removed from `TeamEntry`; existing config files with the field are handled gracefully (field ignored on load)
5. Unit tests cover JWT generation, token exchange (mocked), and cache expiry logic

### Phase 12: Manifest Flow in `bm hire`

**Goal:** `bm hire` creates a GitHub App per member via the Manifest flow and stores credentials in the keyring.

**Requirements:** AUTH-01, AUTH-02, MANIFEST-01, MANIFEST-02, MANIFEST-03, MANIFEST-04, MANIFEST-05, INFRA-03

**Success Criteria:**
1. `bm hire <role> --name <name>` opens a browser, the operator clicks once, and a GitHub App named `{team}-{name}` is created
2. App ID, private key, and installation ID are stored in the keyring under `botminter/{team}/{member}/`
3. The App is installed on the team repo with the correct permissions
4. Name collisions produce a clear error with suggested resolution
5. `bm hire` works in non-interactive mode with pre-provisioned App credentials (for CI/testing)

### Phase 13: Per-Member Token Lifecycle in Runtime

**Goal:** Member processes (Ralph and Brain) autonomously manage their own installation tokens — generate on startup, refresh at 50 minutes.

**Requirements:** TOKEN-01, TOKEN-02, TOKEN-03, TOKEN-04, CLEANUP-03

**Success Criteria:**
1. `bm start <member>` reads App credentials from keyring, generates an installation token, and passes it to the member process
2. The member process refreshes the token before expiry via a background task
3. All `gh` CLI calls within the member use the current installation token
4. Token refresh failure is logged and retried; the member continues operating with the existing token until it expires
5. `launch_ralph()` and `launch_brain()` no longer accept `gh_token` as a parameter

### Phase 14: Profile & Docs Update

**Goal:** Profile skill documentation, `gh` skill setup scripts, and user-facing docs reflect the new App-based auth model.

**Requirements:** CLEANUP-05

**Success Criteria:**
1. Profile `gh` skill references are updated — no mention of operator-provided PATs
2. `docs/content/` pages covering auth, init wizard, and CLI reference are updated
3. Knowledge files in profiles that reference `GH_TOKEN` are updated to describe auto-managed tokens

### Phase 15: E2E & Exploratory Test Migration

**Goal:** All test infrastructure uses per-member App auth instead of shared PAT.

**Requirements:** TEST-01, TEST-02, TEST-03, TEST-04

**Success Criteria:**
1. E2E tests create or use a pre-provisioned test App — `TESTS_GH_TOKEN` is no longer used
2. The full hire-to-start flow is tested end-to-end with real GitHub App creation
3. Exploratory tests on `bm-test-user@localhost` use per-member App credentials
4. All existing test scenarios pass with the new auth model

## Open Questions

1. **E2E test App provisioning:** Should E2E tests create real GitHub Apps (slow, requires cleanup) or use a single pre-provisioned test App with multiple installations? Pre-provisioned is faster but doesn't test the manifest flow itself.

2. **`bm hire --non-interactive`:** CI and scripted flows can't open a browser. Options: (a) accept pre-generated App credentials via flags/env vars, (b) use the GitHub API directly with a PAT to create the App (requires the operator to have a PAT for CI only). Option (a) is simpler and avoids reintroducing PATs.

3. **App cleanup on `bm fire` (future):** Should removing a member also delete the GitHub App? The Apps API supports `DELETE /app/installations/{id}` for uninstalling, but deleting the App itself requires the App's credentials. Worth designing but not blocking for v0.08.

4. **GitHub Enterprise Server:** The manifest flow URL is hardcoded to `github.com`. GHES support would need a configurable base URL. Not blocking for v0.08 (Alpha policy).
