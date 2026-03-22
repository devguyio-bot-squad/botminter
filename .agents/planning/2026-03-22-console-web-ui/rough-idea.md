# Rough Idea: BotMinter Console Web UI

## Summary

Build a browser-accessible web dashboard for botminter -- a local console served by the `bm` CLI that gives operators real-time visibility into their agentic team's status, activity, and coordination.

## Context

BotMinter currently has only `bm status` (a one-shot CLI table) for observability. Operators managing multi-member agentic teams need richer, live-updating visibility into:

- Member status (running, crashed, stopped, brain mode)
- What each member is working on (current hat, issue, branch)
- GitHub issue flow and status transitions
- Bridge activity (Telegram, Matrix messages)
- Logs and activity streams
- Ralph loop state (iterations, events, memories)

A competitor (Symphony) ships "terminal dashboard + Phoenix LiveView + JSON API out of the box" -- botminter needs a comparable observability story.

## Chosen Direction

Web-based dashboard served locally by `bm` (or a sidecar process), accessible at something like `http://localhost:3000`. Not a TUI, not a cloud service -- a local dev console similar to Docker Desktop or Grafana's local UI.

## Key Constraints

- botminter is a Rust CLI (`bm`) -- the web server must integrate with or be spawned by the existing binary
- Data sources: `state.json`, Ralph CLI queries, `gh` CLI, daemon process, bridge state
- Must work for operators running teams locally (Lima VMs, podman containers)
- Alpha-stage project -- breaking changes are expected, no backwards compatibility needed
