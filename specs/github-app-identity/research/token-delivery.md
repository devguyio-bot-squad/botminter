# Research: Token Delivery via GH_CONFIG_DIR + hosts.yml

## How `gh` Resolves Tokens (priority order)

1. `GH_TOKEN` env var (literal string, highest priority)
2. `~/.config/gh/hosts.yml` (or `$GH_CONFIG_DIR/hosts.yml`)
3. System credential store (platform keyring)

## `GH_CONFIG_DIR` Isolation

`gh` respects `GH_CONFIG_DIR` env var to override config location. Per-process isolation:
```bash
GH_CONFIG_DIR=/path/to/member/.config/gh gh issue list
```

Each member workspace gets its own `GH_CONFIG_DIR` so tokens are fully isolated.

## `hosts.yml` Format

```yaml
github.com:
  oauth_token: ghs_xxxxxxxxxxxx  # installation access token
  git_protocol: https
  user: my-team-superman[bot]
```

### Required Fields
- `oauth_token` -- the token value (any valid GitHub token format)

### Optional Fields
- `git_protocol` -- `https` (default) or `ssh`
- `user` -- the authenticated user/bot login

## Installation Token Compatibility

GitHub App installation access tokens:
- Prefix: `ghs_` (GitHub Server token)
- TTL: 1 hour (3600 seconds), configurable up to 1 hour
- Scoped to the installation's permissions and repos
- Multiple valid tokens can coexist (refreshing early doesn't invalidate the old one)

`gh` does NOT validate the token prefix — it passes the `oauth_token` value directly as a `Bearer` token in the `Authorization` header. Any valid GitHub token works in `hosts.yml`.

## Token Refresh Mechanism

1. Daemon generates installation token via `POST /app/installations/{id}/access_tokens`
2. Daemon writes token to `{workspace}/.config/gh/hosts.yml` (atomic write: write to temp file, rename)
3. `gh` reads `hosts.yml` on every invocation — always gets latest token
4. Daemon refreshes at 50-minute mark (10 minutes before 1-hour expiry)
5. Old token remains valid until its own expiry — no disruption during refresh

### Atomic Write Pattern
```rust
// Write to temp file first, then atomically rename
let temp = hosts_path.with_extension("tmp");
fs::write(&temp, new_content)?;
fs::rename(&temp, &hosts_path)?;
```

This prevents `gh` from reading a partially-written file.

## Git Credential Helper Chain

The credential helper is written directly into the workspace `.git/config` (NOT via `gh auth setup-git`, which writes to global `~/.gitconfig` — see requirements.md R6):

```
# {workspace}/.git/config
[credential "https://github.com"]
  helper =
  helper = !/usr/bin/gh auth git-credential
```

This makes `git push/pull` go through `gh`, which reads `hosts.yml` (via `GH_CONFIG_DIR`), which has the latest installation token. The full chain:

```
git push -> git credential fill -> gh auth git-credential -> reads GH_CONFIG_DIR/hosts.yml -> returns token
```

### Setup During Token Delivery

During `formation.setup_token_delivery()` (first member start):
1. Create `{workspace}/.config/gh/` directory
2. Write initial `hosts.yml` with installation token and bot user
3. Write credential helper config into `{workspace}/.git/config` (per-workspace, not global)
4. Verify with `GH_CONFIG_DIR={workspace}/.config/gh gh auth status`

## Advantages Over GH_TOKEN Env Var

| Aspect | GH_TOKEN env var | hosts.yml |
|--------|-----------------|-----------|
| Refresh | Requires process restart | File write, immediate |
| Git integration | Needs separate credential helper | `gh auth setup-git` handles it |
| Isolation | Per-process env | Per-directory config |
| Debugging | Hidden in env | `gh auth status` works |
| Multiple hosts | Single token | Per-host config |
