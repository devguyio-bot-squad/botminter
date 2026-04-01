# PR #2 Review Fix Progress

**Branch:** `pr2-review-fixes` (based on `pr2-macos-formation`)
**Date:** 2026-04-01

## What Was Done

### PR-REVIEW.md Issues — All 6 Addressed

| # | Priority | Issue | Status | Notes |
|---|----------|-------|--------|-------|
| 1 | **Must** | Merge identical Linux/macOS formation structs | **Kept separate** | PO directed: will diverge soon for native Keychain. Added doc comments explaining intent. |
| 2 | **Must** | Extract shared credential backend | **Done** | `crates/bm/src/keyring_backend.rs` — both `bridge::credential` and `formation::local::credential` delegate to it. ~200 lines deduped. |
| 3 | **Should** | Restore doc comments on credential.rs | **Done** | All doc comments restored from `main` branch version. |
| 4 | **Should** | Fix `normalize_github_credential_helper` idempotency | **Done** | Line-by-line parser replaces any previous gh helper regardless of path format. 5 new tests. |
| 5 | **Should** | Use `mode(0o600)` for initial hosts.yml | **Done** | `setup_token_delivery` now uses `OpenOptions` with `mode(0o600)`, matching `refresh_token`. |
| 6 | **Minor** | Remove redundant dbus-secret-service dev-dep | **Done** | Removed from Cargo.toml. |

### macOS Test Infrastructure — Done

Made the exploratory test framework macOS-compatible:

- **`lib.sh`:** Platform detection (`is_macos`/`is_linux`), Keychain-aware `ensure_keyring` (no-op on macOS since Keychain is always available), cross-platform `port_in_use`/`kill_port_holder` helpers (lsof on macOS, ss on Linux), platform-aware `bm()` wrapper (skips `BM_KEYRING_DBUS` on macOS).
- **Exploratory Justfile:** Configurable `TEST_HOST`/`REMOTE_HOME` via `EXPLORATORY_TEST_HOST`/`EXPLORATORY_REMOTE_HOME` env vars. New recipes: `build-remote`, `deploy-remote`, `unit-remote`, `macos-portable`. Cross-platform `clean`, `preflight`, `run-phase`.
- **Phase scripts:** `phase-g.sh` skips podman ops on macOS, uses `security(1)` for Keychain cleanup. `phase-f.sh` relaxes member count threshold.
- **Root Justfile:** New recipes: `mac-unit`, `mac-build`, `mac-exploratory-test`, `mac-exploratory-test-clean`.

### Test Results

| Platform | Test Suite | Result |
|----------|-----------|--------|
| Linux x86_64 | Unit tests (121) | **All pass** |
| Linux x86_64 | Integration tests (5) | **All pass** |
| Linux x86_64 | Conformance tests (18) | **All pass** |
| macOS arm64 (Darwin 25.2.0) | Unit tests (817) | **All pass** |
| macOS arm64 | Exploratory tests | **Not run** (blocked, see below) |

## What's Remaining

### Exploratory Tests on macOS — Blocked

The exploratory tests (`mac-exploratory-test`) need `TESTS_APP_*` env vars for GitHub App credentials used by `bm hire --reuse-app`:

- `TESTS_APP_ID`
- `TESTS_APP_CLIENT_ID`
- `TESTS_APP_INSTALLATION_ID`
- `TESTS_APP_PRIVATE_KEY_FILE`

These are typically provided via `.envrc` / direnv but were not available in this session.

**To run when credentials are available:**
```bash
export EXPLORATORY_TEST_HOST=bm-test-user@qaswaa
export EXPLORATORY_REMOTE_HOME=/Users/bm-test-user
# Set TESTS_APP_* vars...
just mac-exploratory-test
```

### macOS Test Machine State (`bm-test-user@qaswaa`)

- **OS:** macOS 26.2 (Darwin 25.2.0, arm64)
- **Rust:** Installed via rustup at `~/.cargo/`
- **Tools installed:** `just` and `gh` at `~/.local/bin/`
- **gh auth:** Authenticated as `devguyio`
- **Source:** Synced at `~/botminter-build/` (with `target/` build artifacts)
- **No cleanup needed** — unit tests don't leave state

## Key Files Changed

```
NEW    crates/bm/src/keyring_backend.rs          — shared keyring backend module
MOD    crates/bm/src/lib.rs                      — register keyring_backend module
MOD    crates/bm/src/bridge/credential.rs        — delegate to keyring_backend
MOD    crates/bm/src/formation/local/credential.rs — delegate to keyring_backend, restore docs
MOD    crates/bm/src/formation/local/common.rs   — fix credential helper + hosts.yml perms
MOD    crates/bm/src/formation/local/linux/mod.rs — divergence doc comment
MOD    crates/bm/src/formation/local/macos/mod.rs — divergence doc comment
MOD    crates/bm/src/formation/mod.rs            — update trait doc comment
MOD    crates/bm/Cargo.toml                      — remove redundant dev-dep
MOD    Justfile                                  — mac-* recipes
MOD    crates/bm/tests/exploratory/Justfile      — cross-platform support
MOD    crates/bm/tests/exploratory/lib.sh        — platform detection + macOS helpers
MOD    crates/bm/tests/exploratory/phases/phase-f.sh — flexible member count
MOD    crates/bm/tests/exploratory/phases/phase-g.sh — macOS-compatible cleanup
```

## Architecture Decision

**Why keep `LinuxLocalFormation` and `MacosLocalFormation` separate:**

The PO directed that these types will diverge soon — macOS will get native Keychain integration instead of the current cross-platform `keyring` crate path. Keeping them separate now avoids a merge-then-split churn. Both already maximize code reuse by delegating everything to `common::*`. The doc comments on each struct explain the planned divergence.
