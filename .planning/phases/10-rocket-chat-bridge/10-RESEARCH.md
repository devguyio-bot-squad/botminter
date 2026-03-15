# Phase 10: Rocket.Chat Bridge - Research

**Researched:** 2026-03-10
**Domain:** Rocket.Chat bridge implementation (Podman Pod lifecycle, REST API identity/room management, bridge-type-aware credential injection)
**Confidence:** HIGH

## Summary

Phase 10 ships the Rocket.Chat bridge as the first local-type bridge implementation, proving the bridge abstraction works end-to-end. The implementation requires: (1) a spike proving Ralph + RC Podman Pod bidirectional messaging, (2) Justfile recipes for lifecycle (start/stop/health), identity (onboard/rotate/remove), and room (create/list) management via Rocket.Chat REST API, (3) bridge-type-aware credential injection in `bm start` (replacing hardcoded `RALPH_TELEGRAM_BOT_TOKEN`), (4) `inject_robot_enabled()` extension to write `RObot.rocketchat.*` fields into ralph.yml, and (5) an E2E test that replaces Telegram as the primary operator journey test.

All building blocks exist. Ralph's `ralph-rocketchat` crate provides the full REST client (129 tests). Ralph's `RobotConfig` already supports `RObot.rocketchat` with `server_url`, `bot_user_id`, `auth_token`, `room_id`, `operator_id` fields plus env var resolution (`RALPH_ROCKETCHAT_AUTH_TOKEN`, `RALPH_ROCKETCHAT_SERVER_URL`). The bridge spec already has a Rocket.Chat example in `bridge-spec.md`. The Telegram bridge provides a complete reference for the `bridge.yml` + `schema.json` + `Justfile` pattern.

**Primary recommendation:** Follow the spike-first approach from CONTEXT.md. Prove Ralph + RC works before integrating into the bridge structure. Use `podman pod` with RC + MongoDB containers and Rocket.Chat's REST API for all admin operations (user creation, token generation, channel creation).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **Spike-first approach:** First deliverable is a standalone spike proving Ralph + RC Podman Pod works end-to-end (boot, bot user, Ralph sends, human replies, Ralph receives). If Ralph's `ralph-rocketchat` crate doesn't work, phase stops.
- **User journeys (ordered):** 8 journeys from spike through full E2E test.
- **E2E testing:** Real Podman + RC. RC E2E test replaces Telegram as primary operator journey. Minimal Telegram E2E kept for external bridge coverage. Podman is required (no skip-if-missing).
- **Bridge-type-aware credential injection:** `bm start` must become bridge-type-aware. RC needs `RALPH_ROCKETCHAT_AUTH_TOKEN`, `RALPH_ROCKETCHAT_SERVER_URL` env vars, plus `RObot.rocketchat.*` fields in ralph.yml.
- **User-journey-driven plans:** Every plan framed as user journey. Verification = run the journey. No plan without a user scenario.

### Claude's Discretion
- Podman Pod topology (single pod vs separate containers, port mapping)
- MongoDB replica set initialization strategy
- Rocket.Chat admin bootstrap automation (REST API sequence)
- Data persistence model (ephemeral vs volumes for development use)
- Exact env var mapping between BotMinter credential store and Ralph's RC config fields
- Spike script location and structure

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| RC-01 | RC bridge ships bridge.yml, schema.json, and Justfile with all recipes | Telegram bridge provides exact template; bridge spec has RC example manifest |
| RC-02 | RC lifecycle recipes (start via Podman Pod, stop, health via REST API) | Podman Pod topology researched; RC REST API `/api/v1/info` for health check |
| RC-03 | Podman Pod for RC + MongoDB (single-node replica set) with auto-init | MongoDB `--replSet rs0` + `rs.initiate()` pattern documented; RC env vars for admin bootstrap |
| RC-04 | Per-agent bot identity via REST API (onboard creates user with bot role) | `users.create` + `users.createToken` REST API documented; curl examples verified |
| RC-05 | Team channel provisioned during `bm teams sync` | `channels.create` REST API; `provision_bridge()` already handles room creation via recipes |
| RC-06 | Bot commands work via RC using Ralph's command handler | Ralph `ralph-rocketchat` crate already implements commands; `RObot.rocketchat` config fields documented |
| RC-07 | RC bridge schema.json includes operator identity | Ralph `RobotConfig.operator_id` field exists; schema.json needs `operator_id` property |
</phase_requirements>

## Standard Stack

### Core
| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| Rocket.Chat | 7.x (latest stable) | Self-hosted team chat | Official Docker image at `registry.rocket.chat/rocketchat/rocket.chat` |
| MongoDB | 7.0+ | RC database backend | Required by Rocket.Chat; must be replica set |
| Podman | System-installed | Container runtime | Already used for tg-mock in E2E tests; required per CONTEXT.md |
| `just` | System-installed | Recipe runner | Bridge contract uses Justfile recipes |
| `curl` | System-installed | REST API calls from Justfile | Shell-level HTTP client for bridge recipes |

### Supporting
| Component | Purpose | When to Use |
|-----------|---------|-------------|
| `jq` | JSON processing in shell | Justfile recipes parsing REST API responses |
| `podman pod` | Pod management (grouped containers) | Grouping RC + MongoDB in a single network namespace |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `podman pod` | Separate containers + `podman network` | Pod is simpler: shared localhost, single start/stop unit |
| `curl` in Justfile | Rust HTTP client | Justfile recipes must be shell-based per bridge contract |
| Docker Hub RC image | `registry.rocket.chat` | RC publishes to own registry first; Docker Hub may lag |

## Architecture Patterns

### Recommended Bridge Directory Structure
```
profiles/scrum-compact/bridges/rocketchat/
  bridge.yml       # Local bridge manifest with lifecycle + identity + room
  schema.json      # Config schema with host, port, operator_id
  Justfile          # All recipes: start, stop, health, onboard, rotate, remove, room-create, room-list
```

Also add to:
```
profiles/scrum/bridges/rocketchat/
  bridge.yml       # Same structure (scrum profile also supports bridges)
  schema.json
  Justfile
```

### Pattern 1: Podman Pod Topology (Single Pod)

**What:** RC + MongoDB run in a single Podman Pod sharing a network namespace (localhost communication).
**When to use:** Local bridge lifecycle management.
**Recommendation:** Use a single pod named `bm-rocketchat-{team_name}`.

```bash
# Create pod with port mapping
podman pod create --name "bm-rc-${BM_TEAM_NAME}" -p "${RC_PORT}:3000"

# Start MongoDB in the pod with single-node replica set
podman run -d --pod "bm-rc-${BM_TEAM_NAME}" \
  --name "bm-rc-mongo-${BM_TEAM_NAME}" \
  docker.io/mongo:7.0 \
  mongod --replSet rs0 --oplogSize 128

# Wait for MongoDB, then init replica set
podman exec "bm-rc-mongo-${BM_TEAM_NAME}" \
  mongosh --eval 'rs.initiate({_id:"rs0",members:[{_id:0,host:"localhost:27017"}]})'

# Start Rocket.Chat in the pod
podman run -d --pod "bm-rc-${BM_TEAM_NAME}" \
  --name "bm-rc-app-${BM_TEAM_NAME}" \
  -e ROOT_URL="http://localhost:${RC_PORT}" \
  -e MONGO_URL="mongodb://localhost:27017/rocketchat?replicaSet=rs0" \
  -e MONGO_OPLOG_URL="mongodb://localhost:27017/local?replicaSet=rs0" \
  -e OVERWRITE_SETTING_Show_Setup_Wizard=completed \
  -e ADMIN_USERNAME=rcadmin \
  -e ADMIN_PASS=rcadmin123 \
  -e ADMIN_EMAIL=admin@botminter.local \
  registry.rocket.chat/rocketchat/rocket.chat:latest
```

**Key insight:** Using a pod means MongoDB is accessible at `localhost:27017` from RC's perspective -- no container networking complexity.

### Pattern 2: Admin Bootstrap Sequence (REST API)

**What:** After RC starts, authenticate as admin and use REST API for all operations.
**When to use:** Identity onboard, room creation, health checks.

```bash
# 1. Login as admin to get auth token
ADMIN_AUTH=$(curl -s http://localhost:${RC_PORT}/api/v1/login \
  -d "user=rcadmin&password=rcadmin123")
ADMIN_TOKEN=$(echo "$ADMIN_AUTH" | jq -r '.data.authToken')
ADMIN_ID=$(echo "$ADMIN_AUTH" | jq -r '.data.userId')

# 2. Create a bot user
curl -s -H "X-Auth-Token: $ADMIN_TOKEN" \
  -H "X-User-Id: $ADMIN_ID" \
  -H "Content-type: application/json" \
  http://localhost:${RC_PORT}/api/v1/users.create \
  -d "{\"name\":\"$USERNAME\",\"email\":\"${USERNAME}@bot.local\",\"password\":\"$(openssl rand -hex 16)\",\"username\":\"$USERNAME\",\"roles\":[\"bot\"]}"

# 3. Generate personal access token for the bot
curl -s -H "X-Auth-Token: $ADMIN_TOKEN" \
  -H "X-User-Id: $ADMIN_ID" \
  -H "Content-type: application/json" \
  http://localhost:${RC_PORT}/api/v1/users.createToken \
  -d "{\"username\":\"$USERNAME\"}"
```

### Pattern 3: Bridge-Type-Aware Credential Injection

**What:** `bm start` inspects the bridge type and passes the correct env vars to Ralph.
**When to use:** The `launch_ralph()` function in `start.rs`.

Currently hardcoded at `start.rs:290`:
```rust
if let Some(token) = telegram_token {
    cmd.env("RALPH_TELEGRAM_BOT_TOKEN", token);
}
```

Must become:
```rust
// Bridge-type-aware env var injection
match bridge_type.as_deref() {
    Some("rocketchat") => {
        if let Some(token) = member_token {
            cmd.env("RALPH_ROCKETCHAT_AUTH_TOKEN", token);
        }
        if let Some(url) = service_url {
            cmd.env("RALPH_ROCKETCHAT_SERVER_URL", url);
        }
    }
    _ => {
        // Default: Telegram (backward compat)
        if let Some(token) = member_token {
            cmd.env("RALPH_TELEGRAM_BOT_TOKEN", token);
        }
    }
}
```

Also, `daemon.rs:713` has the same hardcoded pattern and needs the same fix.

### Pattern 4: inject_robot_enabled Extension

**What:** `workspace.rs:inject_robot_enabled()` currently only sets `RObot.enabled`. For RC, it must also set `RObot.rocketchat.bot_user_id`, `RObot.rocketchat.room_id`, and `RObot.rocketchat.server_url`.
**When to use:** During `bm teams sync` when bridge type is rocketchat.

The function signature should be extended or a new function created:
```rust
pub fn inject_robot_config(
    ralph_yml_path: &Path,
    member_has_credentials: bool,
    bridge_type: Option<&str>,
    bridge_config: Option<&BridgeConfig>,  // Contains user_id, room_id, service_url
) -> Result<()>
```

For rocketchat bridge, inject:
```yaml
RObot:
  enabled: true
  timeout_seconds: 300
  rocketchat:
    bot_user_id: "<from bridge identity>"
    room_id: "<from bridge room>"
    server_url: "<from bridge state service_url>"
  operator_id: "<from schema.json config>"
```

Secrets (`auth_token`) stay as env vars per ADR-0003.

### Anti-Patterns to Avoid
- **Hardcoding bridge-specific behavior in core modules:** Use the bridge manifest's `bridge_type` field to dispatch, not string checks scattered through code.
- **Writing secrets to ralph.yml:** Per ADR-0003, tokens go in env vars only. Only non-secret config (user IDs, URLs, room IDs) goes in ralph.yml.
- **Polling for RC startup with fixed sleep:** Use health check loop with backoff against `/api/v1/info` endpoint instead.
- **Skipping replica set init:** RC requires MongoDB replica set even for single-node. Forgetting `rs.initiate()` causes RC to fail silently or error on oplog access.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Bot user management | Custom user management | RC REST API `users.create` + `users.createToken` | RC handles auth, roles, permissions |
| Channel management | Custom channel logic | RC REST API `channels.create` | RC handles membership, permissions |
| Bot messaging | Custom WebSocket client | Ralph's `ralph-rocketchat` crate | Already has 129 tests, handles polling, state |
| Container lifecycle | Custom container management | `podman pod` commands | Handles network, start/stop as unit |
| JSON parsing in shell | Manual grep/sed | `jq` | Robust JSON parsing in Justfile recipes |
| Health checking | Custom TCP probing | RC `/api/v1/info` endpoint | Returns server status, version, build info |

**Key insight:** The RC REST API and Ralph's existing crate handle all the complex parts. The bridge Justfile is pure glue -- calling REST endpoints and formatting output for config exchange.

## Common Pitfalls

### Pitfall 1: MongoDB Replica Set Not Initialized
**What goes wrong:** Rocket.Chat starts but fails to function (no oplog tailing).
**Why it happens:** MongoDB requires explicit `rs.initiate()` even for single-node replica sets. The container starts, but RC can't use change streams without it.
**How to avoid:** Include an explicit wait-and-init step in the `start` recipe: wait for MongoDB to accept connections, then run `rs.initiate()`, then start RC.
**Warning signs:** RC logs show "MongoError: not master" or oplog-related errors.

### Pitfall 2: Race Condition on RC Startup
**What goes wrong:** REST API calls fail because RC isn't ready yet.
**Why it happens:** RC takes 15-30 seconds to fully boot (initial setup, database migrations, admin user creation). Calling the API immediately after `podman run` fails.
**How to avoid:** Poll `/api/v1/info` in a loop with backoff (max 60 seconds). Only proceed with admin login after this endpoint returns 200.
**Warning signs:** "Connection refused" or 502 errors from REST API calls.

### Pitfall 3: Admin User Already Exists on Re-Start
**What goes wrong:** RC's `ADMIN_USERNAME`/`ADMIN_PASS` env vars only create the admin on first boot. On subsequent starts (pod restart, re-creation), the database may already have the admin user.
**Why it happens:** These env vars are "first-run only" -- they don't reset the admin password on restart.
**How to avoid:** Use ephemeral data (no volumes) for development/testing. For the E2E test, create a fresh pod each time. The `start` recipe should be idempotent -- if admin login succeeds, skip creation.
**Warning signs:** Admin login returns 401 after pod recreation with existing data.

### Pitfall 4: Hardcoded Telegram Env Var in Multiple Places
**What goes wrong:** Fixing `start.rs` but forgetting `daemon.rs` leaves the daemon path broken for RC.
**Why it happens:** The hardcoded `RALPH_TELEGRAM_BOT_TOKEN` appears in both `start.rs:290` and `daemon.rs:713`.
**How to avoid:** Search for all occurrences of `RALPH_TELEGRAM_BOT_TOKEN` and make them bridge-type-aware.
**Warning signs:** `bm start` works but `bm start --formation` (daemon path) still uses Telegram env var.

### Pitfall 5: Config Exchange Whitespace/Formatting
**What goes wrong:** `config.json` written by Justfile recipes has leading whitespace or trailing newlines, causing JSON parse failures.
**Why it happens:** Heredocs in Justfile recipes may include indentation.
**How to avoid:** Use `jq` to format JSON output, or strip whitespace explicitly. Test recipe output by piping through `jq .` to validate.
**Warning signs:** "Failed to parse config.json" errors from BotMinter.

### Pitfall 6: Port Conflicts in E2E Tests
**What goes wrong:** RC pod fails to bind port 3000 because another test or service is using it.
**Why it happens:** Hardcoded port in Justfile recipes.
**How to avoid:** Use dynamic port allocation in E2E tests (same pattern as `find_free_port()` in `telegram.rs`). Pass port as recipe argument or env var.
**Warning signs:** "address already in use" from podman.

## Code Examples

### RC bridge.yml (verified against bridge spec)
```yaml
# Source: .planning/specs/bridge/bridge-spec.md local bridge example
apiVersion: botminter.dev/v1alpha1
kind: Bridge
metadata:
  name: rocketchat
  displayName: "Rocket.Chat"
  description: "Self-hosted team chat via Rocket.Chat and MongoDB"

spec:
  type: local
  configSchema: schema.json

  lifecycle:
    start: start
    stop: stop
    health: health

  identity:
    onboard: onboard
    rotate-credentials: rotate
    remove: remove

  room:
    create: room-create
    list: room-list

  configDir: "$BRIDGE_CONFIG_DIR"
```

### RC schema.json
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Rocket.Chat Bridge Configuration",
  "type": "object",
  "properties": {
    "host": {
      "type": "string",
      "description": "Rocket.Chat server hostname",
      "default": "localhost"
    },
    "port": {
      "type": "integer",
      "description": "Rocket.Chat server port",
      "default": 3000
    },
    "admin_username": {
      "type": "string",
      "description": "Admin username for RC server management",
      "default": "rcadmin"
    },
    "admin_password": {
      "type": "string",
      "description": "Admin password for RC server management"
    },
    "operator_id": {
      "type": "string",
      "description": "Rocket.Chat user ID of the team operator (human-in-the-loop)"
    }
  },
  "required": ["host"]
}
```

### Ralph RObot.rocketchat config shape (from ralph-core/src/config.rs)
```yaml
# Source: /opt/workspace/ralph-orchestrator/crates/ralph-core/src/config.rs
RObot:
  enabled: true
  timeout_seconds: 300
  rocketchat:
    server_url: "http://localhost:3000"   # or RALPH_ROCKETCHAT_SERVER_URL env
    auth_token: null                       # via RALPH_ROCKETCHAT_AUTH_TOKEN env ONLY
    bot_user_id: "user123"                # from onboard recipe output
    room_id: "room456"                    # from room-create recipe output
  operator_id: "operator789"              # from schema.json config
```

### Env var mapping: BotMinter credential store -> Ralph
```
BotMinter credential store key: member_name
  -> keyring: botminter.{team}.rocketchat / {member_name}
  -> env var priority: BM_BRIDGE_TOKEN_{MEMBER_NAME_UPPER}

Ralph expects:
  RALPH_ROCKETCHAT_AUTH_TOKEN  <- the stored credential (auth token from onboard)
  RALPH_ROCKETCHAT_SERVER_URL  <- from bridge-state.json service_url field

Ralph config (ralph.yml, non-secret):
  RObot.rocketchat.bot_user_id  <- from bridge identity user_id
  RObot.rocketchat.room_id      <- from bridge state rooms[0].room_id
  RObot.operator_id             <- from schema.json operator_id
```

### Health check recipe pattern
```bash
# Source: RC REST API docs
health:
    #!/usr/bin/env bash
    set -euo pipefail
    RC_URL="http://localhost:${RC_PORT:-3000}"
    response=$(curl -sf "$RC_URL/api/v1/info" 2>/dev/null) || {
        echo "Rocket.Chat is not responding at $RC_URL" >&2
        exit 1
    }
    echo "Rocket.Chat is healthy" >&2
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hardcoded `RALPH_TELEGRAM_BOT_TOKEN` | Bridge-type-aware env var injection | Phase 10 | Enables RC and future bridge types |
| `inject_robot_enabled()` (bool only) | `inject_robot_config()` (full RC config) | Phase 10 | ralph.yml gets bridge-specific non-secret config |
| Telegram-only E2E operator journey | RC as primary + minimal Telegram test | Phase 10 | Proves local bridge lifecycle in E2E |
| RC Docker images on Docker Hub | `registry.rocket.chat` official registry | 2024+ | More current images, first-party |

## Open Questions

1. **RC admin password persistence across pod restarts**
   - What we know: `ADMIN_USERNAME`/`ADMIN_PASS` env vars only work on first boot. Ephemeral data avoids the issue.
   - What's unclear: Whether `start` recipe should always create fresh pods (delete old + create new) or attempt to reuse.
   - Recommendation: Default to ephemeral (always fresh). This is simplest for alpha. Add volume support later if needed.

2. **`users.createToken` vs Personal Access Token**
   - What we know: `users.createToken` creates temporary session tokens. Ralph uses `X-Auth-Token` + `X-User-Id` headers.
   - What's unclear: Whether session tokens expire during long Ralph sessions.
   - Recommendation: Use `users.createToken` for bot auth tokens. If expiration becomes an issue, switch to Personal Access Tokens (requires enabling in RC admin settings). The spike will reveal if this is a problem.

3. **Spike location and structure**
   - What we know: Must be standalone, outside bridge structure, proves full roundtrip.
   - Recommendation: Place at `.planning/spikes/rc-podman-spike/` with a shell script and README. Not committed to main crate.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | libtest-mimic (custom E2E harness) + cargo test (unit/integration) |
| Config file | `crates/bm/tests/e2e/main.rs` |
| Quick run command | `just unit` |
| Full suite command | `just test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RC-01 | bridge.yml + schema.json + Justfile conform to spec | conformance | `cargo test -p bm conformance` | Wave 0 (new bridge files + conformance entry) |
| RC-02 | start/stop/health lifecycle via Podman Pod | e2e | `just e2e` (RC scenario) | Wave 0 (new E2E scenario) |
| RC-03 | Podman Pod boots RC + MongoDB with replica set | e2e | `just e2e` (RC scenario, start case) | Wave 0 |
| RC-04 | onboard creates user + returns credentials | e2e | `just e2e` (RC scenario, identity case) | Wave 0 |
| RC-05 | team channel provisioned during sync | e2e | `just e2e` (RC scenario, sync case) | Wave 0 |
| RC-06 | bot commands work via RC | manual/spike | Spike script validates bidirectional messaging | Wave 0 (spike) |
| RC-07 | schema.json includes operator_id | conformance + unit | `cargo test -p bm` (schema validation) | Wave 0 |

### Sampling Rate
- **Per task commit:** `just unit`
- **Per wave merge:** `just test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `profiles/scrum-compact/bridges/rocketchat/` -- bridge.yml + schema.json + Justfile
- [ ] `profiles/scrum/bridges/rocketchat/` -- same bridge files for scrum profile
- [ ] E2E scenario for RC operator journey (replaces Telegram as primary)
- [ ] Conformance test entry for rocketchat bridge
- [ ] Spike script proving Ralph + RC Podman Pod bidirectional messaging
- [ ] Bridge-type-aware `launch_ralph()` in start.rs and daemon.rs
- [ ] Extended `inject_robot_config()` or updated `inject_robot_enabled()` in workspace.rs

## Sources

### Primary (HIGH confidence)
- Rocket.Chat source code at `/opt/workspace/rocket-chat/` -- full server source, REST API definitions, env var handling, Docker/Podman configs. Use for verifying exact API endpoints, env var names, and bootstrap sequences rather than relying on external docs.
- Ralph `ralph-rocketchat` crate at `/opt/workspace/ralph-orchestrator/crates/ralph-rocketchat/` -- full REST client, service, daemon, handler, state manager
- Ralph `RobotConfig` at `/opt/workspace/ralph-orchestrator/crates/ralph-core/src/config.rs` -- `RocketChatConfig` struct with `server_url`, `bot_user_id`, `auth_token`, `room_id` fields
- BotMinter bridge spec at `.planning/specs/bridge/bridge-spec.md` -- complete contract definition
- Existing Telegram bridge at `profiles/scrum-compact/bridges/telegram/` -- reference implementation
- BotMinter `bridge.rs` -- credential store, provision_bridge(), invoke_recipe()
- BotMinter `start.rs` -- hardcoded Telegram env var at lines 113-114, 290
- BotMinter `workspace.rs` -- inject_robot_enabled() at line 532

### Secondary (MEDIUM confidence — can be verified against local RC source)
- [Rocket.Chat REST API - Create User](https://developer.rocket.chat/apidocs/create-user) -- users.create endpoint (verify against `/opt/workspace/rocket-chat/apps/meteor/app/api/`)
- [Rocket.Chat REST API - Create Users Token](https://developer.rocket.chat/reference/api/rest-api/endpoints/user-management/users-endpoints/create-users-token) -- users.createToken endpoint
- [Rocket.Chat REST API - Create Channel](https://developer.rocket.chat/reference/api/rest-api/endpoints/rooms/channels-endpoints/create-channel) -- channels.create endpoint
- [Rocket.Chat Docker Deployment](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose) -- env vars for admin bootstrap (verify against `/opt/workspace/rocket-chat/docker-compose-local.yml`)
- [Rocket.Chat Podman Deployment](https://docs.rocket.chat/docs/deploy-with-podman) -- Podman-specific guidance
- [Rocket.Chat Environment Variables](https://docs.rocket.chat/docs/manage-settings-using-environmental-variables) -- OVERWRITE_SETTING_* pattern (verify against RC source settings)

### Tertiary (LOW confidence)
- RC image version 7.9.7 as "latest stable" -- should be verified at implementation time by checking `registry.rocket.chat`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all components verified in existing codebase (Ralph crate, bridge spec, Telegram reference)
- Architecture: HIGH -- patterns derived from existing code (bridge.rs, workspace.rs, start.rs) plus verified RC REST API
- Pitfalls: HIGH -- identified from code analysis (hardcoded env vars, replica set requirement) and RC docs (admin bootstrap timing)

**Research date:** 2026-03-10
**Valid until:** 2026-04-10 (stable domain, no fast-moving dependencies)
