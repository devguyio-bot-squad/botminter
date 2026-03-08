---
created: 2026-03-07T05:23:10.705Z
title: Show created workspace names in bm teams sync default output
area: cli
files:
  - crates/bm/src/commands/teams.rs
---

## Problem

When running `bm teams sync` without `-v`, the output only shows a summary count like "Synced 1 workspace (1 created, 0 updated)". A user doesn't know which workspace was created without re-running with `-v`. For a first-time user provisioning workspaces, seeing the actual workspace name/path in default output would make the experience clearer.

## Solution

In the sync command's non-verbose output path, include the workspace name (member name) for created workspaces. E.g., "Created workspace: superman-bob" before the summary line. Keep verbose mode as-is since it already provides full detail.
