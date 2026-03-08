---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Sprint 1 Documentation Updates

## Description
Update all documentation pages affected by the coding-agent-agnostic cleanup. Remove hardcoded Claude Code references where the design is now agent-agnostic. This is a docs-only step with no code changes.

## Background
Sprint 1 introduced the coding-agent abstraction: `CodingAgentDef` config, `coding-agent/` directories, `context.md` with inline tags, parameterized workspace code. The documentation needs to reflect these changes.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (documentation impact matrix, Sprint 1)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 6)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. `docs/content/concepts/profiles.md` — add coding-agent abstraction concept; explain `coding-agent/` directory and inline agent tags
2. `docs/content/reference/configuration.md` — document `coding_agents` and `default_coding_agent` in `botminter.yml`; add `coding_agent` team-level override
3. `docs/content/reference/cli.md` — update `bm init` for coding agent; add `--show-tags` to `bm profiles describe`
4. `docs/content/getting-started/index.md` — generalize prerequisites: "a supported coding agent" with Claude Code as current option
5. `docs/content/faq.md` — update "Do I need Claude Code?" answer
6. Update root `CLAUDE.md` if it references old `agent/` directory convention

## Dependencies
- Steps 1-5 complete (all Sprint 1 code changes landed)

## Implementation Approach
1. Read each affected doc page
2. Identify sections with hardcoded Claude Code assumptions
3. Update to reference the coding-agent abstraction
4. Add new sections where needed (coding-agent concept, new config fields)
5. Verify no broken internal links

## Acceptance Criteria

1. **Profiles concept page covers coding-agent abstraction**
   - Given `docs/content/concepts/profiles.md`
   - When reading the page
   - Then it explains `coding-agent/` directory, inline agent tags, and config-driven mapping

2. **Configuration reference documents new fields**
   - Given `docs/content/reference/configuration.md`
   - When reading the page
   - Then `coding_agents`, `default_coding_agent`, and team-level `coding_agent` override are documented with examples

3. **CLI reference updated**
   - Given `docs/content/reference/cli.md`
   - When reading the page
   - Then `bm profiles describe --show-tags` is documented

4. **Getting started generalized**
   - Given `docs/content/getting-started/index.md`
   - When reading prerequisites
   - Then it says "a supported coding agent" not just "Claude Code"

5. **No broken internal links**
   - Given all updated doc pages
   - When checking internal markdown links
   - Then all links resolve correctly

## Metadata
- **Complexity**: Low
- **Labels**: documentation, sprint-1
- **Required Skills**: Technical writing, Markdown
