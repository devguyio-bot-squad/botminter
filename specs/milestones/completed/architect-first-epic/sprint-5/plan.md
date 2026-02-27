# Sprint 5 Plan: Compact Single-Member Profile

> New `compact` profile: one Ralph instance, 15 hats, all roles. Same `.github-sim/`
> model, supervised mode, story TDD flow with direct chain dispatch.
>
> Design reference: [design.md](design.md)
> Parent plan: [../plan.md](../plan.md)

## Checklist

- [ ] Step 1: Profile skeleton — PROCESS.md, CLAUDE.md, knowledge, invariants, skills
- [ ] Step 2: All-in-one member skeleton — structure, PROMPT.md, CLAUDE.md, invariants
- [ ] Step 3: ralph.yml — unified board scanner + 15 hats
- [ ] Step 4: Generator compatibility + workspace verification
- [ ] Step 5: Synthetic fixtures + end-to-end validation

---

## Step 1: Profile Skeleton — PROCESS.md, CLAUDE.md, Knowledge, Invariants, Skills

**Objective:** Create the `compact` profile directory with all team-level files adapted
from `rh-scrum`.

**Implementation:**

Create `skeletons/profiles/compact/` with the following files:

### `PROCESS.md`

Copy and adapt from `skeletons/profiles/rh-scrum/PROCESS.md`. Key changes:

- **Remove write-lock protocol** — single member, no contention (design Section 6.2).
  Remove the write-lock settings section, lock file references, and stale lock cleanup.
  Replace with a note: "The compact profile has a single member — no write-locks needed."
- **Add team lead statuses** — `status/lead:design-review`, `status/lead:plan-review`,
  `status/lead:breakdown-review` (design Section 5.2).
- **Add story statuses** — full story lifecycle labels from design Section 5.2:
  `status/qe:test-design`, `status/dev:implement`, `status/dev:code-review`,
  `status/qe:verify`, `status/arch:sign-off`, `status/po:merge`.
- **Add SRE statuses** — `status/sre:infra-setup`.
- **Add content writer statuses** — `status/cw:write`, `status/cw:review`.
- **Add `kind/docs` label** — for documentation stories routed to cw_writer/cw_reviewer.
- **Add auto-advance section** — document that `status/arch:sign-off` auto-advances to
  `status/po:merge`, and `status/po:merge` auto-advances to `status/done` for stories
  (design Section 4.5).
- **Update communication protocols** — single-member model. Comment headers still use
  role prefix (`@architect`, `@dev`, `@qe`) per design Section 5.3, even though it's
  the same agent. Replace multi-agent coordination sections with single-agent
  self-transition model.
- **Supervised mode default** — document that only `po:design-review`, `po:plan-review`,
  and `po:accept` require human approval. All other transitions auto-advance (design
  Section 4.2).

### `CLAUDE.md`

Adapt from `skeletons/profiles/rh-scrum/CLAUDE.md`. Key changes:

- Describe the single-member model: one `superman` agent with 15 hats covering
  all roles.
- Same `.botminter/` workspace model as `rh-scrum`.
- Same knowledge/invariant scoping (5 levels: team, project, member, member+project, hat).
- Note: no write-locks, no concurrent agent concerns.
- Reference compact PROCESS.md.

### `knowledge/`

- `commit-convention.md` — copy from `rh-scrum`
- `communication-protocols.md` — adapt from `rh-scrum`: single-member, supervised mode,
  self-transition via status labels
- `pr-standards.md` — copy from `rh-scrum`

### `invariants/`

- `code-review-required.md` — adapt: self-review via `dev_code_reviewer` hat
  (design Section 4.4)
- `test-coverage.md` — copy from `rh-scrum`

### `agent/`

- `agent/skills/board/SKILL.md` — copy from `rh-scrum`
- `agent/skills/create-epic/SKILL.md` — copy from `rh-scrum`
- `agent/agents/.gitkeep`

**Test:** Directory structure matches design Section 3.1. PROCESS.md contains story,
SRE, and CW statuses. No write-lock references. Auto-advance documented.

**Integration:** Foundation for the member skeleton. PROCESS.md defines the vocabulary
all 15 hats use.

---

## Step 2: All-in-One Member Skeleton — Structure, PROMPT.md, CLAUDE.md, Invariants

**Objective:** Create the `superman` member directory structure with role identity
files. ralph.yml is deferred to Step 3 (the largest single artifact).

**Implementation:**

Create `skeletons/profiles/compact/members/superman/` with:

### Directory structure

Per design Section 3.1:

```
members/superman/
├── ralph.yml                    # Step 3
├── PROMPT.md
├── CLAUDE.md
├── knowledge/.gitkeep
├── invariants/
│   └── design-quality.md        # From rh-scrum architect
├── agent/
│   ├── skills/.gitkeep
│   └── agents/.gitkeep
├── hats/
│   ├── lead_reviewer/knowledge/.gitkeep
│   ├── arch_designer/knowledge/.gitkeep
│   ├── arch_planner/knowledge/.gitkeep
│   ├── dev_implementer/knowledge/.gitkeep
│   ├── qe_test_designer/knowledge/.gitkeep
│   ├── dev_code_reviewer/knowledge/.gitkeep
│   ├── qe_verifier/knowledge/.gitkeep
│   ├── cw_writer/knowledge/.gitkeep
│   └── cw_reviewer/knowledge/.gitkeep
└── projects/.gitkeep
```

### `PROMPT.md`

All-role identity and cross-hat behavioral rules. Per design-principles Section 1:
PROMPT.md defines role identity + cross-hat concerns, not hat-specific details.

Content:

- Role identity: "You are the superman agent. You wear all hats — PO, architect,
  dev, QE, SRE, content writer. You self-transition through the full issue lifecycle."
- **Supervised mode declaration** — `## !IMPORTANT — OPERATING MODE` with
  `SUPERVISED MODE: ENABLED`. Only `po:design-review`, `po:plan-review`, and
  `po:accept` require human approval via `human.interact`. All other transitions
  auto-advance.
- Codebase access: CWD is project repo, fork chain.
- Team configuration: `.botminter/`.
- No write-lock protocol (single member — design Section 6.2).
- Workspace sync: `just -f .botminter/Justfile sync`.
- Comment format: use `@<role>` prefix matching the active hat's role origin
  (design Section 5.3).

### `CLAUDE.md`

Per design-principles Section 1: workspace model, codebase access, invariant locations.

- Role: superman agent, single member covering all roles.
- Workspace model: CWD is project repo, `.botminter/` is team repo clone.
- Knowledge resolution paths (all `.botminter/` prefixed, 5 levels).
- Invariant compliance paths.
- No write-lock protocol.

### `invariants/design-quality.md`

Copy from `rh-scrum` architect: `skeletons/profiles/rh-scrum/members/architect/invariants/design-quality.md`.

**Test:** Directory structure matches design Section 3.1. PROMPT.md contains supervised
mode declaration. CLAUDE.md references `.botminter/` paths. Hat knowledge directories
exist for all non-trivial hats.

**Integration:** Member skeleton ready for ralph.yml (Step 3).

---

## Step 3: ralph.yml — Unified Board Scanner + 15 Hats

**Objective:** Create the ralph.yml with all 15 hats, including the unified board
scanner, epic lifecycle hats (adapted from rh-scrum), story TDD flow hats (new), and
auxiliary hats. This is the core deliverable of Sprint 5.

**Implementation:**

Create `skeletons/profiles/compact/members/superman/ralph.yml`.

Per design-principles Section 2 (validated runtime patterns):
- `persistent: true`, MUST NOT set `starting_event` (design-principles Section 2)
- `default_publishes` on every hat
- Self-clear in board scanner
- No `cooldown_delay_seconds` (design-principles Section 5)

Per design-principles Sections 7-8 (knowledge, backpressure, hat examples):
- `### Knowledge` in hats that produce artifacts (arch_designer, arch_planner, dev_implementer,
  qe_test_designer, cw_writer, lead_reviewer)
- `### Backpressure` in every work hat
- `### On Failure` in every work hat

### Hat 1: `board_scanner` (unified)

Per design Section 4.3. Merges rh-scrum architect and human-assistant scanners.
Watches ALL `status/*` labels with priority-ordered dispatch table.

- Triggers: `board.scan` (ONLY — design-principles Section 2)
- Publishes: all dispatch events (epic: `po.backlog`, `po.review`, `lead.review`,
  `arch.design`, `arch.plan`, `arch.breakdown`, `arch.in_progress`; story:
  `qe.test_design`, `dev.implement`, `dev.code_review`, `qe.verify`; SRE:
  `sre.setup`; content: `cw.write`, `cw.review`)
- Default publishes: `LOOP_COMPLETE`
- Auto-advance handling: when detecting `status/arch:sign-off` or `status/po:merge`,
  auto-advance without hat dispatch (design Section 4.5)
- Self-clear, sync, scan, dispatch, poll-log — same patterns as rh-scrum
- No write-lock cleanup (single member)

### Hats 2-3: `po_backlog`, `po_reviewer` (human-assistant)

Per design Section 4.1 (hat table rows 2-3) and Section 4.2 (supervised mode gates).

- `po_backlog`: triggered by `po.backlog`. Handles `po:triage`, `po:backlog`,
  `po:ready`. Presents board state to human, awaits decision.
- `po_reviewer`: triggered by `po.review`. Handles `po:design-review`,
  `po:plan-review`, `po:accept`. Presents artifacts to human via `human.interact`,
  gates approval/rejection. On rejection: append feedback comment, revert status.

### Hat 4: `lead_reviewer` (team lead review)

Per design Section 4.4. Reviews work artifacts before they reach the human.

- Triggered by `lead.review` (direct chain from arch_designer, arch_planner, arch_breakdown).
- Reads current status to determine review type: `lead:design-review` (design doc),
  `lead:plan-review` (story breakdown), `lead:breakdown-review` (story issues).
- On approved: advances to next human gate (`po:design-review`, `po:plan-review`,
  or `po:ready`). Publishes `lead.approved`.
- On rejected: appends feedback comment, reverts status, publishes `lead.rejected`.
  No hat subscribes to `lead.rejected` — the hatless Ralph orchestrator examines
  context and routes directly back to the originating work hat. lead_reviewer does
  not encode return destinations.
- Knowledge from all scopes (reviews artifacts from all producers).

### Hats 5-8: `arch_designer`, `arch_planner`, `arch_breakdown`, `arch_monitor` (architect)

Adapt from rh-scrum architect hats. arch_designer, arch_planner, and arch_breakdown
publish `lead.review` (direct chain to lead_reviewer).

- `arch_designer`: triggered by `arch.design`. Produces design doc. Knowledge from all
  scopes. Backpressure gates. Rejection-awareness (scan for feedback comments from
  lead_reviewer or human). Transitions to `lead:design-review`.
  Publishes `lead.review` (direct chain to lead_reviewer).
- `arch_planner`: triggered by `arch.plan`. Proposes story breakdown. Knowledge from
  all scopes. Rejection-awareness. Transitions to `lead:plan-review`.
  Publishes `lead.review` (direct chain to lead_reviewer).
- `arch_breakdown`: triggered by `arch.breakdown`. Creates story issues at
  `status/qe:test-design` (not `status/dev:ready` as in rh-scrum — compact starts
  stories at the TDD flow entry point). Reads LATEST breakdown comment.
  Transitions to `lead:breakdown-review`.
  Publishes `lead.review` (direct chain to lead_reviewer).
- `arch_monitor`: triggered by `arch.in_progress`. Monitors story progress.
  Fast-forwards to `po:accept`.

### Hats 9-12: `qe_test_designer`, `dev_implementer`, `dev_code_reviewer`, `qe_verifier` (story TDD flow)

New hats. Per design Section 4.4. These use **direct chain dispatch** — hats trigger
the next hat directly via Ralph events instead of returning to the board scanner.

Flow: qe_test_designer → dev_implementer → dev_code_reviewer → qe_verifier. Every work step
is followed by a review/verification step.

- `qe_test_designer` (QE): triggered by `qe.test_design`. Reads acceptance criteria and
  parent epic design. Writes test plan + stubs. Transitions to `status/dev:implement`.
  Publishes `dev.implement` (direct chain to dev_implementer).
- `dev_implementer` (dev): triggered by `dev.implement`, `dev.rejected`,
  `qe.rejected`. Rejection-aware (checks for feedback comments from dev_code_reviewer or
  qe_verifier). Implements code, ensures tests pass. Transitions to
  `status/dev:code-review`. Publishes `dev.code_review` (direct chain to dev_code_reviewer).
- `dev_code_reviewer` (dev): triggered by `dev.code_review`. Reviews code quality and
  invariant compliance. Publishes `dev.approved` (routes to qe_verifier) or
  `dev.rejected` (routes to dev_implementer). Does not encode destinations —
  Ralph handles routing via `triggers`.
- `qe_verifier` (QE): triggered by `dev.approved` (direct chain from dev_code_reviewer)
  or `qe.verify` (board scanner recovery). Final quality gate.
  Verifies implementation against acceptance criteria. Publishes `qe.approved`
  (unmatched — persistent loop restarts, auto-advance handles the rest) or
  `qe.rejected` (routes to dev_implementer via triggers). Decoupled — does not
  encode destinations.

### Hat 13: `sre_setup` (SRE)

Per design Section 4.4. On-demand service hat.

- Triggered by `sre.setup`. Sets up test infrastructure. Documents state in
  a comment. Returns issue to previous status.

### Hats 14-15: `cw_writer`, `cw_reviewer`

Per design Section 4.4. For `kind/docs` stories.

- `cw_writer`: triggered by `cw.write` and `cw.rejected`. Produces
  documentation. Publishes `cw.review` (direct chain to cw_reviewer).
- `cw_reviewer`: triggered by `cw.review`. Reviews content. Publishes
  `cw.approved` or `cw.rejected`. Decoupled — does not
  encode destinations. On approval, transitions story to `status/po:merge`
  → auto-advance to `done` (same terminal path as regular stories —
  design Section 4.5).

### `core.guardrails`

Per design-principles Section 1. Universal rules for all hats:
- Invariant compliance from all applicable scopes
- Comment format: `### @<role> — <ISO-timestamp>` with role matching the hat's origin
- No write-lock needed (single member)
- Commit and push after every state change

### RObot configuration

Enable Telegram integration (same pattern as Sprint 3):

```yaml
RObot:
  enabled: true
  timeout_seconds: 600
  checkin_interval_seconds: 300
```

### `skills.dirs`

Team, project, and member agent/skills paths through `.botminter/`.

**Test:** ralph.yml parses as valid YAML. All 15 hats have `triggers`, `publishes`,
`default_publishes`, and `instructions`. Board scanner dispatch table covers all
statuses from design Section 4.3 including `lead:*` statuses. Direct chain events
(`lead.review`, `dev.implement`, `dev.code_review`) are published by the correct hats.
Review hats publish only `.approved`/`.rejected` events — no return destinations
encoded. Work hats declare rejection events in their `triggers` when the reviewer
is unique (e.g., `dev.rejected` → `dev_implementer`); shared reviewer rejections
(e.g., `lead.rejected`) go unmatched and the hatless Ralph orchestrator routes them.
Knowledge paths
present in artifact-producing hats. Backpressure gates present in all work hats.

**Integration:** Combined with Steps 1-2, the complete profile skeleton is ready for
generator validation.

---

## Step 4: Generator Compatibility + Workspace Verification

**Objective:** Verify that the existing generator infrastructure works with the compact
profile without modifications. Validate the full workspace creation flow.

**Implementation:**

No generator changes expected — `just init` already supports arbitrary profiles. This
step validates the assumption.

### Verification sequence

1. Generate team repo:
   `just init --repo=/tmp/test-compact --profile=compact project=hypershift`

2. Verify generated structure:
   - PROCESS.md with story/SRE/CW statuses, no write-locks
   - CLAUDE.md with single-member model
   - `knowledge/` with commit-convention, communication-protocols, pr-standards
   - `invariants/` with code-review-required (adapted), test-coverage
   - `agent/skills/board/`, `agent/skills/create-epic/`
   - `.team-template/` includes compact profile for self-contained member addition

3. Add member:
   `cd /tmp/test-compact && just add-member superman`

4. Verify member structure:
   - `team/superman/ralph.yml` with 15 hats
   - `team/superman/PROMPT.md` with supervised mode
   - `team/superman/CLAUDE.md` with `.botminter/` paths
   - `team/superman/invariants/design-quality.md`
   - Hat knowledge directories under `team/superman/hats/`

5. Create synthetic project repo:
   ```bash
   git init /tmp/synth-compact-project
   cd /tmp/synth-compact-project
   echo "# Synthetic Project" > README.md
   git add -A && git commit -m "init"
   ```

6. Create workspace:
   `cd /tmp/test-compact && just create-workspace superman /tmp/synth-compact-project`

7. Verify workspace layout:
   - PROMPT.md is symlink → `.botminter/team/superman/PROMPT.md`
   - CLAUDE.md is symlink → `.botminter/team/superman/CLAUDE.md`
   - ralph.yml is a copy
   - `.botminter/.member` contains `superman`
   - `.botminter/.github-sim/` exists
   - `.claude/agents/` assembled

8. Dry-run launch:
   `cd /tmp/test-compact && just launch superman --dry-run`

**Test:** All 8 verification steps pass. No generator Justfile changes needed.

**Demo:** Compact team repo generated, member added, workspace created — all using
existing generator infrastructure.

---

## Step 5: Synthetic Fixtures + End-to-End Validation

**Objective:** Validate the full epic + story lifecycle with the compact profile.
A single agent processes a synthetic epic through the complete cycle: triage → design →
plan → breakdown → story TDD flow → done.

**Implementation:**

### Fixture adaptation

Adapt M2 fixtures (`specs/milestone-2-architect-first-epic/fixtures/`) for the compact
profile. Create `specs/milestone-2-architect-first-epic/sprint-5/fixtures/`:

- **Reuse** existing knowledge and invariant fixtures from `../fixtures/` — same files,
  same detection markers (commit convention → `issue number`, HCP architecture →
  `reconciler`, design patterns → `composition`).
- **Adapt `deploy.sh`** — update to work with compact profile structure. Copies fixtures
  to correct locations, seeds epic at `status/po:triage` (full lifecycle, not bypassing
  triage as in Sprint 1). Commits so `.botminter/` clone includes them.
- **Synthetic epic** — `[SYNTHETIC] Add health check endpoint` at `status/po:triage`
  with acceptance criteria for the TDD flow to exercise.

### End-to-end validation sequence

Per design Section 8 (acceptance criteria):

**Epic lifecycle (AC1-AC5, AC8):**
1. Generate team repo with compact profile
2. Add `superman` member
3. Deploy fixtures
4. Create workspace with synthetic project repo
5. Seed epic at `status/po:triage`
6. Launch agent: `just launch superman --telegram-bot-token <token>`
7. Verify epic traverses:
   - `po:triage → po:backlog → arch:design` (po_backlog, human gates triage)
   - `arch:design → lead:design-review` (arch_designer produces design doc, direct to lead_reviewer)
   - `lead:design-review → po:design-review` (lead_reviewer reviews, approves)
   - `po:design-review → arch:plan` (po_reviewer, human approves design)
   - `arch:plan → lead:plan-review` (arch_planner produces breakdown, direct to lead_reviewer)
   - `lead:plan-review → po:plan-review` (lead_reviewer reviews, approves)
   - `po:plan-review → arch:breakdown` (po_reviewer, human approves plan)
   - `arch:breakdown → lead:breakdown-review` (arch_breakdown creates stories, direct to lead_reviewer)
   - `lead:breakdown-review → po:ready` (lead_reviewer reviews, approves)
   - `po:ready → arch:in-progress → po:accept` (arch_monitor)
   - `po:accept → done` (po_reviewer, human accepts)

**Story TDD flow (AC6):**
8. Verify story issues created at `status/qe:test-design`
9. Verify story traverses:
   - `qe:test-design → dev:implement` (qe_test_designer writes tests)
   - `dev:implement → dev:code-review` (dev_implementer writes code)
   - `dev:code-review → qe:verify` (dev_code_reviewer reviews)
   - `qe:verify → arch:sign-off` (qe_verifier verifies)
   - `arch:sign-off → po:merge → done` (auto-advance)

**Knowledge propagation (AC7):**
10. Verify design doc contains markers from all scopes:
    - `issue number` (team knowledge: commit convention)
    - `reconciler` (project knowledge: HCP architecture)
    - `composition` (member knowledge: design patterns)

**Supervised mode (AC8):**
11. Verify human gated only at: `po:design-review`, `po:plan-review`, `po:accept`
12. Verify all other transitions auto-advanced without HIL

**Structural checks:**
13. Verify comment audit trail uses correct role headers (`@architect`, `@dev`, `@qe`)
14. Verify no lock files remain
15. Verify `poll-log.txt` contains clean scan cycles

**Test:** All 15 verification points pass. Full epic + story lifecycle completes with
a single agent.

**Demo:** One agent, all roles. Epic seeded at triage, traverses the complete lifecycle
including story TDD flow, design with knowledge propagation, supervised mode gates at
major decisions only. Complete in a single workspace with no coordination overhead.
