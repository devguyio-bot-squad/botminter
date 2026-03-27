---
status: pending
created: 2026-03-21
started: null
completed: null
---
# Task: Role Management Skill (Team-Manager)

## Description
Create a conversational skill for the team-manager that handles adding, removing, and modifying roles in the team. This is a day-2 operation — the team already exists and the operator wants to change its composition. Every role change is recorded as a team agreement (decision record).

## Background
Today, `bm hire <role>` adds a member and `bm roles list` shows available roles, but there is no `bm fire`, no way to add new role definitions, and no way to remove roles from the team's profile. The team-manager currently has no tooling for structural team changes.

Role management is more than just hiring/firing members — it includes:
- Adding a new role definition to `botminter.yml` (e.g., "we need a dedicated QE role")
- Removing a role that's no longer needed
- Understanding the impact of role changes on the workflow (which statuses, hats, and transitions are affected)

The team-manager owns this because it has team context (who's doing what, what's on the board). The skill is conversational — it guides the operator through the implications before making changes.

## Reference Documentation
**Required — read before implementation:**
- **Skill development guide**: `knowledge/claude-code-skill-development-guide.md` — the SKILL.md MUST comply with this guide (frontmatter format, naming, progressive disclosure, trigger phrases in description)
- Team agreements convention: task-01
- Profile manifest format: `profiles/scrum-compact/botminter.yml` and `profiles/scrum/botminter.yml`
- Existing hire-guide skill (Minty): `minty/.claude/skills/hire-guide/SKILL.md`
- Team-manager ralph.yml: `profiles/scrum-compact/roles/team-manager/ralph.yml`

**Additional References:**
- PROCESS.md for status/role relationships
- Role skeleton structure: `profiles/scrum-compact/roles/`

## Technical Requirements

1. **Skill file**: `profiles/<profile>/roles/team-manager/coding-agent/skills/role-management/SKILL.md` for both profiles

2. **Supported operations**:
   - **List roles**: Show current roles from `botminter.yml`, with member count per role
   - **Add role**: Guide through defining a new role — name, description, member skeleton (PROMPT.md, CLAUDE.md, ralph.yml with hats), associated statuses
   - **Remove role**: Impact analysis first (active members? issues in role's statuses? hats referencing the role?), then guided removal
   - **Inspect role**: Show a role's full configuration — hats, skills, knowledge, invariants, associated statuses

3. **Conversational flow for adding a role**:
   - Ask: What does this role do? What gap does it fill?
   - Ask: What hats should it wear? (Can reference existing hats as templates)
   - Ask: What statuses does it need? (Check for conflicts with existing statuses)
   - Generate: Member skeleton files (PROMPT.md, CLAUDE.md, ralph.yml, hat dirs)
   - Generate: Agreement record in `agreements/decisions/`
   - Guide: "Run `bm hire <new-role> --name <name>` to hire a member into this role"

4. **Conversational flow for removing a role**:
   - Check: Are there active members in this role? List them.
   - Check: Are there issues in statuses owned by this role?
   - Warn: These statuses will become orphaned — suggest reassignment
   - Generate: Agreement record documenting the removal and rationale
   - Execute: Remove the role directory from `roles/` and update `botminter.yml`
   - Guide: "Members in this role should be stopped and their workspaces cleaned up"

5. **Impact analysis**: Before any role change, the skill must:
   - List affected statuses in `botminter.yml`
   - List affected hats across all members
   - List affected knowledge/invariant files scoped to the role
   - Check for references in PROCESS.md
   - Present the impact summary to the operator before proceeding

6. **Agreement record**: Every role change writes a decision to `agreements/decisions/` with:
   - Context: why the change was needed
   - The change: what role was added/removed/modified
   - Impact: what was affected
   - Follow-up: what the operator needs to do (hire, fire, sync, etc.)

7. **Skill registration**: Add to team-manager's ralph.yml skill dirs

## Dependencies
- Task 01 (Team Agreements Convention) — for decision record format

## Implementation Approach

1. Create the SKILL.md with the full role management procedure
2. Include templates for member skeleton generation (PROMPT.md, CLAUDE.md, ralph.yml stubs)
3. Document the impact analysis queries
4. Add to both profile's team-manager skill directories

## Acceptance Criteria

1. **Skill is discoverable**
   - Given a team-manager workspace
   - When I run `ralph tools skill list`
   - Then `role-management` appears in the list

2. **List roles shows composition**
   - Given a team with hired members
   - When the skill lists roles
   - Then each role shows its description, member count, and associated statuses

3. **Add role creates skeleton**
   - Given the operator wants to add a "security-auditor" role
   - When the skill completes the conversation
   - Then a role directory exists with PROMPT.md, CLAUDE.md, ralph.yml, and the role is added to botminter.yml

4. **Remove role shows impact**
   - Given a role with active members and owned statuses
   - When the operator asks to remove it
   - Then the skill shows affected members, statuses, and asks for confirmation before proceeding

5. **Agreement record is created**
   - Given any role change operation
   - When the change is applied
   - Then a decision record exists in `agreements/decisions/` with context, change, and impact

6. **Skill exists in both profiles**
   - Given the `scrum` and `scrum-compact` profiles
   - When I inspect `roles/team-manager/coding-agent/skills/role-management/`
   - Then SKILL.md exists with the role management procedure

## Metadata
- **Complexity**: High
- **Labels**: skill, team-manager, roles, team-management, day-2
- **Required Skills**: Markdown, SKILL.md format, botminter.yml schema, profile structure, agreements convention
