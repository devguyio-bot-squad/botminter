---
status: accepted
date: 2026-03-14
decision-makers: operator (ahmed), claude
---

# E2E Test Environment Management and Isolation Patterns

## Problem

E2E tests run commands against real infrastructure (GitHub, podman, system keyring). Each command needs a specific environment — test HOME, auth tokens, isolated backends for certain subsystems — while still inheriting real system resources for others. When this environment logic is scattered across test cases, it gets missed, duplicated, or contradicted, causing subtle failures.

**Motivating case:** Keyring isolation. Tests need an isolated D-Bus for keyring operations (don't pollute real keyring) but podman needs the real system D-Bus for cgroup management. Some `bm` commands do both in the same process. A single missed env var call caused 23 cascading test failures.

## Constraints

* Process-wide environment variables must NEVER be replaced for isolation or test environment purposes
* Bridge recipes (Justfiles) and other external subprocesses must NEVER need isolation-related env var handling
* Test cases must NEVER access `std::process::Command` directly — all commands go through `TestEnv`/`TestCommand`
* All environment decisions must live in one place — `TestEnv`
* Isolation env vars absent = zero behavior change in production
* `TestEnv` state is ephemeral — it does not persist across progressive mode runs

## Decision

### `TestEnv` is the shell session

`TestEnv` is the test equivalent of a shell session. It holds environment state and produces commands. All command execution in E2E tests goes through it.

`TestEnv` applies three layers when producing a command:

1. **Base** — applied to every command: test HOME, GH_TOKEN, PATH, git identity vars.
2. **Include** — vars added only for specific binaries. Example: `bm` gets `BM_KEYRING_DBUS` because it's the only binary that reads it.
3. **Override** — replaces a base var's value for specific binaries. Example: `podman` gets real HOME instead of test HOME because it needs real container storage.

Like a shell, `TestEnv` supports two scopes for setting env vars:

* **`env.export("KEY", val)`** — in-memory. All future commands in this run get it. Like `export KEY=val` in a shell.
* **`env.bm().env("KEY", val)`** — one-shot. Only this command gets it. Like `KEY=val bm start` in a shell.

### Construction and state snapshots

`TestEnv` has two constructors:

* **`TestEnv::fresh(home, gh_token)`** — starts clean. Deletes any existing snapshot. Used for the first pass or after `reset_home`.
* **`TestEnv::resume(home, gh_token)`** — loads a previous snapshot if it exists, restoring all exports from the prior run. Used in progressive mode to continue from where the last run left off.

State snapshot operations:

* **`env.save()`** — snapshots all current exports to a state file in HOME.
* **`env.clear()`** — deletes the snapshot. Idempotent — safe to call whether or not a snapshot exists.

`TestEnv` is a living environment — its state evolves over the test lifecycle. Before keyring isolation starts, `BM_KEYRING_DBUS` isn't available. After the isolated keyring is created, it's added to `bm`'s includes. Commands produced at any point get the correct environment for that moment.

### Known per-binary rules

| Binary | Layer | Var | Value | Reason |
|--------|-------|-----|-------|--------|
| `bm` | include | `BM_KEYRING_DBUS` | isolated D-Bus address | Only `bm` reads this for keyring boundary swap |
| `podman` | override | `HOME` | real system HOME | Needs real container storage at `~/.local/share/containers` |
| `podman` | override | `XDG_RUNTIME_DIR` | real system value | Needs real cgroup socket |

### `TestCommand` wraps `Command`

`TestEnv` never returns a raw `std::process::Command`. It returns a `TestCommand` wrapper that exposes a controlled API:

* `.args()` — add command arguments
* `.env("KEY", val)` — one-shot env var for this command only
* `.run()` — execute, assert success, return stdout
* `.run_fail()` — execute, assert failure, return stderr
* `.output()` — raw output without assertions

The underlying `Command` is never exposed. This prevents test cases from calling `.current_dir()`, `.stdin()`, or other `Command` methods that bypass `TestEnv`'s control.

### `TestEnv` owns setup and teardown

`TestEnv` manages the full test lifecycle:

* **Setup** — starts isolated daemons (dbus, gnome-keyring), captures real system values for overrides, builds the base environment and per-binary rules.
* **Command execution** — produces `TestCommand` instances with base + includes + overrides + one-shot vars applied.
* **Teardown** — kills daemons, cleans temp directories.

### Cross-case state persistence

Exports are in-memory by default — they do not survive across progressive mode runs. To persist state across runs, call `env.save()` which snapshots all current exports to disk. On the next run, `TestEnv::resume()` restores them.

Test cases should not write state files directly. Use `env.export()` + `env.save()` to persist cross-case state (bridge ports, container names, guard info) through `TestEnv`'s snapshot mechanism.

### Subsystem isolation pattern

When a subsystem needs test isolation:

1. **Named env var in `bm`.** Define a `BM_<SUBSYSTEM>_<RESOURCE>` env var. At the boundary of the subsystem's operations, `bm` reads the env var and temporarily routes to the isolated backend. When absent, no-op.

2. **`TestEnv` includes it.** `TestEnv` adds the isolation env var as an include rule for `bm` (or whichever binary reads it). Only the relevant binary gets it.

3. **Process-wide env stays real.** All commands inherit real system env vars. Only `bm`'s internal boundary swap uses the isolation var.

### Current isolation instances

| Env var | Subsystem | Boundary behavior |
|---------|-----------|-------------------|
| `BM_KEYRING_DBUS` | Keyring (Secret Service) | At each keyring operation (store/retrieve/remove), `bm` temporarily swaps `DBUS_SESSION_BUS_ADDRESS` to this value, then restores it. |

## Rejected Alternatives

### Process-wide env var replacement

Rejected because: replacing env vars process-wide (e.g., `DBUS_SESSION_BUS_ADDRESS`) affects ALL subprocesses, breaking subsystems that need the real value. In the keyring case, this broke podman and caused tmpfs inode exhaustion from orphaned container overlay storage in redirected `XDG_DATA_HOME` tmpdirs.

### Per-command patching

Rejected because: requires every test case to apply env vars per command. Was tried and failed — a single missed call caused 23 cascading test failures.

### Helper function instead of `TestEnv` type

Rejected because: a bare function can only handle concerns known at definition time. The `bm_cmd()` function started simple, then grew D-Bus injection via a thread-local, then needed per-command patching layered on top. A struct with state is the natural shape.

### `TestEnv` for `bm` only, direct Command for everything else

Rejected because: env var logic leaks into test cases. `gh` calls manually set `GH_TOKEN`, `podman` calls manage HOME — the same scattered env var problem, just for different binaries. `TestEnv`'s purpose is not "bm isolation" — it is "test environment." All commands run in the test environment.

### Returning raw `std::process::Command`

Rejected because: exposing the full `Command` API tempts callers to bypass `TestEnv` with `.current_dir()`, `.stdin()`, or direct `.env()` calls that skip the include/override layers. `TestCommand` exposes only the operations test cases need.

## Consequences

* Zero env var logic in test cases — `TestEnv` handles everything, for every binary
* Zero Justfile changes — bridge recipes never see isolation env vars
* Adding a new env concern (base, include, or override) = one change in `TestEnv`, zero test case changes
* The env var swap inside `bm` (production code) uses process-global `std::env::set_var` — safe because `bm` is single-threaded
* Isolation env vars are test-oriented but live in production code — the no-op fast path has zero overhead
* Test cases use `TestCommand`'s constrained API — they can add args and one-shot env vars but cannot bypass `TestEnv`'s layers

## Anti-patterns

* **Do NOT** replace process-wide env vars for any test purpose — isolation, HOME redirection, or otherwise. It breaks other tools that depend on the real values.
* **Do NOT** set `XDG_DATA_HOME`, `XDG_RUNTIME_DIR`, or similar directory env vars process-wide in tests. Other tools (podman, systemd) use them and create state there.
* **Do NOT** construct `std::process::Command` directly in E2E test cases. Use `TestEnv::command()` or `TestEnv::bm()` which return `TestCommand`. Direct `Command::new()` is only acceptable in panic-safety Drop implementations (guard cleanup).
* **Do NOT** use thread-locals or process-wide env mutation to pass test context to commands. `TestEnv` sets env vars explicitly on each `TestCommand` instance.
* **Do NOT** rely on `TestEnv` exports persisting across progressive mode runs without calling `env.save()`. Exports are in-memory by default.
* **Do NOT** write state files directly for cross-case persistence. Use `env.export()` + `env.save()` — `TestEnv` manages its own snapshot file.
