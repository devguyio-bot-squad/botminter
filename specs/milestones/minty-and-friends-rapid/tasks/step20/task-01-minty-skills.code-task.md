---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Minty Skills Implementation

## Description
Implement Minty's four composable skills: team-overview, profile-browser, hire-guide, and workspace-doctor. Each skill follows the SKILL.md format and is extracted to `~/.config/botminter/minty/skills/` by `bm profiles init`.

## Background
Minty's capabilities are entirely skill-driven. The persona shell (prompt.md) tells the coding agent to use skills for BotMinter operations. Each skill is a self-contained SKILL.md file that instructs the agent how to perform a specific operation using the BotMinter config, CLI, and filesystem.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Minty Skills")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 20)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Create skills in `minty/skills/`:
   - **`team-overview/SKILL.md`** — reads `~/.botminter/config.yml`, lists teams, members, status. Shows workspace repo URLs, member roles, running state.
   - **`profile-browser/SKILL.md`** — reads `~/.config/botminter/profiles/`, lists and describes available profiles, roles, coding agents, statuses.
   - **`hire-guide/SKILL.md`** — interactive guide for `bm hire` decisions. Shows available roles, explains implications, suggests names.
   - **`workspace-doctor/SKILL.md`** — diagnoses common workspace issues: stale submodules, broken symlinks, missing files, outdated context. Runs checks and reports findings.
2. Each skill follows SKILL.md format (YAML frontmatter + markdown)
3. Skills should reference BotMinter file paths and CLI commands correctly
4. Skills should handle missing data gracefully (e.g., no teams configured yet)

## Dependencies
- Task 2 of Step 19 (Minty launch command working, skills directory exists)

## Implementation Approach
1. Study existing skills (gh skill, status-workflow) for the SKILL.md format
2. Write team-overview skill with config reading instructions
3. Write profile-browser skill with filesystem browsing instructions
4. Write hire-guide skill with interactive decision flow
5. Write workspace-doctor skill with diagnostic checks
6. Verify all skills are extracted by `bm profiles init`

## Acceptance Criteria

1. **All four skills exist**
   - Given `minty/skills/`
   - When listing subdirectories
   - Then `team-overview/`, `profile-browser/`, `hire-guide/`, `workspace-doctor/` exist

2. **Skills follow SKILL.md format**
   - Given each skill's `SKILL.md`
   - When reading the file
   - Then it has YAML frontmatter (name, description, version) and markdown instructions

3. **team-overview reads config**
   - Given the team-overview skill
   - When reading its instructions
   - Then it references `~/.botminter/config.yml` and describes how to list teams/members

4. **profile-browser reads profiles dir**
   - Given the profile-browser skill
   - When reading its instructions
   - Then it references `~/.config/botminter/profiles/` and describes profile browsing

5. **hire-guide is interactive**
   - Given the hire-guide skill
   - When reading its instructions
   - Then it describes an interactive decision flow for hiring

6. **workspace-doctor runs diagnostics**
   - Given the workspace-doctor skill
   - When reading its instructions
   - Then it describes checking for stale submodules, broken symlinks, missing files

7. **Skills extracted by profiles init**
   - Given `bm profiles init` running
   - When extraction completes
   - Then `~/.config/botminter/minty/skills/` contains all four skills

8. **Skills handle missing data**
   - Given no teams configured
   - When a skill references team data
   - Then instructions include graceful handling for missing data

## Metadata
- **Complexity**: Medium
- **Labels**: minty, skills, sprint-6
- **Required Skills**: SKILL.md format, BotMinter domain knowledge, Markdown
