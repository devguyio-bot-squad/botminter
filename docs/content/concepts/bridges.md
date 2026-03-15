# Bridges

A **bridge** is a pluggable communication integration that connects your team members to a messaging platform. Each bridge is a self-contained directory with a YAML manifest, JSON config schema, and Justfile recipes -- no Rust code or recompilation needed.

Bridges give your agents presence on a chat platform. Each team member gets their own bot user and token, so messages in the team channel are attributable to individual agents.

## Bridge types

BotMinter supports two categories of bridges:

### External bridges

An external bridge integrates with a SaaS messaging platform that runs independently. BotMinter manages identity only -- it does not start, stop, or monitor the service.

**Example:** Telegram. The operator creates bot users via @BotFather, supplies tokens during `bm hire`, and BotMinter injects those credentials at launch time.

External bridges:

- Accept operator-supplied tokens (one per member)
- Validate and register tokens but do not create accounts
- Have no lifecycle management (no start/stop/health)

### Local (managed) bridges

A local bridge manages the full service lifecycle. BotMinter starts the service, provisions identities automatically, and monitors health. The operator supplies no tokens -- the bridge creates them.

**Example:** Rocket.Chat (self-hosted). BotMinter starts the service, creates bot accounts, generates tokens, and monitors health.

Local bridges:

- Auto-provision per-member identities (create user, generate token)
- Manage service lifecycle (start, stop, health check)
- Run on the same infrastructure as BotMinter

## Per-member identity model

Every hired team member gets their own bot user and token on the bridge, regardless of bridge type. This enables per-agent traceability in chat channels -- you can see which agent posted which message.

For external bridges, the operator creates one bot per member (e.g., one Telegram bot per agent via @BotFather). For local bridges, BotMinter creates the bot accounts automatically during `bm teams sync --bridge`.

## Credential flow

Credentials follow a strict path from collection to runtime injection:

1. **Collection** -- During `bm hire` (external bridges: operator provides token) or `bm teams sync --bridge` (local bridges: auto-provisioned)
2. **Config exchange** -- Bridge recipes write credentials to `$BRIDGE_CONFIG_DIR/config.json` (file-based, never stdout)
3. **Storage** -- BotMinter stores credentials in the system keyring (local formations) via the CredentialStore trait
4. **Injection** -- At `bm start`, credentials are resolved from the keyring and injected as environment variables to each member's Ralph process

**Key principle:** Secrets live in the keyring, never in `bridge-state.json`, `ralph.yml`, or `config.yml`. They are injected as environment variables at runtime.

## Formation-aware credential storage

Credential storage is formation-aware through the CredentialStore trait:

| Formation | Storage backend | Status |
|-----------|----------------|--------|
| **Local** | System keyring (via `keyring` crate) | Implemented |
| **Kubernetes** | K8s Secrets | Planned |

The CredentialStore trait provides `store`, `retrieve`, `remove`, and `list` operations. The active formation determines which backend is used. This means the same bridge code works across formations -- only the credential storage changes.

## Headless and CI environments

When the system keyring is unavailable (CI pipelines, containers, headless servers), credentials are supplied via environment variables:

```bash
export BM_BRIDGE_TOKEN_SUPERMAN_01=your-bot-token-here
```

The naming convention is `BM_BRIDGE_TOKEN_{USERNAME}` where the username is uppercased with hyphens replaced by underscores. This is the primary credential mechanism for CI pipelines and containers.

BotMinter checks environment variables first, then falls back to the system keyring. Keyring operations are best-effort -- if they fail, BotMinter prints a warning and guides you to the env var approach.

## Profile bridge declaration

Profiles declare supported bridges in their `botminter.yml` manifest:

```yaml
bridges:
  - name: telegram
    display_name: Telegram
    description: "Telegram bot integration (external service)"
    type: external
```

The bridge implementation files live in `profiles/{profile}/bridges/{bridge}/`:

```
profiles/scrum-compact/
  bridges/
    telegram/
      bridge.yml       # Bridge manifest (Knative-style resource format)
      schema.json      # Config schema (JSON Schema)
      Justfile         # Command recipes
```

Operators select a bridge (or none) during `bm init`. The selected bridge name is recorded in the team's `botminter.yml`.

## Security considerations

- **Credentials are stored in the system keyring** -- never in `bridge-state.json`, `ralph.yml`, or `config.yml`
- **Environment variables** are visible to the Ralph process and its children -- this is the injection mechanism, consistent with standard secret management practices
- **Keyring entries are machine-local** -- after migrating to a new machine, re-provision bridge credentials with `bm bridge identity add`
- **bridge-state.json** tracks provisioning state (usernames, user IDs, room IDs) but never contains tokens or secrets

## Bridge spec

The bridge plugin contract is formally specified in the [Bridge Plugin Specification](../../.planning/specs/bridge/bridge-spec.md). The spec uses Knative-style resource format (`apiVersion`/`kind`/`metadata`/`spec`) and defines:

- Required manifest fields and their constraints
- Lifecycle commands (local bridges only): `start`, `stop`, `health`
- Identity commands (all bridges): `onboard`, `rotate-credentials`, `remove`
- Room commands (optional): `create`, `list`
- Config exchange protocol via `$BRIDGE_CONFIG_DIR/config.json`
- Output shapes for each command category
- Conformance checklist for validation

## Related topics

- [Bridge Setup Guide](../how-to/bridge-setup.md) -- step-by-step setup walkthrough
- [Profiles](profiles.md) -- how profiles define team methodology
- [Workspace Model](workspace-model.md) -- how workspaces are structured
- [CLI Reference](../reference/cli.md) -- full command documentation
