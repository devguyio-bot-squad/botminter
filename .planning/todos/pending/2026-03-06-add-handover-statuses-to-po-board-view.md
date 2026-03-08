---
created: 2026-03-06T06:56:00Z
title: Add handover statuses to PO board view
area: profiles
files:
  - profiles/scrum/PROCESS.md
  - profiles/scrum/botminter.yml
  - profiles/scrum-compact/PROCESS.md
  - profiles/scrum-compact/botminter.yml
---

## Problem

The PO (Product Owner) board view currently only shows statuses owned by the PO role (e.g., `po:triage`, `po:backlog`). But "handover" statuses like `arch:plan`, `arch:design`, and similar states where work is waiting for human review/approval also need to appear in the PO view. These are decision points where the human operator acts on the board — approving designs, reviewing plans, accepting epics. If these statuses aren't visible in the PO's filtered view, the human misses work that's waiting for their input.

Additionally, team manager statuses (`mgr:todo`, `mgr:in-progress`, `mgr:done`) must also be visible in the PO board view. The PO can hand work over to either the architect or the team manager, so the PO needs visibility into both pipelines to track where their dispatched work ends up.

## Solution

- Identify all statuses that represent human decision points / handover gates (e.g., `po:design-review`, `po:plan-review`, `po:accept`, `arch:design`, `arch:plan`)
- Include team manager statuses (`mgr:todo`, `mgr:in-progress`, `mgr:done`) since PO dispatches work to the manager
- Include these in the PO board view filter so the human sees all items requiring their attention
- This may involve updating the GitHub Projects v2 view filters or the profile's status configuration to tag certain statuses as "human-visible"
- Check both `scrum` and `scrum-compact` profiles
