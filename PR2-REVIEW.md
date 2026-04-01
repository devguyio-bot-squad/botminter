# PR #2 Review: feat(formation): add macOS local formation support

**Author:** mikeyobrien (Ralph Orchestrator maintainer)
**Branch:** `mikeyobrien:feat/formation-macos-local-support` -> `main`
**Files changed:** 15 | **+859 / -630**
**PR URL:** https://github.com/botminter/botminter/pull/2

---

## Summary

This PR replaces the macOS formation stub with a real local-process implementation.
The core approach is sound: extract shared formation logic into a `common` module
so both Linux and macOS delegate to the same code. The PR also adds platform-conditional
Cargo dependencies, `#[cfg(target_os)]` gating on dbus-secret-service code,
XDG_CONFIG_HOME-aware config resolution, and fixes a test that relied on mutating
the global `HOME` env var.

**Verdict: Good direction, needs fixes before merge.**

---

## What the PR Does Well

1. **Correct extraction pattern.** Moving shared logic from `linux/mod.rs` into
   `common.rs` is the right architecture. Token delivery, process control, topology,
   and environment checks are OS-neutral and belong in shared code.

2. **`hire_member_from` test fix.** The old `hire_existing_member_returns_already_existed`
   test mutated the global `HOME` env var via `env::set_var`, which is unsound in
   multi-threaded test runners and a known footgun. The PR introduces `hire_member_from`
   with an explicit `profiles_base: Option<&Path>` parameter, eliminating the env mutation.
   This is a clean, proper fix.

3. **`botminter_config_dir()` centralizes config path resolution.** Previously,
   `profiles_dir()` and `minty_dir()` independently called `dirs::config_dir()`.
   The PR extracts `botminter_config_dir()` with explicit `XDG_CONFIG_HOME` handling,
   and both functions now use it. This is correct and makes macOS path resolution
   (`~/Library/Application Support/botminter/`) work automatically via the `dirs` crate.

4. **Documentation updates are accurate.** The bridges.md, prerequisites.md, and cli.md
   changes correctly describe the platform-specific credential backends.

5. **E2E `test_env.rs` is unchanged.** The PR does not touch the E2E test infrastructure,
   which means Linux E2E tests continue to work exactly as before. The dbus/keyring
   setup in `TestEnv` remains Linux-only (it uses `dbus-daemon` and `gnome-keyring-daemon`),
   which is correct since those tools don't exist on macOS.

---

## Issues

### MUST FIX: `LinuxLocalFormation` and `MacosLocalFormation` are character-for-character identical

**Files:**
- `crates/bm/src/formation/local/linux/mod.rs`
- `crates/bm/src/formation/local/macos/mod.rs`

After the extraction, both structs have identical `Formation` trait implementations.
The only difference is the struct name and doc comment. Every method is a one-liner
delegating to `common::*`. The `credential_store()` method is identical too - same
service name format, same keys path construction, same `LocalKeyValueCredentialStore`.

**What to do:** Merge into a single `LocalFormation` struct in `local/mod.rs`.
The `create_local_formation()` function already handles the platform gate - it can
just return `LocalFormation` on both platforms. The separate `linux/` and `macos/`
module directories should be removed.

If the intent is to allow future platform-specific divergence (e.g., macOS Keychain
integration via a different credential store), document that with a comment. But
right now, two identical 115-line files with duplicated tests is pure noise.

**Evidence - identical trait impls (diff the two files):**
```
linux/mod.rs:27  impl Formation for LinuxLocalFormation {
macos/mod.rs:27  impl Formation for MacosLocalFormation {
```
Every method body is `common::some_function(args)` in both files.

---

### MUST FIX: Massive credential code duplication between bridge and formation

**Files:**
- `crates/bm/src/bridge/credential.rs` (bridge credential store)
- `crates/bm/src/formation/local/credential.rs` (formation credential store)

Both files now independently contain:

| Function | bridge/credential.rs | formation/local/credential.rs |
|----------|---------------------|-------------------------------|
| `connect_secret_service()` | Lines 143-154 | Lines 29-40 |
| `get_or_create_collection()` | Lines 122-140 | Lines 43-59 |
| `dss_store()` | Lines 206-228 | Lines 62-84 |
| `dss_retrieve()` | Lines 232-258 | Lines 95-121 |
| `dss_delete()` | Lines 262-279 | Lines 129-146 |
| `check_keyring_unlocked_for()` | Lines 160-197 | Lines 154-191 |
| `with_keyring_dbus()` pattern | Lines 103-116 | Lines 214-227 |
| Non-Linux stubs for all of above | Lines 281-306, 191-197 | Lines 86-92, 123-126, 148-151, 185-191 |

The implementations are nearly identical. The bridge version has slightly different
error messages (mentions `BM_BRIDGE_TOKEN_*` env vars) and also manages
`bridge-state.json` identities, but the core dbus-secret-service operations are
copy-pasted.

**What to do:** Extract a shared `credential_backend` module (e.g.,
`crates/bm/src/credential_backend.rs` or `crates/bm/src/keyring.rs`) containing:
- `connect_secret_service()`
- `get_or_create_collection()`
- `dss_store()` / `dss_retrieve()` / `dss_delete()`
- `check_keyring_unlocked_for()`
- `with_keyring_dbus()` (as a free function taking a `BM_KEYRING_DBUS` value)

Both `bridge::credential::LocalCredentialStore` and
`formation::local::credential::LocalKeyValueCredentialStore` should delegate to
this shared module. The bridge-specific identity tracking and error messages stay
in their respective implementations.

**Risk of NOT fixing:** A bug found in one copy will not be fixed in the other.
Platform stubs (`#[cfg(not(target_os = "linux"))]`) must be kept in sync manually.
This has already happened once - the bridge version has `check_keyring_unlocked()`
as a backward-compat wrapper while the formation version does not.

---

### SHOULD FIX: Doc comments stripped from credential.rs

**File:** `crates/bm/src/formation/local/credential.rs` (was `linux/credential.rs`)

The move from `linux/credential.rs` to `local/credential.rs` stripped all doc
comments from the file. On main, the file has:

```rust
/// Loads the set of known keys from a JSON tracking file.
fn load_tracked_keys(...)

/// Stores a secret in a named collection via dbus-secret-service.
fn dss_store(...)

/// Key-value credential store backed by the system keyring.
pub struct LocalKeyValueCredentialStore { ... }
```

The PR version has no doc comments on any function or struct. This is a
maintainability regression. The doc comments explain *why* the tracking file exists
(keyring doesn't support enumeration), *what* the collection parameter does, etc.

**What to do:** Restore the doc comments from the original file.

---

### SHOULD FIX: `normalize_github_credential_helper` has a latent idempotency bug

**File:** `crates/bm/src/formation/local/common.rs:255-272`

The `old_helpers` list:
```rust
let old_helpers = [
    "!/usr/bin/gh auth git-credential",
    "!gh auth git-credential",
];
```

But `resolve_gh_helper_command()` produces a quoted-path format:
```rust
"!'/opt/homebrew/bin/gh' auth git-credential"
```

If `gh` moves between runs (e.g., Homebrew upgrade changes path), the previous
run's output (`!'/usr/bin/gh' auth git-credential`) would NOT match any entry in
`old_helpers`, so `normalize` would try to append a second `[credential]` block.
The idempotency guard on line 266 prevents a duplicate block, but within the
existing block the stale helper string would remain.

**What to do:** Add the quoted-path pattern to `old_helpers`, or better, use a
regex/prefix match on `auth git-credential` within `[credential "https://github.com"]`
blocks to find and replace any previous gh helper regardless of path format.

**Note:** This is a pre-existing issue carried forward by the extraction, not
introduced by this PR. But since the PR touches this code and moves it to a shared
module, it's the right time to fix it.

---

### SHOULD FIX: Initial `hosts.yml` written with default permissions

**File:** `crates/bm/src/formation/local/common.rs:62`

```rust
// setup_token_delivery:
fs::write(&hosts_yml, &hosts_content)  // Uses default perms (0o644)
```

But `refresh_token` (line 96-100) writes with `0o600`:
```rust
fs::OpenOptions::new()
    .write(true).create(true).truncate(true)
    .mode(0o600)
    .open(&tmp_path)
```

The initial write creates a world-readable file containing "placeholder" as the
token. Not a real security risk (placeholder is not a real token), but the
inconsistency is sloppy. If `refresh_token` fails for any reason, the file stays
world-readable.

**What to do:** Use the same `OpenOptions` with `mode(0o600)` in `setup_token_delivery`.

---

### MINOR: `_members` parameter silently ignored in `write_topology`

**File:** `crates/bm/src/formation/local/common.rs:226-233`

```rust
pub(crate) fn write_topology(
    workzone: &Path,
    team_name: &str,
    _members: &[(String, MemberHandle)],
) -> Result<()> {
    let runtime_state = state::load()?;
    formation::write_local_topology(workzone, team_name, &runtime_state)
}
```

The trait API passes members explicitly, but the implementation loads
`RuntimeState` independently and ignores the parameter. This is a pre-existing
design issue carried forward, but worth noting since the trait contract suggests
the caller's view of members should be used.

---

### MINOR: Redundant `dbus-secret-service` dev-dependency on Linux

**File:** `crates/bm/Cargo.toml:59-60`

```toml
[target.'cfg(target_os = "linux")'.dependencies]
dbus-secret-service = { version = "4.1.0", features = ["vendored"] }

[target.'cfg(target_os = "linux")'.dev-dependencies]
dbus-secret-service = { version = "4.1.0", features = ["vendored"] }
```

The dev-dependency is redundant since the regular dependency already includes it
on Linux. Not harmful, but dead weight.

---

### MINOR: macOS keyring check is a no-op

**File:** `crates/bm/src/formation/local/credential.rs:185-191`

```rust
#[cfg(not(target_os = "linux"))]
fn check_keyring_unlocked_for(collection_name: Option<&str>) -> Result<()> {
    if collection_name.is_some() {
        anyhow::bail!("Custom keyring collections are only supported on Linux")
    }
    Ok(())
}
```

On macOS, the default keyring check silently succeeds without verifying that
macOS Keychain is accessible. On Linux, the function validates the daemon is
running and the collection is unlocked. If macOS Keychain is locked or
inaccessible, the error will come from the `keyring` crate at store time,
which may produce a less helpful error message.

This is acceptable for now since the `keyring` crate on macOS uses
`security-framework` which handles Keychain access natively and produces
reasonable errors. But it's worth noting the asymmetry.

---

### MINOR: `with_keyring_dbus` uses `env::set_var` / `env::remove_var`

**File:** `crates/bm/src/formation/local/credential.rs:214-227`

Starting with Rust 1.83, `env::set_var` and `env::remove_var` are marked as
`unsafe` because they are not thread-safe. The comment says "bm is
single-threaded," but `tokio` with `rt-multi-thread` is in the dependency list.
This is a pre-existing issue that the PR carries forward. On macOS,
`BM_KEYRING_DBUS` is unlikely to be set (no D-Bus on macOS), so the code path
is effectively dead on macOS. Not urgent, but a ticking time bomb.

---

## No Linux Breakage Detected

The Linux path delegates to the same `common::` functions with the same logic
that was previously inline. Behavioral parity is maintained. The Cargo.toml
changes are additive (Cargo merges features for `keyring`). The credential.rs
move from `linux/credential.rs` to `local/credential.rs` preserves the same
public API.

---

## Recommended Fix Priority

| # | Priority | Issue | Effort |
|---|----------|-------|--------|
| 1 | **Must** | Merge identical Linux/macOS formation structs into single `LocalFormation` | Medium |
| 2 | **Must** | Extract shared credential backend from bridge + formation duplication | Medium-High |
| 3 | **Should** | Restore doc comments on credential.rs | Low |
| 4 | **Should** | Fix `normalize_github_credential_helper` idempotency with quoted paths | Low |
| 5 | **Should** | Use `mode(0o600)` for initial `hosts.yml` write | Trivial |
| 6 | **Minor** | Remove redundant dbus-secret-service dev-dependency | Trivial |

---

## Testing Notes

- The PR was tested by the author on macOS: "Formed a team and board on MacOS successfully"
- No CI cross-compilation setup exists for macOS targets, so the macOS-gated modules
  (`#[cfg(target_os = "macos")]`) are not compiled in Linux CI. Any compilation error
  in the macOS module would go undetected until someone builds on macOS.
- The macOS tests (`macos_formation_*`) are behind `#[cfg(target_os = "macos")]` and
  will never run in Linux CI. They exercise the same `common::` functions that the
  common module's own tests already exercise.
- E2E tests (`test_env.rs`) are unchanged and remain Linux-only (they depend on
  `dbus-daemon` and `gnome-keyring-daemon`).

---

*Review date: 2026-04-01*
*Reviewer: Claude Opus 4.6 (1M context)*
