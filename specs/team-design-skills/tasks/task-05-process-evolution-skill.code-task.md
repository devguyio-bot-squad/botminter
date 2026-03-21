---
status: pending
created: 2026-03-21
started: null
completed: null
---
# Task: Process Evolution Skill (Team-Manager)

## Description
Create a conversational skill for the team-manager that handles deliberate, team-wide process changes. This covers updating PROCESS.md, modifying statuses and labels in `botminter.yml`, changing status transitions, and evolving the workflow. Every process change is recorded as a team agreement.

## Background
PROCESS.md and `botminter.yml` define the team's workflow — status transitions, comment conventions, review gates, auto-advance rules, and coordination protocols. Today, process evolution is mentioned in PROCESS.md but has no tooling support. The section says "formal path = PR, informal path = direct edit" but provides no structured way to analyze the current process, identify improvements, or safely apply changes.

Process changes are high-impact — a bad status transition can break the board scanner, orphan issues, or create infinite loops. This skill provides guardrails: it analyzes the current process, validates proposed changes against the status graph, and records decisions.

Unlike member-tuning (which does surgical fixes), process evolution handles team-wide workflow redesign: "we want to add a security review gate", "we want to remove the lead review step", "we want to change how auto-advance works."

## Reference Documentation
**Required — read before implementation:**
- **Skill development guide**: `knowledge/claude-code-skill-development-guide.md` — the SKILL.md MUST comply with this guide (frontmatter format, naming, progressive disclosure, trigger phrases in description)
- Team agreements convention: task-01
- PROCESS.md (scrum-compact): `profiles/scrum-compact/PROCESS.md`
- PROCESS.md (scrum): `profiles/scrum/PROCESS.md`
- Profile manifest: `profiles/scrum-compact/botminter.yml` (statuses, labels, views sections)
- Board-scanner skill: `profiles/scrum-compact/roles/team-manager/coding-agent/skills/board-scanner/SKILL.md`

**Additional References:**
- Status-workflow skill: `profiles/scrum-compact/coding-agent/skills/status-workflow/`
- Superman ralph.yml hat triggers: `profiles/scrum-compact/roles/superman/ralph.yml`

## Technical Requirements

1. **Skill file**: `profiles/<profile>/roles/team-manager/coding-agent/skills/process-evolution/SKILL.md` for both profiles

2. **Supported operations**:
   - **Show current process**: Render the status graph (epic lifecycle, story lifecycle) as a readable summary
   - **Add status**: Add a new status to the lifecycle with role, position, and transitions
   - **Remove status**: Impact analysis (issues in this status? hats triggered by it? board scanner dispatching it?) then safe removal
   - **Modify transitions**: Change what status follows another, add/remove rejection loops
   - **Add/remove review gate**: Add or remove a supervised-mode gate
   - **Modify auto-advance rules**: Change which statuses auto-advance
   - **Update comment format**: Change emoji mappings, attribution format
   - **Add/remove labels**: Add new kind labels or status-related labels

3. **Status graph validation**: Before applying any change, the skill must validate:
   - **No orphan statuses**: Every status is reachable from an entry point
   - **No dead ends**: Every non-terminal status has at least one outgoing transition
   - **No infinite loops**: Rejection loops terminate (there's always a forward path)
   - **Role coverage**: Every status has a role that can handle it (a hat with matching triggers)
   - **Board scanner consistency**: Dispatch table matches the defined statuses

4. **Conversational flow for adding a status**:
   - Ask: What role owns this status? (Must be an existing role or triggers task-03 role management)
   - Ask: Where in the lifecycle does it go? (After which status? Before which status?)
   - Ask: Is it a review gate? (If yes, add to supervised mode config)
   - Ask: Does it auto-advance? (If yes, add to auto-advance config)
   - Validate: Run status graph validation
   - Apply: Update PROCESS.md, `botminter.yml` statuses, relevant hat triggers in ralph.yml
   - Record: Write agreement decision

5. **Conversational flow for removing a status**:
   - Check: Are there issues currently in this status?
   - Check: Which hats trigger on this status?
   - Check: Is this status referenced in PROCESS.md rejection loops?
   - Propose: Status reassignment for affected issues
   - Propose: Hat trigger updates
   - Validate: Run status graph validation on the proposed state
   - Apply: Update all affected files
   - Record: Write agreement decision

6. **Cross-file consistency**: Process changes often touch multiple files:
   - `PROCESS.md` — human-readable workflow documentation
   - `botminter.yml` — machine-readable statuses, labels, views
   - `roles/*/ralph.yml` — hat triggers and instructions
   - `coding-agent/skills/board-scanner/SKILL.md` — dispatch tables
   - `coding-agent/skills/status-workflow/` — GraphQL mutations
   The skill must identify ALL affected files and update them together.

7. **Retro integration**: The skill can optionally start from a retrospective output:
   - Read `agreements/retros/` for recent action items tagged `process-change`
   - Present them as starting points for the conversation
   - Cross-reference the final decision back to the retro

8. **Skill registration**: Add to team-manager's ralph.yml skill dirs

## Dependencies
- Task 01 (Team Agreements Convention) — for decision record format
- Task 02 (Retrospective Skill) — optional, for retro-driven process changes

## Implementation Approach

1. Create the SKILL.md with the full process evolution procedure
2. Include the status graph validation rules
3. Document the cross-file update procedure
4. Include example status additions and removals
5. Add to both profile's team-manager skill directories

## Acceptance Criteria

1. **Skill is discoverable**
   - Given a team-manager workspace
   - When I run `ralph tools skill list`
   - Then `process-evolution` appears in the list

2. **Show process renders status graph**
   - Given a team repo with PROCESS.md
   - When the skill shows the current process
   - Then it displays the epic and story lifecycle statuses with transitions

3. **Add status updates all files**
   - Given a request to add a `security:review` status after `dev:code-review`
   - When the skill applies the change
   - Then PROCESS.md, botminter.yml, relevant ralph.yml hats, and board-scanner are all updated consistently

4. **Remove status shows impact**
   - Given a request to remove `lead:design-review`
   - When the skill analyzes the impact
   - Then it lists affected issues, hats, and transitions before asking for confirmation

5. **Status graph validation catches errors**
   - Given a proposed change that creates an orphan status
   - When the skill validates the status graph
   - Then it rejects the change with a clear explanation of the orphan

6. **Agreement record is created**
   - Given any process change
   - When the change is applied
   - Then a decision record exists in `agreements/decisions/` with the before/after process state

7. **Retro integration works**
   - Given a retro with `process-change` action items
   - When the skill starts from the retro
   - Then it presents the action items as conversation starting points

8. **Skill exists in both profiles**
   - Given the `scrum` and `scrum-compact` profiles
   - When I inspect `roles/team-manager/coding-agent/skills/process-evolution/`
   - Then SKILL.md exists with the process evolution procedure

## Metadata
- **Complexity**: High
- **Labels**: skill, team-manager, process, workflow, day-2
- **Required Skills**: Markdown, SKILL.md format, botminter.yml schema, status graph analysis, cross-file consistency
