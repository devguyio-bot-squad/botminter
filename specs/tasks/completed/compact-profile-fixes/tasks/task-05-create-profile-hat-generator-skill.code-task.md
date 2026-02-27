---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Create botminter profile hat generator skill

## Description
Create a botminter-specific skill for generating hat collections tailored to botminter profiles. This encodes botminter's architectural decisions: board scanner as the entry point, status label conventions, knowledge path scoping, comment format attribution, supervised mode gates, and the persistent poll-based loop model.

## Background
Botminter profiles require hats that follow specific patterns not enforced by any existing tooling. A dedicated skill ensures generated hats are correct by construction. The patterns it must encode:

- **Board scanner pattern**: Every profile must start with a `board_scanner` hat triggered by `board.scan` that dispatches to role-specific hats based on `status/*` labels
- **Status label convention**: `status/<role>:<phase>` with defined epic and story lifecycles
- **Knowledge path scoping**: Team → Project → Member → Member+Project → Hat levels
- **Comment attribution**: `### <emoji> <role> — <ISO-timestamp>` format
- **Supervised mode**: Human gates at specific review points (po:design-review, po:plan-review, po:accept)
- **No LOOP_COMPLETE**: Persistent loop, no `completion_promise` or `default_publishes`
- **Auto-advance statuses**: Certain transitions (arch:sign-off → po:merge → done) are handled by the board scanner
- **Rejection-awareness**: Hats check for rejection feedback before starting work
- **Backpressure checks**: Quality gates before status transitions
- **gh CLI pattern**: All GitHub operations use `--repo "$TEAM_REPO"` with auto-detection

A botminter-specific skill encodes these decisions so generated hats are correct by construction, preventing the class of bugs found in testing.

## Reference Documentation
**Required:**
- `profiles/compact/members/superman/ralph.yml` — reference implementation (compact profile hats)
- `profiles/compact/PROCESS.md` — process conventions the skill must encode

**Additional References:**
- `profiles/rh-scrum/` — second profile for cross-referencing patterns
- `profiles/compact/agent/skills/gh/SKILL.md` — gh CLI conventions

## Technical Requirements
1. Create `skills/create-bm-profile-hats/SKILL.md` (or similar location in the botminter repo)
2. Encode botminter's mandatory hat patterns (board scanner, knowledge paths, comment format, etc.)
3. Encode the status label lifecycle for epics, stories, and specialists
4. Provide templates for common hat types: scanner, worker, reviewer, gater, monitor
5. Enforce constraints: no LOOP_COMPLETE, no Justfile references, proper `gh` CLI patterns
6. Include examples derived from the compact and rh-scrum profiles

## Dependencies
- Task 4 (remove LOOP_COMPLETE) should complete first so the skill reflects the corrected patterns

## Implementation Approach
1. Study the compact and rh-scrum profiles' actual hats to extract common patterns
2. Codify the botminter-specific patterns into a structured skill document
3. Organize the skill into phases: understand profile requirements → design hat topology → generate hats
4. Include a constraints/checklist section that prevents known bug patterns
5. Provide hat templates for each common type with placeholder instructions

## Acceptance Criteria

1. **Skill document exists and is well-structured**
   - Given the skill directory
   - When reading SKILL.md
   - Then it follows the standard skill format (frontmatter, phases, examples)

2. **Board scanner pattern is mandatory**
   - Given the skill's hat generation guidance
   - When generating hats for any botminter profile
   - Then a `board_scanner` hat triggered by `board.scan` is always included

3. **Known anti-patterns are blocked**
   - Given the skill's constraints section
   - When reviewing the checklist
   - Then it explicitly prohibits: `LOOP_COMPLETE` in any form, `Justfile` references, bare `git init`, local file path remotes

4. **Status label lifecycle is documented**
   - Given the skill's reference section
   - When looking up epic or story lifecycles
   - Then the full status label chain is documented with transitions

5. **Hat templates cover common types**
   - Given the skill's templates
   - When generating a new profile
   - Then templates exist for: scanner, worker (implementer), reviewer (decoupled), gater (supervised), monitor

## Metadata
- **Complexity**: High
- **Labels**: feature, skill, profile, tooling
- **Required Skills**: Markdown, YAML, botminter architecture
