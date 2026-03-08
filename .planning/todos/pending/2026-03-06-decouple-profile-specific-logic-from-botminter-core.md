---
created: 2026-03-06T06:58:00Z
title: Decouple profile-specific logic from BotMinter core
area: architecture
files:
  - crates/bm/src/commands/init.rs
  - crates/bm/src/workspace.rs
  - crates/bm/src/profile.rs
  - profiles/scrum/botminter.yml
  - profiles/scrum-compact/botminter.yml
---

## Problem

BotMinter core (`bm` CLI) contains hardcoded logic that assumes specific profile conventions — things like project generation steps, label bootstrapping sequences, status field configurations, and workspace assembly patterns. This couples the CLI to profile-specific knowledge that should live in the profiles themselves. Adding a new profile or changing a profile's workflow shouldn't require changes to `bm` source code.

## Solution

Research and design a clean boundary between BotMinter core and profile-specific logic:

- Audit `init.rs`, `workspace.rs`, and `profile.rs` for hardcoded assumptions about scrum/scrum-compact conventions
- Identify which behaviors should be profile-driven (declared in `botminter.yml` or profile scripts) vs truly generic (applicable to any profile)
- Consider a hook/lifecycle model where profiles declare what happens at each stage (init, hire, sync, start) and the CLI executes those declarations
- Goal: a new profile should work by just defining its manifest and content — zero changes to `bm` source
- This is a research task first — understand the current coupling before designing the decoupling
