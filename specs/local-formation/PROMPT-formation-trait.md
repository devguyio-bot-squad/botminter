# Local Formation — Trait Refactor

## Objective

Introduce the `Formation` trait from ADR-008, implement `LinuxLocalFormation`, move `LocalCredentialStore` into the formation module, and make all commands formation-agnostic via `Box<dyn Formation>`.

## Prerequisites

The bootstrap and attach features from `PROMPT-bootstrap.md` MUST be complete and all tests passing before starting this work.

## Spec Directory

`specs/local-formation/`

## Execution Order

1. `tasks/task-04-formation-trait-refactor.code-task.md` — single task covering the full refactor

## Critical Design Decisions

These were identified via adversarial review and MUST be followed:

### `start_local_members()` orchestration logic
The current `start_local_members()` in `formation/start_members.rs` (~150 lines) interleaves bridge auto-start, credential resolution, member discovery, stale state cleanup, process launching, 2-second alive verification, and topology writing. This does NOT map to a single `Formation::launch_member()` call.

**Resolution:** Keep the orchestration logic in `commands/start.rs` (or a shared helper). The `Formation` trait methods are low-level primitives:
- `launch_member()` → spawns the process, returns `MemberHandle`
- `is_member_alive()` → checks if PID is alive
- `stop_member()` → sends signal

The verify-after-launch pattern, stale cleanup, and result aggregation stay in the command layer. `auto_start_bridge()` moves to a bridge helper (it's not a formation concern per ADR-008).

### `LocalCredentialStore` and bridge-state.json coupling
`LocalCredentialStore::store()` currently writes to `bridge-state.json` (identity recording). After the move to `formation/local/linux/credential.rs`, this creates a `formation → bridge` dependency. This direction already exists (`start_members.rs` imports `bridge::*`), so it's consistent. But the identity-recording side effect should be noted — it's a bridge concern leaking into the formation.

### Call sites that construct `LocalCredentialStore` directly (6 total)
1. `commands/bridge/mod.rs` — `make_credential_store()`
2. `commands/hire.rs` — inline construction
3. `formation/start_members.rs` — `resolve_bridge_credentials()`
4. `workspace/team_sync.rs` — two call sites (lines ~206 and ~266)
5. `tests/e2e/isolated.rs` — direct construction for keyring testing

All must be refactored to go through `Box<dyn Formation>` or the formation module's public API.

### `formation::create()` vs `formation::load()`
Both coexist with different purposes:
- `load()` → reads `formation.yml` YAML config, returns `FormationConfig` data struct (existing)
- `create()` → factory that returns `Box<dyn Formation>` trait object (new)
- `create()` uses `load()` internally

### Module registration
When restructuring `formation/` into `formation/local/linux/`, update:
- `formation/mod.rs` — declare `pub mod local;`
- `formation/local/mod.rs` — declare `pub mod linux;` and `pub mod process;` and `pub mod topology;`
- Re-export key types from `formation/mod.rs` so existing imports keep working during migration

## Key Requirements

1. The `Formation` trait MUST define: `name()`, `setup()`, `check_environment()`, `check_prerequisites()`, `credential_store()`, `launch_member()`, `stop_member()`, `is_member_alive()`, `write_topology()`
2. `formation/` MUST be restructured into `formation/local/linux/` hierarchy per ADR-008
3. `LocalCredentialStore` MUST move from `bridge/credential.rs` to `formation/local/linux/credential.rs`. `CredentialStore` trait and `InMemoryCredentialStore` stay in `bridge/credential.rs`
4. Commands MUST use `Box<dyn Formation>` — no direct imports of `LocalCredentialStore` or `LinuxLocalFormation`
5. `just test` MUST pass after all changes

## Deviations from ADR-008

- `MacosLocalFormation` is out of scope — only Linux is implemented
- `setup()` wires to the bootstrap logic rather than being standalone
- Orchestration logic (start all members, bridge auto-start) stays in commands, not in the formation

## Key References

- ADR: `.planning/adrs/0008-local-formation-as-first-class-concept.md`
- Formation audit: `.planning/reports/formation-audit.md`
- Current formation module: `crates/bm/src/formation/` (flat structure to be restructured)
- Current credential code: `crates/bm/src/bridge/credential.rs` (LocalCredentialStore to move)
- Current commands: `crates/bm/src/commands/start.rs`, `stop.rs`, `status.rs`
- Current bridge commands: `crates/bm/src/commands/bridge/mod.rs` (`make_credential_store()`)
- E2E keyring test: `crates/bm/tests/e2e/isolated.rs` (import path will change)
