---
gsd_state_version: 1.0
milestone: v0.07
milestone_name: Team Bridge
status: executing
stopped_at: Phase 9 complete — bridge lifecycle e2e test added, teams show + identity add fixed
last_updated: "2026-03-09T06:30:00Z"
last_activity: 2026-03-09 -- Phase 9 complete with bridge lifecycle e2e test
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 14
  completed_plans: 14
  percent: 100
---

# Project State: BotMinter

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-08)

**Core value:** Operators can stand up and manage autonomous agentic teams that coordinate through GitHub
**Current focus:** Phase 10 -- Rocket.Chat Bridge

## Current Position

Phase: 10 of 10 (Rocket.Chat Bridge)
Plan: 0 of 3 in current phase
Status: Not started
Last activity: 2026-03-09 -- Phase 9 complete with bridge lifecycle e2e test

Progress: [███████░░░] 75%

## Current Milestone

**Version:** v0.07 -- Team Bridge
**Status:** Ready to plan

## Prior Milestones

| Version | Name | Status |
|---------|------|--------|
| v0.01 | Structure + Human-Assistant | complete |
| v0.02 | Autonomous Ralph (spike) | complete |
| v0.03 | Architect + First Epic | complete |
| v0.04 | GitHub Migration | complete |
| v0.05 | `bm` CLI | complete |
| v0.06 | Minty and Friends | shipped 2026-03-08 |

Artifacts: `.planning/milestones/`

## Performance Metrics

**Velocity:**
- Total plans completed: 1 (v0.07)
- Average duration: 2min
- Total execution time: 2min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 07 | 1 | 2min | 2min |
| Phase 07 P01 | 5min | 2 tasks | 8 files |
| Phase 07 P03 | 2min | 2 tasks | 4 files |
| Phase 08 P01 | 3min | 3 tasks | 9 files |
| Phase 08 P02 | 4min | 2 tasks | 4 files |
| Phase 08 P03 | 2min | 2 tasks | 7 files |
| Phase 08 P04 | 5min | 2 tasks | 6 files |
| Phase 09 P01 | 11min | 2 tasks | 24 files |
| Phase 09 P02 | 12min | 2 tasks | 16 files |
| Phase 09 P03 | 10min | 2 tasks | 5 files |
| Phase 09 P04 | 3min | 2 tasks | 6 files |
| Phase 09 P05 | 2min | 2 tasks | 6 files |
| Phase 09 P06 | 2min | 2 tasks | 2 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.

- Bridge spec uses Knative-style resource format (apiVersion/kind/metadata/spec)
- Config exchange output shapes defined per command category with required fields
- [Phase 07]: MADR 4.0.0 adopted for ADR practice with 4-digit numbering in .planning/adrs/
- [Phase 07]: Shell script bridge with YAML manifest chosen over Rust traits, gRPC, REST
- [Phase 07]: Bridge outputs credentials via file-based exchange, BotMinter maps to ralph.yml
- [Phase 07]: Conformance tests use serde_yml::Value/serde_json::Value for generic field access, separate from integration tests
- [Phase 08]: Phases 8+9 merged — Telegram validates bridge abstraction in same phase (walking skeleton)
- [Phase 08]: Phases renumbered: Profile Integration → Phase 9, Rocket.Chat → Phase 10
- [Phase 08-01]: Bridge state at {workzone}/{team}/bridge-state.json with atomic write + 0600 perms
- [Phase 08-01]: Config exchange uses system tempdir per invocation with BM_BRIDGE_TOKEN_{USERNAME} env var priority
- [Phase 08-02]: Bridge start invokes start recipe once and extracts service_url from config exchange
- [Phase 08-02]: Room list prefers live data from recipe over persisted state
- [Phase 08-03]: Telegram bridge uses eval for dynamic env var resolution (portable across shell versions)
- [Phase 08-03]: Profile bridge directory pattern: profiles/{profile}/bridges/{bridge}/ with bridge.yml + schema.json + Justfile
- [Phase 08-04]: Bridge auto-start runs before member launch; auto-stop runs after member stop
- [Phase 08-04]: bm stop always attempts bridge stop even when no members are running
- [Phase 09-01]: CredentialStore trait with keyring backend for formation-aware secret storage
- [Phase 09-01]: Keyring operations best-effort with env var fallback (BM_BRIDGE_TOKEN_{NAME})
- [Phase 09-01]: BridgeIdentity.token optional with skip_serializing for backward compat
- [Phase 09-01]: --push replaced by --repos/--bridge/--all (Alpha policy, no deprecation)
- [Phase 09-03]: provision_bridge() in bridge.rs for managed/external identity provisioning
- [Phase 09-03]: inject_robot_enabled() runs on ALL syncs when bridge configured
- [Phase 09-03]: Per-member credential resolution replaces team-wide telegram_bot_token
- [Phase 09-03]: Formation manager retains legacy token fallback (separate path)
- [Phase 09-02]: Init wizard bridge selection via cliclack::select with "No bridge" option
- [Phase 09-02]: Bridge recorded in team botminter.yml before initial commit
- [Phase 09-02]: Hire token prompt only for external bridges in interactive mode
- [Phase 09-02]: scrum-compact-telegram deleted per Alpha policy (no migration)
- [Phase 09]: Bridge spec link in concepts page references .planning/specs/ directly
- [Phase 09]: Full bm bridge commands section added to CLI reference
- [Phase 09-06]: eprintln! used for workspace repo info messages (log crate not a dependency)
- [Phase 09-05]: Bridge completion names collected from all profiles for init --bridge tab completion
- [Phase 09-06]: ProfileManifest.bridge: Option<String> preserves selected bridge during round-trip serialization
- [Phase 09-06]: gh repo view pre-existence check before gh repo create (idempotency)
- [Phase 09]: bm teams show displays bridge configuration (name + type) when bridge configured
- [Phase 09]: bm bridge identity add prompts for token interactively on external bridges (mirrors hire.rs pattern)
- [Phase 09]: CLI idempotency invariant codified in invariants/cli-idempotency.md
- [Phase 09]: E2e bridge lifecycle test covers scrum-compact + telegram happy path (init → hire → show → identity add → list → sync)

### Pending Todos

- Improve local formation keyring UX (detect collection state, system-aware errors, docs)

### Blockers/Concerns

- Ralph robot abstraction assumed as prerequisite (not in this milestone's scope)

## Test Health

- 576 non-e2e tests passing (371 unit + 66 cli_parsing + 114 integration + 12 conformance + 10 bridge_sync + 3 profile_roundtrip)
- 20 e2e tests passing (including bridge lifecycle happy path)
- `cargo test -p bm` -- all green
- `cargo clippy -p bm -- -D warnings` -- clean

## Session Continuity

Last session: 2026-03-09T06:30:00Z
Stopped at: Phase 9 complete — bridge lifecycle e2e test added, teams show + identity add fixed
Resume file: None

---
*Last updated: 2026-03-09 -- Phase 9 complete with bridge lifecycle e2e test*
