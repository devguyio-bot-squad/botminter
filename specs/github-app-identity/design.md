# Design: Team Runtime Architecture + Per-Member GitHub App Identity

## Overview

This milestone implements two merged architectural changes:

1. **Team as API boundary** (ADR-0008): The Team becomes the unified runtime abstraction. Operators interact with teams and members — never with formations, daemons, or deployment internals. The Formation trait is an internal deployment strategy that manages environment, credentials, credential delivery, and member lifecycle.

2. **Per-member GitHub App identity** (ADR-0011): Each team member gets its own GitHub App with distinct bot identity on GitHub. Short-lived installation tokens are refreshed automatically. Operator commands use the operator's `gh auth` session.

These are merged because GitHub App credentials require platform-specific storage (a formation concern) and automatic token refresh (a member lifecycle concern managed by the daemon, which is internal to the formation).

### Concept Hierarchy

```
Team (operator-facing API boundary)
  └── Formation (internal deployment strategy)
       ├── Environment management (verify machine, create VM, configure K8s)
       ├── Credential storage (key-value: keyring, K8s Secrets)
       ├── Credential delivery (setup_token_delivery + refresh_token)
       └── Member lifecycle (daemon is an impl detail here)
```

### What Changes

| Concern | Before | After |
|---------|--------|-------|
| API boundary | Commands call formation free functions | Commands call Team methods |
| GitHub auth | Shared PAT in `config.yml` | Per-member GitHub App, credentials in keyring |
| Operator auth | Stored `gh_token` | `gh auth login` session at runtime |
| Member identity | All actions → operator's user | `{team}-{member}[bot]` per member |
| Token lifecycle | Static, manual rotation | JWT → installation token, 50-min daemon refresh |
| Token delivery | `GH_TOKEN` env var baked at launch | `GH_CONFIG_DIR` + `hosts.yml`, daemon writes |
| Git credentials | Global `~/.gitconfig` | Per-workspace `.git/config` credential helper |
| Formation module | Free functions, hardcoded local | `Formation` trait, `LinuxLocalFormation` |
| Member lifecycle | `bm start` → fork/exec directly | `bm start` → Team → Formation → daemon → members |
| VM/Runtime | Separate `bm runtime` concept | `bm env create` → `formation.setup()` |
| Credential store | Bridge-specific, single value | Key-value, multi-domain (bridge + GitHub App) |
| State management | CLI + daemon both write `state.json` | Daemon owns `state.json`, CLI reads only |
| Personal accounts | Supported | Not supported (org required for `organization_projects`) |
| `bm fire` | Does not exist | New command |

### What Does NOT Change

- Profile system, team repo structure
- Bridge abstraction and bridge lifecycle
- `bm teams sync` workspace provisioning logic
- Ralph Orchestrator (upstream dependency)
- Commit conventions, doc site structure
- Native issue types (Epic, Task, Bug) and sub-issues — these are profile-level, not auth-level
- `github-project` skill and its GraphQL-based scripts — formation/auth changes are orthogonal
- `{{member_dir}}` template placeholders in profiles — rendered during `bm hire` before App creation

---

## Architecture

### Operator-Facing Commands (Team Level)

```
bm init                              # Create team (org required), hire members, add projects
bm start [member] [-t team]          # team.start() → formation → daemon → launch
bm stop [member] [-t team]           # team.stop() → formation → daemon → stop
bm status [-t team]                  # team.status() → formation → daemon → health
bm hire <role> [-t team]             # Create GitHub App, store credentials
bm fire <member> [-t team]           # Uninstall App, clean credentials, remove member
bm chat <member> [-t team]           # formation.exec_in() → interactive session
bm attach [-t team]                  # formation.shell() → environment shell
bm env create [-t team]              # formation.setup() → prepare environment
bm env delete [-t team]              # Tear down environment
bm projects add <url> [-t team]      # Add project + install member Apps on it
bm credentials export -o <file>      # Export all keyring secrets to portable file
bm init --credentials-file <file>    # Import credentials during init on new machine
```

No `bm formation` or `bm daemon` commands. The formation and daemon are never exposed.

### Internal Architecture

```
bm CLI
  │
  ├── Team (API boundary)
  │     │
  │     ├── team.start(member_filter)
  │     │     └── formation.start_members(params)
  │     │           ├── Ensure daemon process is running (spawn if needed)
  │     │           ├── HTTP request to daemon: "launch member X"
  │     │           └── Daemon internally:
  │     │                 ├── Read App credentials from in-memory cache
  │     │                 ├── Sign JWT → exchange for installation token
  │     │                 ├── formation.setup_token_delivery() (first time)
  │     │                 ├── formation.refresh_token() (write hosts.yml)
  │     │                 ├── Bridge auto-start (if configured)
  │     │                 └── Spawn member process (ralph or brain)
  │     │
  │     ├── team.stop(member_filter)
  │     │     └── formation.stop_members(params)
  │     │           └── HTTP request to daemon: "stop member X"
  │     │
  │     ├── team.status()
  │     │     └── formation.member_status()
  │     │           └── HTTP request to daemon: "status"
  │     │
  │     ├── team.hire(role, name, creds)
  │     │     ├── Create member dir in team repo
  │     │     ├── GitHub App creation (manifest flow or pre-generated)
  │     │     ├── formation.credential_store(GitHubApp).store(key, value)
  │     │     └── Install App on team repo + project repos
  │     │
  │     └── team.fire(member, keep_app)
  │           ├── Stop member (via formation)
  │           ├── Uninstall App (JWT → DELETE /app/installations/{id})
  │           ├── formation.credential_store(GitHubApp).remove(keys)
  │           └── Remove member dir + workspace
  │
  └── Formation (internal — never called by commands directly)
        ├── LinuxLocalFormation (implemented)
        ├── MacosLocalFormation (stub — "not yet supported")
        └── LimaFormation (environment = VM)
```

### CLI ↔ Daemon Communication

The daemon exposes an HTTP API on `127.0.0.1:{port}` using the existing axum server. The formation spawns the daemon as a child process and writes PID + port to a state file.

| Endpoint | Purpose |
|----------|---------|
| `POST /members/start` | Launch a member (or all) |
| `POST /members/stop` | Stop a member (or all) |
| `GET /members/status` | Member status + token health |
| `GET /health` | Daemon health check |
| `POST /webhook` | GitHub webhook events (existing) |

The daemon owns `state.json`. All mutations go through the daemon's HTTP API. The CLI reads `state.json` only for display purposes.

---

## Formation Trait

```rust
pub trait Formation {
    fn name(&self) -> &str;

    // Environment
    fn setup(&self, params: &SetupParams) -> Result<()>;
    fn check_environment(&self) -> Result<EnvironmentStatus>;
    fn check_prerequisites(&self) -> Result<()>;

    // Credentials (key-value store)
    fn credential_store(&self, domain: CredentialDomain) -> Result<Box<dyn CredentialStore>>;

    // Token delivery (split: one-time setup vs frequent refresh)
    fn setup_token_delivery(&self, member: &str, workspace: &Path, bot_user: &str) -> Result<()>;
    fn refresh_token(&self, member: &str, workspace: &Path, token: &str) -> Result<()>;

    // Member lifecycle (daemon is internal)
    fn start_members(&self, params: &StartParams) -> Result<StartResult>;
    fn stop_members(&self, params: &StopParams) -> Result<StopResult>;
    fn member_status(&self) -> Result<Vec<MemberStatus>>;

    // Interactive access
    fn exec_in(&self, workspace: &Path, cmd: &[&str]) -> Result<()>;
    fn shell(&self) -> Result<()>;

    // Topology
    fn write_topology(&self, workzone: &Path, team_name: &str, members: &[(String, MemberHandle)]) -> Result<()>;
}
```

**All methods are sync.** The daemon is a separate process — `start_members()` spawns it and communicates via HTTP. No async needed on the CLI side.

### CredentialStore Trait (Key-Value)

```rust
pub trait CredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<()>;
    fn retrieve(&self, key: &str) -> Result<Option<String>>;
    fn remove(&self, key: &str) -> Result<()>;
    fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
}
```

Key conventions per domain:

| Domain | Key pattern | Value |
|--------|------------|-------|
| Bridge | `{member}` | Bridge token |
| GitHub App | `{member}/github-app-id` | Numeric App ID |
| GitHub App | `{member}/github-app-client-id` | `Iv1.xxx` Client ID |
| GitHub App | `{member}/github-app-private-key` | PEM string |
| GitHub App | `{member}/github-installation-id` | Installation ID |

### CredentialDomain

```rust
pub enum CredentialDomain {
    Bridge { team_name: String, bridge_name: String, state_path: PathBuf },
    GitHubApp { team_name: String, member_name: String },
}
```

### Per-Formation Behavior

| Method | Local | Lima | K8s (future) |
|--------|-------|------|-------------|
| `setup()` | Verify prerequisites (ralph, keyring, gh auth) | Create VM via limactl, install tools | Configure namespace |
| `credential_store()` | `LocalCredentialStore` (system keyring) | Keyring inside VM | K8s Secrets |
| `setup_token_delivery()` | Create `.config/gh/`, write initial `hosts.yml`, configure `.git/config` credential helper | Same (inside VM) | Configure mounted secret |
| `refresh_token()` | Atomic write to `hosts.yml` | Same (inside VM) | Update K8s Secret |
| `start_members()` | Spawn local daemon, send HTTP requests | SSH into VM, start daemon there | Deploy pods with daemon |
| `exec_in()` | Direct exec | `limactl shell <vm> -- <cmd>` | `kubectl exec` |
| `shell()` | No-op or subshell | `limactl shell <vm>` | `kubectl exec -it` |

---

## GitHub App Identity

### App Creation Flow (`bm hire`)

Three credential acquisition paths (all work in interactive and `--non-interactive`):

1. **Pre-generated credentials** (highest priority): `--app-id`, `--client-id`, `--private-key-file`, `--installation-id`
2. **Browser manifest flow**: auto-submitting form → GitHub → localhost callback
3. **URL fallback (headless)**: print localhost URL, operator opens manually

Manifest JSON:
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

**App ownership:** Org-owned (team repo must be in an org). Personal accounts not supported.

**Callback timeout:** 5 minutes. After timeout, server shuts down with error and retry instructions.

**Code exchange:** `POST /app-manifests/{code}/conversions` (no auth needed). Returns App ID, Client ID, PEM, client secret, webhook secret.

### `bm hire` Idempotency

| Member dir exists? | Flags | Behavior |
|---|---|---|
| No | (none) | Create member dir + create new App |
| Yes | (none) | Create new App, keep member dir (App replacement after deletion) |
| Yes | `--reuse-app` + creds | Store creds, keep member dir (machine migration) |
| No | `--reuse-app` + creds | Create member dir, store creds (adopt existing App) |

Additional flags: `--save-credentials <path>` saves credentials to file during creation.

### Token Lifecycle (Daemon-Managed)

```
Daemon startup:
  1. Read all members' App credentials from credential store → cache in memory
  2. For each member:
     a. Sign JWT (RS256, Client ID as iss, iat=now-60, exp=now+600)
     b. Exchange JWT → installation token (POST /app/installations/{id}/access_tokens)
     c. formation.setup_token_delivery() (first time only)
     d. formation.refresh_token(member, workspace, token)
  3. Start refresh loop per member (50-minute interval)

Refresh cycle:
  1. Sign JWT → exchange for new token
  2. formation.refresh_token(member, workspace, token) → atomic hosts.yml write
  3. Old token remains valid until its own 1-hour expiry
  4. On failure: log, exponential backoff, existing token valid until expiry
```

Installation tokens are NOT validated via `/user` endpoint (returns 403). Trust the JWT exchange — if it succeeds, the token works.

### Token Delivery

```
{workspace}/.config/gh/
  hosts.yml     # Written by formation.refresh_token()
  config.yml    # git_protocol: https

{workspace}/.git/config   # Git credential helper (NOT global ~/.gitconfig)
  [credential "https://github.com"]
      helper =
      helper = !/usr/bin/gh auth git-credential
```

Member process launched with `GH_CONFIG_DIR={workspace}/.config/gh/`. `gh` reads `hosts.yml` on every invocation. `git` goes through `gh` as credential helper via workspace `.git/config`.

### Operator Auth

Operator commands use `gh auth token` at runtime. `detect_token()` / `detect_token_non_interactive()` are the sole auth path. `credentials.gh_token` removed from `TeamEntry`. `require_gh_token()` removed.

All `gh_token: Option<&str>` parameters removed from `git/github.rs` functions — they resolve the operator token internally via `detect_token()`.

---

## `bm fire`

```
bm fire <member> [-t team] [--keep-app]
```

1. Stop member (via team → formation → daemon)
2. Uninstall App via `DELETE /app/installations/{id}` (JWT-authenticated)
3. Remove credentials from credential store
4. Remove member directory from team repo
5. Remove member workspace
6. Print instructions for manual App deletion via GitHub UI (no API exists)

`--keep-app`: skip step 2, preserve installation for reuse.

---

## Machine Migration

Two-step flow:

```bash
# Old machine:
bm credentials export -o team-creds.yml

# New machine:
bm init --credentials-file team-creds.yml
bm teams sync -a
```

### Credentials Export Format

```yaml
team: my-team
members:
  superman:
    github_app:
      app_id: "123456"
      client_id: "Iv1.abc123"
      private_key: |
        -----BEGIN RSA PRIVATE KEY-----
        ...
        -----END RSA PRIVATE KEY-----
      installation_id: "789012"
    bridge:
      token: "syt_xxxxxxxxx"
  batman:
    github_app:
      app_id: "234567"
      client_id: "Iv1.def456"
      private_key: |
        -----BEGIN RSA PRIVATE KEY-----
        ...
        -----END RSA PRIVATE KEY-----
      installation_id: "890123"
    bridge:
      token: "syt_yyyyyyyyy"
```

File written with 0600 permissions. CLI prints explicit security warning. Contains everything from the keyring — team repo and config are in git / recreated by `bm init`.

---

## Module Structure

### New Modules

| Module | Purpose |
|--------|---------|
| `git/app_auth.rs` | JWT generation, installation token exchange |
| `formation/local/mod.rs` | Platform detection, delegates to linux/macos |
| `formation/local/process.rs` | Shared POSIX process lifecycle |
| `formation/local/topology.rs` | Shared topology writing |
| `formation/local/daemon.rs` | Daemon management (spawn, health check, HTTP client) |
| `formation/local/linux/mod.rs` | `LinuxLocalFormation` |
| `formation/local/linux/credential.rs` | `LocalCredentialStore` (keyring, key-value) |
| `formation/local/linux/setup.rs` | Prerequisite verification |
| `formation/local/macos/mod.rs` | `MacosLocalFormation` (stub) |
| `commands/fire.rs` | `bm fire` command |
| `commands/credentials.rs` | `bm credentials export/import` |
| `commands/env.rs` | `bm env create/delete` |

### Modified Modules

| Module | Change |
|--------|--------|
| `config/mod.rs` | Remove `gh_token` from `Credentials`, remove `require_gh_token()` |
| `git/github.rs` | Remove `gh_token: Option<&str>` from all functions |
| `formation/mod.rs` | Add `Formation` trait, `CredentialStore` trait, `CredentialDomain` |
| `commands/hire.rs` | Add manifest flow, `--reuse-app`, `--save-credentials` |
| `commands/init.rs` | Remove token prompt, require org, wire App creation into hire |
| `commands/start.rs` | Delegate to `team.start()` |
| `commands/stop.rs` | Delegate to `team.stop()` |
| `daemon/run.rs` | Add token refresh loops, member management HTTP API |
| `daemon/process.rs` | Remove direct `gh_token` handling |
| `brain/` | Update system prompt, delegate loops to daemon |
| `cli.rs` | Add `fire`, `env`, `credentials` subcommands |

---

## Error Handling

### Manifest Flow

| Error | Handling |
|-------|----------|
| Browser fails to open | Print localhost URL (headless fallback) |
| Callback timeout (5 min) | Shut down server, print retry instructions |
| Code exchange 404 | Code expired — retry from start |
| Name collision | Suggest `{team}-{member}-{hash}` alternatives |

### Token Refresh

| Error | Handling |
|-------|----------|
| JWT signing fails | Log, retry next cycle, existing token valid |
| Token exchange 401 | Credentials invalid — log, alert operator |
| Token exchange rate limit | Exponential backoff |
| hosts.yml write fails | Log, retry — member uses current token |

### Formation

| Error | Handling |
|-------|----------|
| Keyring inaccessible | Clear error with platform-specific fix |
| Daemon unreachable | `start_members()` spawns daemon, retries |
| Daemon crash | Members continue (orphaned), next `bm start` re-adopts via state.json |
| Member launch fails | Report error, continue launching others |

---

## Acceptance Criteria

### Team as API Boundary

1. **Given** `bm start superman`, **when** the team is resolved, **then** the command delegates to `team.start("superman")` — never directly to a formation or daemon.
2. **Given** a formation type, **when** any command runs, **then** the formation type is never visible in CLI output or error messages to the operator.

### Formation Trait

3. **Given** `formation::create("local")` on Linux, **then** a `LinuxLocalFormation` is returned.
4. **Given** `formation.credential_store(GitHubApp { .. })`, **then** a key-value `CredentialStore` is returned backed by the system keyring.
5. **Given** `formation.credential_store(Bridge { .. })`, **then** existing bridge credential behavior is preserved.
6. **Given** macOS, **then** `formation::create("local")` returns "not yet supported."

### GitHub App Creation

7. **Given** `bm hire <role> --name superman`, **when** the operator completes the manifest flow, **then** a GitHub App named `{team}-superman` is created, credentials stored in keyring, App installed on team repo + project repos.
8. **Given** `bm hire --reuse-app --app-id 123 --private-key-file key.pem --installation-id 456`, **then** credentials are stored without triggering manifest flow.
9. **Given** a headless environment, **when** browser fails to open, **then** the localhost URL is printed for manual access.
10. **Given** no callback within 5 minutes, **then** the server shuts down with clear error.
11. **Given** `bm hire` for an existing member (no `--reuse-app`), **then** a new App is created and old credentials replaced (member dir preserved).

### Token Lifecycle

12. **Given** a member with stored App credentials, **when** the daemon starts, **then** an installation token is generated and delivered via `refresh_token()`.
13. **Given** 50 minutes elapsed, **then** the daemon refreshes the token and updates `hosts.yml`.
14. **Given** a token refresh failure, **then** the daemon retries with backoff and the existing token remains valid.
15. **Given** `GH_CONFIG_DIR` set on a member process, **when** `gh issue list` runs, **then** it uses the token from `hosts.yml`.
16. **Given** `git push` in a member workspace, **then** git uses the credential helper from `.git/config` (not global), which reads from `GH_CONFIG_DIR`.

### Operator Auth

17. **Given** `bm init`, **then** it validates `gh auth` session, requires an org, does NOT prompt for a token, and creates Apps during hire.
18. **Given** no `gh auth` session, **then** any operator command fails with guidance to run `gh auth login`.
19. **Given** a personal GitHub account (no org), **then** `bm init` fails with clear guidance to use an org.

### Daemon

20. **Given** `bm start` with no daemon running, **then** the formation starts the daemon first, then launches members.
21. **Given** `bm stop`, **then** members stop but daemon keeps running.
22. **Given** `bm stop --all`, **then** both members and daemon stop.
23. **Given** `bm start` called twice, **then** the second call communicates with the existing daemon (no double-spawn).

### bm fire

24. **Given** `bm fire superman`, **then** App uninstalled, credentials removed, member dir removed, manual deletion instructions printed.
25. **Given** `bm fire --keep-app`, **then** credentials and member dir removed but App installation preserved.

### Machine Migration

26. **Given** `bm credentials export -o creds.yml`, **then** all members' App + bridge credentials written to file with 0600 permissions.
27. **Given** `bm init --credentials-file creds.yml` on a new machine, **then** team is set up and all credentials imported — `bm teams sync -a` produces working workspaces.

---

## Testing Strategy

### Unit Tests

- JWT generation with mock private key
- Token exchange with mocked HTTP
- `hosts.yml` generation and atomic write
- Key-value credential store operations
- `CredentialDomain` routing
- Manifest JSON construction
- Name collision detection
- Formation trait method dispatch

### Integration Tests

- Full hire-to-start flow with `InMemoryCredentialStore`
- `bm fire` cleanup verification
- Formation factory (Linux works, macOS errors)
- Credentials export/import round-trip
- Daemon HTTP API (start/stop/status endpoints)

### E2E Tests (Hybrid)

- **One manifest flow test**: real App creation via URL fallback, full flow, cleanup
- **Pre-provisioned App tests**: use `--reuse-app` with CI secrets for all other scenarios
- Full operator journey: init → hire → projects add → sync → start → verify gh works → stop → fire

### Exploratory Tests

- Per-member App credentials instead of shared PAT
- Token refresh verification
- Daemon lifecycle across member start/stop cycles
- Credentials export/import on `bm-test-user@localhost`

---

## Appendices

### A. Technology Choices

| Technology | Purpose | Rationale |
|------------|---------|-----------|
| `jsonwebtoken` v10 | RS256 JWT signing | Mature, supports PEM loading, `aws_lc_rs` backend |
| `axum` (existing) | Daemon HTTP API + manifest callback server | Already a dependency |
| `GH_CONFIG_DIR` | Per-member token isolation | Native `gh` support |

### B. Research Findings

- Manifest flow: form POST, code exchange needs no auth, 1-hour code TTL
- `organization_projects:admin` required for Projects v2 (not `projects:admin`)
- No API to delete GitHub Apps — UI only
- `ghs_` tokens work in `hosts.yml` — `gh` doesn't validate prefix
- JWT `iss` uses Client ID (not App ID) per current docs
- Multiple installation tokens can coexist — refresh doesn't invalidate old one

### C. Open Items

1. **Daemon ↔ Brain communication**: how Brain requests loop launches from daemon (HTTP API endpoint vs `bm-agent` command)
2. **Lima formation details**: full VM lifecycle via Formation trait (can be a later sprint)
3. **Credential file encryption**: for Alpha, plaintext YAML with warning. Post-Alpha, consider `age` encryption
4. **`bm projects add` triggering early token refresh**: existing tokens may not include new repo until next refresh cycle
