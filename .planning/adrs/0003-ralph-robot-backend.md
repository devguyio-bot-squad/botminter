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

`bm teams sync` generates valid `ralph.yml` files with the robot section populated from bridge credentials stored in `$BRIDGE_CONFIG_DIR/config.json`. This will be validated in Phase 10 (runtime integration).

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

```
Bridge (onboard) --> $BRIDGE_CONFIG_DIR/config.json --> bm teams sync --> ralph.yml [robot]
```

1. `bm` invokes the bridge's `onboard` recipe with the member's username
2. The bridge writes credentials to `$BRIDGE_CONFIG_DIR/config.json`
3. During `bm teams sync`, BotMinter reads the credentials and generates the appropriate `ralph.yml` robot section
4. The Ralph instance reads `ralph.yml` at startup and connects to the chat platform

### Related

- ADR-0002: Bridge abstraction design decisions (the bridge contract that produces credentials)
- Ralph Orchestrator: `RobotConfig` and `RobotService` trait define the robot backend interface
