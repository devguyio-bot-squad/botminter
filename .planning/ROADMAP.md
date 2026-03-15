# Roadmap: BotMinter

## Milestones

- Shipped: **v0.06 Minty and Friends** -- Phases 1-6 (shipped 2026-03-08)
- Active: **v0.07 Team Bridge** -- Phases 7-10 (in progress)

## Phases

<details>
<summary>v0.06 Minty and Friends (Phases 1-6) -- SHIPPED 2026-03-08</summary>

- [x] Phase 1: Coding-Agent-Agnostic Completion (10/10 plans) -- completed 2026-03-06
- [x] Phase 2: Profile Externalization Completion (1/1 plan) -- completed 2026-03-04
- [x] Phase 3: Workspace Repository Completion (1/1 plan) -- completed 2026-03-04
- [x] Phase 4: Skills Extraction (1/1 plan) -- completed 2026-03-04
- [x] Phase 5: Team Manager + Chat (4/4 plans) -- completed 2026-03-07
- [x] Phase 6: Minty (2/2 plans) -- completed 2026-03-08

Full details: `.planning/milestones/v0.06-ROADMAP.md`

</details>

### v0.07 Team Bridge (In Progress)

**Milestone Goal:** Decouple communication into a pluggable bridge abstraction, ship a Rocket.Chat reference implementation, migrate Telegram into the same abstraction, and establish ADRs with Knative-style specs for extensible interfaces.

**Prerequisite:** Ralph Orchestrator's robot backend is already pluggable (RobotConfig supports backend selection, robot service factory dispatches by config).

- [ ] **Phase 7: Specs Foundation & Bridge Contract** - Establish ADR practice, create specs directory, and define the bridge plugin contract with spec and schema
- [x] **Phase 8: Bridge Abstraction, CLI & Telegram** - Build the Rust bridge module, all `bm bridge` CLI commands, Telegram migration, and `bm start/stop/status` integration — validated end-to-end with Telegram as the first real bridge (completed 2026-03-08)
- [ ] **Phase 9: Profile Integration & Cleanup** - Connect bridge to profiles, init wizard, teams sync provisioning, and verify full cycle end-to-end with Telegram
- [ ] **Phase 10: Rocket.Chat Bridge** - Ship the reference bridge implementation proving the full abstraction with Podman-based Rocket.Chat

## Phase Details

### Phase 7: Specs Foundation & Bridge Contract
**Goal**: The bridge plugin contract is formally specified and any developer can read the spec to build a conformant bridge implementation
**Depends on**: Nothing (first phase of v0.07)
**Requirements**: SPEC-01, SPEC-02, SPEC-03, SPEC-04, SPEC-05, BRDG-01, BRDG-02, BRDG-03, BRDG-04, BRDG-07
**Success Criteria** (what must be TRUE):
  1. `.planning/adrs/` directory exists with MADR 4.0.0 template and at least two ADRs (bridge abstraction and Ralph robot backend)
  2. `.planning/specs/bridge/` contains a bridge spec document using RFC 2119 language that defines `bridge.yml` format, `schema.json` contract, lifecycle operations, identity operations, and file-based config exchange
  3. A minimal conformance test suite exists that can validate whether a bridge implementation satisfies the spec
  4. Prior `specs/` contents (master-plan, milestones, prompts, tasks) are removed from the tree (preserved in git history)
  5. The spec clearly distinguishes local bridges (full lifecycle) from external bridges (identity-only) and documents both contract shapes
**Plans**: 3 plans

Plans:
- [ ] 07-01-PLAN.md -- ADR practice, specs practice, legacy cleanup
- [ ] 07-02-PLAN.md -- Bridge spec document and reference examples
- [ ] 07-03-PLAN.md -- Stub bridge and conformance tests

### Phase 8: Bridge Abstraction, CLI & Telegram
**Goal**: Operators can manage bridge services and identities through `bm bridge` commands, Telegram is wrapped as the first real bridge implementation validating the abstraction end-to-end, and bridge lifecycle is wired into `bm start/stop/status`
**Depends on**: Phase 7
**Requirements**: BRDG-05, BRDG-06, BRDG-08, BRDG-09, CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06, CLI-07, CLI-08, CLI-09, CLI-10, CLI-11, TELE-01, TELE-02
**Success Criteria** (what must be TRUE):
  1. `bm bridge start` invokes the bridge lifecycle start command, runs health check, and persists state; `bm bridge stop` tears it down
  2. `bm bridge status` displays service health, URL, uptime, and registered identities
  3. `bm bridge identity add/rotate/remove/list` manages bridge users through the bridge identity commands
  4. `bm bridge room create/list` manages rooms/channels on the bridge
  5. Bridge state (service URLs, container IDs, per-user credentials) persists across CLI sessions and a team with no bridge configured operates normally
  6. Telegram bridge exists as an external-type bridge with `bridge.yml` implementing identity-only commands (no start/stop lifecycle)
  7. `bm start` supports `--no-bridge` and `--bridge-only` flags, with default behavior controlled by `bridge.auto_start` config
  8. `bm status` team view shows member bridge identity mapping alongside agent status
  9. Telegram bridge ships as a built-in bridge in supported profiles
**Plans**: 5 plans

Plans:
- [x] 08-01-PLAN.md -- Core bridge module (types, state, discovery, invocation) + stub fixture extension + CLI enums
- [x] 08-02-PLAN.md -- Bridge CLI command handlers (start/stop/status, identity CRUD, room CRUD) + integration tests
- [x] 08-03-PLAN.md -- Telegram bridge implementation for scrum-compact and scrum profiles
- [x] 08-04-PLAN.md -- Wire bridge into bm start/stop/status with --no-bridge and --bridge-only flags
- [ ] 08-05-PLAN.md -- E2E tests for Telegram bridge abstraction with tg-mock (gap closure)

### Phase 9: Profile Integration & Cleanup
**Goal**: Bridge selection and provisioning are fully integrated into the profile system, init wizard, and teams sync workflow — full cycle verified end-to-end with Telegram bridge
**Depends on**: Phase 8
**Requirements**: PROF-01, PROF-02, PROF-03, PROF-04, PROF-05, PROF-06
**Success Criteria** (what must be TRUE):
  1. Profiles declare supported bridges in a `bridges/` directory and operators select one (or none) during `bm init`
  2. `bm init` wizard offers bridge selection from the profile's supported bridges, including a "No bridge" option
  3. `bm teams sync` provisions bridge resources (rooms, member identities) and generates `ralph.yml` RObot section based on active bridge config and member credentials
  4. `scrum-compact-telegram` profile is removed; Telegram is a supported bridge on `scrum-compact` instead
  5. Documentation covers bridge abstraction, CLI commands, bridge spec, and profile bridge configuration
**Plans**: TBD

Plans:
- [ ] 09-01: TBD
- [ ] 09-02: TBD
- [ ] 09-03: TBD

### Phase 10: Rocket.Chat Bridge
**Goal**: A complete Rocket.Chat bridge ships as the reference implementation, proving the bridge abstraction works end-to-end with full lifecycle management
**Depends on**: Phase 7 (contract), Phase 8 (CLI & Telegram), Phase 9 (full cycle verified)
**Requirements**: RC-01, RC-02, RC-03, RC-04, RC-05, RC-06, RC-07
**Success Criteria** (what must be TRUE):
  1. `bm bridge start` launches Rocket.Chat + MongoDB via Podman Pod and `bm bridge stop` tears it down cleanly
  2. `bm bridge identity add <name>` creates a Rocket.Chat user with bot role and returns credentials; `bm bridge identity list` shows all bot users
  3. A team channel is provisioned during `bm teams sync` if it does not already exist
  4. Bot commands (`/status`, `/tasks`) work in Rocket.Chat by reusing Ralph's command handler
  5. Operator identity is configured in the bridge's `schema.json` config
**Plans**: TBD

Plans:
- [ ] 10-01: TBD
- [ ] 10-02: TBD
- [ ] 10-03: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 7 -> 8 -> 9 -> 10

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Coding-Agent-Agnostic | v0.06 | 10/10 | Complete | 2026-03-06 |
| 2. Profile Externalization | v0.06 | 1/1 | Complete | 2026-03-04 |
| 3. Workspace Repository | v0.06 | 1/1 | Complete | 2026-03-04 |
| 4. Skills Extraction | v0.06 | 1/1 | Complete | 2026-03-04 |
| 5. Team Manager + Chat | v0.06 | 4/4 | Complete | 2026-03-07 |
| 6. Minty | v0.06 | 2/2 | Complete | 2026-03-08 |
| 7. Specs Foundation & Bridge Contract | v0.07 | 0/3 | Planning | - |
| 8. Bridge Abstraction, CLI & Telegram | v0.07 | 4/5 | Gap closure | 2026-03-08 |
| 9. Profile Integration & Cleanup | v0.07 | 0/3 | Not started | - |
| 10. Rocket.Chat Bridge | v0.07 | 0/3 | Not started | - |

---
*Roadmap updated: 2026-03-08 -- Phase 8 gap closure plan added (08-05, E2E Telegram bridge tests)*
