# Requirements: BotMinter v0.07 Team Bridge

**Defined:** 2026-03-08
**Core Value:** Profiles -- Git-backed convention packages with layered knowledge scoping, where agents coordinate through GitHub issues

## v0.07 Requirements

Requirements for milestone v0.07. Each maps to roadmap phases.

### ADRs & Specs

- [x] **SPEC-01**: ADR practice established with MADR 4.0.0 format in `.planning/adrs/` with template and numbering convention
- [x] **SPEC-02**: Bridge abstraction ADR documenting design decisions for the contract, lifecycle model, and config exchange
- [x] **SPEC-03**: Bridge spec document in `.planning/specs/bridge/` with RFC 2119 conformance language defining `bridge.yml` format, `schema.json` contract, lifecycle operations, identity operations, and I/O conventions
- [x] **SPEC-04**: Minimal conformance test suite that validates a bridge implementation against the spec
- [x] **SPEC-05**: `.planning/adrs/` and `.planning/specs/` directories created. Existing top-level `specs/` directory contents removed (preserved in git history).

### Bridge Abstraction

- [x] **BRDG-01**: Bridge definition file (`bridge.yml`) declares all integration points -- lifecycle commands, identity management commands, config schema reference, and bridge type (local vs external). Commands implemented as Justfile recipes. No hardcoded command names.
- [x] **BRDG-02**: Bridge config schema (`schema.json`) validates bridge-specific configuration values. BotMinter validates config against the schema before invoking commands.
- [x] **BRDG-03**: Bridge contract supports "external" bridges (like Telegram) that skip start/stop and only implement identity management
- [x] **BRDG-04**: Bridge identity management commands defined in `bridge.yml` (onboard, rotate-credentials, remove) for per-user bot lifecycle, implemented as Justfile recipes
- [x] **BRDG-05**: Bridge config model with bridge type resolution, state tracking, and per-user credentials
- [x] **BRDG-06**: Bridge state persisted across sessions tracking service URLs, container IDs, and registered user credentials
- [x] **BRDG-07**: Config exchange between bridge commands and BotMinter uses file-based output (`$BRIDGE_CONFIG_DIR/config.json`), not stdout
- [x] **BRDG-08**: Bridge is optional -- a team can operate without any bridge. All bridge-dependent features degrade gracefully.
- [x] **BRDG-09**: Bridge credentials resolved in priority order: env var `BM_BRIDGE_TOKEN_{USERNAME}` → system keyring (formation credential store). Credentials are NOT stored in config files or bridge-state.json.

### Bridge CLI

- [x] **CLI-01**: `bm bridge start` starts the bridge service -- runs lifecycle start command + health check, stores state. No team or member logic.
- [x] **CLI-02**: `bm bridge stop` stops the bridge service. Pure bridge lifecycle.
- [x] **CLI-03**: `bm bridge status` shows bridge service health, URL, uptime, and registered identities. Operational view, not team view.
- [x] **CLI-04**: `bm bridge identity add <username>` creates a user on the bridge. Bridge-native, not team-aware.
- [x] **CLI-05**: `bm bridge identity rotate <username>` rotates credentials for a bridge user.
- [x] **CLI-06**: `bm bridge identity list` lists all users registered on the bridge.
- [x] **CLI-07**: `bm bridge identity remove <username>` removes a user from the bridge.
- [x] **CLI-08**: `bm start` supports flexibility flags: `--no-bridge` (skip bridge), `--bridge-only` (bridge without members). Default behavior controlled by `bridge.auto_start` config.
- [x] **CLI-09**: `bm status` team view includes member bridge identity mapping alongside agent status.
- [x] **CLI-10**: `bm bridge room create <name>` creates a room/channel on the bridge.
- [x] **CLI-11**: `bm bridge room list` lists rooms on the bridge.

### Rocket.Chat Bridge

- [ ] **RC-01**: Rocket.Chat bridge ships `bridge.yml`, `schema.json`, and `Justfile` with all lifecycle and identity recipes
- [ ] **RC-02**: Rocket.Chat lifecycle recipes implementing the bridge contract (start via Podman Pod, stop, health check via REST API)
- [ ] **RC-03**: Podman Pod definition for Rocket.Chat + MongoDB (single-node replica set) with automated initialization
- [ ] **RC-04**: Per-agent bot identity -- onboard recipe creates a Rocket.Chat user with bot role via REST API, returns credentials
- [ ] **RC-05**: Team channel/room provisioned during `bm teams sync` if it doesn't exist, same pattern as GitHub repo provisioning
- [ ] **RC-06**: Bot commands (`/status`, `/tasks`, etc.) available through Rocket.Chat by reusing Ralph's command handler
- [ ] **RC-07**: Rocket.Chat bridge config (`schema.json`) includes operator identity — the human user ID for the team operator

### Telegram Bridge

- [x] **TELE-01**: Existing Telegram support wrapped as a bridge implementation with external type and identity-only commands in `bridge.yml`
- [x] **TELE-02**: Telegram bridge ships as a built-in bridge in supported profiles

### Profile & Config

- [x] **PROF-01**: Bridge config at team level -- bridge type and credentials validated against the bridge's `schema.json`
- [x] **PROF-02**: Profiles declare supported bridges in `bridges/` directory. Operator selects one (or none) during team setup. No separate profile per bridge.
- [x] **PROF-03**: `bm teams sync` provisions bridge resources (rooms, member identities) reusing the same bridge module as `bm bridge` commands. Generates `ralph.yml` `RObot` section based on active bridge config and member credentials.
- [x] **PROF-04**: Documentation updates for bridge abstraction, CLI commands, bridge spec, and profile bridge configuration
- [x] **PROF-05**: `bm init` wizard offers bridge selection from profile's supported bridges, including "No bridge"
- [x] **PROF-06**: `scrum-compact-telegram` profile removed. Telegram added as supported bridge on `scrum-compact`.

## Future Requirements

Deferred to future milestones. Tracked but not in current roadmap.

### Bridge Enhancements

- **BRDG-F01**: Bridge health monitoring with auto-recovery and restart backoff
- **BRDG-F02**: Multi-bridge support -- run multiple bridges simultaneously (e.g., Telegram for notifications, Rocket.Chat for agent channels)
- **BRDG-F03**: Agent-to-agent coordination through shared bridge channels
- **BRDG-F04**: Wake team from chat -- operator sends message on bridge to trigger agent work
- **BRDG-F05**: Optimize routing: when agents use bridge vs GitHub issues (profile convention work)
- **BRDG-F06**: GSD integration for ADRs and specs as part of planning workflow

### Additional Bridges

- **BRDG-F07**: Slack bridge implementation (requires paid workspace, OAuth)
- **BRDG-F08**: Discord bridge implementation
- **BRDG-F09**: Matrix bridge implementation

### Ralph Upstream

- **RLPH-F01**: Contribute robot abstraction changes upstream to Ralph Orchestrator

## Out of Scope

| Feature | Reason |
|---------|--------|
| Bidirectional chat commands (operator commands via chat) | Duplicates Ralph's `human.interact` event flow. Two command channels = confusion. |
| Rocket.Chat Apps-Engine integration | Deprecated bot SDK path, massive complexity. REST API is stable and sufficient. |
| Real-time WebSocket message streaming | Agents need to send messages, not receive in real-time. REST API sufficient for v0.07. |
| Upstream Ralph PR | Local fork changes only this milestone. Upstream contribution is future work. |
| Ralph robot abstraction (RLPH-01..05) | Assumed as prerequisite — Ralph robot backend is already pluggable before this milestone starts. |
| Custom Rocket.Chat app | Adds deployment artifact, TypeScript build, RC version coupling. External REST API is simpler. |
| Migration paths for existing teams | Alpha policy -- operators re-create from scratch. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SPEC-01 | Phase 7 | Complete |
| SPEC-02 | Phase 7 | Complete |
| SPEC-03 | Phase 7 | Complete |
| SPEC-04 | Phase 7 | Complete |
| SPEC-05 | Phase 7 | Complete |
| BRDG-01 | Phase 7 | Complete |
| BRDG-02 | Phase 7 | Complete |
| BRDG-03 | Phase 7 | Complete |
| BRDG-04 | Phase 7 | Complete |
| BRDG-05 | Phase 8 | Complete |
| BRDG-06 | Phase 8 | Complete |
| BRDG-07 | Phase 7 | Complete |
| BRDG-08 | Phase 8 | Complete |
| BRDG-09 | Phase 8 | Complete |
| CLI-01 | Phase 8 | Complete |
| CLI-02 | Phase 8 | Complete |
| CLI-03 | Phase 8 | Complete |
| CLI-04 | Phase 8 | Complete |
| CLI-05 | Phase 8 | Complete |
| CLI-06 | Phase 8 | Complete |
| CLI-07 | Phase 8 | Complete |
| CLI-08 | Phase 8 | Complete |
| CLI-09 | Phase 8 | Complete |
| CLI-10 | Phase 8 | Complete |
| CLI-11 | Phase 8 | Complete |
| RC-01 | Phase 10 | Pending |
| RC-02 | Phase 10 | Pending |
| RC-03 | Phase 10 | Pending |
| RC-04 | Phase 10 | Pending |
| RC-05 | Phase 10 | Pending |
| RC-06 | Phase 10 | Pending |
| RC-07 | Phase 10 | Pending |
| TELE-01 | Phase 8 | Complete |
| TELE-02 | Phase 8 | Complete |
| PROF-01 | Phase 9 | In Progress (09-01: CredentialStore + keyring backend) |
| PROF-02 | Phase 9 | In Progress (09-01: bridges field + YAML declarations) |
| PROF-03 | Phase 9 | Complete |
| PROF-04 | Phase 9 | Complete |
| PROF-05 | Phase 9 | Complete |
| PROF-06 | Phase 9 | Pending |

**Coverage:**
- v0.07 requirements: 34 total
- Mapped to phases: 34
- Unmapped: 0

---
*Requirements defined: 2026-03-08*
*Last updated: 2026-03-08 after Phase 8/9 merge -- renumbered 10→9, 11→10*
