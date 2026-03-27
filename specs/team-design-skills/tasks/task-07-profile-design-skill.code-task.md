---
status: pending
created: 2026-03-21
started: null
completed: null
---
# Task: Profile Design Skill (Minty)

## Description
Create a conversational skill for Minty that handles designing and tweaking profiles at the template level — before any team exists, or when the team is broken and the operator needs to fix things from outside. This is Minty's equivalent of the team-manager's design capabilities, but operating on profile templates rather than live team repos.

## Background
Minty is the operator's always-available assistant. It works even when no teams are configured or when a team is broken. Minty's current skills (hire-guide, profile-browser, team-overview, workspace-doctor) help operators understand and troubleshoot their setup, but don't help them *design* profiles.

Profile design is about crafting the template that `bm init` will extract: defining roles, processes, statuses, hats, skills, and knowledge at the profile level. This is useful for:
- **New profile creation**: "I want a profile for a data engineering team"
- **Profile customization**: "I want to fork scrum-compact and add a security auditor role"
- **Troubleshooting**: "My team is broken because the profile has a bad hat configuration — help me fix it"

Minty operates on profiles stored at `~/.config/botminter/profiles/` (extracted copies) or on the embedded profiles in the source tree (for development). It cannot modify live team repos — that's the team-manager's job.

## Reference Documentation
**Required — read before implementation:**
- **Skill development guide**: `knowledge/claude-code-skill-development-guide.md` — the SKILL.md MUST comply with this guide (frontmatter format, naming, progressive disclosure, trigger phrases in description)
- Existing profile-browser skill: `minty/.claude/skills/profile-browser/SKILL.md`
- Existing workspace-doctor skill: `minty/.claude/skills/workspace-doctor/SKILL.md`
- Profile structure: `profiles/scrum-compact/` and `profiles/scrum/`
- Profile manifest: `profiles/scrum-compact/botminter.yml`

**Additional References:**
- Member skeleton examples: `profiles/scrum-compact/roles/superman/`
- Hat configuration examples: `profiles/scrum-compact/roles/superman/ralph.yml`
- Team agreements convention: task-01 (for understanding the convention Minty should know about)

## Technical Requirements

1. **Skill file**: `minty/.claude/skills/profile-design/SKILL.md`

2. **Supported operations**:
   - **Browse profiles**: Enhanced version of profile-browser — show roles, statuses, hats, skills for any profile
   - **Design new role**: Guided creation of a role template within a profile — PROMPT.md, CLAUDE.md, ralph.yml, hats, skills
   - **Design process**: Create or modify PROCESS.md and statuses in botminter.yml for a profile
   - **Design hats**: Create or modify hat collections for a role — triggers, instructions, publish events
   - **Fork profile**: Create a new profile by copying and customizing an existing one
   - **Troubleshoot profile**: Diagnose why a profile isn't working — validate hat triggers against statuses, check for missing skills, verify board-scanner dispatch tables

3. **Profile validation**: After any design change, validate:
   - All statuses in botminter.yml have corresponding roles that handle them
   - All hat triggers in ralph.yml reference valid statuses or events
   - Board-scanner dispatch tables match the status lifecycle
   - PROCESS.md is consistent with botminter.yml statuses
   - Required files exist in each role skeleton (PROMPT.md, CLAUDE.md, ralph.yml, .botminter.yml)

4. **Conversational flow for role design**:
   - Ask: What does this role do? What's its purpose?
   - Ask: Is it single-hat or multi-hat? What hats does it need?
   - For each hat: What triggers it? What does it publish? What are its instructions?
   - Generate: Role directory with all skeleton files
   - Validate: Run profile validation
   - Show: Summary of what was created

5. **Conversational flow for process design**:
   - Show: Current status lifecycle (if modifying existing)
   - Ask: What lifecycle stages does the team need?
   - Ask: Which stages need human review gates?
   - Ask: Which stages should auto-advance?
   - Generate: PROCESS.md and botminter.yml status entries
   - Validate: Status graph validation (no orphans, no dead ends, no infinite loops)

6. **Troubleshooting flow**:
   - Inspect: Read the profile's botminter.yml, PROCESS.md, and role ralph.yml files
   - Diagnose: Check for common issues:
     - Hat triggers that don't match any status
     - Statuses with no role handler
     - Missing board-scanner dispatch entries
     - Broken skill references
     - Inconsistent comment format / emoji mappings
   - Fix: Propose targeted fixes
   - Validate: Re-run profile validation after fixes

7. **Relationship with team-manager skills**: Minty should be aware that live teams have a team-manager with richer context. When the operator has a working team, Minty should suggest using `bm chat team-manager` for team-level changes. Minty handles profile-level design and broken-team troubleshooting.

## Dependencies
- Task 01 (Team Agreements Convention) — Minty should know the convention exists so it can include agreements/ in new profiles

## Implementation Approach

1. Create the SKILL.md with the full profile design procedure
2. Include role skeleton templates
3. Include profile validation checklist
4. Include troubleshooting decision tree
5. Place in `minty/.claude/skills/profile-design/`

## Acceptance Criteria

1. **Skill is discoverable in Minty**
   - Given a Minty session
   - When the operator asks "help me design a profile"
   - Then Minty loads the profile-design skill

2. **Browse shows full profile detail**
   - Given an extracted profile at `~/.config/botminter/profiles/`
   - When the operator asks to browse it
   - Then roles, statuses, hats, skills, and process are displayed

3. **Role design creates valid skeleton**
   - Given the operator designs a new "data-engineer" role
   - When the conversation completes
   - Then the role directory contains PROMPT.md, CLAUDE.md, ralph.yml, .botminter.yml with valid content

4. **Process design generates consistent files**
   - Given the operator designs a 5-status lifecycle
   - When the conversation completes
   - Then PROCESS.md and botminter.yml statuses are consistent and the status graph validates

5. **Troubleshooting identifies issues**
   - Given a profile with a hat trigger that doesn't match any status
   - When the operator asks to troubleshoot
   - Then the skill identifies the orphaned trigger and proposes a fix

6. **Profile validation catches errors**
   - Given a profile with missing role skeleton files
   - When validation runs
   - Then it reports the missing files with their expected paths

7. **Minty defers to team-manager when appropriate**
   - Given a working team exists
   - When the operator asks to change a live team's process
   - Then Minty suggests using `bm chat team-manager` instead

## Metadata
- **Complexity**: High
- **Labels**: skill, minty, profiles, design, troubleshooting
- **Required Skills**: Markdown, SKILL.md format, profile structure, botminter.yml schema, ralph.yml schema, validation
