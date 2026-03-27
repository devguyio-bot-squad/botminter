---
status: pending
created: 2026-03-17
started: null
completed: null
---
# Task: Formation Trait Refactor

## Description

Introduce the `Formation` trait from ADR-008, implement `LinuxLocalFormation`, move `LocalCredentialStore` from `bridge/` into the formation module, and make commands formation-agnostic via `Box<dyn Formation>`.

## Background

The current `formation/` module has flat files with free functions (`launch_ralph`, `start_local_members`, `stop_local_members`). Commands hardcode `LocalCredentialStore` directly. ADR-008 defines a trait-based architecture where formations are deployment strategies and commands interact only through `Box<dyn Formation>`.

## Reference Documentation

- ADR: `.planning/adrs/0008-local-formation-as-first-class-concept.md`
- Formation audit: `.planning/reports/formation-audit.md`
- Requirements: `specs/local-formation/requirements.md`

## Technical Requirements

### Formation trait and types

1. Define `Formation` trait in `formation/mod.rs` with methods:
   - `name()`, `setup()`, `check_environment()`, `check_prerequisites()`
   - `credential_store()` returning `Box<dyn CredentialStore>`
   - `launch_member()`, `stop_member()`, `is_member_alive()`
   - `write_topology()`
2. Define supporting types: `SetupParams`, `EnvironmentStatus`, `EnvironmentCheck`, `LaunchParams`, `MemberHandle`
3. Add factory functions: `formation::resolve()`, `formation::create()`, `formation::load()`, `formation::list()`

### Module restructure

4. Restructure `formation/` into:
   ```
   formation/
     mod.rs              # Trait, types, factory functions
     config.rs           # FormationConfig (keep existing)
     init.rs             # Keep existing
     local/
       mod.rs            # Platform detection, delegates to linux/
       process.rs        # Shared POSIX: launch, stop, is_alive (from launch.rs, stop_members.rs)
       topology.rs       # Shared local topology (from local_topology.rs)
       linux/
         mod.rs          # LinuxLocalFormation impl
         credential.rs   # LocalCredentialStore (from bridge/credential.rs)
         setup.rs        # Keyring setup, tool installation checks
   ```
5. Move `LocalCredentialStore` and all D-Bus/keyring helpers from `bridge/credential.rs` to `formation/local/linux/credential.rs`
6. Keep `CredentialStore` trait and `InMemoryCredentialStore` in `bridge/credential.rs` (formation-neutral)

### LinuxLocalFormation

7. Implement `Formation` trait for `LinuxLocalFormation`:
   - `launch_member()` delegates to shared `process.rs`
   - `stop_member()` delegates to shared `process.rs`
   - `is_member_alive()` delegates to shared `process.rs`
   - `credential_store()` returns `LocalCredentialStore`
   - `write_topology()` delegates to shared `topology.rs`
   - `check_prerequisites()` checks `ralph` in PATH, keyring accessible
   - `setup()` and `check_environment()` wire to bootstrap logic

### Command refactor

8. Update `commands/start.rs` to resolve formation and call trait methods instead of `start_local_members()`
9. Update `commands/stop.rs` to use formation trait instead of `stop_local_members()`
10. Update `commands/status.rs` to use formation for health checks
11. Update bridge credential commands to get `CredentialStore` from formation instead of constructing `LocalCredentialStore` directly
12. No command should import formation-specific types — only `Box<dyn Formation>`

## Dependencies

- Tasks 1-3 must be complete (bootstrap, attach, and e2e verification working)
- `bridge/credential.rs` — `CredentialStore` trait stays here
- All commands in `commands/` that touch member lifecycle or credentials

## Implementation Approach

1. Start by defining the trait and types alongside existing code (additive, no breakage)
2. Create the `local/` module hierarchy and move code file by file
3. Implement `LinuxLocalFormation` delegating to moved code
4. Update commands one at a time, verifying tests pass after each
5. Remove old flat files (`launch.rs`, `start_members.rs`, `stop_members.rs`, `local_topology.rs`) once commands are migrated

## Acceptance Criteria

1. **Formation trait exists**
   - Given the `formation` module
   - When inspecting `formation/mod.rs`
   - Then it defines the `Formation` trait with all methods from ADR-008

2. **LinuxLocalFormation implements trait**
   - Given a Linux platform
   - When `formation::create()` is called with `"local"`
   - Then it returns a `Box<dyn Formation>` backed by `LinuxLocalFormation`

3. **LocalCredentialStore moved**
   - Given `bridge/credential.rs`
   - When inspecting its contents
   - Then it contains only `CredentialStore` trait, `InMemoryCredentialStore`, and `resolve_credential_from_store()`
   - And `LocalCredentialStore` lives in `formation/local/linux/credential.rs`

4. **Commands are formation-agnostic**
   - Given any command that touches member lifecycle or credentials
   - When inspecting its imports
   - Then it does not import `LocalCredentialStore`, `LinuxLocalFormation`, or any formation-specific type directly

5. **Unsupported platform**
   - Given a non-Linux platform
   - When `formation::create("local")` is called
   - Then it returns an error: "Local formation is not supported on this platform"

6. **No regressions**
   - Given the full test suite
   - When `just test` is run
   - Then all tests pass

## Metadata
- **Complexity**: High
- **Labels**: formation, refactor, architecture
- **Required Skills**: Rust, trait design, module organization
