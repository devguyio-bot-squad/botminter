# Implementation Notes: v0.07 Team Bridge

Captured during requirements gathering for reference during planning/execution.
Not part of requirements — these are technical details and design hints from research.

## Bridge Contract

- Bridge definition file is `bridge.yml`; config schema is `schema.json`
- Commands are Justfile recipes (`just start`, `just onboard <username>`, etc.). Each bridge ships a Justfile.
- Bridge directory structure: `bridges/<name>/bridge.yml`, `schema.json`, `Justfile`
- File-based config exchange writes to `$BRIDGE_CONFIG_DIR/config.json`
- External bridges (Telegram) have no-op lifecycle — the service is SaaS

## BotMinter Architecture

- New `bridge.rs` module in `crates/bm/src/` with shared logic for both CLI and sync commands
- New `commands/bridge.rs` for CLI subcommands
- Bridge state stored in `~/.botminter/` (URLs, container IDs, per-user credentials)
- `commands/sync.rs` and `commands/start.rs` call into `bridge.rs` — not subprocess calls to `bm bridge`
- Profile bridge definitions live in `profiles/<name>/bridges/` directory
- Bridge CLI commands are fully decoupled from team logic — bridge knows about users and rooms, not members

## Rocket.Chat Specifics

- REST API endpoints: `/api/v1/login`, `/api/v1/users.create` (with `"roles": ["bot"]`), `/api/v1/channels.create`, `/api/v1/chat.postMessage`, `/api/v1/channels.history`
- Auth is header-based: `X-Auth-Token` + `X-User-Id` on every request. No OAuth needed for local deployment.
- Bot users get deterministic emails (`{name}@botminter.local`)
- MongoDB requires single-node replica set (non-negotiable for RC)
- Podman Pod groups RC + MongoDB containers sharing localhost network namespace
- RC deprecated Bot SDK but REST API for user creation and messaging is stable and correct
- Rocket.Chat supports markdown natively — message format translation may be simpler than Telegram

## Ralph Orchestrator

- Local fork at `/opt/workspace/ralph-orchestrator` — no upstream PR this milestone
- Ralph is a Rust project (NOT Node.js), uses teloxide for Telegram
- `RobotService` trait in `ralph-proto/src/robot.rs` — 6 methods: `send_question`, `wait_for_response`, `send_checkin`, `timeout_secs`, `shutdown_flag`, `stop`
- All methods synchronous; Telegram impl uses `tokio::task::block_in_place()` internally
- `RobotConfig` in `ralph-core/src/config.rs` — hardcodes `telegram: Option<TelegramBotConfig>`
- `create_robot_service()` in `ralph-cli/src/loop_runner.rs` — private, hardcodes `TelegramService`
- Bot commands in `ralph-telegram/src/commands.rs` — `handle_command()` is filesystem-based and backend-agnostic, reusable by any backend
- Ralph uses `#[serde(rename = "RObot")]` — YAML key is literally `RObot`
- Config resolution: env var -> config file -> keychain (for tokens)
- Messages use markdown internally, converted to Telegram HTML by `TelegramService`
- `wait_for_response()` polls JSONL events file for `human.response` entries (250ms interval)
- `EventLoop::set_robot_service()` is public and takes `Box<dyn RobotService>`
- `DaemonAdapter` trait also exists for persistent bot mode — may need a parallel adapter for RC
- Bot commands: `/help`, `/status`, `/tasks`, `/memories`, `/tail`, `/model`, `/models`, `/restart`, `/stop`

## ADRs & Specs

- MADR 4.0.0 format (Context, Decision, Status, Consequences)
- ADRs in `specs/adrs/` with sequential numbering (0001, 0002, ...)
- Specs in `specs/<interface>/` (e.g., `specs/bridge/`) with Knative-style RFC 2119 keywords (MUST/SHOULD/MAY)
- Existing `specs/` contents (master-plan, milestones, prompts, tasks) removed — preserved in git history

## Integration Strategy

- Ralph upstream contribution deferred — all changes in local fork
- The `RobotService` trait is already backend-agnostic; only `RobotConfig` and factory need changes
- New `ralph-rocketchat` crate implements `RobotService` using `reqwest` (already in Ralph workspace deps)
- BotMinter generates `ralph.yml` `RObot` section during `bm teams sync` based on bridge config
- Member credentials stored in bridge state, injected into ralph.yml per-member

## Research Files

| File | Contents |
|------|----------|
| `STACK.md` | Technology recommendations, API endpoints, dependency analysis |
| `FEATURES.md` | Feature landscape: table stakes, differentiators, anti-features |
| `ARCHITECTURE.md` | System overview, component boundaries, data flows, build order |
| `PITFALLS.md` | 14 domain pitfalls with prevention strategies |
| `RALPH-ROBOT-INTERNALS.md` | Deep dive into Ralph's robot system (trait, config, wiring, commands) |
| `SUMMARY.md` | Synthesized research with roadmap implications |
