## v0.2.0-pre-alpha

BotMinter brings conventions to running a team of coding agents. This release adds pluggable communication bridges, interactive chat with your agents, a built-in team assistant, and scriptable team creation.

> **Pre-Alpha** - Commands, configuration format, and behavior may change without notice between releases.

---

### Install

#### bm

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/botminter/botminter/releases/download/v0.2.0-pre-alpha/bm-installer.sh | sh
```

Or download a binary from the [release assets](https://github.com/botminter/botminter/releases/tag/v0.2.0-pre-alpha).

#### Ralph Orchestrator

> [!WARNING]
> This release requires a patched build of Ralph Orchestrator. This is temporary — the patches will be merged upstream.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/botminter/ralph-orchestrator/releases/download/v2.8.1-bm.137b1b3.1/ralph-cli-installer.sh | sh
```

Or download `ralph-cli` from [botminter/ralph-orchestrator v2.8.1-bm.137b1b3.1](https://github.com/botminter/ralph-orchestrator/releases/tag/v2.8.1-bm.137b1b3.1).

#### Other prerequisites

[Claude Code](https://claude.ai/code), [gh CLI](https://cli.github.com/), and Git. See the [getting started guide](https://www.botminter.ai/getting-started/).

---

### Chat with your team - bridges

Your agents now have real chat presence. Each team member gets their own account on a messaging platform, posts to shared rooms, and you can follow along from any client. No more reading terminal logs to see what your agents decided.

#### Matrix (Tuwunel) - the default bridge

Pre-selected during `bm init`. Launches a [Tuwunel](https://github.com/avitex/tuwunel) homeserver in a Podman container - a full Matrix experience with zero external dependencies. Your agents get their own Matrix accounts, rooms are provisioned automatically, and you follow the conversation from Element or any Matrix client.

#### Experimental bridges

- Telegram - external bridge for teams that already run their own Telegram bot
- Rocket.Chat - local bridge launching Rocket.Chat + MongoDB via Podman Pod

Bridges are pluggable - adding a new one means satisfying a spec, not writing Rust code.

#### Bridge commands

| Command | Description |
|---------|-------------|
| `bm bridge start` | Start bridge service |
| `bm bridge stop` | Stop bridge service |
| `bm bridge status [--reveal]` | Service health, identities, and operator credentials |
| `bm bridge identity add <name>` | Create bridge user |
| `bm bridge identity show <name> [--reveal]` | Show stored credentials |
| `bm bridge identity rotate <name>` | Rotate credentials |
| `bm bridge identity remove <name>` | Remove bridge user |
| `bm bridge identity list` | List bridge users |
| `bm bridge room create <name>` | Create a channel |
| `bm bridge room list` | List channels |

All bridge commands accept `-t <team>` to target a specific team.

---

### Talk to your agents directly - `bm chat`

No more context-switching to interact with a team member. `bm chat` assembles the member's full context - knowledge, invariants, skills, PROMPT.md - into a system prompt and drops you into an interactive session.

```bash
bm chat <member> [-t team] [--hat <hat>]
bm chat <member> --render-system-prompt   # Print the generated prompt without starting a session
```

---

### Meet Minty - your team's interactive assistant

While `bm chat` connects you to a specific team member, Minty is something different - a built-in assistant that knows your team's structure, process, and state. It's not a team member; it's an operator-facing tool.

```bash
bm minty [-t team]
```

Ships with four skills:

| Skill | What it does |
|-------|-------------|
| **profile-browser** | Browse and compare embedded profiles |
| **team-overview** | Inspect team state, members, and configuration |
| **hire-guide** | Walk through adding a member step by step |
| **workspace-doctor** | Diagnose and fix workspace issues |

---

### Automate team creation - `bm init --non-interactive`

Scripted and CI-friendly team creation without the interactive wizard:

```bash
bm init --non-interactive \
  --profile scrum-compact \
  --team-name my-team \
  --org my-org \
  --repo my-repo \
  [--bridge tuwunel]
```

---

### Credential store

Bridge credentials are stored in your OS keyring instead of plaintext config files. Credentials are resolved per-member at launch time.

---

### Team manager role

The `scrum-compact` profile now ships with a `team-manager` role - a coordination-focused agent for process improvement, team health, and workflow orchestration.

```bash
bm hire team-manager
```

> The `scrum` profile (multi-role teams with architect, human-assistant, and team-manager) is in development and will ship in a future release.

---

### Other changes

#### Profiles

- `scrum-compact-telegram` profile removed - use `scrum-compact` with bridge selection during `bm init`
- Profiles now live on disk at `~/.config/botminter/profiles/` - run `bm profiles init` to extract, inspect, and customize them
- Each profile ships a `botminter.yml` manifest declaring coding agents, bridges, roles, labels, and statuses

#### Workspaces

- Git submodules for team repo and project forks (replaces embedded copies)
- Member config path: `members/<member>/` (was `team/<member>/`)
- New `.botminter.workspace` marker file

#### CLI flag changes

| Before | After |
|--------|-------|
| `bm teams sync --push` | `bm teams sync --repos` |
| - | `bm teams sync --bridge` (new - provision bridge resources) |
| - | `bm teams sync --all` / `-a` (new - repos + bridge) |
| - | `bm start --no-bridge` / `--bridge-only` (new) |
| - | `bm stop --bridge` (new - also stops the bridge service) |

Members launch in tmux sessions. Bridge starts before members, stops after.

#### Experimental features

Introduced in v0.1.0-pre-alpha and still experimental:

- `bm daemon` - event-driven background process for GitHub issue updates
- `bm start --formation` - non-local deployment targets (e.g., Kubernetes)
- `bm knowledge` - inspect and manage knowledge and invariants

#### Documentation

- [Bridge documentation](https://www.botminter.ai/concepts/bridges/) - concepts, setup guide, and CLI reference
- Docs updated across the board to reflect all changes in this release

---

### Under the hood

- Release builds ship `scrum-compact` only (with tuwunel + telegram). The `scrum` profile and Rocket.Chat bridge are available in development builds
- Formation abstraction - local formation designed as a first-class concept
- E2E test harness rewritten with hermetic environments, isolated keyring, and progressive stepping (`just e2e-step`)
- Bridge conformance suite (`just conformance`) for validating bridge implementations
- 8 ADRs established using Spotify-style format

---

### Breaking changes

All changes are breaking. Teams and workspaces created with v0.1.0-pre-alpha must be re-created from scratch.

| What changed | Details |
|-------------|---------|
| Profile directories | `members/` -> `roles/`, `agent/` -> `coding-agent/` |
| Profile removed | `scrum-compact-telegram` (use `scrum-compact` + bridge selection) |
| Sync flag | `--push` replaced by `--repos` |
| Workspaces | Submodules, new marker file, new surfacing paths |
| Profile schema | Reset to v1 for `botminter.yml` format |

