---
phase: 09-profile-integration-cleanup
verified: 2026-03-09T12:30:00Z
status: passed
score: 13/13 must-haves verified
re_verification:
  previous_status: passed
  previous_score: 13/13
  gaps_closed: []
  gaps_remaining: []
  regressions: []
gaps: []
---

# Phase 9: Profile Integration & Cleanup Verification Report

**Phase Goal:** Bridge selection and provisioning are fully integrated into the profile system, init wizard, and teams sync workflow -- full cycle verified end-to-end with Telegram bridge
**Verified:** 2026-03-09T12:30:00Z
**Status:** passed
**Re-verification:** Yes -- regression check confirms all 13 truths still hold

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | ProfileManifest deserializes profiles with a bridges field | VERIFIED | `profile.rs:221` has `pub bridges: Vec<BridgeDef>` with `serde(default)`. Both profile YAMLs declare bridges at line 17. |
| 2 | CredentialStore trait defines store/retrieve/remove/list operations | VERIFIED | `bridge.rs:18` defines trait with all 4 methods |
| 3 | LocalCredentialStore stores/retrieves per-member tokens from system keyring | VERIFIED | `bridge.rs:79-84` impl via keyring crate |
| 4 | InMemoryCredentialStore exists for testing | VERIFIED | `bridge.rs:26-44` impl with Mutex HashMap backend |
| 5 | Bridge-state.json stores identity metadata but NOT tokens | VERIFIED | `BridgeIdentity.token` field uses `serde(skip_serializing)` |
| 6 | CLI accepts --repos, --bridge, --all flags on teams sync | VERIFIED | `cli.rs:255-263` defines all three flags |
| 7 | CLI accepts --bridge flag on bm init --non-interactive | VERIFIED | `cli.rs:41` has `bridge: Option<String>` on Init command |
| 8 | Init wizard presents bridge selection from profile | VERIFIED | `init.rs:106-108` validates non-interactive; `init.rs:359` interactive cliclack::select with "No bridge" option |
| 9 | Hire prompts for optional bridge token (external bridges) | VERIFIED | `hire.rs:83-134` discovers bridge, prompts via cliclack::input, stores via CredentialStore |
| 10 | bm teams sync --bridge provisions identities via provision_bridge() | VERIFIED | `bridge.rs:469` `provision_bridge()` called from `teams.rs:250-268` when bridge_flag set |
| 11 | ralph.yml RObot.enabled injection based on credential availability | VERIFIED | `workspace.rs:607` `inject_robot_enabled()` sets `RObot.enabled` via serde_yml::Value mutation |
| 12 | bm start resolves per-member credentials from CredentialStore | VERIFIED | `start.rs:117` creates LocalCredentialStore, `start.rs:188` calls resolve_credential_from_store per member |
| 13 | scrum-compact-telegram profile deleted with no remaining references | VERIFIED | Directory does not exist |

**Score:** 13/13 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/bm/src/bridge.rs` | CredentialStore trait + LocalCredentialStore + provision_bridge | VERIFIED | 1112 lines, trait at L18, LocalCredentialStore at L79, provision_bridge at L469 |
| `crates/bm/src/profile.rs` | BridgeDef struct and bridges field on ProfileManifest | VERIFIED | bridges field at L221 with deserialization tests at L2120+ |
| `crates/bm/src/cli.rs` | Sync --repos/--bridge/--all flags, Init --bridge flag | VERIFIED | Sync flags L255-263, Init bridge L41 |
| `crates/bm/src/commands/init.rs` | Bridge selection wizard step + non-interactive support | VERIFIED | Interactive L359, non-interactive L106-108, validate_bridge_selection present |
| `crates/bm/src/commands/hire.rs` | Optional bridge token prompt during hire | VERIFIED | Bridge discover + cliclack prompt + CredentialStore.store at L83-134 |
| `crates/bm/src/commands/teams.rs` | Bridge provisioning during sync --bridge | VERIFIED | Calls provision_bridge via bridge:: module when bridge_flag is true |
| `crates/bm/src/workspace.rs` | ralph.yml RObot.enabled injection | VERIFIED | inject_robot_enabled at L607 |
| `crates/bm/src/commands/start.rs` | Per-member credential resolution and env var injection | VERIFIED | LocalCredentialStore setup L117, per-member resolve L188 |
| `profiles/scrum-compact/botminter.yml` | Telegram bridge declaration | VERIFIED | bridges: at line 17 |
| `profiles/scrum/botminter.yml` | Telegram bridge declaration | VERIFIED | bridges: at line 17 |
| `docs/content/concepts/bridges.md` | Bridge concepts documentation | VERIFIED | 124 lines |
| `docs/content/how-to/bridge-setup.md` | Bridge setup how-to guide | VERIFIED | 114 lines |
| `docs/mkdocs.yml` | Nav entries for bridge docs | VERIFIED | Bridges under Concepts (L21), Bridge Setup under How-To (L27) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| bridge.rs | system keyring | LocalCredentialStore impl | WIRED | keyring::Entry::new in store/retrieve/remove methods |
| profile.rs | profile YAMLs | serde deserialization | WIRED | `pub bridges: Vec<BridgeDef>` with serde(default) |
| commands/init.rs | profile.rs | reads manifest.bridges | WIRED | `manifest.bridges` used for validation and selection |
| commands/hire.rs | bridge.rs | stores token via CredentialStore | WIRED | `cred_store.store(&member_name, &token)` at L113 |
| commands/teams.rs | bridge.rs | provision_bridge + CredentialStore | WIRED | `bridge::provision_bridge()` call + LocalCredentialStore at L268 |
| workspace.rs | ralph.yml | serde_yml::Value mutation RObot.enabled | WIRED | inject_robot_enabled mutates YAML document |
| commands/start.rs | bridge.rs | CredentialStore.retrieve per member | WIRED | `bridge::resolve_credential_from_store(member_dir_name, store)` at L188 |
| docs/mkdocs.yml | bridge docs | nav entries | WIRED | Both concepts/bridges.md and how-to/bridge-setup.md in nav |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PROF-01 | 09-01 | Bridge config at team level with schema.json validation | SATISFIED | CredentialStore trait, LocalCredentialStore keyring backend, bridge-state.json identity metadata |
| PROF-02 | 09-01 | Profiles declare supported bridges in bridges/ directory | SATISFIED | ProfileManifest.bridges field, both profiles declare Telegram |
| PROF-03 | 09-03 | bm teams sync provisions bridge resources, generates RObot section | SATISFIED | provision_bridge() in bridge.rs, inject_robot_enabled() in workspace.rs |
| PROF-04 | 09-04 | Documentation updates for bridge abstraction and CLI | SATISFIED | concepts/bridges.md (124 lines), how-to/bridge-setup.md (114 lines), CLI ref updated |
| PROF-05 | 09-02 | bm init wizard offers bridge selection including "No bridge" | SATISFIED | Interactive cliclack::select in init.rs, --bridge flag for non-interactive |
| PROF-06 | 09-02 | scrum-compact-telegram profile removed, Telegram on scrum-compact | SATISFIED | Directory deleted, zero references |

No orphaned requirements -- all 6 PROF-0x IDs assigned to Phase 9 in REQUIREMENTS.md are covered by plans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| commands/start.rs | 112 | TODO: bridge-type-aware env var name | Info | Forward-looking design note for multi-bridge support |
| commands/start.rs | 465 | TODO: Formation manager credential resolution | Info | Forward-looking design note for K8s formation |

No blockers or warnings. Both TODOs are legitimate deferred work items for future milestones.

### Human Verification Required

#### 1. Interactive Init Bridge Selection

**Test:** Run `bm init` interactively, verify bridge selection appears after profile selection
**Expected:** cliclack::select shows Telegram and "No bridge" options
**Why human:** Interactive terminal UI behavior

#### 2. Interactive Hire Token Prompt

**Test:** Run `bm hire <role>` on a team with Telegram bridge configured
**Expected:** Prompts for optional bot token, accepts empty input gracefully
**Why human:** Interactive terminal UI behavior

#### 3. System Keyring Integration

**Test:** Run `bm hire` with a token on a machine with keyring available, verify `bm teams sync` and `bm start` resolve it
**Expected:** Token stored in keyring, retrieved at sync and start time
**Why human:** Requires system keyring (gnome-keyring/kwallet), cannot test in CI

#### 4. End-to-End Bridge Flow

**Test:** Full cycle: `bm init --bridge telegram` -> `bm hire` (with token) -> `bm teams sync --bridge` -> `bm start`
**Expected:** Ralph instances launch with per-member RALPH_TELEGRAM_BOT_TOKEN env var
**Why human:** Full integration requires running services

### Build Verification

All test targets compile successfully (verified 2026-03-09): unit tests, bridge_sync, cli_parsing, conformance, integration, profile_roundtrip.

### Notes

- daemon.rs intentionally passes `None` for telegram_token (documented decision in Plan 03 key-decisions). Members launched via daemon resolve their own tokens at runtime via env var.
- telegram_bot_token in config.rs Credentials is `skip_serializing` -- reads old configs but never writes, providing backward compatibility during Alpha.

---

_Verified: 2026-03-09T12:30:00Z_
_Verifier: Claude (gsd-verifier)_
