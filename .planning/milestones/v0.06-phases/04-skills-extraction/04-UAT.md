---
status: complete
phase: 04-skills-extraction
source: 04-01-SUMMARY.md
started: 2026-03-07T00:00:00Z
updated: 2026-03-07T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Build & Tests Pass
expected: Run `just build` and `just test`. Both complete without errors.
result: pass
agent-notes: Build clean. 310 unit tests + 19 integration tests all passed (329 total).

### 2. Board-Scanner Skill Exists as SKILL.md
expected: Check `profiles/scrum-compact/coding-agent/skills/board-scanner/SKILL.md`. It exists, has YAML frontmatter (name, description), and contains the board-scanner logic (GitHub Projects v2 dispatch, auto-advance, priority tables). Also check the scrum profile at `profiles/scrum/roles/*/coding-agent/skills/board-scanner/SKILL.md` for role-specific variants.
result: pass
agent-notes: scrum-compact has team-level board-scanner (197 lines). scrum has 3 role-specific variants (architect=135, human-assistant=171, team-manager=129 lines) — genuinely different with distinct status scopes, priority tables, and comment attribution emojis.

### 3. Ralph Prompts Directory Structure
expected: Each profile (scrum, scrum-compact) has a `ralph-prompts/` directory containing at minimum: guardrails.md, orientation.md, hat-template.md, and a `reference/` subdirectory with workflow/event-writing/completion docs. These are reference materials for Ralph, not empty stubs.
result: pass
agent-notes: Both profiles have identical ralph-prompts/ with 8 files totaling 627 lines. All substantive — no empty stubs. Reference docs cover workflows, event-writing, completion, ralph-tools (211 lines), robot-interaction.

### 4. Status-Workflow Skill
expected: `profiles/scrum/coding-agent/skills/status-workflow/SKILL.md` exists with meaningful content covering GitHub Projects v2 status transitions, GraphQL operations, comment attribution format, and label operations. It should be a composable skill usable by any role.
result: pass
agent-notes: 175 lines in both profiles. Covers prerequisites, 5-step transition procedure, GraphQL verification query, label operations, and references. Includes critical `-F` flag note for ID type variables.

### 5. GH Skill with Scripts and References
expected: `profiles/scrum/coding-agent/skills/gh/` contains SKILL.md, a `scripts/` directory with shell scripts for GitHub operations, and a `references/` directory with documentation. The scripts should be executable and cover common gh operations.
result: issue
reported: "Scripts lose executable permission after extraction. Source files are -rwxr-xr-x, extracted files are -rw-r--r--. Also quick-start.md in references/ is not linked from the gh SKILL.md (orphan reference)."
severity: minor

### 6. Two-Level Skill Scoping
expected: Skills are organized at two levels: team-level shared skills in `profiles/scrum/coding-agent/skills/` (status-workflow, gh) and role-level specialized skills in `profiles/scrum/roles/<role>/coding-agent/skills/` (board-scanner per role). The ralph.yml config references multiple skills_dirs for discovery across scopes.
result: pass
agent-notes: Team-level has gh + status-workflow (shared). Role-level has board-scanner per role (specialized). ralph.yml configures 3-scope skills resolution (team, project, member) with board-scanner auto_inject: true.

### 7. Profiles Init Extracts Skills to Disk
expected: Run `bm profiles init --force`. Check `~/.config/botminter/profiles/scrum/coding-agent/skills/` — skills directories (status-workflow, gh, board-scanner) should be present with their full contents extracted verbatim from the embedded profiles.
result: issue
reported: "Extraction works and content matches source, but shell scripts lose executable permission (include_dir crate strips Unix permissions). Source -rwxr-xr-x becomes -rw-r--r-- on disk. Documented invocation via 'bash scripts/...' still works but ./scripts/foo.sh would fail."
severity: minor

## Summary

total: 7
passed: 5
issues: 2
pending: 0
skipped: 0

## Gaps

- truth: "GH skill shell scripts should be executable after extraction"
  status: failed
  reason: "User reported: Scripts lose executable permission after extraction. Source files are -rwxr-xr-x, extracted files are -rw-r--r--. include_dir crate strips Unix permissions."
  severity: minor
  test: 5
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "GH skill SKILL.md should link to all reference docs including quick-start.md"
  status: failed
  reason: "User reported: quick-start.md in references/ is not linked from the gh SKILL.md (orphan reference)"
  severity: cosmetic
  test: 5
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Extracted profile scripts preserve executable permission from source"
  status: failed
  reason: "User reported: Extraction works and content matches source, but shell scripts lose executable permission. Source -rwxr-xr-x becomes -rw-r--r-- on disk."
  severity: minor
  test: 7
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
