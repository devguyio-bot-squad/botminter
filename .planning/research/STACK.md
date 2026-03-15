# Technology Stack

**Project:** BotMinter v0.07 — Team Bridge
**Researched:** 2026-03-08

## Critical Correction

The milestone context states Ralph Orchestrator is "a Node.js/TypeScript project using grammY for Telegram." This is **incorrect**. Ralph Orchestrator is a **Rust** project (Cargo workspace, edition 2024, v2.7.0) that uses **teloxide** (Rust Telegram bot framework, v0.13) for Telegram integration. The web dashboard has a Node.js frontend but the core orchestrator, CLI, and all robot/communication code is Rust.

**Confidence: HIGH** — verified directly from `/opt/workspace/ralph-orchestrator/Cargo.toml` and `crates/ralph-telegram/Cargo.toml`.

## Existing Stack (No Changes Needed)

These are already in `crates/bm/Cargo.toml` and sufficient for the bridge abstraction work on the BotMinter side:

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust + Cargo workspace | edition 2021 | Core language |
| clap | 4 | CLI parsing (new `bm bridge` subcommands) |
| serde + serde_yml | 1 / 0.0.12 | Config serialization (bridge config in botminter.yml) |
| anyhow | 1 | Error handling |
| chrono | 0.4 | Timestamps |
| which | 7 | Binary discovery (already used, useful for Docker detection) |

## New Stack Additions for BotMinter

### Rocket.Chat Deployment (Bridge Reference Implementation)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| Docker + Docker Compose v2 | latest | Run Rocket.Chat + MongoDB locally | Official recommended deployment method. No native install path is supportable. Docker is the only sane way to run Rocket.Chat locally. |
| Rocket.Chat Server | latest (currently 7.x) | Slack-like local communication | Self-hosted, REST API for bot users, per-agent identity via separate user accounts. Free Community Edition supports unlimited users. |
| MongoDB (single-node replica set) | 8.2 (current RC requirement) | Rocket.Chat's required database | RC mandates MongoDB with replica set (even single-node). Cannot avoid this. The bridge start script handles `rs.initiate()`. |

**Do NOT use:**
- `rocketchat` Rust crate from crates.io — appears abandoned/minimal. The Rocket.Chat bridge is a shell script, not Rust code. It calls `curl` against the REST API. This aligns with BotMinter's existing "skills as shell scripts" pattern.
- `rocketchat-message` crate — webhook-only, 3 years old, insufficient.
- Rocket.Chat Node.js SDK — 7 years since last publish, officially deprecated in favor of Apps-Engine.
- Rocket.Chat Apps-Engine — overkill for bot message sending; REST API is simpler and more stable.

**Confidence: HIGH** — Docker deployment verified from [official docs](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose). REST API endpoints verified from [developer docs](https://developer.rocket.chat/apidocs/rocketchat-api).

### Rocket.Chat REST API Integration Points

The bridge shell script needs these REST API endpoints:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/login` | POST | Authenticate admin user and bot users |
| `/api/v1/users.create` | POST | Create per-agent bot users with `"roles": ["bot"]` |
| `/api/v1/channels.create` | POST | Create team channel |
| `/api/v1/chat.postMessage` | POST | Send messages as bot user (supports `#channel` addressing) |
| `/api/v1/channels.history` | GET | Poll for new messages (human responses) |

Auth is header-based: `X-Auth-Token` + `X-User-Id` on every request. No OAuth needed for local deployment.

**Important:** Rocket.Chat's traditional bot integration is deprecated. However, the REST API for user creation and message sending is NOT deprecated — it's the core API. The deprecation applies to the Bot SDK framework, not the underlying HTTP endpoints. For our use case (create users, send/receive messages), the REST API is the correct and stable approach.

**Confidence: HIGH** — verified from [Create User API](https://developer.rocket.chat/api/rest-api/endpoints/users/create) and [Send Message API](https://developer.rocket.chat/apidocs/send-message).

### No New Rust Dependencies

The bridge abstraction in BotMinter is implemented as:
1. **Rust code** in `crates/bm/` for the `bm bridge` CLI commands (start/stop/status) and bridge config model
2. **Shell scripts** for bridge lifecycle (start.sh, stop.sh, health.sh, configure.sh)
3. **curl** for Rocket.Chat REST API calls from shell scripts

This follows BotMinter's established pattern: skills are shell scripts, the Rust CLI orchestrates lifecycle. No new Rust crate dependencies are needed for the bridge itself.

**Confidence: HIGH** — consistent with existing architecture patterns (composable skills, shell scripts for gh CLI, etc.).

## New Stack for Ralph Orchestrator (Upstream Contribution)

### Robot Abstraction

Ralph already has the `RobotService` trait in `ralph-proto::robot`:

```rust
pub trait RobotService: Send + Sync {
    fn send_question(&self, payload: &str) -> anyhow::Result<i32>;
    fn wait_for_response(&self, events_path: &Path) -> anyhow::Result<Option<String>>;
    fn send_checkin(&self, iteration: u32, elapsed: Duration, context: Option<&CheckinContext>) -> anyhow::Result<i32>;
    fn timeout_secs(&self) -> u64;
    fn shutdown_flag(&self) -> Arc<AtomicBool>;
    fn stop(self: Box<Self>);
}
```

The upstream contribution does NOT require adding new crates to Ralph. The work is:

1. **Make `create_robot_service()` in `ralph-cli/src/loop_runner.rs` pluggable** — currently hardcoded to `ralph_telegram::TelegramService::new()`. Needs a factory/registry pattern or config-driven backend selection.
2. **Add a `ralph.yml` config option** for robot backend type (e.g., `robot.backend: telegram | rocketchat | custom`).
3. **Possibly add a `ralph-rocketchat` crate** — implements `RobotService` using reqwest (already a workspace dependency) to call Rocket.Chat REST API.

| What | Where | New Dependencies |
|------|-------|-----------------|
| Robot backend config | `ralph-proto` or `ralph-core` | None |
| Backend selection logic | `ralph-cli/src/loop_runner.rs` | None |
| Rocket.Chat RobotService impl | New `crates/ralph-rocketchat/` | `reqwest` (already in workspace) |

The `ralph-rocketchat` crate would use `reqwest` (already `v0.12` in Ralph's workspace deps) for HTTP calls. No new dependencies needed.

**Confidence: HIGH** — verified from direct inspection of Ralph's `Cargo.toml`, `robot.rs` trait, and `loop_runner.rs` wiring.

## ADR Tooling

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| MADR format | 4.0.0 | ADR template convention | Industry standard, maintained by the official ADR GitHub org, simple Markdown files. No tooling dependency needed. |
| No CLI tool | N/A | Intentional omission | `adr-tools` is a bash script that numbers files. BotMinter can do this trivially with a shell function or just manually. Adding a tool dependency for file numbering is not worth the supply chain complexity. |

**ADR convention for BotMinter:**
- Directory: `specs/adrs/` (consistent with existing `specs/` hierarchy)
- Naming: `NNNN-title-with-dashes.md` (zero-padded 4-digit sequence)
- Template: MADR 4.0.0 bare-minimal (Status, Context, Decision Drivers, Considered Options, Decision Outcome, Consequences)
- First ADR: `0001-bridge-abstraction.md`

**Knative-style specs:**
- Directory: `specs/interfaces/` for formal interface specifications
- Format: Markdown with MUST/SHOULD/MAY (RFC 2119) keywords
- First spec: `bridge-v1.md` defining the shell-script contract

No tools needed — these are documentation conventions, not software dependencies. The convention IS the tool.

**Confidence: HIGH** — MADR 4.0.0 verified from [official repo](https://github.com/adr/madr). Knative spec pattern verified from [Knative specs repo](https://github.com/knative/specs).

## Bridge Shell Script Contract

The bridge plugin is defined by a set of shell scripts with a standardized contract. No new tooling needed — this uses the OS and existing `which`/`Command` APIs in Rust.

| Script | stdin | stdout (JSON) | Purpose |
|--------|-------|---------------|---------|
| `start.sh` | None | `{"url": "...", "admin_token": "..."}` | Start the communication service |
| `stop.sh` | None | None | Stop the communication service |
| `health.sh` | None | Exit code 0/1 | Health check |
| `configure.sh` | JSON config | `{"bot_token": "...", "chat_id": "..."}` | Create/configure a bot user for an agent |

Scripts live in the profile at `bridges/<name>/` (e.g., `bridges/rocketchat/start.sh`). BotMinter's Rust code invokes them via `std::process::Command` (already used extensively for `gh`, `ralph`, `claude` invocations).

**Confidence: MEDIUM** — contract design is speculative (needs validation during design phase). The mechanism (shell scripts + JSON stdout) is proven in the existing skill system.

## What NOT to Add

| Temptation | Why Not |
|------------|---------|
| WebSocket/Realtime API for Rocket.Chat | DDP is officially deprecated. REST polling is simpler and sufficient for bot use case. |
| gRPC between BotMinter and Ralph | Massive complexity for no benefit. Ralph is launched as a subprocess. Config via env vars + files is the right level. |
| A Rust Rocket.Chat client library | Shell scripts with `curl` are simpler, debuggable, and consistent with existing patterns. |
| Docker SDK / bollard crate | Shell `docker compose` commands via `Command` are simpler and match operator mental model. |
| Rocket.Chat Apps-Engine integration | Overkill. REST API is sufficient and more stable for send/receive messages. |
| Any message queue (Redis, NATS, etc.) | BotMinter is not a microservices platform. File-based events + HTTP is the right level. |
| Matrix/Mattermost support in v0.07 | Bridge abstraction enables future backends. Ship one reference impl first. |
| `adr-tools` or `log4brains` CLI | File numbering doesn't justify a dependency. Manual or simple shell function suffices. |

## Installation / Setup

No new Rust dependencies to install. The operator needs:

```bash
# Already required
cargo, rustc, gh CLI

# New requirement for Rocket.Chat bridge
docker, docker compose  # v2 (compose as docker plugin, not standalone docker-compose)
curl                     # For bridge shell scripts (universally available)
```

BotMinter should detect Docker availability at `bm bridge start` time via `which docker` (already has the `which` crate) and provide a clear error if missing.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Local chat | Rocket.Chat (Docker) | Mattermost | RC has better REST API docs, smaller footprint, more flexible user/bot model. Mattermost requires PostgreSQL. |
| Local chat | Rocket.Chat (Docker) | Zulip | Zulip's topic model is interesting but heavier to deploy and less familiar UX. |
| Local chat | Rocket.Chat (Docker) | Matrix (Synapse) | Matrix is a protocol, not a product. Synapse + Element is more complex to deploy for a "just works" local experience. |
| Bot API approach | REST API (curl) | Node.js SDK | SDK is 7 years stale and deprecated. REST is stable and language-agnostic. |
| Ralph robot backend | New `ralph-rocketchat` crate | Shim via BotMinter | Upstream contribution is cleaner. BotMinter shouldn't intercept Ralph's robot protocol. |
| ADR format | MADR 4.0.0 | Nygard (original) | MADR has richer structure (decision drivers, options, consequences) while staying lightweight. |
| ADR tooling | None (convention only) | adr-tools | BotMinter already has a specs/ directory convention. Adding a tool for file numbering is friction, not value. |
| Bridge contract | Shell scripts (JSON stdout) | Rust trait (compiled plugin) | Shell scripts are debuggable, don't require recompilation, and match the existing skill pattern. Compiled plugins would require a plugin ABI. |

## Sources

- [Rocket.Chat Docker Deployment](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose) — official Docker Compose guide
- [Rocket.Chat REST API](https://developer.rocket.chat/apidocs/rocketchat-api) — API reference
- [Rocket.Chat Create User](https://developer.rocket.chat/api/rest-api/endpoints/users/create) — bot user creation
- [Rocket.Chat System Requirements](https://docs.rocket.chat/docs/system-requirements) — minimum resources
- [MADR 4.0.0](https://github.com/adr/madr) — ADR template format
- [Knative API Specification](https://github.com/knative/specs/blob/main/specs/serving/knative-api-specification-1.0.md) — spec convention reference
- [ADR Tooling Overview](https://adr.github.io/adr-tooling/) — tool landscape
- Ralph Orchestrator source: `/opt/workspace/ralph-orchestrator/` — direct inspection of Cargo.toml, robot.rs trait, loop_runner.rs, ralph-telegram crate
- BotMinter source: `/home/sandboxed/workspace/botminter/crates/bm/` — direct inspection of config.rs, start.rs, Cargo.toml
