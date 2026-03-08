---
created: 2026-03-06T07:00:00Z
title: Launch members in tmux sessions
area: cli
files:
  - crates/bm/src/commands/start.rs
  - crates/bm/src/commands/status.rs
  - crates/bm/src/commands/stop.rs
---

## Problem

`bm start` currently launches each member's Ralph instance as a background process. This makes it difficult to observe what members are doing in real time, attach to a running member's session for debugging, or see their output interactively. The operator has no visibility into member activity beyond log files.

## Solution

- Launch each member in its own named tmux session (e.g., `bm-<team>-<member>`)
- `bm status` could show tmux session names alongside PID info
- `bm chat` or a new `bm attach <member>` command could attach to the tmux session
- `bm stop` would kill the tmux sessions
- Tmux provides session persistence, easy attach/detach, and scrollback — all useful for observing long-running coding agent loops
- Consider making tmux optional (fallback to current background process mode if tmux is not installed)
