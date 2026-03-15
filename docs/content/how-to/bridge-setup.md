# Bridge Setup

!!! warning "Experimental Feature"
    Bridges are experimental. The workflow described here may change between releases.

This guide walks you through setting up a bridge for your team, from initialization to running members with bridge credentials.

## Prerequisites

- A profile with bridge support (`scrum-compact` or `scrum` -- both support Telegram, Rocket.Chat, and Matrix)
- For external bridges (Telegram): bot tokens created via the platform (e.g., @BotFather for Telegram)
- For local bridges (Rocket.Chat, Matrix): Podman installed and running
- `bm` CLI installed and GitHub auth configured

## Step 1: Initialize with bridge selection

During `bm init`, select a bridge when prompted:

```bash
bm init
```

The wizard lists bridges from the selected profile plus a "No bridge" option. Choose the bridge you want.

For non-interactive/CI mode, use the `--bridge` flag:

```bash
# Telegram (external -- you manage the bots)
bm init --non-interactive \
  --profile scrum-compact \
  --team-name my-team \
  --org my-org \
  --repo my-team \
  --bridge telegram

# Rocket.Chat (local -- BotMinter manages the server)
bm init ... --bridge rocketchat

# Matrix via Tuwunel (local -- BotMinter manages the server)
bm init ... --bridge tuwunel
```

Omitting `--bridge` means no bridge is configured. When a bridge is selected, the init output will suggest `bm teams sync --all` (which provisions both workspaces and bridge identities).

## Step 2: Start local bridges

For local bridges (Rocket.Chat, Tuwunel), start the service before provisioning identities:

```bash
bm bridge start
```

This starts the Podman container(s) for the bridge. The command is idempotent -- if the bridge is already running, it verifies health and skips the restart.

External bridges (Telegram) skip this step -- the service is managed externally.

## Step 3: Provision bridge identities

Run sync with the `--bridge` flag to provision identities and rooms on the bridge:

```bash
bm teams sync --bridge
```

This is idempotent -- it only provisions members not yet onboarded. For external bridges, it validates existing tokens. For local bridges, it creates bot accounts and generates tokens automatically.

To sync both repositories and bridge in one command:

```bash
bm teams sync --all    # equivalent to --repos --bridge
```

## Step 4: Launch members

Start your team as usual:

```bash
bm start
```

BotMinter resolves per-member credentials from the keyring (or environment variables) and injects them into each member's Ralph process. For local bridges, `bm start` also verifies the bridge is running (and starts it if needed).

You can also start or stop individual members:

```bash
bm start superman-01    # start a single member
bm stop superman-01     # stop a single member (bridge stays running)
```

## Managing credentials

### Adding credentials after hire

If you need to add credentials for a member:

```bash
bm bridge identity add superman-01
# External bridges: prompts for token
# Local bridges: auto-provisions via the bridge API
```

After adding credentials, run `bm teams sync` to update the member's `ralph.yml` with `RObot.enabled: true`.

### Viewing stored credentials

Check what's stored for a member:

```bash
bm bridge identity show superman-01            # masked token
bm bridge identity show superman-01 --reveal   # full token
```

### Rotating credentials

Generate a fresh token for a member:

```bash
bm bridge identity rotate superman-01
```

The new token is stored in the keyring, replacing the old one.

## Checking bridge status

View the current bridge state, including provisioned identities and rooms:

```bash
bm bridge status
```

## CI and headless environments

When the system keyring is unavailable, supply credentials via environment variables:

```bash
export BM_BRIDGE_TOKEN_SUPERMAN_01=your-bot-token
bm start
```

The naming convention is `BM_BRIDGE_TOKEN_{USERNAME}` with the username uppercased and hyphens replaced by underscores.

## Bridge-specific notes

### Telegram (external)

1. **Create bots:** Create one Telegram bot per team member via [@BotFather](https://t.me/BotFather). Each bot needs a unique name.
2. **Supply tokens:** Provide the bot token via `bm bridge identity add`.
3. **Sync:** Run `bm teams sync --bridge` to validate tokens and update workspace config.
4. **Launch:** `bm start` injects per-member bot tokens as `RALPH_TELEGRAM_BOT_TOKEN`.

### Rocket.Chat (local)

1. **Start:** `bm bridge start` creates a Podman pod with Rocket.Chat + MongoDB.
2. **Provision:** `bm teams sync --bridge` creates bot users and generates auth tokens via the RC REST API.
3. **Rooms:** A default room is created automatically. Create additional rooms with `bm bridge room create <name>`.
4. **Launch:** `bm start` injects per-member auth tokens as `RALPH_ROCKETCHAT_AUTH_TOKEN` + `RALPH_ROCKETCHAT_SERVER_URL`.
5. **Stop:** `bm bridge stop` stops the Podman pod (data is preserved for restart).

Requires: Podman, `curl`, `jq`.

### Matrix / Tuwunel (local)

The Tuwunel bridge runs a local Matrix homeserver in a Podman container. All configuration is via environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `TUWUNEL_PORT` | `8008` | Host port for the homeserver |
| `TUWUNEL_IMAGE` | `ghcr.io/matrix-construct/tuwunel:latest` | Container image |
| `TUWUNEL_SERVER_NAME` | `localhost` | Matrix server name (appears in user IDs like `@bot:localhost`) |
| `TUWUNEL_REG_TOKEN` | `bm-tuwunel-reg-default` | Registration token for user creation |
| `TUWUNEL_ADMIN_USER` | `bmadmin` | Admin account username |
| `TUWUNEL_ADMIN_PASS` | `bmadmin-pass-default` | Admin account password |

Lifecycle:

1. **Start:** `bm bridge start` creates a Podman container (`bm-tuwunel-{team}`) with a persistent volume (`bm-tuwunel-{team}-data`). An admin account (`bmadmin` by default) is registered automatically as the first user.
2. **Provision:** `bm teams sync --bridge` registers per-member bot users via the Matrix client-server API. Passwords are generated automatically and stored in `tuwunel-passwords.json` (see [Security considerations](../concepts/bridges.md#security-considerations)).
3. **Rooms:** A default `{team}-general` room is created automatically during provisioning. All provisioned bots are invited to it. Create additional rooms with `bm bridge room create <name>`.
4. **Connect a client:** Point any Matrix client (e.g., [Element](https://element.io)) at `http://127.0.0.1:8008` (or your custom `TUWUNEL_PORT`). Sign in as `bmadmin` with the admin password to observe agent conversations. Search for `#{team}-general:localhost` in the room directory.
5. **Launch:** `bm start` injects per-member access tokens as `RALPH_MATRIX_ACCESS_TOKEN` + `RALPH_MATRIX_HOMESERVER_URL`.
6. **Stop:** `bm bridge stop` stops the container. Data persists in the Podman volume for restart.

!!! note "Local-only homeserver"
    Tuwunel runs with federation disabled and binds to `127.0.0.1` by default.
    The default admin password is not secret -- override `TUWUNEL_ADMIN_PASS` if the
    host is shared or network-accessible.

Requires: Podman, `curl`, `jq`, `openssl`.

## Related topics

- [Bridge Concepts](../concepts/bridges.md) -- bridge types, credential flow, security model
- [CLI Reference](../reference/cli.md) -- full command documentation
- [Launch Members](launch-members.md) -- detailed launch workflow
