---
created: 2026-03-05T04:20:00Z
title: Default hire-another to yes when roles unfilled
area: cli
files:
  - crates/bm/src/commands/init.rs
---

## Problem

During `bm init`, after hiring the first member, the wizard asks "Hire another member?" but the default answer doesn't adapt to context. When there are still unfilled roles (e.g., hired 1 of 3 roles), the default should be "yes" to guide the user toward a complete team. Only when all roles have at least one member should the default flip to "no".

## Solution

- Track which roles have been filled during the init hiring loop
- Compare filled roles against available roles from the profile
- Set default to "yes" while unfilled roles remain, "no" when all are filled
