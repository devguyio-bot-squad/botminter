---
created: 2026-03-07T05:30:00Z
title: Preserve executable permissions in bm profiles init extraction
area: cli
files:
  - crates/bm/src/commands/profiles_init.rs
  - crates/bm/src/profile.rs
---

## Problem

When `bm profiles init` extracts embedded profiles to `~/.config/botminter/profiles/`, shell scripts lose their executable permission. Source files are `-rwxr-xr-x` but extracted files are `-rw-r--r--`. This is because the `include_dir` crate used for compile-time embedding does not preserve Unix file permissions. The documented invocation pattern (`bash scripts/foo.sh`) works around this, but `./scripts/foo.sh` fails unexpectedly.

Affects: `profiles/scrum/coding-agent/skills/gh/scripts/` (10 shell scripts) and any future executable files in profiles.

## Solution

After extracting files, scan for known executable extensions (`.sh`) and `chmod +x` them. Alternatively, store a manifest of executable files alongside the profile, or check the git index for the executable bit during extraction.
