---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Team Manager Member Content Files

## Description
Write the Team Manager's operational content files: board-scanner skill (scoped to `mgr:*` statuses), ralph.yml (persistent loop with executor hat), context.md (role context with agent tags), executor hat instructions, and knowledge files. After this, the Team Manager can be hired and launched.

## Background
The Team Manager runs as a regular Ralph instance. Board scanning is handled by the `board-scanner` auto-inject skill (scoped to `mgr:*` statuses), following the pattern established in Step 13. Its executor hat picks up tasks and executes them in the `team/` submodule.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Team Manager Role")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 16)
- Existing board-scanner skills in `profiles/scrum/roles/*/coding-agent/skills/board-scanner/` for pattern

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Write `coding-agent/skills/board-scanner/SKILL.md` — auto-inject skill:
   - Scoped to `mgr:todo` status on the team repo's GitHub Project
   - Filter by `role/team-manager` label
   - Follow the board-scanner skill pattern from existing roles (e.g., `profiles/scrum/roles/architect/coding-agent/skills/board-scanner/`)
2. Write `ralph.yml`:
   - Persistent loop configuration
   - Single `executor` hat
   - `skills.dirs` pointing to `team/coding-agent/skills`
3. Write `context.md`:
   - Team manager role context
   - Wrap Claude Code-specific sections with agent tags
   - Include guidance on operating within the `team/` submodule
4. Write executor hat instructions in `hats/executor/`:
   - Pick up tasks from board (dispatched by coordinator via board-scanner skill)
   - Execute in `team/` submodule
   - Transition through `mgr:todo` -> `mgr:in-progress` -> `mgr:done`
5. Write `.botminter.yml`: `role: team-manager`, `comment_emoji: "📋"`

## Dependencies
- Task 1 of this step (skeleton structure created)

## Implementation Approach
1. Study existing board-scanner SKILL.md files for pattern (e.g., `profiles/scrum/roles/architect/coding-agent/skills/board-scanner/`)
2. Study existing ralph.yml configurations for persistent loop pattern
3. Write board-scanner SKILL.md scoped to mgr:* statuses
4. Write ralph.yml with executor hat definition
5. Write context.md with agent tags for Claude Code content
6. Write executor hat instructions
7. Test by hiring and inspecting the resulting member directory

## Acceptance Criteria

1. **Board-scanner skill targets mgr:todo**
   - Given the Team Manager's `coding-agent/skills/board-scanner/SKILL.md`
   - When reading the content
   - Then it scans the board for `mgr:todo` status issues with `role/team-manager` label

2. **ralph.yml has persistent loop**
   - Given the hired Team Manager's ralph.yml
   - When reading the configuration
   - Then it's configured with a persistent loop and executor hat

3. **context.md has agent tags**
   - Given the Team Manager's `context.md`
   - When scanning for agent tags
   - Then Claude Code-specific sections are properly tagged

4. **Executor hat transitions through mgr: statuses**
   - Given the executor hat instructions
   - When reading the content
   - Then it describes transitioning `mgr:todo` -> `mgr:in-progress` -> `mgr:done`

5. **Full hire + inspect flow**
   - Given `bm hire team-manager --name bob`
   - When inspecting the member directory
   - Then PROMPT.md, ralph.yml, context.md, and hat instructions are all present and well-formed

## Metadata
- **Complexity**: Medium
- **Labels**: team-manager, content, sprint-5
- **Required Skills**: Ralph configuration, board-scanner skill pattern, YAML, Markdown
