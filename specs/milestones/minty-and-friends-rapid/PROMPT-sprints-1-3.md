# Minty and Friends — Sprints 1–3

## Objective

Implement coding-agent-agnostic architecture, disk-based profile externalization, and the workspace repository model for the `bm` CLI. Steps 1–12 of the implementation plan, covering 22 tasks across 3 sprints.

## Spec Directory

`specs/milestones/minty-and-friends-rapid/`

## Required Reading

- **Design:** `specs/milestones/minty-and-friends-rapid/design.md`
- **Plan:** `specs/milestones/minty-and-friends-rapid/plan.md` (Steps 1–12)
- **Requirements:** `specs/milestones/minty-and-friends-rapid/requirements.md`
- **Research:** `specs/milestones/minty-and-friends-rapid/research/`

Read the design document before beginning any task. Each task file references specific design sections.

## Task Execution Order

Tasks are in `specs/milestones/minty-and-friends-rapid/tasks/` organized by step. Execute in step order — each step builds on the previous:

| Step | Dir | Tasks | Sprint |
|------|-----|-------|--------|
| 1 | `step01/` | 2 — Agent tag filter library | 1 |
| 2 | `step02/` | 2 — CodingAgentDef data model + schema | 1 |
| 3 | `step03/` | 2 — Profile restructuring (renames + tags) | 1 |
| 4 | `step04/` | 2 — Extraction pipeline update | 1 |
| 5 | `step05/` | 1 — Workspace parameterization | 1 |
| 6 | `step06/` | 1 — Sprint 1 documentation | 1 |
| 7 | `step07/` | 2 — `bm profiles init` command | 2 |
| 8 | `step08/` | 2 — Disk-based profile API + auto-prompt | 2 |
| 9 | `step09/` | 1 — Sprint 2 documentation | 2 |
| 10 | `step10/` | 3 — Workspace repo creation | 3 |
| 11 | `step11/` | 2 — Workspace sync + `bm start` adaptation | 3 |
| 12 | `step12/` | 2 — Status commands + Sprint 3 documentation | 3 |

Within each step, execute tasks in filename order (`task-01-*`, `task-02-*`, `task-03-*`).

## Constraints

- `schema_version` MUST remain `"1.0"` — no version bump for this milestone
- All profile content MUST use `team/` paths — no `.botminter/` path references
- E2E tests required for GitHub API operations per `invariants/e2e-testing.md`
- Alpha policy: breaking changes expected, no migration paths
- Existing tests MUST continue to pass after each step
