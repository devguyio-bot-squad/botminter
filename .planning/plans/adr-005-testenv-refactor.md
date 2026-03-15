# Plan: ADR-005 TestEnv/TestCommand Refactor — Trial 2

## Context

The design is settled (ADR-005 committed).
A previous attempt failed because it never ran e2e-v2 tests and never committed incrementally.

This trial enforces: **every step ends with `just e2e-v2` green and a commit.**

## Design

```
GithubSuite
  .build() / .build_progressive()
    │
    1. Create TestEnv
    │   TestEnv::fresh(gh_token, gh_org, repo)
    │   or TestEnv::resume(home, gh_token, gh_org, repo)
    │
    │   Internally:
    │     a. Create tempdir (HOME)  [resume: reuse]
    │     b. Capture originals (full env snapshot)
    │     c. Bootstrap profiles into HOME
    │     d. Install stub ralph, build PATH
    │     e. Setup git auth (.gitconfig)
    │     f. Start isolated dbus-daemon
    │     g. Start gnome-keyring-daemon
    │     h. Build base { HOME, GH_TOKEN, PATH, GIT_* }
    │        (no DBUS/XDG vars — commands inherit real values from originals)
    │     i. includes: "bm" → { BM_KEYRING_DBUS = isolated dbus addr }
    │     j. overrides: "podman" → { HOME = real HOME }
    │     k. [resume] Load exports from snapshot
    │
    │   No process-wide env mutation. No SetupCtx.
    │
    2. Run cases: case(&mut TestEnv)
    │     env.command("bm")  → TestCommand (hermetic)
    │     env.export/get_export → cross-case state
    │     env.reset_keyring()
    │
    3. [progressive] env.save()
    4. TestEnv drops:
    │     Always: kill dbus-daemon, clean dbus tmpdir
    │     fresh only: delete test HOME (TestEnv owns it)
    │     resume: leave HOME alone (persists across runs)
    5. Report

TestCommand (from env.command("binary"))
    resolved = originals → base → exports → includes[bin] → overrides[bin]
    Command::new(binary).env_clear().envs(resolved)
    .args() .env("K","V") .current_dir() .run() .run_fail() .output()
```

## Execution Discipline

Every step:
1. Change `e2e-v2/`
2. **`just e2e-v2`**
3. Red → fix. Green → commit.

## Steps

### Step 1: Add `just e2e-v2` recipe, copy, verify

- Add `e2e-v2` recipe to Justfile (mirrors `e2e`, uses `--test e2e-v2`)
- Copy `tests/e2e/` → `tests/e2e-v2/`
- Register `e2e-v2` test binary in `Cargo.toml`
- **`just e2e-v2`** — green
- Commit

**Finding:** Both `just e2e` and `just e2e-v2` had identical pre-existing failures in
operator_journey's second pass (after `reset_home`). Two bugs:
1. `reset_home` didn't remove the old Tuwunel container/volume — second pass tried to
   restart a stale container and failed with "did not become healthy within 120s".
2. `daemon_start_poll` didn't pre-seed poll state — after HOME wipe, daemon saw all
   first-pass GitHub events as new and immediately launched ralph before the assertion.
Fixed in a separate commit (`a4a7230`) applied to both `e2e/` and `e2e-v2/`.

### Step 2: Add TestEnv + TestCommand types

- Add `test_env.rs` in `e2e-v2/` with `TestEnv` and `TestCommand` per ADR-005 and the design above
- `mod test_env;` in `main.rs`
- **`just e2e-v2`** — green (new types unused)
- Commit

**Deviations from design:**
- Added `reset_home()` method (needed for operator_journey's HOME wipe between passes).
- Added `remove_export()` method.
- `dbus-daemon` and `gnome-keyring-daemon` receive env vars explicitly via `.env()` on
  the subprocess — stronger isolation than the design assumed (no process env inheritance).
- Base env includes `DBUS_SESSION_BUS_ADDRESS`, `XDG_RUNTIME_DIR`, `XDG_DATA_HOME`
  (pointing to isolated dirs). Podman override restores all three to real system values.

### Step 3: Port everything to TestEnv

TestEnv replaces the entire GithubSuite build flow. Cases receive `&mut TestEnv` instead of `&SuiteCtx`. Atomic step — signature change forces all cases to change together.

**Pre-step: fix TestEnv base env.** Current implementation puts isolated DBUS,
XDG_RUNTIME_DIR, and XDG_DATA_HOME in base. This violates the ADR — base env must
not contain any D-Bus/XDG vars. Per ADR section "Subsystem isolation pattern":
process-wide env stays real, commands inherit real system values from originals.
The only isolation mechanism is `BM_KEYRING_DBUS` as an include for `bm`.

Corrected base env (only test-specific vars):
- `HOME` = test HOME
- `PATH` = stub-bin + real PATH
- `GH_TOKEN`, `GIT_*` = test values

No DBUS, XDG_RUNTIME_DIR, or XDG_DATA_HOME in base — commands inherit real values
from originals. The isolated dbus-daemon + gnome-keyring-daemon are started with
explicit env vars during TestEnv construction; the only external reference is
`BM_KEYRING_DBUS` (include for `bm`).

Corrected overrides: `podman` only needs `HOME` override (everything else is already
real from originals).

Porting tasks:
- Fix TestEnv base env per above
- Rewrite GithubSuite: create TestEnv (replaces tempdir + bootstrap + KeyringGuard + BridgeTestEnv + SuiteCtx)
- Port all case closures: `Fn(&SuiteCtx)` → `Fn(&mut TestEnv)`, replace `bm_cmd()` with `env.command("bm")`, replace `assert_cmd_success/fails` with `.run()/.run_fail()`
- Port `reset_home` case to use `env.reset_home()` + `env.command("podman")` for container cleanup
- Port ProcessGuard/DaemonGuard to capture resolved env HashMap at construction (Drop can't access TestEnv)
- Port isolated.rs: each isolated test creates its own TestEnv
- `apply_bridge_env` calls become no-ops — delete them all
- Cross-case state files (`.tuwunel-port`, `.rc-port`, etc.) → `env.export()`/`env.get_export()`
- Remove dead code: KeyringGuard, BridgeTestEnv, ISOLATED_DBUS_ADDR, bm_cmd(), SuiteCtx, assert_cmd_success/fails, setup_git_auth, install_stub_ralph, path_with_stub, bootstrap_profiles_to_tmp
- `cargo check` frequently during porting
- **`just e2e-v2`** — green
- Commit

### Step 4: Swap

- **`just e2e`** + **`just e2e-v2`** — both green
- Delete `tests/e2e/`, rename `tests/e2e-v2/` → `tests/e2e/`
- Update Cargo.toml + Justfile (remove e2e-v2 entries)
- **`just test`** — green (unit + conformance + e2e)
- Commit

### Step 4: Swap

- **`just e2e`** + **`just e2e-v2`** — both green
- Delete `tests/e2e/`, rename `tests/e2e-v2/` → `tests/e2e/`
- Update Cargo.toml + Justfile
- **`just e2e`** — green
- Commit

## Key files

| File | Role |
|------|------|
| `.planning/adrs/0005-e2e-test-environment-and-isolation.md` | ADR |
| `crates/bm/tests/e2e/helpers.rs` | Current infrastructure (to replace) |
| `crates/bm/tests/e2e/scenarios/*.rs` | Current scenarios (to port) |
| `crates/bm/tests/e2e/isolated.rs` | Isolated tests (to port) |
| `Justfile` | Recipe entry points |
| `crates/bm/Cargo.toml` | Test binary registration |

## Tracking

- [x] Step 1: Add `just e2e-v2` recipe, copy, verify — `2f5bb4d`
- [x] Step 2: Add TestEnv + TestCommand types — `ffbc872`
- [x] Pre-existing fix: operator_journey second-pass cascading failures — `a4a7230`
- [x] Step 3: Port everything to TestEnv — `3117cb5`
- [ ] Step 4: Swap e2e-v2 → e2e

## Verification

Every step: **`just e2e-v2`**. Final: **`just e2e`**.

Anti-pattern: `grep -rn "Command::new" crates/bm/tests/e2e/scenarios/` — only Drop impls.
