---
created: 2026-03-06T07:02:00Z
title: Fix board scanner errors and ralph.yml skill path templating
area: profiles
files:
  - profiles/scrum-compact/coding-agent/skills/board-scanner/SKILL.md
  - profiles/scrum-compact/roles/team-manager/ralph.yml
  - profiles/scrum-compact/roles/superman/ralph.yml
  - profiles/scrum/roles/team-manager/ralph.yml
  - profiles/scrum/roles/architect/ralph.yml
  - crates/bm/src/commands/hire.rs
  - crates/bm/src/workspace.rs
---

## Problem

The superman member's board_scanner hat is producing multiple errors during its scan loop. Observed in hatless ralph output:

### Error 1: jq parse failure on gh project list
```
Exit code 5 OWNER=devguyio-bot-squad jq: error (at <stdin>:1): Cannot index object with number
```
The board scanner is piping `gh project list` output through jq but the JSON structure doesn't match what the jq expression expects. The project list returns objects but the jq filter tries to index with a number.

### Error 2: gh project item-list "invalid number"
```
invalid number:   Usage: gh project item-list [<number>] [flags]   --format string   Output format: {json}
```
`PROJECT_NUM=69` is being passed to `gh project item-list` but it's failing with "invalid number". This may be a quoting issue or the project number format is wrong.

### Error 3: cd into non-existent "team" directory
```
Exit code 1 (eval):cd:1: no such file or directory: team  (eval):cd:1: no such file or directory: team
```
The board scanner tries to `cd team` but the directory doesn't exist in the workspace. The `team/` submodule path is correct per the workspace layout convention, but the submodule may not have been checked out or synced properly.

### Error 4: gh project list returns empty/malformed projects
```
{ "number": 0, "title": "", "id": "" } { "number": 0, "title": "", ...
```
Multiple project entries with number=0 and empty titles are returned, suggesting phantom/deleted projects or a pagination issue.

### Error 5: Glob paths resolve to "No files found" for knowledge/invariants
```
[Glob] team/projects/*/knowledge/**/*  → No files found
[Glob] team/invariants/**/*  → No files found
[Glob] team/members/superman/invariants/**/*  → No files found
```
Knowledge and invariant resolution globs against `team/` paths find nothing. The `team/` submodule path is correct but may not be checked out or initialized in the workspace.

### Error 6: ralph.yml skill dirs use unresolved template placeholders

All role `ralph.yml` files have skill dirs with placeholder paths that never get substituted:

```yaml
skills:
  dirs:
    - team/coding-agent/skills
    - team/projects/<project>/coding-agent/skills        # <project> never resolved
    - team/members/team-manager/coding-agent/skills      # bare role name, never includes hire suffix
```

After `bm hire team-manager --name boss`, the actual directory is `team/members/team-manager-boss/coding-agent/skills` — but the path in ralph.yml still says `team/members/team-manager/`. The member-level skills (including the team manager's own board-scanner) are never found. Ralph falls back to the team-level board-scanner (superman's), which dispatches `po:*`/`arch:*` statuses instead of `mgr:*`.

Same bug affects `team/projects/<project>/` — it's a literal `<project>` string, never replaced with the actual project name.

This affects ALL roles across ALL profiles (scrum, scrum-compact, scrum-compact-telegram).

## Solution

- **Error 1 (jq):** FIXED in commit a2e3e37 — changed `.[0].number` to `.projects[0].number` across all 7 board-scanner SKILL.md files
- Investigate why `team/` submodule is not checked out — `bm teams sync` may not have initialized it, or the workspace was created without `--push`
- Validate project number before passing to `gh project item-list`
- Filter out phantom projects (number=0, empty title) from the project list
- **Error 6 (skill path templating):** `bm hire` or `bm teams sync` (workspace surfacing) must resolve template placeholders in `ralph.yml`:
  - `team/members/<role>/` → `team/members/<role>-<name>/`
  - `team/projects/<project>/` → expanded to actual project names from config
  - This is the root cause of the team manager running the wrong board scanner
- Note: the GraphQL "could not resolve issue #3" error was a user mistake (issue created in wrong repo) — not a bug
