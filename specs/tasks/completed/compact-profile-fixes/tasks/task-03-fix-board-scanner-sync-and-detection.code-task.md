---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Fix board scanner sync and team repo detection

## Description
The board_scanner hats in **both** the compact and rh-scrum profiles have broken commands in their "Every cycle" steps:
1. Step 2 references `just -f .botminter/Justfile sync` — no Justfile exists at that path
2. Step 3 uses `gh repo view` for team repo detection with no fallback when the remote is a local path

Both cause agents to waste turns on error recovery before eventually finding a workaround. The rh-scrum profile also duplicates the Justfile reference in `architect/PROMPT.md`.

## Background
Every board_scanner hat runs sync and team repo detection on every cycle. The Justfile was never part of either profile template — the correct sync mechanism is `git pull` on the `.botminter/` directory. The `gh repo view` detection fails when `.botminter/`'s remote is a local filesystem path (see task-02 for the root cause fix). Even after task-02 is fixed, a fallback is good defense-in-depth.

Both profiles share an identical `gh` skill (`agent/skills/gh/SKILL.md`) with the same detection pattern.

## Reference Documentation
**Required:**

Compact profile:
- `profiles/compact/members/superman/ralph.yml` — board_scanner hat, lines 51-57
- `profiles/compact/agent/skills/gh/SKILL.md` — Repo Auto-Detection section, line 23

rh-scrum profile:
- `profiles/rh-scrum/members/architect/ralph.yml` — board_scanner hat, lines 41-43
- `profiles/rh-scrum/members/architect/PROMPT.md` — line 79
- `profiles/rh-scrum/members/human-assistant/ralph.yml` — board_scanner hat, lines 33-35
- `profiles/rh-scrum/agent/skills/gh/SKILL.md` — Repo Auto-Detection section, line 23

## Technical Requirements
1. Replace `just -f .botminter/Justfile sync` with `git -C .botminter pull --ff-only 2>/dev/null || true` in all board_scanner hats
2. Add `2>/dev/null` to the `gh repo view` command in all board_scanner step 3 instructions
3. Add a fallback instruction: if `gh repo view` fails, extract `owner/repo` from the git remote URL
4. Apply the same fallback to both profiles' `gh` skill Repo Auto-Detection sections
5. Fix the Justfile reference in `profiles/rh-scrum/members/architect/PROMPT.md`
6. Fix the glob bug in `profiles/rh-scrum/members/human-assistant/ralph.yml:38` — `gh issue list --label "status/po:*"` uses a shell glob which the `--label` flag does not support (it expects exact label names). Replace with explicit label queries or multiple `--label` flags
7. Replace hardcoded project name `hypershift` in `profiles/rh-scrum/members/architect/ralph.yml` (lines 116, 138, 182 and others) with the dynamic `<project>` placeholder used elsewhere in the file

## Dependencies
- Should be done alongside or after task-02 (fixes root cause), but this task provides defense-in-depth

## Implementation Approach
1. Fix compact profile:
   - `profiles/compact/members/superman/ralph.yml` — board_scanner steps 2-3
   - `profiles/compact/agent/skills/gh/SKILL.md` — Repo Auto-Detection
2. Fix rh-scrum profile:
   - `profiles/rh-scrum/members/architect/ralph.yml` — board_scanner steps 2-3
   - `profiles/rh-scrum/members/human-assistant/ralph.yml` — board_scanner steps 2-3
   - `profiles/rh-scrum/members/architect/PROMPT.md` — Justfile reference
   - `profiles/rh-scrum/agent/skills/gh/SKILL.md` — Repo Auto-Detection

## Acceptance Criteria

1. **Sync step uses git pull in all profiles**
   - Given the board_scanner hat instructions in any profile's `ralph.yml`
   - When reading step 2 of "Every cycle"
   - Then the command is `git -C .botminter pull --ff-only` (no Justfile reference)

2. **Team repo detection has fallback in all profiles**
   - Given the board_scanner hat instructions in any profile's `ralph.yml`
   - When reading step 3 of "Every cycle"
   - Then `gh repo view` is tried first with stderr suppressed, and a fallback to URL parsing is documented

3. **gh skill has matching fallback in both profiles**
   - Given the Repo Auto-Detection section in each profile's `agent/skills/gh/SKILL.md`
   - When reading the detection instructions
   - Then it matches the board_scanner's fallback approach

4. **No Justfile references remain in any profile**
   - Given `profiles/compact/` and `profiles/rh-scrum/`
   - When searching for `Justfile`
   - Then zero matches are found

5. **rh-scrum board scanner uses exact label names**
   - Given `profiles/rh-scrum/members/human-assistant/ralph.yml`
   - When reading the board_scanner's issue query command
   - Then `--label` flags use exact label names (no glob patterns like `status/po:*`)

6. **No hardcoded project names in rh-scrum**
   - Given `profiles/rh-scrum/members/architect/ralph.yml`
   - When searching for `hypershift`
   - Then zero matches are found — all project references use `<project>` placeholder

## Metadata
- **Complexity**: Medium
- **Labels**: bugfix, profile, compact, rh-scrum
- **Required Skills**: YAML, shell, markdown
