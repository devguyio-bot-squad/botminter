---
status: proposed
date: 2026-03-24
decision-makers: operator (ahmed), claude
---

# Per-member GitHub App identity replaces shared PAT

## Problem

BotMinter currently uses a single Personal Access Token (PAT) stored in `~/.botminter/config.yml` and shared across all team members at runtime. This creates three problems: (1) all member actions are attributed to the operator's GitHub user, making audit trails ambiguous; (2) a single rate-limit pool is shared across all members and the operator; (3) the PAT has account-wide scope, granting every member access to all repos the operator can see — far broader than necessary.

PATs cannot be created programmatically — operators must visit the GitHub UI, check the right permission boxes, copy the token, and paste it into `bm init`. This is error-prone and the most common support friction during onboarding.

## Constraints

* Each member MUST appear as a distinct identity on GitHub (separate bot user for attribution and audit)
* Operator commands (`bm init`, `bm teams sync`, etc.) MUST use the operator's own GitHub identity — not a bot
* No backward compatibility with PAT-based auth — PAT support is removed entirely (Alpha policy: hard break, re-run `bm init`)
* Token lifecycle MUST be automatic — no manual token rotation by operators
* All GitHub operations continue to go through the `gh` CLI (no direct HTTP client for GitHub API in `bm`)
* Private keys MUST be stored via the formation's credential store (system keyring for local formation), not in config files on disk. `bm credentials export` is an intentional escape hatch for machine portability — file written with 0600 permissions and explicit security warning
* Token delivery uses the formation's `setup_token_delivery()` (one-time) and `refresh_token()` (per cycle) methods — different formations deliver tokens differently
* The team repo MUST be in a GitHub organization (personal accounts are not supported — required for `organization_projects` permission)
* Installation tokens are NOT validated via `/user` endpoint (which returns 403 for installation tokens) — trust the JWT exchange flow

## Decision

Each team member gets its own GitHub App, created during `bm hire` via the GitHub App Manifest flow. Operator commands use the operator's existing `gh auth login` session. The shared `credentials.gh_token` field is removed from `TeamEntry`.

### Identity model

| Actor | GitHub identity | Auth source |
|-------|----------------|-------------|
| Operator (`bm init`, `bm teams sync`, `bm hire`, etc.) | Operator's GitHub user | `gh auth token` at runtime |
| Member runtime (issues, PRs, board scans) | `{team}-{member}[bot]` | GitHub App installation token via `GH_CONFIG_DIR` |

### App creation via Manifest flow

During `bm hire <role> --name <name>` (called standalone or from within `bm init`):

1. `bm` constructs a JSON manifest specifying the App name (`{team}-{name}`), permissions, `redirect_url`, and `setup_url` (both pointing to the local server)
2. `bm` starts a temporary axum server on `127.0.0.1:{port}` (using a scoped `tokio::runtime::Runtime::new().block_on()`) serving an auto-submitting HTML form
3. `bm` opens the operator's browser to the local start page (or prints URL for headless environments)
4. The form auto-submits to GitHub, the operator clicks "Create GitHub App" (click 1 — permissions are pre-filled)
5. GitHub redirects to the `/callback` endpoint with a temporary `code`
6. `bm` calls `POST /app-manifests/{code}/conversions` (no auth needed) to receive: App ID, Client ID, private key (PEM), client secret, webhook secret, and `html_url`
7. `bm` redirects the browser to `{html_url}/installations/new` to prompt App installation
8. The operator clicks "Install" on their org (click 2 — selects repos)
9. GitHub redirects to the `/installed` endpoint (`setup_url`). `bm` signs a JWT from the new PEM and queries `GET /app/installations` to get the installation ID
10. `bm` stores all credentials (App ID, Client ID, private key, installation ID) via `formation.credential_store(GitHubApp { .. })`
11. `bm` adds the team repo and all existing project repos to the installation

The local server has a 5-minute timeout per step. If no callback arrives, it shuts down with a clear error and instructions to retry. For headless environments, URLs are printed at each step.

### Credential acquisition modes

All modes work in both interactive and `--non-interactive`:

1. **Pre-generated credentials** (highest priority): `--app-id`, `--client-id`, `--private-key-file`, `--installation-id` flags
2. **Browser manifest flow**: opens browser for one-click App creation
3. **URL fallback (headless)**: prints localhost URL for manual browser access

### App ownership

Org-owned. The team repo must be in a GitHub organization (personal accounts are blocked — `organization_projects` permission requires an org). Apps are created under the org (`/organizations/{org}/settings/apps/new`). If the operator lacks org owner permissions for App creation, the error is surfaced with guidance.

### App permissions (manifest)

```json
{
  "name": "{team}-{member}",
  "url": "https://github.com/{org}/{team-repo}",
  "redirect_url": "http://127.0.0.1:{port}/callback",
  "default_permissions": {
    "issues": "write",
    "contents": "write",
    "pull_requests": "write",
    "organization_projects": "admin"
  },
  "default_events": [],
  "public": false
}
```

Note: `organization_projects` (not `projects`) is required for Projects v2. The `projects` key only covers deprecated classic project boards.

### App installation scope

Team repo + project repos. The App is installed on the team repo during `bm hire` and on each project repo during `bm projects add`. A single installation ID per member covers all repos (GitHub installations are per-org/per-account with selectable repos).

### Credential storage

Stored via `formation.credential_store(GitHubApp { team, member })`:

```
Keyring entries (per member, local formation):
  botminter/{team}/{member}/github-app-id           -> "123456"
  botminter/{team}/{member}/github-app-client-id    -> "Iv1.abc123"
  botminter/{team}/{member}/github-app-private-key  -> "-----BEGIN RSA PRIVATE KEY-----\n..."
  botminter/{team}/{member}/github-installation-id  -> "789012"
```

No GitHub credentials in `config.yml`.

### Token lifecycle at runtime

Managed by the daemon (an implementation detail of the formation's member lifecycle):

1. On startup, cache all members' App credentials in memory (read from credential store once)
2. Sign a JWT using the App's private key (RS256, Client ID as `iss`, `iat` = now-60, `exp` = now+600)
3. Exchange the JWT for an installation access token via `POST /app/installations/{id}/access_tokens` (1 hour TTL)
4. Deliver token via `formation.refresh_token()` — for local formation, atomically writes `{workspace}/.config/gh/hosts.yml`
5. Spawn a refresh task that re-generates the token at the 50-minute mark
6. On failure: log, retry with exponential backoff; existing token remains valid until expiry

### Token delivery via GH_CONFIG_DIR

Each member workspace gets isolated `gh` configuration:

```
{workspace}/.config/gh/
  hosts.yml     # Written by formation.deliver_token(), contains installation token
  config.yml    # git_protocol: https
```

Member process launched with `GH_CONFIG_DIR={workspace}/.config/gh/`. `gh` reads `hosts.yml` on every invocation.

Git credential helper is configured per-workspace in `.git/config` (NOT global `~/.gitconfig`):
```
[credential "https://github.com"]
    helper =
    helper = !/usr/bin/gh auth git-credential
```
This is written by `formation.setup_token_delivery()` during first member start. `gh auth git-credential` respects `GH_CONFIG_DIR` set on the member process, so `git push/pull` uses the correct member token.

Token refresh is a file write (atomic: temp file + rename) via `formation.refresh_token()`. No IPC, no sockets, no process signaling.

### Operator auth

Operator commands call `gh auth token` at runtime. The existing `detect_token()` / `detect_token_non_interactive()` functions already implement this. All `gh_token: Option<&str>` parameters are removed from `git/github.rs` functions.

### `bm hire` for existing members

`bm hire` is idempotent on the member directory:

| Member dir exists? | Flags | Behavior |
|---|---|---|
| No | (none) | Create member dir + create new App (normal hire) |
| Yes | (none) | Create new App, keep member dir (App replacement — e.g., after accidental deletion) |
| Yes | `--reuse-app` + creds | Store provided creds, keep member dir (machine migration / reconnect) |
| No | `--reuse-app` + creds | Create member dir, store provided creds (adopt existing App) |

### `bm fire`

```
bm fire <member> [-t team] [--keep-app]
```

1. Stop the member (via team → formation)
2. Uninstall the App via `DELETE /app/installations/{id}` (JWT-authenticated)
3. Remove credentials from credential store
4. Remove member directory from team repo
5. Print instructions for manual App deletion via GitHub UI (no API exists for App deletion)

`--keep-app`: skip step 2, preserve installation for reuse.

### Machine migration

Two-step flow matching today's `bm init` + `bm teams sync -a`:

```bash
# Old machine (before decommissioning):
bm credentials export -o team-creds.yml

# New machine:
bm init --credentials-file team-creds.yml
bm teams sync -a
```

`bm credentials export` reads all members' App credentials from the credential store (via formation) and writes a portable YAML file. `bm init --credentials-file` imports them into the new machine's credential store.

### New dependency

`jsonwebtoken` crate (v10, with `aws_lc_rs` + `use_pem` features) for RS256 JWT signing.

## Rejected Alternatives

### Shared PAT (current approach)

Rejected because: all members share one identity, one rate-limit pool, and account-wide scope. PATs require manual creation with no programmatic path.

### One GitHub App per team (shared by all members)

Rejected because: all members appear as the same bot identity. Loses per-member attribution.

### PAT with backward compatibility shim

Rejected because: Alpha policy allows breaking changes. Two auth paths doubles test surface.

### `GH_TOKEN` env var for token delivery

Rejected because: env vars are baked at process launch and cannot be refreshed without restarting the process. `GH_CONFIG_DIR` + `hosts.yml` allows file-based refresh.

### Token refresh inside member process (Ralph/Brain)

Rejected because: Ralph is an upstream dependency that can't be modified. Brain could handle it, but having the daemon manage all token refresh keeps crypto concerns in one place and works uniformly for both member types.

## Consequences

* `bm hire` requires browser interaction (one click per member) for App creation — headless environments use URL fallback or pre-generated credentials
* Each member has a globally unique GitHub App name — name collisions are possible
* `credentials.gh_token` is removed from `TeamEntry` — `config.yml` no longer contains GitHub secrets
* Rate limits are fully independent per member (5,000 req/hr each)
* GitHub audit log, blame, and notifications work with per-member bot identities
* Machine migration requires `bm credentials export` before decommissioning
* App deletion is UI-only — `bm fire` can uninstall but cannot delete the App registration
* E2E tests use a hybrid strategy: one test for manifest flow, rest use pre-provisioned App

## Anti-patterns

* **Do NOT** store private keys in `config.yml` or any file on disk — they go in the credential store via the formation
* **Do NOT** share installation tokens across members — each member has independent token lifecycle
* **Do NOT** cache installation tokens on disk — they are short-lived (1 hour), generated fresh by the daemon
* **Do NOT** use the operator's `gh auth` session for member runtime — member actions must use the bot identity
* **Do NOT** fall back to PAT if App auth fails — clean break, fail with instructions to re-run `bm hire`
* **Do NOT** create GitHub Apps during `bm init` — Apps are per-member and belong in the `bm hire` lifecycle (which `bm init` calls internally)
* **Do NOT** deliver tokens via `GH_TOKEN` env var — use `GH_CONFIG_DIR` + `hosts.yml` for refreshability
* **Do NOT** read App credentials from keyring on every token refresh — cache in daemon memory at startup
* **Do NOT** bypass the formation for credential storage — always go through `formation.credential_store()`
* **Do NOT** call `validate_token()` (which uses `/user` endpoint) with installation tokens — they return 403. Trust the JWT exchange flow.
* **Do NOT** run `gh auth setup-git` globally — configure git credential helper per-workspace in `.git/config`
* **Do NOT** support personal GitHub accounts for team repos — `organization_projects` permission requires an org
