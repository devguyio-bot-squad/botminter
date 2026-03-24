---
status: proposed
date: 2026-03-24
decision-makers: human operator, claude
---

# Per-member GitHub App identity replaces shared PAT

## Problem

BotMinter currently uses a single Personal Access Token (PAT) stored in `~/.botminter/config.yml` and shared across all team members at runtime. This creates three problems: (1) all member actions are attributed to the operator's GitHub user, making audit trails ambiguous; (2) a single rate-limit pool is shared across all members and the operator; (3) the PAT has account-wide scope, granting every member access to all repos the operator can see — far broader than necessary.

PATs cannot be created programmatically — operators must visit the GitHub UI, check the right permission boxes, copy the token, and paste it into `bm init`. This is error-prone and the most common support friction during onboarding.

## Constraints

* Each member MUST appear as a distinct identity on GitHub (separate bot user for attribution and audit)
* Operator commands (`bm init`, `bm teams sync`, etc.) MUST use the operator's own GitHub identity — not a bot
* No backward compatibility with PAT-based auth — PAT support is removed entirely
* Token lifecycle MUST be automatic — no manual token rotation by operators
* All GitHub operations continue to go through the `gh` CLI (no direct HTTP client for GitHub API in `bm`)
* Private keys MUST be stored in the system keyring, not in config files on disk

## Decision

Each team member gets its own GitHub App, created during `bm hire` via the GitHub App Manifest flow. Operator commands use the operator's existing `gh auth login` session. The shared `credentials.gh_token` field is removed from `TeamEntry`.

### Identity model

| Actor | GitHub identity | Auth source |
|-------|----------------|-------------|
| Operator (`bm init`, `bm teams sync`, `bm hire`, etc.) | Operator's GitHub user | `gh auth token` at runtime |
| Member runtime (issues, PRs, board scans) | `{team}-{member}[bot]` | GitHub App installation token |

### App creation via Manifest flow

During `bm hire <role> --name <name>`:

1. `bm` constructs a JSON manifest specifying the App name (`{team}-{name}`), permissions, and a localhost callback URL
2. `bm` starts a temporary local HTTP server to receive the callback
3. `bm` opens the operator's browser to `https://github.com/settings/apps/new?state=...` with the manifest
4. The operator clicks "Create GitHub App" (one click — permissions are pre-filled)
5. GitHub redirects to the localhost callback with a temporary `code`
6. `bm` calls `POST /app-manifests/{code}/conversions` (via `gh api`) to receive: App ID, private key (PEM), client ID, client secret, webhook secret
7. `bm` stores the App ID and private key in the system keyring under `botminter/{team}/{member}/`
8. `bm` installs the App on the team repo via `POST /app/installations` and stores the installation ID

### Credential storage

```
Keyring entries (per member):
  botminter/{team}/{member}/github-app-id           -> "123456"
  botminter/{team}/{member}/github-app-private-key   -> "-----BEGIN RSA PRIVATE KEY-----\n..."
  botminter/{team}/{member}/github-installation-id   -> "789012"
```

No GitHub credentials in `config.yml`. The `Credentials` struct retains only `webhook_secret` (for daemon webhook verification).

### Token lifecycle at runtime

Each member process (Ralph or Brain) manages its own installation token:

1. On startup, sign a JWT using the App's private key (RS256, 10 min TTL)
2. Exchange the JWT for an installation access token via `POST /app/installations/{id}/access_tokens` (1 hour TTL)
3. Set `GH_TOKEN` env var for all `gh` CLI calls
4. Spawn a background refresh task that re-generates the token at the 50-minute mark
5. On refresh, update the env var for subsequent `gh` calls

This runs inside the member process — no external token provider or sidecar.

### App permissions (manifest)

```json
{
  "name": "{team}-{member}",
  "url": "https://github.com/{org}/{team-repo}",
  "default_permissions": {
    "issues": "write",
    "contents": "write",
    "pull_requests": "write",
    "projects": "admin"
  },
  "default_events": []
}
```

### Operator auth

Operator commands call `gh auth token` at runtime to get the operator's token. If no session exists, commands fail with a message directing the operator to run `gh auth login`. No token is stored by `bm`.

The existing `detect_token()` and `detect_token_non_interactive()` functions in `git/github.rs` already implement this pattern — they check `GH_TOKEN` env var first, then fall back to `gh auth token`.

### New dependency

`jsonwebtoken` crate for RS256 JWT signing (App authentication requires signing a JWT with the App's private key).

## Rejected Alternatives

### Shared PAT (current approach)

Rejected because: all members share one identity, one rate-limit pool, and account-wide scope. PATs require manual creation via the GitHub UI with no programmatic path. Token rotation is manual.

### One GitHub App per team (shared by all members)

Rejected because: all members appear as the same `{team}[bot]` identity. Loses per-member attribution — the primary goal of this change. Rate limits are still shared across members.

### GitHub App with per-member tokens but shared identity

Rejected because: installation tokens from the same App all carry the same bot identity. GitHub attributes actions to the App, not to individual tokens. There is no way to distinguish which member performed an action.

### OAuth Apps

Rejected because: OAuth Apps act on behalf of a user (the operator), not as their own identity. They don't solve the attribution problem and are being deprecated by GitHub in favor of GitHub Apps.

### PAT with backward compatibility shim

Rejected because: Alpha policy explicitly allows breaking changes. Supporting two auth paths doubles the test surface and creates confusion about which path to use. Clean cut.

## Consequences

* `bm hire` requires browser interaction (one click per member) for App creation — no fully headless hiring
* Each member has a globally unique GitHub App name (`{team}-{member}`) — name collisions are possible across unrelated teams using the same names
* The `jsonwebtoken` crate is added as a dependency for JWT signing
* `credentials.gh_token` is removed from `TeamEntry` and `Credentials` — config.yml shrinks
* All GitHub credentials move to the system keyring — `config.yml` no longer contains secrets (except optional `webhook_secret`)
* Rate limits are fully independent per member (5,000 req/hr each)
* GitHub's audit log, blame, notifications, and filtering all work with per-member bot identities
* E2E tests need a new strategy — they currently use a shared `TESTS_GH_TOKEN` PAT
* Exploratory tests need updated credential setup for the per-member App model

## Anti-patterns

* **Do NOT** store private keys in `config.yml` or any file on disk — they belong in the system keyring via the `CredentialStore` trait.
* **Do NOT** share installation tokens across members — each member independently manages its own token lifecycle. A central token provider is unnecessary complexity.
* **Do NOT** cache installation tokens on disk — they are short-lived (1 hour) and must be generated fresh on each `bm start`.
* **Do NOT** use the operator's `gh auth` session for member runtime operations — member actions must be attributed to the member's bot identity, not the operator.
* **Do NOT** fall back to PAT if App auth fails — this is a clean replacement, not a migration. Fail clearly with instructions to re-run `bm hire`.
* **Do NOT** create GitHub Apps during `bm init` — Apps are per-member and belong in the `bm hire` lifecycle.
