---
status: complete
created: 2026-03-04
completed: 2026-03-04
---
# Task: Profile Directory Restructuring

Rename directories to fix naming confusion and stale paths.

- Profile skeleton: `members/` → `roles/` (these are role templates, not hired instances)
- Team repo inner dir: `team/` → `members/` (fixes `team/team/<member>/` → `team/members/<member>/`)
- Fixed 149 stale `.botminter/` path references → `team/`
- Updated docs, context.md files, CLAUDE.md

**Commit:** `1f8d406`
