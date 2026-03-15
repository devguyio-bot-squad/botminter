# Project Research Summary

**Project:** BotMinter v0.07 -- Team Bridge
**Domain:** Pluggable communication bridge abstraction for agentic team orchestration
**Researched:** 2026-03-08
**Confidence:** HIGH

## Executive Summary

BotMinter v0.07 introduces a pluggable communication bridge layer that sits between BotMinter's lifecycle management (`bm start/stop`) and Ralph Orchestrator's `RobotService` trait. The bridge enables teams to use local, self-hosted chat platforms (starting with Rocket.Chat) instead of or alongside Telegram, giving each agent its own bot identity. The architecture follows BotMinter's established "composable shell scripts" pattern: bridges are directories of lifecycle scripts (`start.sh`, `stop.sh`, `health.sh`, `configure.sh`) that output JSON config to stdout, managed by new `bm bridge` CLI commands.

The recommended approach is to define the bridge contract and spec first (ADRs + Knative-style spec), then build the Rocket.Chat reference implementation as a Docker Compose-based local service, and finally migrate existing Telegram support into the bridge abstraction. The Ralph Orchestrator upstream contribution (making `RobotConfig` backend-agnostic) should proceed in parallel but must not gate BotMinter progress -- develop against a local fork branch and upstream when ready. No new Rust crate dependencies are needed for BotMinter; the bridge scripts use `curl` against Rocket.Chat's REST API.

The primary risks are: (1) the upstream Ralph PR blocking the milestone if review stalls -- mitigated by developing against a local fork; (2) leaky abstractions where bridge scripts bleed into Ralph's message semantics or vice versa -- mitigated by a clear spec with explicit "what the bridge does NOT do" boundaries; and (3) the Rocket.Chat Docker setup being fragile due to MongoDB's replica set requirement -- mitigated by shipping a complete, tested `docker-compose.yml` with pinned versions and proper initialization. The estimated scope is approximately 2,100 lines of new Rust code in BotMinter plus 400 lines in the upstream Ralph PR.

## Key Findings

### Recommended Stack

No new Rust dependencies are needed for BotMinter. The bridge is implemented as shell scripts (invoked via `std::process::Command`, already used extensively) with `curl` for REST API calls. The only new operator requirement is Docker + Docker Compose v2 for running Rocket.Chat locally.

**Core technologies:**
- **Rocket.Chat (Docker):** Local self-hosted chat -- free community edition, REST API for bot user management, per-agent identity via separate user accounts
- **MongoDB 8.2 (single-node replica set):** Required by Rocket.Chat -- must be configured as replica set even for single node
- **MADR 4.0.0 format:** ADR template convention -- industry standard, no tooling dependency, files in `specs/adrs/`
- **Shell scripts + curl:** Bridge lifecycle and Rocket.Chat API integration -- matches existing skills pattern, no compilation needed for new backends

**Critical correction from research:** Ralph Orchestrator is a **Rust** project using **teloxide** for Telegram, not Node.js/TypeScript with grammy as previously stated in milestone context.

### Expected Features

**Must have (table stakes):**
- Bridge trait/contract definition (shell-script lifecycle with JSON stdout)
- `bm bridge start/stop/status` CLI commands
- Bridge auto-start from `bm start` (configurable)
- Rocket.Chat bridge implementation (reference impl)
- Per-agent bot identity on Rocket.Chat
- Telegram bridge migration (wrap existing support)
- Ralph robot abstraction upstream PR (make `RobotConfig` pluggable)
- ADR practice establishment and bridge spec (Knative-style)

**Should have (differentiators):**
- Bridge health monitoring with auto-recovery
- Agent-to-agent visibility in shared chat channels
- Bridge configuration in profile (profile-level defaults)
- Docker Compose auto-provisioning from `bm bridge start`
- Multi-bridge support (e.g., Telegram + Rocket.Chat simultaneously)

**Defer (v2+):**
- Spec conformance tests (need 2+ bridge implementations first)
- Matrix/Mattermost/Slack bridge implementations
- Chat-based command interface (slash commands)
- Real-time WebSocket/DDP message streaming
- Bidirectional message bridging (operator chat to agent action)

### Architecture Approach

The architecture has three distinct integration surfaces: (1) BotMinter bridge abstraction -- shell scripts in profile `bridges/` directories managed by a new `bridge.rs` module and `bm bridge` CLI commands; (2) Ralph robot abstraction -- upstream changes adding a `backend` field to `RobotConfig` and a new `ralph-rocketchat` crate implementing `RobotService`; (3) config plumbing -- bridge credentials stored in bridge state, injected as environment variables into Ralph at launch time.

**Major components:**
1. **`bridge.rs` + `commands/bridge.rs`** -- Bridge lifecycle management and CLI surface (start, stop, status, health checks)
2. **Profile `bridges/` directory** -- Per-backend shell scripts (`start.sh`, `stop.sh`, `health.sh`, `configure.sh`) and `bridge.yml` manifest
3. **Bridge state management** -- Runtime state in `~/.botminter/` tracking bridge URLs, container IDs, and per-member credentials
4. **Ralph `RobotConfig` extension** -- Backend dispatch field with `telegram` default, plus new `ralph-rocketchat` crate (upstream)
5. **Workspace injection** -- Modified `bm start` passes bridge credentials as env vars to Ralph instances

**Key patterns to follow:**
- Shell scripts are pure functions: env vars in, JSON stdout out, diagnostics to stderr
- Bridge lifecycle is independent from member lifecycle (`bm bridge start/stop` are separate commands)
- Per-agent identity via `configure.sh` called once per hired member
- Backend dispatch in Ralph via single `create_robot_service()` factory function

### Critical Pitfalls

1. **Leaky abstraction between bridge and Ralph robot** -- The bridge manages service lifecycle and connection config; Ralph's robot manages message semantics. If bridge scripts reference event types or Ralph's robot calls Docker, the abstraction is broken. Prevention: write the bridge spec first with an explicit "does NOT do" section.

2. **Upstream Ralph PR blocks milestone** -- BotMinter does not own Ralph Orchestrator. If the PR stalls, the milestone stalls. Prevention: develop against a local fork branch from day one, keep the PR minimal (config-driven backend selection only), start the upstream conversation early with an issue before code.

3. **stdout config corruption from shell diagnostic output** -- Docker pull progress, `set -x` traces, or `echo` debug output mixed into stdout breaks JSON parsing. Prevention: enforce "all diagnostics to stderr" in the bridge spec; validate JSON output in `bm`; consider file-based config exchange as a more robust alternative.

4. **MongoDB replica set requirement** -- Rocket.Chat requires MongoDB as a replica set, not standalone. Without proper initialization (`rs.initiate()`), Rocket.Chat enters a crash loop with cryptic errors. Prevention: ship a complete `docker-compose.yml` with init script, pin versions, health check against API not just container status.

5. **N bot users require admin API access** -- Creating per-agent bot users on Rocket.Chat requires admin credentials. Prevention: validate admin creds in `configure.sh` upfront, generate deterministic emails (`{name}@botminter.local`), provide fallback single-bot mode.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: ADRs, Specs, and Bridge Contract
**Rationale:** Zero code dependencies. Establishes the contract that all subsequent work builds against. ADRs document rationale for bridge design decisions before implementation commits to a direction. The bridge spec defines conformance criteria.
**Delivers:** `specs/adrs/0001-bridge-abstraction.md`, `specs/adrs/0002-ralph-robot-backend.md`, `specs/bridge-spec.md` (Knative-style), bridge contract definition (shell script lifecycle + JSON stdout protocol)
**Addresses:** ADR practice establishment, bridge spec, bridge contract definition
**Avoids:** ADR scope creep (Pitfall 11) -- time-box to 2-3 days

### Phase 2: Bridge CLI and Abstraction Layer
**Rationale:** The Rust-side bridge abstraction can be built and tested with a stub/no-op bridge before any real backend exists. This validates the CLI UX and the lifecycle management code independently.
**Delivers:** `bridge.rs` module, `bm bridge start/stop/status` commands, profile `bridges/` directory support, bridge state management
**Addresses:** `bm bridge start/stop/status` CLI, bridge state management
**Avoids:** Coupling bridge lifecycle to member lifecycle (Anti-pattern 5)

### Phase 3: Rocket.Chat Bridge Implementation
**Rationale:** The reference implementation proves the abstraction works. Must follow Phase 2 (needs the bridge contract to implement against). Highest complexity phase -- REST API integration, Docker Compose, user provisioning.
**Delivers:** Rocket.Chat bridge scripts, Docker Compose file, per-agent bot identity, `configure.sh` for N bot users
**Uses:** Docker + Docker Compose, Rocket.Chat REST API (curl), MongoDB replica set
**Addresses:** Rocket.Chat bridge implementation, per-agent bot identity, Docker Compose provisioning
**Avoids:** MongoDB replica set breakage (Pitfall 5), admin API gotcha (Pitfall 2), stdout corruption (Pitfall 3), port conflicts (Pitfall 12)

### Phase 4: Ralph Robot Abstraction (Upstream)
**Rationale:** Can proceed in parallel with Phases 2-3 but listed as Phase 4 because BotMinter should not be gated on it. Work against a local fork. The upstream PR adds `backend` field to `RobotConfig` and creates `ralph-rocketchat` crate.
**Delivers:** Upstream PR to Ralph Orchestrator (config-driven backend selection), `ralph-rocketchat` crate implementing `RobotService`
**Addresses:** Ralph robot abstraction, pluggable backend selection
**Avoids:** Upstream PR scope creep (Pitfall 4), leaky abstraction (Pitfall 1)

### Phase 5: Integration and Telegram Migration
**Rationale:** Depends on Phases 2-4. Wires bridge credentials into `bm start` flow, adds auto-start support, wraps existing Telegram support into the bridge abstraction. This is where the full system comes together.
**Delivers:** Bridge auto-start from `bm start`, credential injection into Ralph env vars, Telegram bridge scripts (wrapping existing support), updated profiles
**Addresses:** Bridge auto-start, Telegram bridge migration, end-to-end integration
**Avoids:** Service ordering issues (Pitfall 9), breaking existing Telegram profiles (Phase warning)

### Phase Ordering Rationale

- **Spec-first (Phase 1)** because the bridge contract is the foundation -- getting it wrong means rework in every subsequent phase. The existing codebase follows a specs-first workflow.
- **CLI before implementation (Phase 2 before 3)** because the Rust abstraction layer can be validated with stubs. Building the Rocket.Chat bridge first would risk embedding backend-specific logic into the CLI layer.
- **Rocket.Chat before Ralph upstream (Phase 3 before 4)** because the reference implementation exercises the bridge contract from BotMinter's side. The Ralph upstream work is high-risk (external dependency) and should not gate progress. Developing against a fork is explicitly recommended.
- **Integration last (Phase 5)** because it depends on all preceding phases and is where the full system is validated end-to-end. Telegram migration is low-risk (wrapping existing code) and belongs here.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (Rocket.Chat Bridge):** Complex Docker + REST API integration. Needs validation of Rocket.Chat version compatibility, MongoDB replica set initialization sequence, and bot user creation flow. The API docs are available but scattered across deprecated and current references.
- **Phase 4 (Ralph Upstream):** Requires understanding Ralph's event loop internals deeply. The `create_robot_service()` dispatch pattern needs careful design to preserve backwards compatibility. Upstream maintainer buy-in is uncertain.

Phases with standard patterns (skip research-phase):
- **Phase 1 (ADRs + Specs):** Well-documented conventions (MADR 4.0.0, Knative specs). No unknowns.
- **Phase 2 (Bridge CLI):** Mirrors existing `bm start/stop/status` patterns. Standard Rust CLI work with clap + serde.
- **Phase 5 (Integration):** Wiring and config plumbing. Follows established BotMinter patterns for workspace provisioning and env var injection.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified from source code (Ralph Cargo.toml, BotMinter Cargo.toml) and official docs. No new Rust deps needed. Critical correction on Ralph being Rust, not Node.js. |
| Features | HIGH | Feature landscape well-mapped. Dependency graph is clear. MVP phasing is grounded in technical dependencies. |
| Architecture | HIGH | Component boundaries verified against existing codebase. Shell-script bridge pattern consistent with existing skill system. Ralph's RobotService trait verified from source. |
| Pitfalls | HIGH | Top pitfalls verified from official docs and source code. MongoDB replica set requirement, admin API for bot users, and stdout corruption are well-documented issues. |

**Overall confidence:** HIGH

### Gaps to Address

- **Rocket.Chat REST API stability for bot use cases:** The Bots SDK is deprecated but REST API itself is not. However, there is no explicit guarantee that `users.create` with `bot` role will remain supported. Monitor Rocket.Chat release notes. ADR should document this risk.
- **Ralph upstream maintainer responsiveness:** Unknown whether the upstream PR will get timely review. The fork strategy mitigates this but adds maintenance burden. Start the conversation (GitHub issue) in Phase 1.
- **Bridge config exchange mechanism:** Research suggests stdout JSON is fragile (Pitfall 3). The spec should evaluate file-based config exchange as an alternative. Decide during Phase 1 spec writing.
- **O(N) WebSocket connections for message receiving:** Per-agent identity creates N connections to Rocket.Chat. For typical team sizes (3-5) this is fine, but the architecture should support a dispatcher pattern for larger teams. Defer detailed design to Phase 3.

## Sources

### Primary (HIGH confidence)
- Ralph Orchestrator source: `/opt/workspace/ralph-orchestrator/` -- `RobotService` trait, `RobotConfig`, `create_robot_service()`, teloxide-based Telegram implementation
- BotMinter source: `/home/sandboxed/workspace/botminter/crates/bm/` -- config.rs, start.rs, profile.rs, workspace.rs, Cargo.toml
- [MADR 4.0.0](https://github.com/adr/madr) -- ADR template format

### Secondary (MEDIUM confidence)
- [Rocket.Chat Docker Deployment](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose) -- Docker Compose setup, MongoDB requirements
- [Rocket.Chat REST API](https://developer.rocket.chat/apidocs/rocketchat-api) -- endpoints for user creation, messaging, auth
- [Rocket.Chat Bots Architecture](https://developer.rocket.chat/docs/bots-architecture) -- bot role, SDK deprecation notice
- [Knative API Spec](https://github.com/knative/specs/blob/main/specs/serving/knative-api-specification-1.0.md) -- spec format conventions

### Tertiary (LOW confidence)
- [Rocket.Chat MongoDB Keyfile Issue #35039](https://github.com/RocketChat/Rocket.Chat/issues/35039) -- Docker deployment blocker (community report, may be resolved)

---
*Research completed: 2026-03-08*
*Ready for roadmap: yes*
