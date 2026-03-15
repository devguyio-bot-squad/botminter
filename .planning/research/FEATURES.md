# Feature Landscape

**Domain:** Pluggable communication bridge abstraction for agentic team tooling
**Researched:** 2026-03-08

## Table Stakes

Features users expect from a bridge abstraction milestone. Missing = the abstraction feels incomplete or unfinished.

| Feature | Why Expected | Complexity | Dependencies | Notes |
|---------|--------------|------------|--------------|-------|
| Bridge trait/contract definition | Users need a clear, stable contract that bridge implementations conform to. Without it, "pluggable" is meaningless. | Low | None | Shell-script lifecycle (start/stop/health/configure) matches existing skill pattern. Outputs config on stdout like Unix conventions. |
| `bm bridge start/stop/status` CLI | Operators need explicit lifecycle control over bridge services, independent of agent lifecycle. | Med | Bridge contract | Mirrors `bm start/stop/status` pattern. Status must show bridge health + connected agents. |
| Bridge auto-start from `bm start` | When an operator runs `bm start`, the bridge should optionally come up automatically. Requiring separate manual steps breaks flow. | Low | `bm bridge start`, `bm start` | Controlled by config flag (e.g., `bridge.auto_start: true` in team config). Must start bridge *before* agents. |
| Rocket.Chat bridge implementation | A reference implementation proves the abstraction works. Rocket.Chat is the right choice: self-hosted, Docker-friendly, REST API, free community edition. | High | Bridge contract, Docker | REST API for messaging (`chat.sendMessage`/`chat.postMessage`). Create bot users via `users.create` API. Docker Compose for local deployment. |
| Per-agent bot identity | Each team member must appear as its own bot user on the chat platform. Without this, you cannot tell which agent said what. | Med | Rocket.Chat bridge | Create N bot users on Rocket.Chat (one per hired member). Each agent authenticates with its own credentials. Messages labeled with bot badge. |
| Telegram bridge (migration) | Existing `scrum-compact-telegram` profile users must not lose functionality. Telegram support must be wrapped into the bridge abstraction. | Med | Bridge contract, Ralph robot abstraction | Wraps existing Ralph `RObot` Telegram integration. Config migration: `RObot.telegram` in `ralph.yml` maps to bridge config. |
| Ralph robot abstraction (upstream) | Ralph's `RobotService` trait already exists in `ralph-proto` but only `TelegramService` implements it. The config (`RobotConfig`) is hardcoded to Telegram fields. Making the config pluggable upstream enables BotMinter bridges to provide arbitrary backends. | High | Ralph Orchestrator codebase | Upstream PR to ralph-orchestrator. Must preserve backwards compatibility with existing `RObot.telegram` config. Add `backend` field with `telegram` as default. |
| ADR practice establishment | First formally specified extension point needs decision records explaining *why* the bridge contract looks the way it does. Without ADRs, future contributors cannot understand rationale. | Low | None | Numbered markdown files in `specs/adrs/`. Michael Nygard format (Context, Decision, Status, Consequences). |
| Bridge spec (Knative-style) | The bridge contract needs a formal spec using RFC 2119 keywords (MUST/SHOULD/MAY) so implementors know exactly what conformance means. | Med | ADR for bridge design | Lives in `specs/specs/bridge/`. Uses Knative conventions: RFC 2119 keywords, conformance profiles, normative examples. First spec = bridge lifecycle + messaging contract. |

## Differentiators

Features that set the bridge abstraction apart from ad-hoc chat integrations. Not expected, but valuable.

| Feature | Value Proposition | Complexity | Dependencies | Notes |
|---------|-------------------|------------|--------------|-------|
| Bridge health monitoring with auto-recovery | Bridge services can crash. Auto-detecting failure and restarting (with backoff) keeps the team running without operator intervention. | Med | `bm bridge start` | Ralph already has retry-with-backoff patterns in `TelegramService`. Apply same pattern to bridge lifecycle. |
| Agent-to-agent visibility in chat | Agents can see each other's messages in shared channels, enabling emergent coordination through the chat platform (not just GitHub issues). | Med | Per-agent identity, Rocket.Chat bridge | Each agent joins team channels. Agents see messages from other agents. Novel coordination surface beyond issue tracker. |
| Bridge configuration in profile | Bridge settings defined at the profile level (not just team level) so profiles can ship with opinionated bridge defaults (e.g., `scrum-compact-rocketchat` profile). | Low | Bridge contract, profile system | Follows existing profile/team override pattern. Profile defines bridge type + defaults, team config overrides credentials/URLs. |
| Docker Compose bridge provisioning | `bm bridge start` auto-generates and runs a Docker Compose file for the Rocket.Chat stack (Rocket.Chat + MongoDB), reducing setup to one command. | High | Rocket.Chat bridge | Generates `docker-compose.yml` in `~/.botminter/bridges/rocketchat/`. Manages container lifecycle. Detects if already running. |
| Multi-bridge support | Run Telegram and Rocket.Chat simultaneously — e.g., Telegram for operator notifications, Rocket.Chat for agent-to-agent channels. | Low | Bridge contract | Contract already supports multiple instances. Config is a list/map of bridges. Implementation is straightforward once contract exists. |
| Spec conformance tests | Automated tests that validate a bridge implementation against the spec, ensuring new bridges meet the contract. | Med | Bridge spec | Similar to Knative conformance test approach. Shell-based tests that exercise lifecycle + messaging. |

## Anti-Features

Features to explicitly NOT build in this milestone.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Apps-Engine Rocket.Chat integration | Rocket.Chat recommends Apps-Engine over bot SDK, but Apps-Engine requires TypeScript, runs inside RC server, and adds massive complexity. The REST API is stable and sufficient. | Use REST API (`chat.sendMessage`, `users.create`) with direct HTTP calls. No SDK dependency. |
| Bidirectional message bridging (operator chat -> agent action) | Tempting to let operators send commands to agents via chat, but this duplicates the `human.interact` event flow that Ralph already handles. Two command channels = confusion. | Keep chat as a read/notification surface for the bridge. `human.interact` via Ralph events remains the interaction path. Operator guidance flows through Ralph's robot, not through bridge messages. |
| Matrix protocol bridge | Matrix is architecturally elegant (decentralized, federation) but massive overhead for a local dev tool. Self-hosted Matrix requires Synapse server + database + bridge infrastructure. | Rocket.Chat is lighter for local dev. Matrix can be a future bridge implementation if demand exists. |
| Matterbridge integration | Matterbridge (Go multi-protocol chat bridge) seems like it could replace custom bridge code, but it bridges *between* chat platforms, not between agents and chat. Wrong abstraction level. | Build BotMinter-native bridge contract that owns agent identity and lifecycle. |
| Slack bridge implementation | Slack requires a paid workspace for bot features, cannot be self-hosted, and has aggressive rate limits. Not suitable as a reference implementation for local dev. | Rocket.Chat for local/self-hosted. Slack can be a community-contributed bridge later. |
| Chat-based command interface | Building a chat bot that responds to slash commands (`/status`, `/deploy`) is a different product. BotMinter agents communicate through GitHub issues and events. | Chat is for visibility and notifications. Commands go through `bm` CLI or GitHub. |
| Real-time message streaming (WebSocket/DDP) | Rocket.Chat supports real-time message streams via DDP, but agents don't need real-time chat input. They need to *send* messages, not *receive* them from chat. | Use REST API for sending. If receiving is ever needed (future), add DDP support then. |
| Custom Rocket.Chat Apps-Engine app | Building a custom RC app for agent management adds a deployment artifact, TypeScript build pipeline, and RC version coupling. | Keep bridge as external process using REST API. Simpler, fewer moving parts. |

## Feature Dependencies

```
Bridge contract definition
  |
  +---> bm bridge start/stop/status
  |       |
  |       +---> Bridge auto-start from bm start
  |       +---> Bridge health monitoring (differentiator)
  |
  +---> Rocket.Chat bridge implementation
  |       |
  |       +---> Per-agent bot identity
  |       |       |
  |       |       +---> Agent-to-agent visibility (differentiator)
  |       |
  |       +---> Docker Compose provisioning (differentiator)
  |
  +---> Telegram bridge (migration)
  |       |
  |       +---> Ralph robot abstraction (upstream) [prerequisite]
  |
  +---> Bridge spec (Knative-style)
  |       |
  |       +---> Spec conformance tests (differentiator)
  |
  +---> Bridge config in profile (differentiator)

ADR practice establishment (independent, can start first)
  |
  +---> ADR-001: Bridge abstraction design rationale
  +---> ADR-002: Shell-script lifecycle convention
  +---> ADR-003: Knative-style spec adoption

Ralph robot abstraction (upstream, independent but prerequisite for Telegram bridge)
  |
  +---> Make RobotConfig backend-agnostic
  +---> Preserve RObot.telegram backwards compat
  +---> Telegram bridge wraps new abstraction
```

## MVP Recommendation

### Phase 1: Foundation (do first)

1. **ADR practice establishment** — Low complexity, no dependencies. Create `specs/adrs/` directory, write ADR template, write first ADR for bridge design rationale. Establishes the practice that carries through all later work.

2. **Bridge contract definition** — The shell-script lifecycle contract (start/stop/health/configure). Define the interface before implementing it. Write as both an ADR and a Knative-style spec.

3. **`bm bridge start/stop/status` CLI** — The operator surface. Can be built with a stub/no-op bridge to validate the CLI UX before any real implementation exists.

### Phase 2: Reference Implementation (do second)

4. **Rocket.Chat bridge implementation** — The first real bridge. REST API integration (`chat.sendMessage`, `users.create`). Docker Compose for local Rocket.Chat provisioning.

5. **Per-agent bot identity** — Each hired member gets a Rocket.Chat bot user. Messages attributed to the correct agent. This is what makes the bridge valuable vs. a single notification bot.

### Phase 3: Migration and Upstream (do third)

6. **Ralph robot abstraction (upstream)** — PR to ralph-orchestrator making `RobotConfig` backend-agnostic. Must land before Telegram bridge can use the new abstraction.

7. **Telegram bridge (migration)** — Wrap existing Telegram support into the bridge contract. Existing `scrum-compact-telegram` profile continues to work.

### Defer to later milestones

- **Agent-to-agent visibility**: Valuable but not required for bridge to be useful. Agents already coordinate through GitHub issues.
- **Docker Compose auto-provisioning**: Nice UX but operators can run `docker-compose up` manually for now. Document the compose file in bridge docs.
- **Multi-bridge support**: Contract supports it naturally; explicit multi-bridge config can wait.
- **Spec conformance tests**: Valuable once there are 2+ bridge implementations to test against.
- **Bridge health monitoring with auto-recovery**: Can be added incrementally after basic lifecycle works.

## Complexity Assessment

| Feature | Lines of Rust (estimate) | Shell/Config | Risk |
|---------|--------------------------|--------------|------|
| Bridge contract | ~200 | ~50 (spec) | Low — clear pattern from existing skill contract |
| CLI commands | ~400 | ~0 | Low — mirrors start/stop/status pattern |
| Rocket.Chat bridge | ~600 | ~200 (Docker Compose, config) | Med — REST API integration, user provisioning, error handling |
| Per-agent identity | ~300 | ~50 (config) | Med — requires Rocket.Chat admin API, credential management |
| Telegram bridge | ~200 | ~50 (config) | Low — wrapping existing code, not new logic |
| Ralph upstream PR | ~400 (Rust, in ralph repo) | ~0 | High — upstream coordination, backwards compat, review process |
| ADR practice | ~0 | ~200 (markdown) | Low — documentation only |
| Bridge spec | ~0 | ~300 (markdown) | Low — documentation, but requires precise language |

**Total estimated new Rust code:** ~2,100 lines (in BotMinter) + ~400 lines (upstream Ralph PR)

## Sources

- Ralph Orchestrator `RobotService` trait: `/opt/workspace/ralph-orchestrator/crates/ralph-proto/src/robot.rs` (HIGH confidence)
- Ralph `RobotConfig` with hardcoded Telegram fields: `/opt/workspace/ralph-orchestrator/crates/ralph-core/src/config.rs:1756` (HIGH confidence)
- Ralph `TelegramService` implementation: `/opt/workspace/ralph-orchestrator/crates/ralph-telegram/src/service.rs` (HIGH confidence)
- BotMinter `bm start` flow: `/home/sandboxed/workspace/botminter/crates/bm/src/commands/start.rs` (HIGH confidence)
- Existing `scrum-compact-telegram` profile with RObot config: `profiles/scrum-compact-telegram/roles/superman/ralph.yml` (HIGH confidence)
- [Rocket.Chat REST API](https://developer.rocket.chat/reference/api) — `chat.sendMessage`, `users.create` endpoints (MEDIUM confidence — docs hard to scrape but API is stable)
- [Rocket.Chat Docker deployment](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose) (MEDIUM confidence)
- [Rocket.Chat bots architecture](https://developer.rocket.chat/docs/bots-architecture) — per-bot user accounts, bot role, message streams (MEDIUM confidence)
- [Rocket.Chat bot SDK deprecation notice](https://developer.rocket.chat/docs/develop-a-rocketchat-sdk-bot) — recommends Apps-Engine, but REST API remains available (MEDIUM confidence)
- [Michael Nygard ADR template](https://www.cognitect.com/blog/2011/11/15/documenting-architecture-decisions) (HIGH confidence)
- [ADR templates collection](https://adr.github.io/adr-templates/) (HIGH confidence)
- [Knative specs repository](https://github.com/knative/specs) — RFC 2119 keyword usage, conformance profiles (MEDIUM confidence)
- [Knative API spec format](https://github.com/knative/specs/blob/main/specs/serving/knative-api-specification-1.0.md) (MEDIUM confidence)
- [Matterbridge multi-protocol bridge](https://github.com/matterbridge-org/matterbridge) — reference for pluggable chat backend patterns (MEDIUM confidence)
- [Matrix bridges overview](https://matrixdocs.github.io/docs/bridges/overview) — architectural patterns for chat bridging (MEDIUM confidence)
