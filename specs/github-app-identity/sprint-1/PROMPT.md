# Sprint 1: Formation Trait + CredentialStore + Team API Boundary

## Objective

Establish the Team → Formation architectural foundation by defining the `Formation` trait, generalizing the `CredentialStore` to a key-value interface, and creating the Team struct as the operator-facing API boundary. All existing behavior MUST be preserved — this is a structural refactor with no auth model changes.

## Prerequisites

None — this is the first sprint.

## Deviations from Design

- `gh_token: Option<&str>` parameters in `git/github.rs` are NOT removed in this sprint. The daemon still needs the stored token during Sprint 2. Removal happens in Sprint 3 when the auth model swaps.
- `setup_token_delivery()` and `refresh_token()` on the Formation trait are no-ops — implemented in Sprint 3.
- `start_members()` delegates to existing `start_local_members()` — daemon-mediated launch is Sprint 2.

## Key References

- Design: `specs/github-app-identity/design.md`
- ADR-0008: `.planning/adrs/0008-team-runtime-architecture.md` (Formation trait, Team boundary, CredentialStore)
- Sprint plan: `specs/github-app-identity/sprint-1/plan.md`

## Requirements

1. The `Formation` trait MUST be defined in `formation/mod.rs` with all method signatures per ADR-0008: `setup`, `check_environment`, `check_prerequisites`, `credential_store`, `setup_token_delivery`, `refresh_token`, `start_members`, `stop_members`, `member_status`, `exec_in`, `shell`, `write_topology`. Ref: design.md "Formation Trait" section.

2. The `CredentialStore` trait MUST be defined as a key-value interface with `store(key, value)`, `retrieve(key)`, `remove(key)`, `list_keys(prefix)`. Ref: ADR-0008 "CredentialStore trait" section.

3. The `CredentialDomain` enum MUST support `Bridge` and `GitHubApp` variants with the fields specified in design.md "CredentialDomain" section.

4. A `formation/local/` module structure MUST be created with platform detection that delegates to `linux/` or `macos/` sub-modules. Ref: ADR-0008 "Module structure" section.

5. `LinuxLocalFormation` MUST implement the `Formation` trait by delegating to existing free functions (`start_local_members`, `stop_local_members`, etc.). No logic moves — this is a thin wrapper.

6. `MacosLocalFormation` MUST return clear "not yet supported" errors for all trait methods.

7. `LocalCredentialStore` MUST be moved from `bridge/credential.rs` to `formation/local/linux/credential.rs` and generalized to the key-value `CredentialStore` interface. The bridge module MUST manage bridge-state.json metadata independently of the credential store.

8. `InMemoryCredentialStore` MUST be updated to implement the key-value interface.

9. A `Team` struct MUST be created that wraps `TeamEntry` + `Box<dyn Formation>` and provides methods (`start`, `stop`, `status`, etc.) that delegate to the formation. Ref: design.md "Architecture" section.

10. Commands SHOULD begin resolving Team and calling team methods instead of formation free functions directly, where the delegation chain is straightforward.

11. All existing E2E, integration, and unit tests MUST pass without modification (behavior is unchanged).

12. The `--formation` CLI flag MUST be preserved (it selects the deployment strategy).

## Acceptance Criteria

1. **Given** `formation::create("local")` on Linux, **when** called, **then** a `LinuxLocalFormation` is returned that implements all `Formation` trait methods.

2. **Given** `formation::create("local")` on macOS, **when** called, **then** a clear "not yet supported" error is returned.

3. **Given** a `LinuxLocalFormation`, **when** `credential_store(Bridge { .. })` is called, **then** a key-value `CredentialStore` backed by the system keyring is returned.

4. **Given** a key-value `CredentialStore`, **when** `store("superman/github-app-id", "123")` and `retrieve("superman/github-app-id")` are called, **then** the value `"123"` is returned.

5. **Given** a key-value `CredentialStore`, **when** `list_keys("superman/")` is called, **then** all keys with that prefix are returned.

6. **Given** `team.start(None)`, **when** called, **then** it delegates through the formation to existing `start_local_members()` — same behavior as before.

7. (Regression) **Given** `just test`, **when** run, **then** all existing tests pass.

8. (Regression) **Given** bridge credential operations, **when** performed through the new key-value interface, **then** bridge-state.json is still updated correctly by the bridge module.
