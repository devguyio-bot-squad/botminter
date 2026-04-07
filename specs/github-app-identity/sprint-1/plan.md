# Sprint 1: Formation Trait + CredentialStore + Team API Boundary

## Checklist

- [x] Define `Formation` trait in `formation/mod.rs`
- [x] Define key-value `CredentialStore` trait in `formation/mod.rs`
- [x] Define `CredentialDomain` enum and supporting types
- [x] Create `formation/local/` module structure with platform detection
- [x] Implement `LinuxLocalFormation` wrapping existing free functions
- [x] Create `MacosLocalFormation` stub
- [x] Move `LocalCredentialStore` to `formation/local/linux/credential.rs` and generalize to key-value
- [x] Decouple bridge-state.json from credential store (bridge module manages its own metadata)
- [x] Create Team struct as API boundary wrapping formation
- [x] Migrate bridge credential usage to new key-value interface
- [x] Update commands to go through Team where applicable
- [x] Tests: unit tests for Formation trait dispatch, CredentialStore key-value, CredentialDomain routing
- [x] Tests: verify all existing E2E and integration tests pass

## Steps

### 1. Formation Trait + Supporting Types

**Objective:** Define the Formation trait, CredentialStore trait, CredentialDomain, and supporting types.

**Implementation:** Create the trait definitions in `formation/mod.rs`. All method signatures per ADR-0008. The `CredentialStore` trait is key-value: `store(key, value)`, `retrieve(key)`, `remove(key)`, `list_keys(prefix)`.

**Tests:** Compile-time verification — trait is object-safe (`Box<dyn Formation>` works).

### 2. Module Structure

**Objective:** Create the `formation/local/` directory module with platform detection.

**Implementation:**
```
formation/
  mod.rs              # Formation trait, CredentialStore, CredentialDomain, resolve, create
  config.rs           # Existing FormationConfig (unchanged)
  local/
    mod.rs            # Platform detection → LinuxLocalFormation or MacosLocalFormation
    process.rs        # Shared POSIX process lifecycle (extract from launch.rs)
    topology.rs       # Extract from local_topology.rs
    daemon.rs         # Daemon management (placeholder — Sprint 2)
    linux/
      mod.rs          # LinuxLocalFormation impl
      credential.rs   # LocalCredentialStore (key-value, from bridge/credential.rs)
      setup.rs        # Prerequisite verification
    macos/
      mod.rs          # MacosLocalFormation stub
```

Existing files (`launch.rs`, `start_members.rs`, `stop_members.rs`, `init.rs`, `manager.rs`, `local_topology.rs`) stay in place initially. `LinuxLocalFormation` delegates to them.

**Tests:** `formation::create("local")` returns `LinuxLocalFormation` on Linux, error on macOS.

### 3. LinuxLocalFormation — Delegation Layer

**Objective:** Implement `LinuxLocalFormation` by delegating all trait methods to existing free functions.

**Implementation:** Each trait method calls the corresponding existing function. No logic moves yet — this is a thin wrapper. `start_members()` calls `start_local_members()`. `stop_members()` calls `stop_local_members()`. `credential_store()` returns a `LocalCredentialStore`. `setup_token_delivery()` and `refresh_token()` are no-ops in this sprint (implemented in Sprint 3).

**Tests:** Integration test — `LinuxLocalFormation` implements `Formation`, can be used as `Box<dyn Formation>`.

### 4. Key-Value CredentialStore

**Objective:** Move `LocalCredentialStore` from `bridge/credential.rs`, generalize to key-value.

**Implementation:**
- Copy keyring operations (`dss_store`, `dss_retrieve`, `dss_delete`, `with_keyring_dbus`, etc.) to `formation/local/linux/credential.rs`
- New `LocalCredentialStore` implements key-value `CredentialStore` trait
- Bridge module's `LocalCredentialStore` becomes a thin wrapper that calls the formation's credential store for keyring ops and manages bridge-state.json itself
- `list_keys(prefix)` implementation: use `dbus-secret-service::Collection::get_all_items()` and filter by attribute prefix client-side. The `dss_retrieve` function already uses `search_items(attrs)` — same pattern, broader query. Keyring doesn't support native prefix enumeration, so client-side filtering is the approach.
- `InMemoryCredentialStore` updated to key-value interface (for testing)
- `resolve_credential_from_store()` updated for new interface

**Tests:** All existing credential store tests pass. New tests for key-value operations with prefixed keys.

### 5. Team Struct

**Objective:** Create Team as the API boundary that holds a formation internally.

**Implementation:** Team struct wraps `TeamEntry` + `Box<dyn Formation>`. Provides methods: `start()`, `stop()`, `status()`, `hire()`, `fire()`, `chat()`, `attach()`. Initially these delegate through the formation to existing code.

Note: `team.hire()` delegates to `hire_member()` in `profile/member.rs`. That function now includes `render_member_placeholders()` (added on main) which renders `{{member_dir}}`, `{{role}}`, and `{{member_name}}` placeholders in profile templates. The Team's `hire()` method must preserve this call chain — the placeholder rendering step is profile-level and happens before any credential storage.

**Tests:** Unit test — Team wraps formation, delegates correctly.

### 6. Command Migration (Partial)

**Objective:** Commands that go through Team instead of calling formation free functions directly.

**Implementation:** Update command handlers to resolve Team and call team methods. The team methods delegate to formation which delegates to existing code. Same behavior, different call path.

Note: `bm start/stop` still call existing code paths through the delegation chain. Full daemon-mediated migration is Sprint 2.

**Tests:** All existing E2E and integration tests pass — behavior unchanged.

## Deviations from Design

| Deviation | Rationale | Resolved in |
|-----------|-----------|-------------|
| `gh_token` parameters NOT removed from `git/github.rs` | Daemon needs stored token during Sprint 2; removing signatures is part of the auth swap | Sprint 3 |
| `setup_token_delivery()` and `refresh_token()` are no-ops | Token delivery is a Sprint 3 concern | Sprint 3 |
| `start_members()` delegates to existing `start_local_members()` | Daemon-mediated launch is Sprint 2 | Sprint 2 |
| `--formation` CLI flag preserved | Selects deployment strategy; not leaking internals | Future cleanup |
