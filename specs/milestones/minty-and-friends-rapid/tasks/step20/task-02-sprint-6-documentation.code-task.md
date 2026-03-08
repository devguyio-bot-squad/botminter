---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Sprint 6 Documentation Updates

## Description
Update documentation for Minty: CLI reference, concepts, and FAQ. This completes the milestone's documentation. Docs-only step.

## Background
Sprint 6 introduced Minty — BotMinter's interactive assistant with skill-driven architecture. Operators need to understand what Minty is, how to launch it, how it differs from team members, and what skills are available.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (documentation impact matrix, Sprint 6)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 20)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. `docs/content/reference/cli.md` — document `bm minty [-t team]` with examples
2. `docs/content/concepts/profiles.md` — mention Minty config alongside profiles under `~/.config/botminter/`
3. `docs/content/faq.md` — add Minty FAQ entry:
   - "What is Minty?" — interactive assistant persona, not a team member
   - "How does Minty differ from team members?" — no Ralph loop, skill-driven, operator-facing
   - "What can Minty do?" — overview of available skills

## Dependencies
- Steps 18-19 Task 1 complete (Minty and skills implemented)

## Implementation Approach
1. Read each affected doc page
2. Add `bm minty` to CLI reference with usage examples
3. Update profiles concept to mention Minty config location
4. Write FAQ entries
5. Verify no broken links

## Acceptance Criteria

1. **bm minty documented in CLI reference**
   - Given `docs/content/reference/cli.md`
   - When reading the page
   - Then `bm minty [-t team]` is documented with description and examples

2. **Minty config location documented**
   - Given `docs/content/concepts/profiles.md`
   - When reading the page
   - Then `~/.config/botminter/minty/` is mentioned alongside profile storage

3. **FAQ explains Minty**
   - Given `docs/content/faq.md`
   - When reading the FAQ
   - Then entries explain what Minty is, how it differs from members, and what it can do

4. **Skills listed in documentation**
   - Given the Minty documentation
   - When reading about capabilities
   - Then all four skills are mentioned (team-overview, profile-browser, hire-guide, workspace-doctor)

## Metadata
- **Complexity**: Low
- **Labels**: documentation, minty, sprint-6
- **Required Skills**: Technical writing, Markdown
