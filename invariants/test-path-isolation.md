# Test Path Isolation

Tests MUST NOT read from or write to real user runtime directories.

## Rule

All tests (unit, integration, E2E) MUST use temporary directories for any paths that would resolve under the user's home directory at runtime. This includes but is not limited to:

- `~/.botminter` / `$HOME/.botminter`
- `~/.config/botminter` / `$XDG_CONFIG_HOME/botminter`
- `~/.local/share/botminter` / `$XDG_DATA_HOME/botminter`

Tests that need a HOME directory MUST set `$HOME` (and/or relevant `XDG_*` vars) to a `tempfile::tempdir()` path. Tests MUST NOT assume or use the real user's home directory for any purpose.

## Applies To

- All test code in `crates/bm/tests/` (integration and E2E tests)
- All `#[cfg(test)]` modules in `crates/bm/src/` (unit tests)
- Test helper functions and fixtures

**Does NOT apply to:**
- Production code that legitimately resolves `~/.config/botminter` at runtime
- Documentation examples

## Examples

**Compliant:**
```rust
let tmp = tempfile::tempdir().unwrap();
let mut cmd = bm_cmd();
cmd.env("HOME", tmp.path());
// Config resolves to tmp/.config/botminter — real dirs untouched
```

**Compliant:**
```rust
let tmp = tempfile::tempdir().unwrap();
std::env::set_var("HOME", tmp.path());
let config = Config::load(); // Reads from tmp/.config/botminter
```

**Violating:**
```rust
let mut cmd = bm_cmd();
// No HOME override — writes to real ~/.config/botminter
cmd.args(["init", ...]);
```

**Violating:**
```rust
let config_path = dirs::config_dir().unwrap().join("botminter");
// Uses the real user's config dir
std::fs::create_dir_all(&config_path).unwrap();
```

## Rationale

Tests that use real user paths pollute the developer's environment with leftover directories and config files. This has happened twice: first when tests hardcoded `~/.botminter`, and again when tests wrote to `~/.config/botminter` without overriding HOME. Temporary directories are cleaned up automatically and prevent cross-test contamination.
