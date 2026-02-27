# Sprint 5: Compact Single-Member Profile

## Objective

Create the `compact` profile — a single Ralph instance with all roles as hats — and
validate it end-to-end with a synthetic epic traversing the full lifecycle including
story TDD flow.

## Prerequisites

Sprints 1-4 complete. The `rh-scrum` profile has two working members (architect,
human-assistant) with Telegram HIL, training mode, rejection loops, and automated
tests. The generator (`just init`, `add-member`, `create-workspace`, `launch`) supports
arbitrary profiles.

## Deviations

None. Sprint 5 introduces a new profile alongside `rh-scrum` — no changes to existing
infrastructure or prior sprint artifacts.

## Key References

- Sprint design: `specs/milestone-2-architect-first-epic/sprint-5/design.md`
- Sprint plan: `specs/milestone-2-architect-first-epic/sprint-5/plan.md`
- Sprint requirements: `specs/milestone-2-architect-first-epic/sprint-5/requirements.md`
- M2 design: `specs/milestone-2-architect-first-epic/design.md`
- Design principles: `specs/design-principles.md`
- Existing rh-scrum profile: `skeletons/profiles/rh-scrum/`
- Existing fixtures: `specs/milestone-2-architect-first-epic/fixtures/`

## Requirements

1. **Profile skeleton** — MUST create `skeletons/profiles/compact/` with PROCESS.md,
   CLAUDE.md, knowledge/, invariants/, and agent/skills/. PROCESS.md MUST be copied
   and adapted from `rh-scrum`: add story lifecycle statuses (`qe:*`, `dev:*`), SRE
   statuses (`sre:*`), content writer statuses (`cw:*`), `kind/docs` label,
   auto-advance rules for `arch:sign-off`, `po:merge`, and content stories
   (`cw:review` approval → `po:merge` → `done`), and remove write-lock protocol.
   MUST NOT include write-lock references. Per design Sections 3.1, 4.5, 5.2, and 6.

2. **Superman member skeleton** — MUST create `members/superman/` with PROMPT.md,
   CLAUDE.md, invariants/, knowledge/, hat knowledge directories, agent/, and
   projects/. PROMPT.md MUST declare supervised mode (not training mode) — human gates
   only `po:design-review`, `po:plan-review`, and `po:accept`. Per design
   Sections 3.1 and 4.2.

3. **ralph.yml** — 15 hats total (design Section 4.1). Unified board scanner MUST
   watch all `status/*` labels including `lead:*` with priority-ordered dispatch table
   and MUST handle auto-advance for `arch:sign-off` and `po:merge`. Team lead hat MUST
   review arch_designer/arch_planner/arch_breakdown output before it reaches the human —
   work hats publish `lead.review` (direct chain to lead_reviewer). Story TDD flow MUST be
   qe_test_designer → dev_implementer → dev_code_reviewer → qe_verifier, using direct chain
   dispatch. Review hats (lead_reviewer, dev_code_reviewer, qe_verifier, cw_reviewer)
   MUST only emit `.approved`/`.rejected` events — they MUST NOT encode return
   destinations. Ralph routes rejections to work hats via the `triggers` configuration.
   `qe_verifier` MUST also trigger on `qe.verify` (board scanner recovery path) in
   addition to `dev.approved` (direct chain path). Per design Sections 3.3, 4.1, 4.3,
   and 4.4.

4. **Generator compatibility** — `just init --profile=compact` MUST work without
   generator changes. Verify `just add-member superman`, `just create-workspace`,
   and `just launch --dry-run`. MUST NOT require changes to the generator Justfile.
   Per design Section 3.2.

5. **Synthetic validation** — MUST adapt M2 fixtures for the compact profile. Seed a
   synthetic epic at `status/po:triage`. MUST validate the full lifecycle: epic design
   phase with human gates, story TDD flow with direct chain, auto-advance at sign-off
   and merge, knowledge propagation from all scopes, and supervised mode gates firing
   only at major decision points. Per design Sections 7 and 8.

## Acceptance Criteria

- Given `skeletons/profiles/compact/` exists, when `just init --repo=<path>
  --profile=compact project=hypershift` runs, then the generated repo MUST contain
  PROCESS.md (with story/SRE/CW statuses, no write-lock protocol), CLAUDE.md,
  knowledge/, invariants/, agent/skills/, and `.team-template/` with the compact
  profile

- Given a generated compact team repo, when `just add-member superman` runs, then
  `team/superman/` MUST contain ralph.yml (15 hats), PROMPT.md (supervised mode),
  CLAUDE.md, invariants/design-quality.md, and hat knowledge directories for all
  non-trivial hats

- Given the compact ralph.yml, when the board scanner dispatch table is inspected,
  then it MUST cover all statuses from design Section 4.3 — epics, `lead:*` statuses,
  stories, SRE, and content — with auto-advance handling for `arch:sign-off` and
  `po:merge`

- Given the compact ralph.yml, when the story TDD hats are inspected, then the flow
  MUST be qe_test_designer → dev_implementer → dev_code_reviewer → qe_verifier. Review hats
  (dev_code_reviewer, qe_verifier) MUST publish only `.approved`/`.rejected` events and
  MUST NOT encode return destinations. Work hats MUST declare rejection events in
  their `triggers`

- Given a workspace created with `just create-workspace superman <url>`, when the
  workspace layout is inspected, then PROMPT.md and CLAUDE.md MUST be symlinks into
  `.botminter/team/superman/`, ralph.yml MUST be a copy, `.botminter/.member` MUST
  contain `superman`, and `.claude/agents/` MUST be assembled

- Given a running superman agent with a seeded epic at `status/po:triage`, when the
  agent processes the full lifecycle, then the epic MUST traverse `po:triage →
  po:backlog → arch:design → lead:design-review → po:design-review → arch:plan →
  lead:plan-review → po:plan-review → arch:breakdown → lead:breakdown-review →
  po:ready → arch:in-progress → po:accept → done` with lead_reviewer reviewing before
  each human gate, and human gated only at design-review, plan-review, and accept

- Given stories created from breakdown at `status/qe:test-design`, when the agent
  processes a story, then the story MUST traverse `qe:test-design → dev:implement →
  dev:code-review → qe:verify → arch:sign-off → po:merge → done` with code review
  before QE verification, and auto-advance at sign-off and merge

- Given a story at `status/arch:sign-off`, when the board scanner processes it, then
  it MUST auto-advance to `status/po:merge` and then to `status/done` without
  dispatching a hat

- Given knowledge files at team, project, and member scopes with detection markers,
  when the arch_designer hat produces a design doc, then the doc MUST contain markers from
  all three scopes: `issue number` (team), `reconciler` (project), `composition`
  (member)

- Given the agent is in supervised mode, when the epic reaches `po:design-review`,
  `po:plan-review`, or `po:accept`, then the po_reviewer MUST present the artifact
  to the human via `human.interact` — and all other transitions MUST auto-advance
  without HIL

- Given comments produced during the lifecycle, when the comment audit trail is
  inspected, then each comment MUST use the correct role header (`### @architect`,
  `### @dev`, `### @qe`, etc.) per the acting hat's role origin, even though it is a
  single agent
