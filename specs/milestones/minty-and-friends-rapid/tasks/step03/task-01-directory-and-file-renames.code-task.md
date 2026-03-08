---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Directory and File Renames Across Profiles

## Description
Rename all `agent/` directories to `coding-agent/` and all `CLAUDE.md` files to `context.md` across every profile and every scope (team-level, project-level, member-level). This is a content migration within `profiles/` — no runtime code changes.

## Background
The coding-agent-agnostic design replaces agent-specific naming conventions with generic ones. `agent/` was implicitly Claude Code-specific; `coding-agent/` is neutral. `CLAUDE.md` is renamed to `context.md` because the context file name is now config-driven (resolved at extraction time from `CodingAgentDef.context_file`).

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Profile Restructuring")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 3)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Rename directories across all profiles and all scopes:
   - Team-level: `agent/` -> `coding-agent/`
   - Project-level: `projects/<p>/agent/` -> `projects/<p>/coding-agent/`
   - Member-level: `members/<m>/agent/` -> `members/<m>/coding-agent/`
2. Rename context files across all profiles:
   - Team-level: `CLAUDE.md` -> `context.md`
   - Member-level: `members/<m>/CLAUDE.md` -> `members/<m>/context.md`
3. Update any internal references to old paths (e.g., in `botminter.yml`, README files, test fixtures)
4. Update `.schema/` if it validates directory structure

## Dependencies
- Steps 1-2 complete (agent_tags module and CodingAgentDef exist)

## Implementation Approach
1. Audit all profiles to find every `agent/` directory and `CLAUDE.md` file
2. Perform renames using git mv to preserve history
3. Search for internal references to old names and update them
4. Verify profile directory structure is consistent

## Acceptance Criteria

1. **No agent/ directories remain**
   - Given all profile directories
   - When searching for directories named `agent/`
   - Then none are found (all renamed to `coding-agent/`)

2. **No CLAUDE.md files in profiles**
   - Given all profile directories
   - When searching for files named `CLAUDE.md`
   - Then none are found (all renamed to `context.md`)

3. **coding-agent/ directories exist at correct scopes**
   - Given each profile
   - When listing directories at team, project, and member levels
   - Then `coding-agent/` exists where `agent/` previously existed

4. **context.md files exist at correct scopes**
   - Given each profile
   - When listing files at team and member levels
   - Then `context.md` exists where `CLAUDE.md` previously existed

5. **Internal references updated**
   - Given profile YAML and documentation files
   - When searching for `"agent/"` or `"CLAUDE.md"` references
   - Then all are updated to `"coding-agent/"` and `"context.md"` respectively

6. **Profile schema validation passes**
   - Given the updated profile structure
   - When schema validation runs
   - Then it passes with the new directory/file names

## Metadata
- **Complexity**: Medium
- **Labels**: coding-agent-agnostic, migration, sprint-1
- **Required Skills**: Git, shell scripting, profile structure knowledge
