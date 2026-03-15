# Bridge Plugin Specification

**Version:** v1alpha1
**Status:** Draft
**Date:** 2026-03-08

## Conformance

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be
interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## Overview

A bridge is a pluggable communication integration for BotMinter teams. Bridges
connect team members to external messaging platforms (Telegram, Slack, Discord)
or self-hosted services (Rocket.Chat, Mattermost). This specification defines
the contract that bridge implementations MUST satisfy so that BotMinter can
manage them uniformly.

This spec is for bridge implementors. A developer reading this document SHOULD
be able to build a conformant bridge without reading BotMinter source code.

## Terminology

| Term | Definition |
|------|------------|
| **Bridge** | A pluggable communication backend that integrates a messaging platform with BotMinter. |
| **Local bridge** | A bridge where BotMinter manages the full service lifecycle (start, stop, health). The service runs on the same infrastructure as BotMinter. |
| **External bridge** | A bridge where the messaging service runs independently (SaaS). BotMinter manages identity only. |
| **Bridge manifest** | The `bridge.yml` file at the bridge root directory that declares all integration points. |
| **Bridge schema** | The `schema.json` file that defines the shape of bridge-specific configuration values. |
| **Config exchange** | The mechanism by which bridge commands communicate configuration back to BotMinter, using file-based output via `$BRIDGE_CONFIG_DIR`. |
| **Identity** | A user or bot account on the messaging platform, managed through bridge identity commands. |
| **Lifecycle** | The start/stop/health cycle of a locally-managed bridge service. |

## Bridge Types

### Local Bridge

A local bridge provides full lifecycle management. BotMinter starts and stops
the service and monitors its health. The bridge implementation controls the
service process.

Examples: Rocket.Chat (self-hosted), Mattermost (self-hosted).

A local bridge MUST implement both lifecycle commands and identity commands.

### External Bridge

An external bridge provides identity management only. The messaging service
runs independently and is not managed by BotMinter.

Examples: Telegram (SaaS), Slack (SaaS), Discord (SaaS).

An external bridge MUST implement identity commands. An external bridge
MUST NOT declare lifecycle commands.

## Bridge Manifest (`bridge.yml`)

A conformant bridge MUST include a `bridge.yml` file at the bridge root
directory. The manifest declares all integration points between the bridge
and BotMinter.

### Required Fields

| Field | Type | Constraint |
|-------|------|------------|
| `apiVersion` | string | MUST be `botminter.dev/v1alpha1` |
| `kind` | string | MUST be `Bridge` |
| `metadata.name` | string | MUST be non-empty, matching `[a-z][a-z0-9-]*` |
| `metadata.displayName` | string | SHOULD be present; human-readable name |
| `metadata.description` | string | SHOULD be present; brief description |
| `spec.type` | string | MUST be `local` or `external` |
| `spec.configSchema` | string | MUST reference a valid JSON Schema file relative to bridge root |
| `spec.lifecycle` | object | MUST be present when `spec.type` is `local`; MUST NOT be present when `spec.type` is `external` |
| `spec.lifecycle.start` | string | MUST be a Justfile recipe name (local only) |
| `spec.lifecycle.stop` | string | MUST be a Justfile recipe name (local only) |
| `spec.lifecycle.health` | string | MUST be a Justfile recipe name (local only) |
| `spec.identity` | object | MUST be present for all bridge types |
| `spec.identity.onboard` | string | MUST be a Justfile recipe name |
| `spec.identity.rotate-credentials` | string | MUST be a Justfile recipe name |
| `spec.identity.remove` | string | MUST be a Justfile recipe name |
| `spec.room` | object | MAY be present for bridges that support room/channel management |
| `spec.room.create` | string | MUST be a Justfile recipe name (if `spec.room` is present) |
| `spec.room.list` | string | MUST be a Justfile recipe name (if `spec.room` is present) |
| `spec.configDir` | string | MUST specify the config exchange directory; typically `$BRIDGE_CONFIG_DIR` |

### Local Bridge Example

````yaml
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
````

See [examples/bridge.yml](examples/bridge.yml) for the complete reference.

### External Bridge Example

````yaml
apiVersion: botminter.dev/v1alpha1
kind: Bridge
metadata:
  name: telegram
  displayName: "Telegram"
  description: "Telegram bot integration (external service)"

spec:
  type: external
  configSchema: schema.json

  identity:
    onboard: onboard
    rotate-credentials: rotate
    remove: remove

  configDir: "$BRIDGE_CONFIG_DIR"
````

See [examples/bridge-external.yml](examples/bridge-external.yml) for the complete reference.

## Config Schema (`schema.json`)

Each bridge MUST include a `schema.json` file referenced by `spec.configSchema`
in the bridge manifest. This file defines the shape of bridge-specific
configuration values that BotMinter validates before invoking any bridge command.

### Requirements

- The schema MUST be a valid JSON Schema. Draft 2020-12 is RECOMMENDED.
- The schema MUST have `"type": "object"` at the root level.
- The schema MUST define a `"properties"` object for bridge-specific configuration fields.
- The schema SHOULD define a `"required"` array listing mandatory configuration values.
- The schema MAY include additional JSON Schema features such as `"default"` values, `"description"` annotations, and type constraints.

### Example

````json
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
    }
  },
  "required": ["host"]
}
````

See [examples/schema.json](examples/schema.json) for the complete reference.

BotMinter validates bridge configuration against this schema before invoking
any bridge command. Invalid configuration MUST result in an error before the
command is called.

## Lifecycle Commands

Lifecycle commands apply to local bridges only. A bridge of type `local` MUST
declare all three lifecycle commands in `spec.lifecycle`. A bridge of type
`external` MUST NOT declare lifecycle commands.

All commands are Justfile recipes. The recipe names in `bridge.yml` are
references to recipes defined in the bridge's `Justfile`. There are no
hardcoded command names in the contract -- bridges choose their own recipe names.

### `start`

- MUST start the bridge service.
- MUST write service configuration to `$BRIDGE_CONFIG_DIR/config.json` on success.
- The output JSON MUST include at minimum a `url` field with the service endpoint.

**Expected output** (`$BRIDGE_CONFIG_DIR/config.json`):
````json
{
  "url": "http://localhost:3000",
  "status": "running"
}
````

### `stop`

- MUST stop the bridge service.
- SHOULD clean up resources (containers, temp files, etc.).
- SHOULD write status to stderr for diagnostic purposes.

### `health`

- MUST exit with code 0 if the service is healthy.
- MUST exit with a non-zero code if the service is unhealthy.
- SHOULD write health status to stderr for diagnostic purposes.

## Identity Commands

Identity commands apply to all bridge types. Every conformant bridge MUST
declare all three identity commands in `spec.identity`.

All commands are Justfile recipes. The username is passed as a recipe argument.

The semantics of identity commands differ by bridge type:

- **Local bridges** provision identities autonomously. The bridge creates
  accounts and generates credentials without operator-supplied tokens.
- **External bridges** accept pre-existing credentials supplied by the
  operator. The bridge validates and registers tokens but MUST NOT create
  accounts on the external platform.

### `onboard <username>`

**Local bridges:**

- MUST create a user or bot account on the bridge platform.
- MUST generate credentials for the new account.
- MUST write credentials to `$BRIDGE_CONFIG_DIR/config.json`.
- The output JSON MUST include at minimum `username`, `user_id`, and `token` fields.

**External bridges:**

- MUST accept a pre-existing token via the environment variable
  `BM_BRIDGE_TOKEN_{USERNAME}` (username uppercased, hyphens replaced with
  underscores).
- MUST validate the token format (bridge-specific validation).
- MUST write the validated credentials to `$BRIDGE_CONFIG_DIR/config.json`.
- MUST NOT create accounts on the external platform.
- MUST exit with a non-zero code if `BM_BRIDGE_TOKEN_{USERNAME}` is not set.
- The output JSON MUST include at minimum `username`, `user_id`, and `token` fields.

**Expected output** (`$BRIDGE_CONFIG_DIR/config.json`):
````json
{
  "username": "agent-alice",
  "user_id": "abc123",
  "token": "generated-or-validated-auth-token"
}
````

### `rotate-credentials <username>`

- MUST generate new credentials for an existing user.
- MUST write new credentials to `$BRIDGE_CONFIG_DIR/config.json`.
- The output JSON MUST include at minimum `username`, `user_id`, and `token` fields (with the new token).

**Expected output** (`$BRIDGE_CONFIG_DIR/config.json`):
````json
{
  "username": "agent-alice",
  "user_id": "abc123",
  "token": "new-rotated-auth-token"
}
````

### `remove <username>`

- MUST remove the user from the bridge platform.
- SHOULD clean up associated resources (channels, permissions, etc.).
- SHOULD write status to stderr for diagnostic purposes.

## Room Commands

Room commands are OPTIONAL. A bridge MAY declare a `spec.room` section to
support room/channel management. If `spec.room` is present, both `create`
and `list` recipes MUST be declared.

### `create <room-name>`

- MUST create a room or channel on the bridge platform.
- MUST write room details to `$BRIDGE_CONFIG_DIR/config.json`.
- The output JSON MUST include at minimum `name` and `room_id` fields.

**Expected output** (`$BRIDGE_CONFIG_DIR/config.json`):
````json
{
  "name": "my-team",
  "room_id": "ch-abc123"
}
````

### `list`

- MUST list all rooms/channels managed by the bridge.
- MUST write room list to `$BRIDGE_CONFIG_DIR/config.json`.
- The output JSON MUST be an array of objects with at minimum `name` and
  `room_id` fields.

**Expected output** (`$BRIDGE_CONFIG_DIR/config.json`):
````json
[
  { "name": "my-team", "room_id": "ch-abc123" }
]
````

## Config Exchange

All bridge commands that produce configuration MUST write output to
`$BRIDGE_CONFIG_DIR/config.json`. Commands MUST NOT write configuration
to stdout.

### Rationale

File-based exchange avoids stdout corruption from diagnostic output. Bridge
commands MAY write diagnostic messages, progress indicators, or warnings to
stdout or stderr freely -- the configuration output channel is separate.

### Protocol

1. BotMinter sets the `$BRIDGE_CONFIG_DIR` environment variable before invoking
   any bridge command.
2. The bridge command creates the directory if it does not exist (`mkdir -p`).
3. The bridge command writes a JSON object to `$BRIDGE_CONFIG_DIR/config.json`.
4. BotMinter reads the JSON file after the command completes.

### Credential Persistence

Credential persistence is BotMinter's responsibility, not the bridge's. After
BotMinter reads credentials from `$BRIDGE_CONFIG_DIR/config.json`, it stores
them according to the active formation's credential backend (e.g., system
keyring for local formations, Kubernetes Secrets for K8s formations). The
bridge MUST NOT assume any particular storage mechanism — its only
responsibility is to produce credentials via config exchange.

### Output Shapes

Each command category produces a specific JSON shape:

| Command | Required Fields | Optional Fields |
|---------|----------------|-----------------|
| `start` | `url` | `status`, bridge-specific fields |
| `onboard` | `username`, `user_id`, `token` | bridge-specific fields |
| `rotate-credentials` | `username`, `user_id`, `token` | bridge-specific fields |
| `room create` | `name`, `room_id` | bridge-specific fields |
| `room list` | array of `name`, `room_id` | bridge-specific fields |
| `stop` | (no file output required) | |
| `health` | (no file output required) | |
| `remove` | (no file output required) | |

## Environment Variables

BotMinter sets the following environment variables before invoking any bridge
command:

| Variable | Description |
|----------|------------|
| `$BRIDGE_CONFIG_DIR` | Directory where commands MUST write `config.json` output. Set by BotMinter. |
| `$BM_TEAM_NAME` | The team name, providing context for multi-team setups. Set by BotMinter. |

Bridge-specific environment variables (e.g., API keys, service URLs) are
defined in the bridge's `schema.json` and resolved by BotMinter from team
configuration before command invocation.

## Directory Structure

A conformant bridge implementation MUST contain the following files at its
root directory:

```
bridge-name/
  bridge.yml       # Bridge manifest (REQUIRED)
  schema.json      # Config schema (REQUIRED)
  Justfile          # Command recipes (REQUIRED)
```

Additional support files (scripts, templates, Docker Compose files, etc.) are
permitted. The three required files are the contract surface between the bridge
and BotMinter.

### Profile Integration

Within a BotMinter profile, bridge implementations live under a `bridges/`
directory:

```
profiles/{profile-name}/
  bridges/
    {bridge-name}/
      bridge.yml
      schema.json
      Justfile
```

Profiles declare their supported bridges in `botminter.yml` under a `bridges:`
key. Operators select a bridge (or none) during `bm init`. This is a BotMinter
deployment concern — bridge implementors need only ensure their files conform
to the directory structure above.

## Non-Goals

This specification does NOT define:

- **Command invocation mechanisms** -- how BotMinter discovers and calls Justfile recipes is an implementation concern.
- **Error handling policies** -- retry logic, error codes, and failure modes are implementation-specific.
- **Retry and backoff logic** -- how BotMinter retries failed commands is outside this contract.
- **Health check intervals** -- how often BotMinter calls the health recipe is a runtime configuration concern.
- **Runtime state management** -- how BotMinter tracks bridge state (started, stopped, degraded) is internal.
- **Multi-bridge coordination** -- running multiple bridges simultaneously is an orchestration concern.
- **Bridge discovery and registration** -- how BotMinter finds and loads bridge implementations is an implementation concern.
- **Bridge versioning and upgrades** -- how bridges are updated is outside the v1alpha1 scope.

These concerns are addressed in subsequent implementation phases (Phase 8+).

## Conformance

A bridge is conformant with this specification if:

1. A `bridge.yml` file exists at the bridge root and parses as valid YAML
   containing all required fields as defined in the Bridge Manifest section.
2. The `apiVersion` field is `botminter.dev/v1alpha1` and the `kind` field
   is `Bridge`.
3. The `spec.type` field is either `local` or `external`.
4. If `spec.type` is `local`, the `spec.lifecycle` section is present with
   `start`, `stop`, and `health` recipe references.
5. If `spec.type` is `external`, no `spec.lifecycle` section is present.
6. The `spec.identity` section is present with `onboard`,
   `rotate-credentials`, and `remove` recipe references.
7. If `spec.room` is present, it contains `create` and `list` recipe
   references.
8. A `schema.json` file exists at the path referenced by `spec.configSchema`
   and parses as valid JSON Schema with `"type": "object"` and `"properties"`.
9. A `Justfile` exists at the bridge root containing recipes matching all
   declared command names.

Conformance is structural -- it validates that the required files and fields
exist with correct types. Runtime behavior testing (do commands actually work?)
is separate from structural conformance.

## References

- [examples/bridge.yml](examples/bridge.yml) -- Complete reference bridge manifest (local type)
- [examples/bridge-external.yml](examples/bridge-external.yml) -- Complete reference bridge manifest (external type)
- [examples/schema.json](examples/schema.json) -- Complete reference config schema
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) -- Key words for use in RFCs to indicate requirement levels
