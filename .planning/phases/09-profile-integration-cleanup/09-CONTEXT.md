# Phase 9: Profile Integration & Cleanup - Context

**Gathered:** 2026-03-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Connect bridge selection and provisioning into the profile system, init wizard, and teams sync workflow. Full operator journey (`bm init` -> `bm hire` -> `bm teams sync` -> `bm start`) includes bridge configuration end-to-end, verified with Telegram as the external bridge. Remove the separate `scrum-compact-telegram` profile. Documentation covers bridge abstraction and profile bridge configuration.

</domain>

<decisions>
## Implementation Decisions

### Bridge Model Refinement
- Two bridge categories with distinct provisioning flows:
  - **Managed bridges** (e.g., Rocket.Chat future) -- full lifecycle (start/stop/health) AND auto-provisioning of per-member identities. Operator supplies no tokens; the bridge creates them.
  - **External bridges** (e.g., Telegram) -- no lifecycle management. Operator supplies per-member tokens. Bridge does not create users -- it accepts operator-provided credentials and hands them to the formation for storage.
- Per-member identity for both types -- each hired member gets their own bot user/token on the bridge

### Token & Credential Flow
- External bridge tokens collected during `bm hire` (prompted, but optional -- hire succeeds without them)
- Members without bridge credentials are flagged; operator can add later via `bm bridge identity add`
- `bm teams sync` skips ralph.yml RObot section generation for members missing credentials (no broken config)
- **Formation-aware secret storage:** design the abstraction in Phase 9, implement the local keyring backend. K8s secret backend comes with K8s formation. This means credentials route through the formation layer for storage rather than living only in bridge-state.json.

### Init Wizard Bridge Selection
- Bridge selection happens immediately after profile selection in the wizard flow
- Wizard lists bridges from the profile's supported bridges plus a "No bridge" option
- Init only records the bridge selection in team config -- does not start the bridge
- `bm init --non-interactive` accepts `--bridge <name>` flag; omitting it means no bridge
- `--bridge` is optional (not required) for non-interactive mode

### Teams Sync Flag Redesign
- Current `--push` flag replaced with granular, composable flags:
  - `bm teams sync` -- local workspace assembly only (current default behavior)
  - `bm teams sync --repos` -- also push/sync git repositories (replaces current `--push`)
  - `bm teams sync --bridge` -- also provision bridge identities and rooms on the bridge
  - `bm teams sync --all` / `-a` -- equivalent to `--repos --bridge` (all remote operations)
- Bridge provisioning during `--bridge` is idempotent: check bridge state for existing identities, only onboard members not yet provisioned
- Bridge provisioning creates team room if missing (same idempotent pattern)

### Profile Bridge Discovery
- `botminter.yml` schema explicitly declares supported bridges (not just directory-based discovery)
- `bridges/` directory in profile contains bridge implementations (`bridge.yml`, `schema.json`, `Justfile`)
- Schema declaration is the source of truth; directory provides the implementation files

### scrum-compact-telegram Removal (PROF-06)
- Audit `scrum-compact-telegram` profile for unique content not present in `scrum-compact` before deleting
- Migrate any unique knowledge, invariants, or skills to `scrum-compact`
- Delete the profile directory and update all references (tests, docs, code)

### Documentation (PROF-04)
- Bridge documentation goes in the existing MkDocs site (`docs/content/`)
- Covers: bridge concepts, CLI command reference updates, bridge spec overview, profile bridge configuration guide

### Claude's Discretion
- Exact `botminter.yml` schema shape for the `bridges` declaration
- Internal organization of formation-aware secret storage abstraction
- MkDocs page structure and navigation for bridge docs
- How to handle the `--push` flag deprecation (remove immediately vs warn+redirect, given Alpha policy)
- Test organization for new sync flag behavior

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/bm/src/bridge.rs`: Full bridge module with `BridgeManifest`, `BridgeState`, `BridgeIdentitySpec` types and command invocation -- sync and init will call into this
- `crates/bm/src/commands/bridge.rs`: CLI handlers for `bm bridge` subcommands -- identity provisioning logic can be reused by sync
- `profiles/scrum-compact/bridges/telegram/`: Complete Telegram bridge implementation (bridge.yml + schema.json + Justfile) already in place from Phase 8
- `profiles/scrum/bridges/telegram/`: Same Telegram bridge in the scrum profile

### Established Patterns
- `commands/init.rs`: Interactive wizard with `dialoguer` for selection prompts -- bridge selection follows same UX
- `workspace.rs`: `sync_workspace()` handles file assembly, submodule setup, and git commits -- bridge provisioning extends this
- `commands/teams.rs`: `sync()` function with `--push` flag controlling remote operations -- being replaced with `--repos`/`--bridge`/`--all`
- `profile.rs`: Profile manifest parsing with `serde_yml` -- `botminter.yml` schema extension for bridges list
- `cli.rs`: Clap derive for flag definitions -- new sync flags follow existing patterns

### Integration Points
- `commands/init.rs`: Add bridge selection step after profile selection
- `cli.rs`: Add `--bridge` flag to init's non-interactive args; replace `--push` with `--repos`/`--bridge`/`--all` on sync
- `workspace.rs` or `commands/teams.rs`: Wire bridge provisioning into sync with `--bridge` flag
- `profile.rs`: Extend `ProfileManifest` to include bridges declaration
- `commands/hire.rs`: Add optional bridge token prompt for external bridges

</code_context>

<specifics>
## Specific Ideas

- Sync flags should be composable and self-documenting: `--repos` and `--bridge` describe what they sync, `--all` is the convenience shorthand
- The formation-aware secret storage abstraction is forward-looking: design the trait/interface now so when K8s formation lands, the storage backend plugs in without restructuring
- Per-member tokens for external bridges (like Telegram) means one Telegram bot per member -- the operator creates N bots and supplies N tokens

</specifics>

<deferred>
## Deferred Ideas

- K8s formation secret storage backend -- comes with K8s formation implementation
- Encrypted credentials at rest -- future milestone
- Multi-bridge support (running multiple bridges simultaneously) -- BRDG-F02

</deferred>

---

*Phase: 09-profile-integration-cleanup*
*Context gathered: 2026-03-08*
