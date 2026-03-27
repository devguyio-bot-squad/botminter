---
status: pending
created: 2026-03-21
started: null
completed: null
---
# Task: Retrospective Skill (Team-Manager)

## Description
Create a conversational skill for the team-manager role that guides a structured retrospective. The retro examines what went well, what didn't, and produces action items. Outputs are written to `agreements/retros/` and can optionally feed into role/process change proposals.

## Background
The team-manager currently has only a board-scanner skill and an executor hat. It picks up `mgr:todo` issues and executes them, but has no structured way to reflect on team performance or propose improvements. Retrospectives are a standard agile ceremony that drives continuous improvement.

In botminter's model, the team-manager has full access to the team repo (via `team/` submodule), GitHub issues, and the project board. It can inspect completed work, read comments, check cycle times, and analyze patterns. This makes it the ideal owner for the retro skill — it has the context.

The retro skill is conversational: it can be triggered by an `mgr:todo` issue requesting a retro, or by the operator via `bm chat team-manager --hat executor`. The skill guides the conversation, gathers input, and produces a structured output.

## Reference Documentation
**Required — read before implementation:**
- **Skill development guide**: `knowledge/claude-code-skill-development-guide.md` — the SKILL.md MUST comply with this guide (frontmatter format, naming, progressive disclosure, trigger phrases in description)
- Team agreements convention: task-01
- Existing team-manager ralph.yml: `profiles/scrum-compact/roles/team-manager/ralph.yml`
- Existing board-scanner skill: `profiles/scrum-compact/roles/team-manager/coding-agent/skills/board-scanner/SKILL.md`
- PROCESS.md: `profiles/scrum-compact/PROCESS.md`

**Additional References:**
- Knowledge-manager skill format: `profiles/scrum/skills/knowledge-manager/SKILL.md`
- Comment format convention in PROCESS.md

## Technical Requirements

1. **Skill file**: `profiles/<profile>/roles/team-manager/coding-agent/skills/retrospective/SKILL.md` for both `scrum` and `scrum-compact` profiles

2. **Retro structure** (guided conversation flow):
   - **Scope**: What period/milestone/epic are we reflecting on?
   - **Data gathering**: Query completed issues, cycle times, error statuses, rejection loops from the board
   - **What went well**: Identify patterns of success (fast completions, clean reviews, good designs)
   - **What didn't go well**: Identify pain points (long cycle times, repeated rejections, error statuses, blocked work)
   - **Action items**: Concrete proposals — each tagged as one of:
     - `process-change` — suggests a Process Evolution action
     - `role-change` — suggests a Role Management action
     - `member-tuning` — suggests a Member Tuning action
     - `knowledge-update` — can be handled by existing knowledge-manager
     - `norm` — proposes a new team norm

3. **Output**: Write a retro summary to `agreements/retros/NNNN-<title>.md` with:
   - YAML frontmatter per the agreements convention
   - Sections for scope, went-well, didn't-go-well, action items
   - Each action item cross-referenced to the type of follow-up

4. **Board data queries**: The skill should instruct the executor to:
   - List completed issues in the retro scope (milestone or date range)
   - Count rejection loops (comments matching rejection patterns)
   - Identify issues that hit `error` status
   - Check for long-lived `in-progress` statuses

5. **Action item follow-through**: After writing the retro, the skill should:
   - For each `process-change` action: suggest creating an `mgr:todo` issue referencing the retro
   - For each `norm` action: write directly to `agreements/norms/`
   - For other types: document the recommendation in the retro, leave execution to other skills

6. **Skill registration**: Add the skill to the team-manager's `ralph.yml` skill dirs so it's discoverable via `ralph tools skill list`

## Dependencies
- Task 01 (Team Agreements Convention) — for output format and directory structure

## Implementation Approach

1. Create the SKILL.md file with the full retro procedure
2. Add skill directory to team-manager's ralph.yml skill paths in both profiles
3. Include example queries for board data gathering
4. Document the action item types and their follow-through paths

## Acceptance Criteria

1. **Skill is discoverable**
   - Given a team-manager workspace
   - When I run `ralph tools skill list`
   - Then `retrospective` appears in the list

2. **Retro produces structured output**
   - Given the skill is loaded
   - When the executor runs a retro for a completed milestone
   - Then a file is written to `agreements/retros/NNNN-<title>.md` with valid frontmatter and all required sections

3. **Board data is queried**
   - Given a team repo with completed issues
   - When the retro skill gathers data
   - Then it queries the project board for completed issues, rejection loops, and error statuses

4. **Action items are typed**
   - Given a retro with action items
   - When the retro summary is written
   - Then each action item is tagged with its follow-up type (process-change, role-change, member-tuning, knowledge-update, norm)

5. **Norms are auto-created**
   - Given a retro with `norm` action items
   - When the retro completes
   - Then the norms are written to `agreements/norms/` with proper frontmatter

6. **Skill exists in both profiles**
   - Given the `scrum` and `scrum-compact` profiles
   - When I inspect `roles/team-manager/coding-agent/skills/retrospective/`
   - Then SKILL.md exists with the retro procedure

## Metadata
- **Complexity**: Medium
- **Labels**: skill, team-manager, retrospective, team-management
- **Required Skills**: Markdown, SKILL.md format, GitHub Projects v2 queries, agreements convention
