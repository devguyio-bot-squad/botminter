# Architecture Patterns

**Domain:** Bridge abstraction, Rocket.Chat integration, Ralph robot abstraction, ADRs/specs
**Researched:** 2026-03-08

## Recommended Architecture

The v0.07 "Team Bridge" milestone introduces a communication layer that sits between BotMinter's existing lifecycle management (`bm start/stop`) and Ralph Orchestrator's `RobotService` trait. The architecture has three distinct integration surfaces:

1. **BotMinter bridge abstraction** -- a shell-script-based lifecycle contract for managing communication services (Rocket.Chat, Telegram mock server, etc.)
2. **Ralph Orchestrator robot abstraction** -- upstream changes to make `RobotConfig` support pluggable backends beyond Telegram
3. **Profile/team config plumbing** -- bridge config in `botminter.yml` and `config.yml`, per-agent identity in workspace provisioning

### System Overview

```
                    bm CLI
                      |
          +-----------+-----------+
          |                       |
    bm bridge start          bm start
          |                       |
    [bridge scripts]        [ralph per member]
    start.sh -> stdout       RobotService trait
    health.sh                    |
    configure.sh            +----+----+
    stop.sh                 |         |
          |            Telegram   Rocket.Chat
    Service Process    Service    Service
    (e.g. Rocket.Chat              |
     Docker container)    REST API + WebSocket
          |
    Per-agent bot users
    created by configure.sh
```

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `bridge.rs` (new module) | Bridge lifecycle: start, stop, status, health checks | Shell scripts in profile `bridges/` dir |
| `commands/bridge.rs` (new) | CLI subcommands: `bm bridge start/stop/status` | `bridge.rs`, `config.rs`, `state.rs` |
| `config.rs` (modified) | Stores bridge credentials and config in `TeamEntry` | `bridge.rs`, `commands/start.rs` |
| `profile.rs` (modified) | Reads `bridges/` directory from profile, validates bridge definitions | `bridge.rs` |
| `workspace.rs` (modified) | Passes bridge connection info to workspace provisioning | `start.rs`, `bridge.rs` |
| `commands/start.rs` (modified) | Optional auto-start of bridge before launching members | `bridge.rs` |
| `state.rs` (modified) | Tracks bridge process PID alongside member PIDs | `bridge.rs`, `commands/stop.rs` |
| Ralph `RobotConfig` (upstream) | Supports backend selection (`telegram`, `rocketchat`, etc.) | `ralph-proto::RobotService` |
| Ralph `ralph-rocketchat` (new upstream crate) | Implements `RobotService` for Rocket.Chat REST/WebSocket API | `ralph-proto` |
| Bridge shell scripts (profile) | `start.sh`, `stop.sh`, `health.sh`, `configure.sh` | Docker/process, Rocket.Chat REST API |

### Data Flow

**Bridge startup flow:**

```
1. User runs `bm bridge start` (or `bm start` with bridge auto-start)
2. bm resolves team -> profile -> bridges/<bridge-name>/
3. bm runs bridges/<bridge-name>/start.sh
   - start.sh launches the service (e.g., docker compose up -d)
   - start.sh prints JSON to stdout: { "url": "...", "admin_token": "..." }
4. bm captures stdout JSON, stores in bridge state (state.json or separate bridge-state.json)
5. bm runs bridges/<bridge-name>/health.sh with URL from step 4
   - Polls until healthy or timeout
6. bm runs bridges/<bridge-name>/configure.sh for each hired member
   - configure.sh creates bot user via REST API, returns credentials JSON
   - { "username": "arch-01", "user_id": "...", "auth_token": "..." }
7. Credentials stored per-member for injection into ralph.yml at launch time
```

**Member launch flow (modified):**

```
1. bm start iterates members (existing flow)
2. For each member, if bridge is active:
   a. Read member's bridge credentials from bridge state
   b. Set env vars: RALPH_ROBOT_BACKEND=rocketchat, RALPH_RC_URL=...,
      RALPH_RC_USER_ID=..., RALPH_RC_AUTH_TOKEN=...
3. Launch ralph as before, but ralph's RobotConfig now resolves backend
   from RALPH_ROBOT_BACKEND env var or config field
```

## Integration Points: New vs Modified

### New Files in BotMinter (`crates/bm/`)

| File | Purpose |
|------|---------|
| `src/bridge.rs` | Bridge lifecycle management: start, stop, health, configure |
| `src/commands/bridge.rs` | `bm bridge start/stop/status` CLI handlers |

### Modified Files in BotMinter

| File | Change |
|------|--------|
| `src/cli.rs` | Add `Bridge` subcommand with `start/stop/status` |
| `src/commands/mod.rs` | Add `pub mod bridge;` |
| `src/commands/start.rs` | Add optional bridge auto-start before member launch; pass bridge env vars to `launch_ralph()` |
| `src/commands/stop.rs` | Add optional bridge auto-stop after member stop |
| `src/config.rs` | Add `bridge` field to `TeamEntry` (bridge name + credentials storage) |
| `src/state.rs` | Add `BridgeRuntime` to track bridge process/container state |
| `src/profile.rs` | Read and validate `bridges/` directory in profile manifest |
| `src/workspace.rs` | Surface bridge connection info to workspace `ralph.yml` |

### New Files in Profile (`profiles/`)

| File | Purpose |
|------|---------|
| `profiles/<name>/bridges/rocketchat/bridge.yml` | Bridge definition: name, description, requirements |
| `profiles/<name>/bridges/rocketchat/start.sh` | Launch Rocket.Chat via Docker Compose |
| `profiles/<name>/bridges/rocketchat/stop.sh` | Stop and clean up containers |
| `profiles/<name>/bridges/rocketchat/health.sh` | Poll `/api/v1/info` endpoint |
| `profiles/<name>/bridges/rocketchat/configure.sh` | Create admin, bot users, channels via REST API |
| `profiles/<name>/bridges/rocketchat/docker-compose.yml` | Rocket.Chat + MongoDB compose file |
| `profiles/<name>/bridges/telegram/bridge.yml` | Telegram bridge definition (wraps existing support) |
| `profiles/<name>/bridges/telegram/start.sh` | No-op (Telegram is external SaaS) |
| `profiles/<name>/bridges/telegram/stop.sh` | No-op |
| `profiles/<name>/bridges/telegram/health.sh` | Call Telegram getMe API |
| `profiles/<name>/bridges/telegram/configure.sh` | Validate bot token, return credentials |

### New/Modified Files in Ralph Orchestrator (upstream)

| File | Change |
|------|--------|
| `crates/ralph-proto/src/robot.rs` | No change needed -- `RobotService` trait already abstract |
| `crates/ralph-core/src/config.rs` | Add `backend` field to `RobotConfig` (default: `telegram`), refactor `TelegramBotConfig` out of `RobotConfig` into backend-specific config |
| `crates/ralph-rocketchat/` (new crate) | New crate implementing `RobotService` for Rocket.Chat |
| `crates/ralph-cli/src/loop_runner.rs` | `create_robot_service()` dispatches on `config.robot.backend` |

### New Files in Repo Structure (ADRs/specs)

| File | Purpose |
|------|---------|
| `specs/adrs/README.md` | ADR index and conventions |
| `specs/adrs/0001-bridge-abstraction.md` | ADR for bridge plugin contract |
| `specs/adrs/0002-ralph-robot-backend.md` | ADR for Ralph robot abstraction approach |
| `specs/bridge-spec.md` | Knative-style spec for bridge lifecycle contract |

## Patterns to Follow

### Pattern 1: Shell-Script Lifecycle Contract (Bridge Plugin)

**What:** Bridges are defined as a directory of shell scripts with a YAML manifest. BotMinter calls them at defined lifecycle points. Scripts communicate results via stdout JSON.

**When:** Any new communication backend (Slack, Discord, Matrix, etc.)

**Why this over a Rust trait/plugin system:** Shell scripts match the existing skills pattern (composable shell scripts in profiles). Profile authors can add bridges without Rust compilation. Docker Compose for service management is natural from shell. The bridge is infrastructure, not hot-path code.

**Example `bridge.yml`:**
```yaml
name: rocketchat
display_name: "Rocket.Chat"
description: "Local Slack-like team communication"
version: "1.0.0"

# What the bridge needs from the operator
requires:
  - docker  # Will be checked before start

# Auto-start with bm start? (default: false)
auto_start: false

# Health check settings
health:
  interval_seconds: 5
  timeout_seconds: 60
  retries: 12
```

**Example `start.sh` output contract:**
```bash
#!/bin/bash
# start.sh -- launch Rocket.Chat
# Receives: BM_BRIDGE_DIR (path to this bridge's profile dir)
#           BM_TEAM_NAME, BM_WORKZONE
# Must print JSON to stdout on success:
#   { "url": "http://localhost:3000", "admin_user": "admin", "admin_password": "..." }
# Exit 0 on success, non-zero on failure (stderr for errors)

BRIDGE_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$BRIDGE_DIR"

docker compose up -d 2>&1 >/dev/null

# Wait for startup
for i in $(seq 1 30); do
  if curl -sf http://localhost:3000/api/v1/info > /dev/null 2>&1; then
    echo '{"url": "http://localhost:3000", "admin_user": "rcadmin", "admin_password": "rcadmin"}'
    exit 0
  fi
  sleep 2
done

echo "Rocket.Chat failed to start" >&2
exit 1
```

**Example `configure.sh` output contract:**
```bash
#!/bin/bash
# configure.sh -- create a bot user for a team member
# Receives: BM_BRIDGE_URL, BM_BRIDGE_ADMIN_USER, BM_BRIDGE_ADMIN_PASSWORD
#           BM_MEMBER_NAME, BM_MEMBER_ROLE, BM_TEAM_NAME
# Must print JSON to stdout:
#   { "username": "...", "user_id": "...", "auth_token": "..." }

# Login as admin
LOGIN=$(curl -s http://localhost:3000/api/v1/login \
  -d "user=$BM_BRIDGE_ADMIN_USER&password=$BM_BRIDGE_ADMIN_PASSWORD")
ADMIN_TOKEN=$(echo "$LOGIN" | jq -r '.data.authToken')
ADMIN_ID=$(echo "$LOGIN" | jq -r '.data.userId')

# Create bot user
PASSWORD=$(openssl rand -hex 16)
curl -s -H "X-Auth-Token: $ADMIN_TOKEN" -H "X-User-Id: $ADMIN_ID" \
  http://localhost:3000/api/v1/users.create \
  -d "{\"name\": \"$BM_MEMBER_NAME\", \"username\": \"$BM_MEMBER_NAME\",
       \"password\": \"$PASSWORD\", \"roles\": [\"bot\"]}" > /dev/null

# Login as bot to get auth token
BOT_LOGIN=$(curl -s http://localhost:3000/api/v1/login \
  -d "user=$BM_MEMBER_NAME&password=$PASSWORD")

echo "{\"username\": \"$BM_MEMBER_NAME\", \
\"user_id\": $(echo "$BOT_LOGIN" | jq '.data.userId'), \
\"auth_token\": $(echo "$BOT_LOGIN" | jq '.data.authToken')}"
```

### Pattern 2: Backend Dispatch in Ralph's RobotConfig

**What:** Add a `backend` field to `RobotConfig` that selects which `RobotService` implementation to instantiate. Keep `TelegramBotConfig` for backward compatibility but introduce a generic config mechanism.

**When:** Adding any new communication backend to Ralph.

**Example config change in `ralph-core/src/config.rs`:**
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RobotConfig {
    #[serde(default)]
    pub enabled: bool,

    /// Backend to use: "telegram" (default), "rocketchat"
    #[serde(default = "default_backend")]
    pub backend: String,

    pub timeout_seconds: Option<u64>,
    pub checkin_interval_seconds: Option<u64>,

    // Backend-specific configs (only the matching one is used)
    #[serde(default)]
    pub telegram: Option<TelegramBotConfig>,
    #[serde(default)]
    pub rocketchat: Option<RocketChatConfig>,
}

fn default_backend() -> String {
    "telegram".to_string()
}
```

**Dispatch in `ralph-cli/src/loop_runner.rs`:**
```rust
fn create_robot_service(
    config: &RalphConfig,
    ctx: &LoopContext,
) -> Option<Box<dyn ralph_proto::RobotService>> {
    match config.robot.backend.as_str() {
        "telegram" => create_telegram_service(config, ctx),
        "rocketchat" => create_rocketchat_service(config, ctx),
        other => {
            warn!(backend = other, "Unknown robot backend");
            None
        }
    }
}
```

### Pattern 3: Bridge State Management

**What:** Bridge runtime state lives alongside member runtime state in `~/.botminter/state.json`, using a new `bridges` field. Bridge process tracking follows the same PID-alive pattern as members.

**When:** Managing bridge lifecycle.

**Example state.json extension:**
```json
{
  "members": { ... },
  "bridges": {
    "my-team/rocketchat": {
      "bridge_type": "rocketchat",
      "started_at": "2026-03-08T10:00:00Z",
      "url": "http://localhost:3000",
      "container_id": "abc123",
      "member_credentials": {
        "superman-01": {
          "username": "superman-01",
          "user_id": "RC_USER_ID",
          "auth_token": "RC_AUTH_TOKEN"
        }
      }
    }
  }
}
```

### Pattern 4: Profile Bridge Directory Structure

**What:** Bridges live under `bridges/` in the profile directory, parallel to `skills/`, `roles/`, and `formations/`. Each bridge is a named directory with lifecycle scripts and a manifest.

**When:** Adding bridge support to any profile.

**Example profile structure:**
```
profiles/scrum-compact-telegram/
  botminter.yml          # Add: bridge: rocketchat (or telegram)
  bridges/
    rocketchat/
      bridge.yml         # Manifest
      start.sh           # Launch service
      stop.sh            # Tear down
      health.sh          # Health check
      configure.sh       # Per-member setup
      docker-compose.yml # Service definition
    telegram/
      bridge.yml
      start.sh           # No-op (SaaS)
      stop.sh            # No-op
      health.sh          # getMe validation
      configure.sh       # Token validation
  roles/
  skills/
  ...
```

### Pattern 5: Per-Agent Identity via configure.sh

**What:** Each team member gets their own bot user on the bridge. The `configure.sh` script is called once per member during bridge configuration, creating isolated bot accounts. This means messages from `arch-01` appear under `arch-01`'s name, not a shared bot.

**When:** Any bridge that supports multiple bot users.

**Example flow:**
```
bm bridge start
  -> start.sh (launch service)
  -> health.sh (wait for ready)
  -> for each member in team/members/:
       configure.sh (create bot user, get credentials)
       -> store credentials in bridge state
```

**Workspace injection:** When `bm start` launches a member, it reads that member's bridge credentials from bridge state and injects them as environment variables into the Ralph process:
```
RALPH_ROBOT_BACKEND=rocketchat
RALPH_RC_URL=http://localhost:3000
RALPH_RC_USER_ID=<member-specific-id>
RALPH_RC_AUTH_TOKEN=<member-specific-token>
RALPH_RC_CHANNEL=general
```

### Pattern 6: ADR and Spec Directory Convention

**What:** ADRs (Architecture Decision Records) live in `specs/adrs/` with sequential numbering. Knative-style specs live in `specs/` at the root level with descriptive names.

**When:** Any architectural decision or extensible interface definition.

**ADR format:**
```markdown
# ADR-NNNN: Title

**Status:** Proposed | Accepted | Deprecated | Superseded by ADR-XXXX
**Date:** YYYY-MM-DD
**Context:** [What prompted the decision]
**Decision:** [What was decided]
**Consequences:** [What follows from the decision]
```

**Spec format (Knative-style):**
```markdown
# Bridge Spec v0.1

## Overview
[What this spec defines]

## Terminology
[Key terms]

## Contract
[Required behaviors, lifecycle hooks, data formats]

## Conformance
[How to verify an implementation meets the spec]
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Embedding Rocket.Chat Logic in bm Binary

**What:** Putting Rocket.Chat-specific API calls directly in the Rust CLI code.

**Why bad:** Violates the bridge abstraction. Every new communication backend would require Rust code changes and recompilation. Profile authors lose the ability to customize bridge behavior.

**Instead:** All backend-specific logic lives in shell scripts within the profile's `bridges/` directory. The bm binary only knows about the generic lifecycle contract (start, stop, health, configure) and the JSON stdout protocol.

### Anti-Pattern 2: Single Shared Bot User for All Members

**What:** Using one Rocket.Chat bot account for all team members, differentiating by message prefix.

**Why bad:** Loses per-agent identity. Messages from different agents look the same. No per-agent permissions or channel access control. Harder to follow conversations.

**Instead:** `configure.sh` creates a unique bot user per member. Each Ralph instance authenticates as its own user.

### Anti-Pattern 3: Hardcoding Backend Selection in Ralph's Event Loop

**What:** Adding `if telegram { ... } else if rocketchat { ... }` branches in the event loop code.

**Why bad:** The `RobotService` trait already provides the abstraction. Adding conditionals bypasses it and creates coupling.

**Instead:** `create_robot_service()` is the single dispatch point. Backend crates implement the trait. The event loop only knows about `Box<dyn RobotService>`.

### Anti-Pattern 4: Bridge Scripts Writing to Config Files

**What:** Having `configure.sh` modify `ralph.yml` or `config.yml` directly.

**Why bad:** Race conditions, file format coupling, hard to debug. Scripts should be pure functions: inputs via env vars, outputs via stdout.

**Instead:** Scripts print JSON to stdout. The bm binary captures it and manages state storage. Environment variables inject the config into Ralph at launch time.

### Anti-Pattern 5: Coupling Bridge Lifecycle to Member Lifecycle

**What:** Starting/stopping the bridge inside `bm start`/`bm stop` with no independent control.

**Why bad:** Bridge startup is slow (Docker pull, service initialization). Operators may want the bridge running continuously while restarting individual members. Bridge health issues should not block member management.

**Instead:** `bm bridge start/stop/status` are independent commands. `bm start` can optionally auto-start the bridge (via `auto_start: true` in `bridge.yml`), but the bridge has its own lifecycle.

## Where Things Live: Config Model

### Profile Level (profile author controls)

```yaml
# botminter.yml additions
bridges:
  - name: rocketchat
    description: "Local Rocket.Chat for team communication"
  - name: telegram
    description: "Telegram bot for notifications"

# Default bridge for this profile (optional)
default_bridge: rocketchat
```

### Team Level (operator controls)

```yaml
# ~/.botminter/config.yml additions
teams:
  - name: my-team
    # ... existing fields ...
    bridge: rocketchat          # Which bridge to use
    credentials:
      gh_token: "..."
      telegram_bot_token: "..."  # Existing
      # Bridge credentials stored in bridge state, not here
```

### Runtime State

```yaml
# ~/.botminter/bridge-state.json (new file, separate from state.json)
# Or: add bridges field to existing state.json
{
  "my-team": {
    "bridge": "rocketchat",
    "url": "http://localhost:3000",
    "started_at": "2026-03-08T...",
    "pid_or_container": "...",
    "members": {
      "superman-01": {
        "user_id": "...",
        "auth_token": "...",
        "username": "superman-01"
      }
    }
  }
}
```

## Ralph Orchestrator Changes (Upstream Contribution)

### Current State

Ralph already has the right abstraction layer:
- `ralph-proto::RobotService` trait is fully backend-agnostic (HIGH confidence -- verified from source)
- `ralph-core::RobotConfig` is Telegram-specific: it has `telegram: Option<TelegramBotConfig>` hardcoded
- `ralph-cli::loop_runner::create_robot_service()` directly instantiates `TelegramService`
- `ralph-telegram` crate implements `RobotService` for Telegram via teloxide

### Required Changes

1. **`ralph-core/src/config.rs`**: Add `backend: String` field to `RobotConfig`, add `rocketchat: Option<RocketChatConfig>`, generalize `validate()` and `resolve_*()` methods to dispatch on backend
2. **`ralph-cli/src/loop_runner.rs`**: Change `create_robot_service()` from direct Telegram instantiation to backend dispatch
3. **New crate `ralph-rocketchat`**: Implement `RobotService` trait using Rocket.Chat REST API for sending messages and WebSocket/polling for receiving responses
4. **`ralph-cli/Cargo.toml`**: Add optional `ralph-rocketchat` dependency

### Backward Compatibility

The `backend` field defaults to `"telegram"`, so all existing `ralph.yml` files continue to work unchanged. The `telegram:` config section is preserved. This is purely additive.

## Build Order (Dependency-Driven)

The following order respects technical dependencies:

### Phase 1: ADRs + Specs (no code deps)
- Write ADR-0001 (bridge abstraction) and ADR-0002 (Ralph robot backend)
- Write bridge-spec.md (Knative-style lifecycle contract)
- These inform all subsequent implementation

### Phase 2: Ralph Robot Abstraction (upstream, no BotMinter deps)
- Add `backend` field to `RobotConfig`
- Refactor `create_robot_service()` dispatch
- Create `ralph-rocketchat` crate with `RobotService` implementation
- This can proceed in parallel with Phase 3

### Phase 3: Bridge Abstraction in BotMinter (profile + CLI)
- Add `bridges/` directory support to profile.rs
- Create `bridge.rs` module (lifecycle management)
- Add `bm bridge start/stop/status` commands
- Write Rocket.Chat bridge scripts (start.sh, stop.sh, health.sh, configure.sh)
- Write Telegram bridge scripts (wrap existing support)

### Phase 4: Integration (depends on Phases 2+3)
- Modify `bm start` to inject bridge credentials into Ralph env
- Modify `bm stop` to optionally stop bridge
- Add bridge auto-start support
- Per-agent identity wiring: configure.sh per member, credentials storage

### Phase 5: Profile Updates
- Create new profile variant with bridge support (or update existing profiles)
- Update botminter.yml schema for bridges
- Documentation

## Scalability Considerations

| Concern | 1 team (3 agents) | 5 teams (15 agents) | Production |
|---------|-------------------|---------------------|------------|
| Bridge processes | 1 Rocket.Chat container | 1 per team (or shared) | Managed Rocket.Chat instance |
| Bot users | 3 bot accounts | 15 bot accounts | Rate limit awareness needed |
| Docker resources | ~512MB RAM for RC | ~2.5GB RAM | Dedicated host or K8s |
| Bridge startup time | ~30s (Docker pull cached) | Sequential per team | Parallel with timeouts |
| Credential storage | In-memory + state file | Same, per-team namespace | Consider secrets manager |

## Sources

- Ralph Orchestrator source code at `/opt/workspace/ralph-orchestrator/` (HIGH confidence -- direct code review)
  - `ralph-proto::RobotService` trait: `crates/ralph-proto/src/robot.rs`
  - `RobotConfig`: `crates/ralph-core/src/config.rs` lines 1756-1773
  - `create_robot_service()`: `crates/ralph-cli/src/loop_runner.rs` lines 4513-4550
- BotMinter source code at `/home/sandboxed/workspace/botminter/crates/bm/src/` (HIGH confidence)
  - `start.rs`, `stop.rs`, `config.rs`, `profile.rs`, `workspace.rs`, `formation.rs`
- [Rocket.Chat Deploy with Docker](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose) (MEDIUM confidence)
- [Rocket.Chat Create User API](https://developer.rocket.chat/api/rest-api/endpoints/users/create) (MEDIUM confidence)
- [Rocket.Chat Bots Architecture](https://developer.rocket.chat/docs/bots-architecture) -- note: legacy bots deprecated, Apps-Engine recommended, but REST API bot users still work (MEDIUM confidence)
- [Rocket.Chat REST API](https://developer.rocket.chat/apidocs/rocketchat-api) (MEDIUM confidence)
- [Rocket.Chat Realtime API](https://developer.rocket.chat/apidocs/realtimeapi) (MEDIUM confidence)
