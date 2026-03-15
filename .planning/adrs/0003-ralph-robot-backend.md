# Bridge Outputs Credentials, BotMinter Maps to Ralph

---
status: accepted
date: 2026-03-08
decision-makers: [BotMinter maintainers]
---

## Context and Problem Statement

Ralph Orchestrator already has a pluggable robot backend abstraction (`RobotConfig`, `RobotService` trait). When a BotMinter bridge onboards a bot user, the resulting credentials must be wired into the team member's `ralph.yml` so Ralph can communicate through the chat platform.

How should bridge credentials flow into Ralph's robot configuration?

## Decision Drivers

* Ralph Orchestrator already has a pluggable robot backend -- BotMinter should not reinvent it
* Bridge credentials (tokens, URLs, user IDs) must become robot credentials in `ralph.yml`
* Minimal coupling between the bridge layer and the Ralph layer
* The bridge author should not need to understand Ralph's internal configuration format

## Considered Options

* Bridge generates `ralph.yml` robot section directly
* Bridge outputs credentials via file-based exchange, BotMinter maps them to `ralph.yml`
* Ralph reads bridge config directly from `$BRIDGE_CONFIG_DIR`

## Decision Outcome

Chosen option: "Bridge outputs credentials via file-based exchange, BotMinter maps them to `ralph.yml`", because it provides clean separation of concerns -- the bridge knows nothing about Ralph, and Ralph knows nothing about bridges. BotMinter is the translation layer between the two.

### Consequences

* Good, because clean separation -- bridge authors never need to learn Ralph's config format
* Good, because BotMinter controls the mapping, enabling bridge-specific translation logic
* Good, because `ralph.yml` generation already happens during `bm teams sync` -- credential injection fits naturally
* Neutral, because BotMinter must maintain a mapping for each bridge type (manageable since bridge types are enumerable)
* Bad, because an extra layer of indirection between bridge output and Ralph config (mitigated by the mapping being straightforward key-value translation)

### Confirmation

`bm teams sync` generates `ralph.yml` files with `RObot.enabled` set based on credential availability — but credentials themselves are NOT written to `ralph.yml`. Instead, `bm start` injects per-member credentials as environment variables at launch time. This separation keeps secrets out of committed files while still enabling Ralph's robot backend.

## Pros and Cons of the Options

### Bridge generates ralph.yml robot section directly

* Good, because no translation layer needed
* Bad, because bridge authors must understand Ralph's internal config format
* Bad, because tight coupling -- changes to Ralph's config format break all bridges
* Bad, because bridge implementations become Ralph-version-specific

### Bridge outputs credentials, BotMinter maps to ralph.yml

* Good, because bridges output a simple, generic credential format (JSON)
* Good, because BotMinter absorbs the complexity of Ralph config generation
* Good, because bridge and Ralph can evolve independently
* Neutral, because BotMinter must maintain bridge-to-Ralph mapping code

### Ralph reads bridge config directly

* Good, because no BotMinter involvement in credential wiring
* Bad, because Ralph would need to understand bridge-specific config formats
* Bad, because Ralph Orchestrator is an external dependency -- modifying it for BotMinter concerns is inappropriate
* Bad, because breaks the layering (Ralph should not know about BotMinter's bridge concept)

## More Information

### Data Flow

Per-member credentials flow through three stages: provisioning, storage, and
injection. Secrets are never written to `ralph.yml` — they are injected as
environment variables at launch time.

```
                        ┌─ local bridge: bridge creates user + token
Bridge (onboard) ──────►│
                        └─ external bridge: operator-supplied token validated
        │
        ▼
$BRIDGE_CONFIG_DIR/config.json  (per-member: username, user_id, token)
        │
        ▼
bm teams sync ──► Formation credential store (keyring / K8s Secret / etc.)
        │
        ├──► ralph.yml [RObot.enabled = true/false]  (no secrets)
        │
        ▼
bm start ──► env var injection per member (e.g., RALPH_TELEGRAM_BOT_TOKEN)
        │
        ▼
Ralph instance reads env var at startup ──► connects to chat platform
```

1. `bm teams sync --bridge` invokes the bridge's `onboard` recipe for each
   member not yet provisioned (idempotent). For local bridges, the bridge
   creates the user. For external bridges, the operator must have already
   supplied a token (via `bm hire` or `bm bridge identity add`).
2. The bridge writes credentials to `$BRIDGE_CONFIG_DIR/config.json`.
3. BotMinter stores credentials in the formation's credential backend (system
   keyring for local formations, K8s Secrets for Kubernetes formations).
4. During workspace sync, `ralph.yml` gets `RObot.enabled = true` if the
   member has bridge credentials, `false` otherwise. No secrets are written
   to `ralph.yml`.
5. At `bm start`, BotMinter resolves each member's credential from the
   formation store and passes it as an environment variable to that member's
   Ralph instance (e.g., `RALPH_TELEGRAM_BOT_TOKEN`).
6. The Ralph instance reads the env var at startup and connects to the chat
   platform via its robot backend.

### Related

- ADR-0002: Bridge abstraction design decisions (the bridge contract that produces credentials)
- Ralph Orchestrator: `RobotConfig` and `RobotService` trait define the robot backend interface
