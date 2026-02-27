# Summary — botminter

> Planning artifacts produced by the Prompt-Driven Development process.
> All milestones through `bm` CLI are complete. Full Team + First Story and Eval/Confidence System remain planned.

---

## Artifacts

| File | Description | Status |
|------|-------------|--------|
| [rough-idea.md](rough-idea.md) | Original concept — GitOps-style agentic team with Ralph instances as team members | Complete |
| [requirements.md](requirements.md) | 26 Q&A entries covering scope, architecture, coordination model, workspace model, knowledge layers, HIL | Complete |
| [research/](research/) | 19 research documents — SME interviews, tooling landscape, Ralph audit, UX visions, workflow guides | Complete |
| [design.md](design.md) | Full design — generator architecture, milestone designs | Complete |
| [plan.md](plan.md) | Implementation plan — M1 has 9 incremental steps; later milestones detailed in own spec dirs | Complete |
| [summary.md](summary.md) | This file | Complete |

---

## Overview

**What:** A generator repo (`botminter`) that stamps out GitOps-style agentic team repos. Each team member is an independent Ralph instance running in its own workspace. Members coordinate through GitHub issues, milestones, and PRs on a shared team repo via the `gh` CLI — no central orchestrator.

**Architecture:** Two-layer model:
1. **Profile** (e.g., `scrum`, `scrum-compact`) — team process, role definitions, member skeletons, knowledge, invariants
2. **Team repo instance** — project-specific knowledge, hired members, runtime state

**Profiles:**
- **`scrum`** — Multi-member team with human-assistant (PO) and architect roles. Epic-driven workflow with GitHub Projects v2 statuses and rejection loops.
- **`scrum-compact`** — Single `superman` member wearing multiple hats. Supervised mode with direct chain dispatch. Minimal coordination overhead.
- **`scrum-compact-telegram`** — Compact profile variant with Telegram HIL via RObot.

---

## Milestone Summary

### Structure + human-assistant ✅

Built the foundational infrastructure: profile skeleton, `scrum` profile, human-assistant member with three-hat model, workspace model, and HIL via Telegram.

**Artifacts:** `specs/milestones/completed/structure-poa/`

### Autonomous Ralph (spike) ✅

Validated how Ralph runs autonomously in a persistent loop, pulling work from a board instead of a single-objective prompt. Delivered validated ralph.yml pattern adopted by all later milestones.

**Artifacts:** `specs/milestones/completed/autonomous-ralph/`

### Architect + First Epic ✅

Added architect as second team member. Proved outer loop coordination — epic creation, design review, story breakdown through GitHub issues. Compact single-member profile (`superman`) also delivered. Sprint 4 (automated tg-mock tests) planned but not implemented.

**Artifacts:** `specs/milestones/completed/architect-first-epic/`

### GitHub Migration ✅

Replaced file-based `.github-sim/` coordination with real GitHub via unified `gh` CLI skill. Coordination model unchanged — only the backing store moved.

**Artifacts:** `specs/milestones/completed/github-migration/`

### `bm` CLI ✅

Replaced Justfile-based tooling with a Rust CLI binary (`bm`). Single operator interface for managing agentic teams with workzone model, embedded profiles, event-driven daemon, knowledge management, and formation-aware deployment. Absorbed the Data Operations scope (daemon, knowledge, formations) which was originally planned as a separate milestone.

Key deliverables:

- `bm` CLI with commands: `init`, `hire`, `start/stop/status`, `teams list/show/sync`, `members list/show`, `roles list`, `profiles list/describe`, `projects add/list/show/sync`, `knowledge list/show` (+ interactive), `daemon start/stop/status`, `completions`
- Profiles embedded at compile time via `include_dir`
- Profile schema with `botminter.yml` + `.schema/`
- Event-driven daemon (webhook and poll modes)
- Formation-aware deployment (local, k8s)
- Integration and E2E test suites

**Artifacts:** `specs/milestones/completed/bm-cli/`

### Completed Task Batches

In addition to milestones, focused task batches were executed:

- **CLI Testing** (9 tasks) — unit tests, integration tests, E2E harness, member discovery bug fix. `specs/tasks/completed/cli-testing/`
- **Compact Profile Fixes** (7 tasks) — branch name fix, remote URL fix, board scanner fixes, LOOP_COMPLETE removal, GitHub Projects v2 migration, hat generator/reviewer skills. `specs/tasks/completed/compact-profile-fixes/`
- **Completions** (2 tasks) — shell completions command with dynamic values. `specs/tasks/completed/completions/`

### Planned Milestones

- **Full Team + First Story** — adds dev, QE, reviewer members; full story kanban TDD flow. Now unblocked by `bm` CLI completion. `specs/milestones/full-team-first-story/`
- **Eval/Confidence System** — distributed eval framework, scored confidence, HIL graduation. Deferred pending practical multi-member experience. `specs/milestones/eval-confidence-system/`

---

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Coordination model | GitHub issues + Projects v2 via `gh` CLI, no central orchestrator | Emergent coordination from shared process and status conventions |
| Inner vs outer loop | Each member is a full Ralph instance (inner); team repo is control plane (outer) | Avoids single-agent-many-masks problem; preserves independent memories/context |
| Workspace model | Each member in isolated workspace, team repo cloned to `.botminter/` | Clean separation of runtime state from team definition |
| File surfacing | Symlink PROMPT.md/CLAUDE.md, copy ralph.yml from team repo to workspace root | Runtime files (memories, scratchpad) stay workspace-only |
| HIL channel | Telegram via RObot | Training mode: observe & report; graduation path to supervised/autonomous |
| GitHub interaction | Unified `gh` CLI skill at profile level, shared `GH_TOKEN` | Single interaction point for all GitHub operations; role attribution via `.botminter.yml` |
| Knowledge scoping | Recursive: team → project → member → member+project | Lower scopes can override/extend higher scopes |
| Status tracking | GitHub Projects v2 status field (not labels) | Richer state model, queryable via `gh project item-list` |

---

## Suggested Next Steps

1. **Plan and implement Full Team + First Story** — add dev, QE, reviewer members; full story kanban TDD flow
2. After multi-member experience, plan **Eval/Confidence System**

---

## References

- Master plan: `specs/master-plan/`
- Design principles: `specs/design-principles.md`
- Planning prompts: `specs/prompts/`
- Completed milestones: `specs/milestones/completed/`
- Planned milestones: `specs/milestones/`
- Completed tasks: `specs/tasks/completed/`
- Roadmap: `docs/content/roadmap.md`
- Documentation site: `docs/`
