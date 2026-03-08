---
created: 2026-03-06T06:54:34.508Z
title: Improve teams sync push speed with concurrency and progress bar
area: cli
files:
  - crates/bm/src/workspace.rs
  - crates/bm/src/commands/teams.rs
---

## Problem

`bm teams sync --push` provisions workspaces sequentially — each member workspace is created, submodules added, committed, and pushed one at a time. For teams with multiple members, this is slow because each `gh repo create` + `git push` cycle involves network round-trips to GitHub. There's also no progress feedback, so the user sees nothing until the entire operation completes.

## Solution

- Run workspace provisioning concurrently where possible (repo creation, submodule setup, push can be parallelized across members since each workspace is independent)
- Add a progress bar (using the existing `indicatif` dependency) showing member-by-member progress
- Consider using `tokio` or `rayon` for parallelism, or simply `std::thread::spawn` since each workspace is independent
- The `update_existing_workspace` path could also benefit from parallel execution
