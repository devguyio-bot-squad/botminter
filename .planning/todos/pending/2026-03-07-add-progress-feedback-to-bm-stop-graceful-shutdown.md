---
created: 2026-03-07T05:23:10.705Z
title: Add progress feedback to bm stop graceful shutdown
area: cli
files:
  - crates/bm/src/commands/start.rs
---

## Problem

When running `bm stop` (graceful mode), the command prints "Stopping superman-bob..." and then appears to hang for several seconds while waiting for Ralph to wind down. There is no progress indicator or timeout message, so the user doesn't know if the command is working or stuck. `bm stop --force` works immediately but a user shouldn't have to reach for `--force` just because the UX doesn't communicate what's happening.

## Solution

Add a timeout indicator or progress message during graceful shutdown. E.g., "Stopping superman-bob... (waiting for graceful shutdown, 30s timeout)" or a spinner. This way the user knows the command is actively waiting and will eventually complete or fall back.
