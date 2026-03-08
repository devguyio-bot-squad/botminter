---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Sprint 3 Documentation Updates

## Description
Major documentation rewrite for the workspace repository model. Update workspace concept, architecture diagram, launch workflow, CLI reference, and knowledge/invariant path references. Docs-only step.

## Background
Sprint 3 replaced the `.botminter/` clone workspace model with dedicated workspace repos using submodules. This is the most impactful documentation change in the milestone — the workspace model is central to how operators understand the system.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (documentation impact matrix, Sprint 3)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 12)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. `docs/content/concepts/workspace-model.md` — major rewrite: workspace repos, submodules, marker file, context surfacing
2. `docs/content/concepts/architecture.md` — update runtime diagram for workspace repo model
3. `docs/content/how-to/launch-members.md` — rewrite `bm teams sync` workflow with `--push` and sync
4. `docs/content/reference/cli.md` — update `bm teams sync`, `bm start`, `bm status`, `bm members show`
5. `docs/content/concepts/knowledge-invariants.md` — update paths from `.botminter/` to `team/` submodule

## Dependencies
- Tasks 1 of this step (status commands updated, providing the user-facing surface to document)

## Implementation Approach
1. Read each affected doc page
2. Replace `.botminter/` references with `team/` submodule model
3. Rewrite workspace concept page from scratch
4. Update architecture diagrams
5. Verify no broken links

## Acceptance Criteria

1. **Workspace model concept rewritten**
   - Given `docs/content/concepts/workspace-model.md`
   - When reading the page
   - Then it describes workspace repos, submodules, `.botminter.workspace` marker, and context surfacing

2. **No .botminter/ clone references**
   - Given all doc pages
   - When searching for `.botminter/` as a workspace clone directory
   - Then no references remain (except in historical context)

3. **Paths updated to team/ submodule**
   - Given docs referencing workspace internals
   - When reading path examples
   - Then they use `team/` (submodule) not `.botminter/` (clone)

4. **CLI reference updated**
   - Given `docs/content/reference/cli.md`
   - When reading `bm teams sync`, `bm start`, `bm status`
   - Then they reflect the workspace repo model

## Metadata
- **Complexity**: Medium
- **Labels**: documentation, workspace-model, sprint-3
- **Required Skills**: Technical writing, Markdown, system architecture
