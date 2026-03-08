---
created: 2026-03-06T06:58:00Z
title: Add lead reviewer view after architect in board
area: profiles
files:
  - profiles/scrum/PROCESS.md
  - profiles/scrum/botminter.yml
---

## Problem

The GitHub Projects board views don't include a dedicated view for the lead reviewer role. The lead reviewer gates architecture work before it reaches the human PO — reviewing arch designs and plans. Without a filtered view positioned after the architect's view, the lead reviewer has no focused way to see items waiting for their review.

## Solution

- Add a lead reviewer board view in the GitHub Projects configuration
- Position it after the architect view in the view ordering
- Filter it to show statuses relevant to lead review (e.g., `lead:review`, or whatever statuses the lead reviewer acts on)
- Update both `scrum` and `scrum-compact` profiles if applicable
