---
status: testing
phase: 05-team-manager-chat
source: 05-02-SUMMARY.md, 05-03-SUMMARY.md, 05-04-SUMMARY.md
started: 2026-03-08T00:00:00Z
updated: 2026-03-08T00:00:00Z
---

## Current Test

number: 1
name: BM Chat Launches Interactive Session
expected: |
  Run `bm chat superman-testbot --hat dev_implementer` (or equivalent member).
  Claude Code opens inside the member's workspace directory — NOT in the bm repo.
  No "Input must be provided" error. The session is interactive.
awaiting: user response

## Tests

### 1. BM Chat Launches Interactive Session
expected: Run `bm chat <member> --hat <hat>`. Claude Code opens inside the member's workspace directory. No "--print" error. The session is interactive.
result: [pending]

### 2. Role Description in Identity
expected: Run `bm chat <member> --render-system-prompt`. The identity section shows the role description (e.g., "All-in-one member -- PO, architect, dev, QE, SRE, content writer") on a line after the identity line. Not just "You are X, a superman on the Y team" with no explanation.
result: [pending]

### 3. Invalid Hat Validation
expected: Run `bm chat <member> --hat nonexistent --render-system-prompt`. The command fails with a clear error listing available hat names. It does NOT silently render an empty capabilities section.
result: [pending]

### 4. Skills Table in Meta-Prompt
expected: Run `bm chat <member> --render-system-prompt`. The output includes a skills table listing available skills from the workspace's coding-agent/skills/ directory with name, description, and load path columns.
result: [pending]

### 5. Hat Selection Filters Capabilities
expected: Run `bm chat <member> --hat dev_implementer --render-system-prompt`. The capabilities section shows ONLY the dev_implementer hat's instructions, not all hats. Compare with running without --hat to confirm the difference.
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0

## Gaps

[none yet]
