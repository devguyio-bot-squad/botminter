---
status: complete
created: 2026-03-04
completed: 2026-03-04
---
# Task: Board-Scanner Hat → Auto-Inject Skill

Replace `board_scanner` hat with `board-scanner` auto-inject skill across all 3 profiles. The hatless coordinator now performs board scanning directly via the skill.

- Created 4 `board-scanner/SKILL.md` files (team-level for compact profiles, member-level for scrum)
- Added role-scoped failure events to all hats (route to coordinator catch-all)
- Removed `github-mutations-hat-only` invariant (incompatible with skill model)

**Commit:** `1f8d406`
