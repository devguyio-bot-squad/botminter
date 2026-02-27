# Rough Idea: `bm` CLI

## The Need

Replace the current Justfile-based tooling with a proper CLI tool (`bm`) that serves as the single operator interface for managing agentic teams. Today, the operator juggles `just` recipes, manual workspace setup, and ad-hoc `gh` commands. `bm` unifies all of that.

## What the Operator Should Be Able To Do

1. **Set up and manage teams** — init repos, hire members into roles, create workspaces
2. **Launch and wake up members** — start Ralph instances, signal idle members to check for work
3. **Manage knowledge and invariants** — add, list, and organize knowledge at the right scope (team/project/member)
4. **Observe what's happening** — see which members are running, what they're working on, their status

## Mental Model

Think of it like a control plane for the team:
- The CLI (`bm`) is the operator's interface — like `kubectl`
- The team repo is the source of truth — like the etcd/API server
- Members are the workers — like pods
- Something needs to manage member lifecycle locally — like kubelet

The specific technical architecture (daemon? sidecar? single process? library?) should emerge from the requirements, not be assumed upfront.

## Inspiration

- [multiclaude](https://github.com/dlorenc/multiclaude) — `multiclaude start` spins up a daemon, agents run in tmux windows, CLI talks to daemon over Unix socket. Useful pattern to study, not necessarily to copy.
