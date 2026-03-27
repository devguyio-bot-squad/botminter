---
status: pending
created: 2026-03-21
started: null
completed: null
---
# Task: Member Tuning Skill (Team-Manager)

## Description
Create a conversational skill for the team-manager that handles tuning individual member configurations. "Tuning" means modifying any artifact in a member's configuration stack: PROMPT.md, CLAUDE.md, ralph.yml (hats), skills, and even PROCESS.md when the process is the root cause. This is the "troubleshooting and improvement" skill for when a member isn't working well.

## Background
Each member's behavior is determined by a stack of configuration artifacts:

| Artifact | Controls | Typical change reason |
|----------|----------|----------------------|
| `PROMPT.md` | Work objective — *what* the member does | Reassignment, scope change |
| `CLAUDE.md` | Context — *how* the coding agent understands the workspace | Missing context, wrong assumptions |
| `ralph.yml` (hats) | Behavioral modes — *how* the member switches roles | Process mismatch, missing capability |
| `skills/` | Capabilities — *what tools* the member can invoke | Adding new capability, fixing broken skill |
| `knowledge/` | Domain context — *what* the member knows | Already handled by knowledge-manager |
| `invariants/` | Constraints — *what rules* must be followed | Already handled by knowledge-manager |
| `PROCESS.md` | Workflow — status transitions and conventions | Process is causing the problem |

When something isn't working, the team-manager needs to diagnose which artifact is the problem and apply a targeted fix. This skill provides a systematic diagnostic flow.

## Reference Documentation
**Required — read before implementation:**
- **Skill development guide**: `knowledge/claude-code-skill-development-guide.md` — the SKILL.md MUST comply with this guide (frontmatter format, naming, progressive disclosure, trigger phrases in description)
- Team agreements convention: task-01
- Team-manager ralph.yml: `profiles/scrum-compact/roles/team-manager/ralph.yml`
- Superman ralph.yml (example of full hat config): `profiles/scrum-compact/roles/superman/ralph.yml`
- Knowledge-manager skill: `profiles/scrum/skills/knowledge-manager/SKILL.md`

**Additional References:**
- PROCESS.md for status transitions
- Member skeleton structure: `profiles/scrum-compact/roles/superman/`

## Technical Requirements

1. **Skill file**: `profiles/<profile>/roles/team-manager/coding-agent/skills/member-tuning/SKILL.md` for both profiles

2. **Diagnostic flow** (conversational):
   - **Identify the member**: Which member needs tuning? Show current members and their status.
   - **Identify the symptom**: What's going wrong? Options:
     - Member is doing the wrong thing → likely PROMPT.md
     - Member doesn't understand the context → likely CLAUDE.md
     - Member isn't switching hats correctly → likely ralph.yml hat triggers/instructions
     - Member is missing a capability → likely skills/
     - Member is blocked by the process → likely PROCESS.md
     - Member keeps making the same mistake → likely knowledge/ or invariants/ (defer to knowledge-manager)
   - **Inspect the artifact**: Read the relevant file and show it to the operator
   - **Propose changes**: Suggest specific edits based on the symptom
   - **Apply changes**: Edit the file in the `team/` submodule
   - **Record the change**: Write an agreement decision if the change is significant

3. **PROMPT.md tuning**:
   - Show current objective, work scope, completion condition
   - Propose updated version based on the problem description
   - Common fixes: narrowing scope, clarifying completion criteria, adding constraints

4. **CLAUDE.md tuning**:
   - Show current context sections
   - Propose additions/removals based on what the member misunderstands
   - Common fixes: adding project-specific context, clarifying workspace layout, fixing stale references

5. **Hat tuning** (ralph.yml):
   - Show current hats with triggers, publishes, and instruction summaries
   - Diagnose: wrong triggers? missing hat? instructions too vague? wrong publish events?
   - Propose hat modifications: adjust instructions, add/remove triggers, refine publish events
   - Can add new hats or remove unused ones
   - Warn if hat changes affect the board-scanner dispatch table

6. **Skill tuning**:
   - List current skills and their auto-inject status
   - Add new skill references to ralph.yml skill dirs
   - Remove stale skill references
   - Create new SKILL.md files for custom skills

7. **PROCESS.md tuning** (when process is the root cause):
   - Identify which process aspect is causing the problem
   - Propose targeted edits to PROCESS.md
   - Cross-reference with agreements convention (log the process change as a decision)
   - Note: large-scale process changes should go through the Process Evolution skill instead

8. **Propagation reminder**: After any change to `team/`, remind the operator to:
   - Commit changes in the team repo
   - Run `bm teams sync` to propagate to workspaces
   - Restart affected members (`bm stop <member> && bm start <member>`)

9. **Skill registration**: Add to team-manager's ralph.yml skill dirs

## Dependencies
- Task 01 (Team Agreements Convention) — for decision record format

## Implementation Approach

1. Create the SKILL.md with the diagnostic decision tree
2. Include example edits for each artifact type
3. Document the inspection commands for each artifact
4. Add to both profile's team-manager skill directories

## Acceptance Criteria

1. **Skill is discoverable**
   - Given a team-manager workspace
   - When I run `ralph tools skill list`
   - Then `member-tuning` appears in the list

2. **Diagnostic flow identifies artifact**
   - Given a member with a behavioral problem
   - When the skill runs through the diagnostic flow
   - Then it correctly identifies which artifact(s) to inspect

3. **PROMPT.md can be tuned**
   - Given a member whose objective needs updating
   - When the skill proposes changes to PROMPT.md
   - Then the updated PROMPT.md has a clearer objective with specific scope and completion criteria

4. **Hat configuration can be tuned**
   - Given a member with a hat that has incorrect triggers
   - When the skill modifies ralph.yml
   - Then the hat triggers are updated and the board-scanner dispatch table is checked for consistency

5. **PROCESS.md can be tuned**
   - Given a process-level problem (e.g., missing auto-advance status)
   - When the skill identifies PROCESS.md as the root cause
   - Then it proposes targeted edits and records a decision in agreements/

6. **Changes are propagated**
   - Given any artifact change
   - When the change is applied
   - Then the skill reminds the operator to commit, sync, and restart

7. **Skill exists in both profiles**
   - Given the `scrum` and `scrum-compact` profiles
   - When I inspect `roles/team-manager/coding-agent/skills/member-tuning/`
   - Then SKILL.md exists with the diagnostic procedure

## Metadata
- **Complexity**: High
- **Labels**: skill, team-manager, member-tuning, troubleshooting, day-2
- **Required Skills**: Markdown, SKILL.md format, ralph.yml schema, hat configuration, profile structure
