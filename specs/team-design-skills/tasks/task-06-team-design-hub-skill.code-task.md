---
status: pending
created: 2026-03-21
started: null
completed: null
---
# Task: Team Design Hub Skill (Team-Manager)

## Description
Create an entry-point skill for the team-manager that routes the operator to the appropriate team design capability: retrospective, role management, member tuning, or process evolution. This is the conversational front door for all day-2 team design operations.

## Background
The team-manager will have four specialized skills (tasks 02-05), each with its own conversational flow. The operator shouldn't need to know which skill to invoke — they should be able to say "something isn't working" or "we need to change our process" and get routed to the right skill.

The hub also provides a unified view of the team's design state: recent agreements, pending retro action items, and the current process summary. It's the team-manager's equivalent of a dashboard for team evolution.

## Reference Documentation
**Required — read before implementation:**
- **Skill development guide**: `knowledge/claude-code-skill-development-guide.md` — the SKILL.md MUST comply with this guide (frontmatter format, naming, progressive disclosure, trigger phrases in description)
- Retrospective skill: task-02
- Role management skill: task-03
- Member tuning skill: task-04
- Process evolution skill: task-05
- Team agreements convention: task-01
- Team-manager ralph.yml: `profiles/scrum-compact/roles/team-manager/ralph.yml`

## Technical Requirements

1. **Skill file**: `profiles/<profile>/roles/team-manager/coding-agent/skills/team-design/SKILL.md` for both profiles

2. **Routing logic** — detect intent and load the appropriate skill:

   | Operator says... | Route to |
   |-----------------|----------|
   | "Let's do a retro" / "what went well" / "how did the sprint go" | Retrospective |
   | "Add a role" / "remove a role" / "we need a new role" | Role Management |
   | "X member isn't working right" / "fix the architect's hats" / "tune the prompt" | Member Tuning |
   | "Change the process" / "add a review step" / "modify the workflow" | Process Evolution |
   | "Show me the team" / "what's our current setup" | Dashboard (see below) |

3. **Dashboard view**: When the operator asks for an overview, show:
   - Current roles and member count per role
   - Recent agreements (last 5 decisions and norms)
   - Pending retro action items (from last retro, if any)
   - Current process summary (key statuses and gates)

4. **Retro-first flow**: The hub should support starting with a retro and then routing to follow-up actions:
   - "Let's design the team" → "Want to start with a retro first?" → retro → action items → route each item to the appropriate skill

5. **Skill loading**: The hub uses `ralph tools skill load <skill-name>` to hand off to specialized skills. It does NOT duplicate their logic — it routes and provides context.

6. **Agreement context**: When routing to any skill, the hub should pass along:
   - Recent relevant agreements (so the operator doesn't repeat decisions)
   - Pending action items from retros (so the operator can pick up where they left off)

7. **Skill registration**: Add to team-manager's ralph.yml skill dirs. Consider making this the default skill (auto-injected or prominently referenced in PROMPT.md).

## Dependencies
- Task 01 (Team Agreements Convention)
- Task 02 (Retrospective Skill)
- Task 03 (Role Management Skill)
- Task 04 (Member Tuning Skill)
- Task 05 (Process Evolution Skill)

## Implementation Approach

1. Create the SKILL.md with routing logic and dashboard procedure
2. Include intent detection patterns for each sub-skill
3. Document the retro-first flow
4. Add to both profile's team-manager skill directories
5. Update team-manager's PROMPT.md to reference the team-design skill as a primary capability

## Acceptance Criteria

1. **Skill is discoverable**
   - Given a team-manager workspace
   - When I run `ralph tools skill list`
   - Then `team-design` appears in the list

2. **Intent routing works**
   - Given the operator says "let's do a retro"
   - When the hub processes the intent
   - Then it loads the retrospective skill

3. **Dashboard shows team state**
   - Given a team with agreements, members, and a defined process
   - When the operator asks "show me the team"
   - Then the hub displays roles, recent agreements, pending action items, and process summary

4. **Retro-first flow works**
   - Given the operator says "let's design the team"
   - When the hub offers a retro-first approach and the operator accepts
   - Then the retro runs, and action items are presented for follow-up routing

5. **Agreements context is passed**
   - Given recent agreements exist
   - When the hub routes to a sub-skill
   - Then the sub-skill receives context about relevant recent decisions

6. **Skill exists in both profiles**
   - Given the `scrum` and `scrum-compact` profiles
   - When I inspect `roles/team-manager/coding-agent/skills/team-design/`
   - Then SKILL.md exists with the routing and dashboard procedure

## Metadata
- **Complexity**: Medium
- **Labels**: skill, team-manager, team-design, routing, day-2
- **Required Skills**: Markdown, SKILL.md format, skill loading, intent detection
