# Bridge Setup

This guide walks you through setting up a bridge for your team, from initialization to running members with bridge credentials.

## Prerequisites

- A profile with bridge support (e.g., `scrum-compact` or `scrum` with Telegram)
- For external bridges (Telegram): bot tokens created via the platform (e.g., @BotFather for Telegram)
- `bm` CLI installed and GitHub auth configured

## Step 1: Initialize with bridge selection

During `bm init`, select a bridge when prompted:

```bash
bm init
```

The wizard lists bridges from the selected profile plus a "No bridge" option. Choose the bridge you want (e.g., Telegram).

For non-interactive/CI mode, use the `--bridge` flag:

```bash
bm init --non-interactive \
  --profile scrum-compact \
  --team-name my-team \
  --org my-org \
  --repo my-team \
  --bridge telegram
```

Omitting `--bridge` means no bridge is configured.

## Step 2: Hire members with bridge tokens

When you hire a member and a bridge is configured, `bm hire` prompts for an optional bridge token (external bridges only, interactive mode):

```bash
bm hire superman
# Prompt: "Bridge token for superman-01 (optional, press Enter to skip):"
```

For Telegram, create a bot via [@BotFather](https://t.me/BotFather) first, then paste the token when prompted. The token is stored in the system keyring.

Tokens are optional during hire -- you can add them later.

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

BotMinter resolves per-member credentials from the keyring (or environment variables) and injects them into each member's Ralph process. Members with valid credentials and `RObot.enabled: true` in their `ralph.yml` will have bridge integration active.

## Adding credentials after hire

If you skipped the token prompt during hire, or need to add credentials for a new member:

```bash
bm bridge identity add superman-01
# Prompt: "Token for superman-01:"
```

After adding credentials, run `bm teams sync` to update the member's `ralph.yml` with `RObot.enabled: true`.

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

## External bridge specifics (Telegram)

1. **Create bots:** Create one Telegram bot per team member via [@BotFather](https://t.me/BotFather). Each bot needs a unique name.
2. **Supply tokens:** Provide the bot token during `bm hire` or via `bm bridge identity add`.
3. **Sync:** Run `bm teams sync --bridge` to validate tokens and provision rooms.
4. **Launch:** `bm start` injects the per-member bot tokens as environment variables.

## Related topics

- [Bridge Concepts](../concepts/bridges.md) -- bridge types, credential flow, security model
- [CLI Reference](../reference/cli.md) -- full command documentation
- [Launch Members](launch-members.md) -- detailed launch workflow
