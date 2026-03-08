---
created: 2026-03-05T04:20:00Z
title: Fix bm init final message missing teams sync step
area: cli
files:
  - crates/bm/src/commands/init.rs
---

## Problem

The final message after `bm init` completes shows:

```
Run `bm projects sync` anytime to see view instructions.
```

The message mentions `bm projects sync` which is valid, but omits the prerequisite `bm teams sync --push` which must run first to provision workspaces and push the team repo to GitHub. Without `bm teams sync --push`, the user has no workspaces to sync projects into.

## Solution

- Add `bm teams sync --push` as the first next step in the final message
- Keep `bm projects sync` but show it after teams sync in the correct order
- Consider a numbered "Next steps" list: 1) `bm teams sync --push`, 2) `bm projects sync`
