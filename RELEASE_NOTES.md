## v0.1.0-pre-alpha - Initial release

A CLI that brings conventions to running a team of coding agents. Built for Claude Code today via [Ralph Orchestrator](https://github.com/mikeyobrien/ralph-orchestrator) (manages Claude Code as a persistent, looping workflow).

Most tools focus on how to run agents. BotMinter solves a different problem: how your agents should work, what they know, and how you stay in the loop. Your process, knowledge, and constraints live in a Git repo, and every agent picks them up automatically.

> **Pre-Alpha** - Commands, configuration format, and behavior may change without notice between releases.

### The workflow

```bash
bm init                           # Pick a profile, connect a GitHub org/repo
bm hire <role>                    # Add a new agent to the team based on a role
bm projects add <url>             # Register a project
bm teams sync --push              # Provision workspaces
bm start                          # Launch agents
bm status                         # See what's running
```

### What makes it work

- **Profiles** - opinionated convention packages (ships with `scrum` and `scrum-compact`) that define roles, process, knowledge, and invariants. Telegram is available as an optional bridge on any profile.
- **Layered knowledge scoping** - define a convention once at the team level, override it per project or per member without forking the config (team > project > member > member+project)
- **GitHub as coordination fabric** - agents pull work from GitHub issues, decisions are traceable on a project board

### Experimental features

These are included but not yet stable:

- **`bm daemon`** - background process that watches for GitHub issue updates and triggers member actions (webhook or poll mode)
- **`bm start --formation`** - deploy agents using non-local formations (e.g., Kubernetes)
- **`bm knowledge`** - inspect and manage knowledge and invariants at any scope, with an interactive Claude Code session for editing

### Get started

Requires Rust, Claude Code, Ralph, and the `gh` CLI. See the [getting started guide](https://www.botminter.ai/getting-started/) for prerequisites and a full walkthrough.

```bash
git clone https://github.com/botminter/botminter.git
cargo install --path botminter/crates/bm
```
