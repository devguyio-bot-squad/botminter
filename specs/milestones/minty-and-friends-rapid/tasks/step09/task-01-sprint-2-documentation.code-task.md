---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Sprint 2 Documentation Updates

## Description
Update documentation for profile externalization. Document `bm profiles init`, the disk-based profile model, auto-prompt behavior, and customization workflow. Docs-only step.

## Background
Sprint 2 moved profiles from compile-time embedded to disk-based at `~/.config/botminter/profiles/`. Operators can now customize profiles. The docs need to reflect this new storage model and the `bm profiles init` command.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (documentation impact matrix, Sprint 2)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 9)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. `docs/content/reference/cli.md` — document `bm profiles init [--force]`
2. `docs/content/concepts/profiles.md` — rewrite storage model: profiles live on disk at `~/.config/botminter/profiles/`, not embedded; explain extraction, customization, and auto-prompt
3. `docs/content/getting-started/bootstrap-your-team.md` — add `bm profiles init` as prerequisite (or explain auto-prompt handles it)
4. `docs/content/how-to/generate-team-repo.md` — update `bm init` flow for disk-based profiles
5. `docs/content/reference/configuration.md` — document `~/.config/botminter/` layout alongside `~/.botminter/`

## Dependencies
- Steps 7-8 complete (all Sprint 2 code changes landed)

## Implementation Approach
1. Read each affected doc page
2. Update storage model references from "embedded" to "disk-based"
3. Add `bm profiles init` documentation
4. Explain the auto-prompt first-run experience
5. Add customization workflow examples

## Acceptance Criteria

1. **CLI reference documents profiles init**
   - Given `docs/content/reference/cli.md`
   - When reading the profiles section
   - Then `bm profiles init [--force]` is documented with description and examples

2. **Profiles concept explains disk model**
   - Given `docs/content/concepts/profiles.md`
   - When reading the page
   - Then it explains disk-based storage, extraction, customization, and auto-prompt

3. **No references to embedded as active source**
   - Given all doc pages
   - When searching for "embedded profiles" as the active data source
   - Then it only appears in `bm profiles init` context (as the seed source)

4. **Config directory layout documented**
   - Given `docs/content/reference/configuration.md`
   - When reading the page
   - Then `~/.config/botminter/profiles/` layout is documented

## Metadata
- **Complexity**: Low
- **Labels**: documentation, sprint-2
- **Required Skills**: Technical writing, Markdown
