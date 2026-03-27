# Rough Idea: Per-Member GitHub App Identity

Source: `github-app-identity.md` (repo root)
ADR: `.planning/adrs/0011-github-app-per-member-identity.md`

## Summary

Replace the shared PAT authentication model with per-member GitHub Apps. Each team member gets its own GitHub App (created during `bm hire` via the Manifest flow), its own bot identity on GitHub, and its own short-lived installation tokens managed autonomously at runtime. Operator commands use the operator's native `gh auth` session. No backward compatibility with PATs.

## Key Points from Initial Document

### Identity Model
- Operator commands use `gh auth token` at runtime (no stored token)
- Member runtime uses `{team}-{member}[bot]` identity via GitHub App installation tokens

### App Creation
- Manifest flow during `bm hire`: browser redirect + one-click confirmation
- Credentials stored in system keyring (App ID, private key, installation ID)

### Token Lifecycle
- JWT signing (RS256) on startup, exchange for 1-hour installation token
- Background refresh at 50-minute mark
- Token delivery via `GH_CONFIG_DIR` + `hosts.yml` (supersedes initial `GH_TOKEN` env var idea — see design.md)

### Existing Infrastructure
- `CredentialStore` trait already exists (`bridge/credential.rs`) for bridge credentials
- `detect_token()` / `detect_token_non_interactive()` already do operator auth detection (`git/github.rs`)
- `require_gh_token()` is the choke point for member auth (`config/mod.rs`)
- `gh_token: Option<&str>` threads through ~22 files

### Proposed Phases (from initial doc)
- Phase 11: App Auth Module + Operator Auth Migration
- Phase 12: Manifest Flow in `bm hire`
- Phase 13: Per-Member Token Lifecycle in Runtime
- Phase 14: Profile & Docs Update
- Phase 15: E2E & Exploratory Test Migration
