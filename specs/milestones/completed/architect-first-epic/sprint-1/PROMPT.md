# Sprint 1: One Agent, One Hat — Architect Produces a Design

## Objective

Build the `.botminter/` workspace infrastructure and create the architect member
skeleton with board_scanner + designer hats. Validate that a single agent can scan
the board, find an epic at `status/arch:design`, and produce a design doc with
knowledge propagation and invariant compliance.

No human-assistant changes. The synthetic epic is seeded directly at
`status/arch:design`, bypassing triage/backlog.

## Sprint 1 Deviations from Design

These are intentional scope decisions. See sprint plan for full rationale.

- **Training mode: DISABLED** (design says ENABLED) — no Telegram/RObot in Sprint 1,
  so no HIL channel. Agent acts autonomously. Re-enabled in Sprint 3.
- **RObot: disabled** (design says enabled) — no Telegram bots. Sprint 3.
- **`just launch` has no `--telegram-bot-token`** — Telegram deferred. Design AC 7.9 deferred.
- **Project name `hypershift` hardcoded** in `skills.dirs` and designer hat output path.
  Design review Finding 47 — single-project assumption for M2.

## Key References

- Design: `specs/milestone-2-architect-first-epic/design.md`
- Sprint plan: `specs/milestone-2-architect-first-epic/sprint-1/plan.md`
- Current skeletons: `skeletons/profiles/rh-scrum/`, `skeletons/team-repo/`
- Current generator: root `Justfile`

## Requirements

1. **PROCESS.md** — append epic lifecycle statuses (design.md Section 4.3), write-lock
   settings, error status, rejection loops, story status placeholder after the existing
   "Status Label Convention" section

2. **Team CLAUDE.md** — rewrite `skeletons/profiles/rh-scrum/CLAUDE.md` to replace
   submodule workspace model with `.botminter/` model (design.md Section 4.11).
   All paths change from `team-repo/` to `.botminter/`. Add agent capabilities scoping
   (design.md Section 4.7), propagation model (Section 4.6.2), write-lock protocol
   summary. Keep existing "What This Repo Is" content.

3. **Profile agent/ directory** — create `skeletons/profiles/rh-scrum/agent/{skills,agents}/`
   with `.gitkeep` files. Team-level skills (`create-epic`, `board`) deferred to Sprint 2.

4. **Generator init** — update root `Justfile` init recipe to overlay `agent/` from
   profile and create project-level `agent/{skills,agents}/` when `project=` is specified.
   Verify `.team-template/` propagation includes the new `agent/` directories.

5. **Team repo Justfile** — rewrite `skeletons/team-repo/Justfile`. Add
   `project_root := parent_directory(justfile_directory())` for recipes invoked from
   workspace context. Three recipes:
   - `create-workspace <member> <project-repo-url>`: `.botminter/` model
     (design.md Section 4.8). Clone project repo, clone team repo into `.botminter/`
     using local path (`justfile_directory()`), symlinks, `.claude/` assembly,
     `.git/info/exclude`, `.gitignore`, `.member` marker.
   - `sync`: new recipe (design.md Section 4.6.3). Invoked from workspace as
     `just -f .botminter/Justfile sync`. Uses `project_root` for project-level ops.
     Pull `.botminter/` + project repo (skip project pull gracefully if no remote),
     re-copy ralph.yml + settings if newer, re-assemble `.claude/agents/`, verify symlinks.
   - `launch <member> [--dry-run]`: requires workspace to already exist (scans for
     sibling directory with matching `.botminter/.member`). Syncs then launches Ralph.
     No `--telegram-bot-token` (Sprint 3).

6. **Architect skeleton** — create `skeletons/profiles/rh-scrum/members/architect/`:
   - `ralph.yml`: board_scanner + designer hats only (design.md Section 4.1.1,
     strip planner/breakdown_executor/epic_monitor). board_scanner publishes:
     `arch.design`, `LOOP_COMPLETE`. designer publishes: `board.rescan`. Both have
     `default_publishes`. Include `cooldown_delay_seconds: 60`. RObot disabled.
     Include agent startup self-cleanup in board_scanner instructions.
   - `PROMPT.md`: training mode DISABLED (design.md Section 4.1.2 modified)
   - `CLAUDE.md`: workspace model, codebase access, protocols (design.md Section 4.1.3)
   - `invariants/design-quality.md` (design.md Section 4.1.4)
   - `knowledge/.gitkeep`, `agent/{skills,agents}/.gitkeep`
   - `hats/designer/knowledge/.gitkeep`, `hats/planner/knowledge/.gitkeep`
   - `projects/.gitkeep`
   - Designer hat output path hardcodes `hypershift`:
     `.botminter/projects/hypershift/knowledge/designs/epic-<number>.md`
   - Designer hat backpressure gates: Security Considerations section, acceptance
     criteria (Given-When-Then), references project knowledge, addresses invariants

7. **Documentation** — create `docs/` at generator repo root:
   - `docs/index.md`: project overview, links to other pages
   - `docs/getting-started.md`: prerequisites, init, add-member, create-workspace, launch
   - `docs/architecture.md`: three-layer generator, runtime model, `.botminter/` workspace
     model with layout diagram, knowledge/invariant scoping, agent capabilities, propagation
   - `docs/workspace-commands.md`: create-workspace, sync, launch, add-member reference
   - `docs/epic-lifecycle.md`: status table, flow diagram, rejection loops, write-locks,
     error escalation

8. **Synthetic fixtures** — create `specs/milestone-2-architect-first-epic/fixtures/`:
   - Knowledge at team, project, member scopes with grep-able detection markers
     (design.md Section 8.1)
   - Invariants at team, project, member scopes
   - Synthetic epic at `status/arch:design` (NOT `status/po:triage` — bypasses HA)
   - Synthetic project repo (minimal git repo with README + stub code containing
     codebase markers for knowledge propagation alignment)
   - `deploy.sh` script: copies fixtures into team repo AND commits them (so
     `.botminter/` clone includes them)

## Acceptance Criteria

- Given `just init --repo=<path> --profile=rh-scrum project=hypershift`, when the repo
  is generated, then PROCESS.md has epic statuses, CLAUDE.md has `.botminter/` model,
  `agent/` directories exist, `projects/hypershift/agent/` directories exist, and
  `.team-template/` includes the `agent/` directories

- Given `just add-member architect` in a generated repo, when the member is added, then
  `team/architect/` has ralph.yml (2 hats), PROMPT.md, CLAUDE.md,
  invariants/design-quality.md, knowledge/, agent/, hats/designer/knowledge/,
  hats/planner/knowledge/, projects/

- Given `just create-workspace architect <project-repo-url>`, when the workspace is
  created, then: PROMPT.md and CLAUDE.md are symlinks into `.botminter/team/architect/`;
  ralph.yml is a copy; `.botminter/` is a git clone of the team repo;
  `.botminter/.member` contains "architect"; `.git/info/exclude` has agent patterns;
  `.gitignore` exists; `.claude/agents/` directory exists

- Given `just -f .botminter/Justfile sync` in a workspace, when run, then `.botminter/`
  is pulled, project repo pull is attempted (skipped gracefully if no remote),
  ralph.yml is re-copied if newer, `.claude/agents/` symlinks are refreshed,
  PROMPT.md and CLAUDE.md symlinks verified

- Given a synthetic epic at `status/arch:design`, when the architect launches and scans,
  then a design doc appears at `.botminter/projects/hypershift/knowledge/designs/epic-1.md`,
  the epic status transitions to `status/po:design-review`, and no lock file remains

- Given synthetic knowledge at team, project, and member scopes, when the architect
  produces a design, then the design doc contains grep-able markers from all three scopes:
  `issue number` (team), `reconciler` (project), `composition` (member)

- Given invariants and backpressure gates, when the architect produces a design, then the
  design includes all required sections: overview, architecture, components and interfaces,
  data models, error handling, acceptance criteria (Given-When-Then), impact on existing
  system, and Security Considerations

- Given the architect completes a scan cycle, then `poll-log.txt` at the project repo
  root contains a START/result/END triplet with ISO 8601 timestamps
