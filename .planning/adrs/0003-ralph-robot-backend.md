---
status: accepted
date: 2026-03-08
decision-makers: [BotMinter maintainers]
---

# Bridge Outputs Credentials, BotMinter Maps to Ralph

## Problem

Ralph Orchestrator has a pluggable robot backend (`RobotConfig`, `RobotService` trait). When a BotMinter bridge onboards a bot user, the resulting credentials must be wired into the member's `ralph.yml` so Ralph can communicate through the chat platform. How should bridge credentials flow into Ralph's robot configuration?

## Constraints

* Ralph Orchestrator is an external dependency — BotMinter must not modify Ralph's internals
* Bridge authors must not need to understand Ralph's config format
* Credentials (tokens) must never be written to `ralph.yml` (committed files)
* The mapping must work for any bridge type without bridge-specific code in Ralph

## Decision

BotMinter is the translation layer. Bridges output generic credentials via `$BRIDGE_CONFIG_DIR/config.json`. BotMinter stores them in the formation's credential backend, writes non-secret config to `ralph.yml` (service URL, user ID, `RObot.enabled`), and injects secrets as env vars at `bm start` time.

### Data flow

```
Bridge (onboard) → $BRIDGE_CONFIG_DIR/config.json (username, user_id, token)
       ↓
bm teams sync → Formation credential store (keyring / K8s Secret)
       ↓
       ├→ ralph.yml [RObot.enabled = true, service_url, user_id] (no secrets)
       ↓
bm start → env var injection (e.g., RALPH_TELEGRAM_BOT_TOKEN)
       ↓
Ralph instance reads env var → connects to chat platform
```

## Rejected Alternatives

### Bridge generates ralph.yml robot section directly

Rejected because: tight coupling — bridge authors must understand Ralph's config format, and changes to Ralph's format break all bridges.

### Ralph reads bridge config directly from $BRIDGE_CONFIG_DIR

Rejected because: breaks layering — Ralph should not know about BotMinter's bridge concept. Ralph Orchestrator is an external project.

## Consequences

* Clean separation — bridges, BotMinter, and Ralph can evolve independently
* BotMinter must maintain a per-bridge-type mapping (manageable since bridge types are enumerable)
* Secrets stay out of committed files — `ralph.yml` has `RObot.enabled` and service URLs but never tokens

## Anti-patterns

* **Do NOT** write tokens or secrets to `ralph.yml` — they must be injected as env vars at `bm start` time. Committed files must never contain credentials.
* **Do NOT** make bridges aware of Ralph's config format — bridges output generic JSON, BotMinter translates. If a bridge needs to know about `RObot.matrix.homeserver_url`, the abstraction is broken.
* **Do NOT** modify Ralph Orchestrator to accommodate BotMinter bridge specifics — Ralph's `RobotConfig` is the stable interface. BotMinter adapts to it, not the other way around.
