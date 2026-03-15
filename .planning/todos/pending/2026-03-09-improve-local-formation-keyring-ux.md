---
created: 2026-03-09T05:16:51.974Z
title: Improve local formation keyring UX
area: cli
files:
  - crates/bm/src/bridge.rs:94-116
  - crates/bm/Cargo.toml:30
  - docs/content/how-to/bridge-setup.md
  - .planning/debug/keyring-report.md
---

## Problem

Local formation is meant to provide frictionless agent operation, but credential storage fails on first use when the system keyring's login collection doesn't exist. The current error message ("System keyring not available") is misleading — the gnome-keyring daemon may be running but the login collection is a phantom entry (referenced in D-Bus but never materialized).

Root cause (from .planning/debug/keyring-report.md): On Linux, the login collection is created/unlocked via PAM during desktop login. Users accessing the system via `su -` or SSH don't trigger PAM keyring integration, so the collection is never created. The daemon runs, D-Bus reports the collection path, but querying it returns "Object does not exist."

This is system-dependent:
- **Linux**: Secret Service collection may not exist depending on login method (su, SSH, console vs desktop)
- **macOS**: Keychain is always available (no setup needed)
- **Windows**: Credential Manager is always available (no setup needed)

## Solution

1. **Detect specific failure modes** in `LocalCredentialStore::store()`:
   - No keyring daemon → "Install gnome-keyring or equivalent"
   - Daemon running but collection missing/locked → "Keyring login collection not initialized. On desktop login this happens automatically. For su/SSH access, either: (a) add pam_gnome_keyring.so to /etc/pam.d/su-l, or (b) run `echo -n 'pw' | gnome-keyring-daemon --replace --unlock`"
   - Other errors → current fallback behavior

2. **Improve error messages** to be system-aware (Linux-specific guidance vs macOS/Windows)

3. **Document keyring prerequisites** in bridge setup guide with platform-specific sections

4. **Consider**: auto-creating a `botminter` collection if login collection unavailable (but must be password-protected to be meaningful — password-less collection is no better than a plaintext file)

5. **Consider**: `bm doctor` or `bm setup check` command that validates system prerequisites including keyring state
