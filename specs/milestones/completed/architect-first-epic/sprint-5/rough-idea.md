# Rough Idea — Sprint 5: Compact Single-Member Profile

A new profile, `hypershift-compact`, that collapses the distributed multi-member team into a single Ralph instance. Instead of separate team members (human-assistant, architect, dev, SRE, QE, content writer), these roles become hats within one member.

## Motivation

Not every task needs the overhead of multi-agent coordination (`.github-sim/` issues, write-locks, submodule syncing, separate workspaces). For small tasks, a single agent with all the capabilities is more practical — less infrastructure, faster feedback loops, no coordination overhead.

## Key Idea

- **Profile:** `hypershift-compact` (or `compact`) — sits alongside `rh-scrum` in `skeletons/profiles/`
- **Single member:** One Ralph instance with multiple hats covering all roles
- **Roles → hats:** human-assistant, architect, dev, SRE, QE, content writer become hats (not necessarily 1:1 — some roles may map to multiple hats)
- **Same generator:** `just init --profile=hypershift-compact` produces a team repo with a single-member workflow
- **Target use case:** Small tasks that don't need distributed coordination
