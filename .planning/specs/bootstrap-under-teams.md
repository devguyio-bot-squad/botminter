# Move Bootstrap Under Teams + Fix Mount Scope

## Context

Bootstrap is currently a standalone top-level command (`bm bootstrap`) that creates a VM before any team exists. The flow should be: `bm init` (create team on host) → `bm teams bootstrap` (provision VM for that team) → `bm attach` (enter VM). This means bootstrap is team-scoped — it knows the team config and can mount the right directories.

The VM should mount `~/.botminter/` (the config + workzone root), not all of `~`.

## Changes

### 1. Move bootstrap to `bm teams bootstrap`

**CLI (`cli.rs`):**
- Remove `Command::Bootstrap` from top-level enum
- Add `Bootstrap` variant to `TeamsCommand` enum with same fields (minus `non_interactive` — team subcommands don't have that pattern, and `--render` stays)
- Keep `--render`, `--name`, `--cpus`, `--memory`, `--disk` flags
- Add `-t`/`--team` flag (consistent with other team subcommands)

**Dispatch (`main.rs`):**
- Remove `Command::Bootstrap` match arm
- Add `TeamsCommand::Bootstrap` match arm that loads team config first, then calls bootstrap

**Command (`commands/bootstrap.rs`):**
- `run()` now takes team context: loads config, resolves team via `-t` flag, bails if no team exists
- Passes team info to `lima.bootstrap()` so the template can mount the right path
- Remove `run_non_interactive()` (team commands don't use this pattern — the team flag is sufficient)

### 2. Team-scoped template generation

**Domain (`formation/lima.rs`):**
- `generate_template()` gets a new parameter for the mount path (the `~/.botminter/` config dir)
- `bootstrap()` gets team context so it can derive the mount path
- Mount changes from `location: "~"` to `location: "~/.botminter"` (the config dir, which contains the workzone)

### 3. Require team exists before bootstrap

**Command (`commands/bootstrap.rs`):**
- Load config via `config::load()`
- Resolve team via `config::resolve_team(cfg, team_flag)`
- If no team found, bail with "Run `bm init` first"

### 4. Revert premature mount change

The mount was already changed from `~` to `~/.botminter` in the working tree. This needs to stay but be done properly (parameterized via config, not hardcoded).

## Files Modified

| File | Change |
|------|--------|
| `crates/bm/src/cli.rs` | Move Bootstrap from Command to TeamsCommand |
| `crates/bm/src/main.rs` | Move dispatch to TeamsCommand match |
| `crates/bm/src/commands/bootstrap.rs` | Team-scoped run(), remove run_non_interactive |
| `crates/bm/src/formation/lima.rs` | Parameterize mount path in template |

## Verification

1. `just clippy` — no warnings
2. `just unit` — all tests pass
3. `bm teams bootstrap --render -t my-team` — shows template with `~/.botminter` mount
4. `bm bootstrap` (top-level) — should no longer exist
5. `bm teams bootstrap` (no team) — should error with "Run bm init first"
