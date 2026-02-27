# Sprint 3 Plan: Telegram, HIL, Training Mode — Human in the Loop

> Vertical slice: add Telegram routing, training mode, HIL review gates, and rejection
> loops. The human gates all decisions via separate Telegram bots.
>
> Prerequisite: Sprint 2 complete. Both agents coordinate through the full lifecycle autonomously.
> Design reference: [../design.md](../design.md)

## Checklist

- [x] Step 1: Telegram routing and just launch
- [x] Step 2: Training mode and HIL gates
- [x] Step 3: Rejection loops
- [x] Step 4: Documentation — `docs/`
- [ ] Step 5: Full integration and stress tests (manual test plan not produced; manual Telegram tests not run — superseded by Sprint 4 automated tests)

---

## Step 1: Telegram Routing and `just launch`

**Objective:** Separate Telegram bot per member, mandatory `--telegram-bot-token`
argument on launch.

**Implementation:**

### `just launch` update

Update `skeletons/team-repo/Justfile` `launch` recipe (design.md Section 2.7):

- Add mandatory `--telegram-bot-token <TOKEN>` argument.
- Parse `--telegram-bot-token` from flags. Abort with clear error if not provided:
  `"Error: --telegram-bot-token <TOKEN> is required. Each member needs its own Telegram bot."`
- Set `RALPH_TELEGRAM_BOT_TOKEN=$TOKEN` before invoking Ralph.
- Keep `--dry-run` support.

### Both agents' ralph.yml

Add the following nested config block to both architect and human-assistant `ralph.yml`:

```yaml
RObot:
  enabled: true
  timeout_seconds: 600
  checkin_interval_seconds: 300
```

**Test (Ralph verifies):**
- `just launch architect` without token → error message, no launch.
- `just launch architect --telegram-bot-token test-token-123 --dry-run` → prints state
  including `RALPH_TELEGRAM_BOT_TOKEN=test-token-123`, does not launch Ralph.
- Both agents' `ralph.yml` contain the `RObot` config block (file inspection).
- MUST NOT actually launch Ralph or connect to Telegram.

**Manual verification (operator, later):** Both agents communicate with the human via
their own Telegram bots. Each `just launch` invocation gets its own token.

---

## Step 2: Training Mode and HIL Gates

**Objective:** Enable training mode. Wire up all hat instructions to gate on human
decisions via `human.interact`.

**Implementation:**

### Both agents' PROMPT.md

Change from `TRAINING MODE: DISABLED` to `TRAINING MODE: ENABLED` (design.md Section 4.1.2).

### Architect hat instructions — add training mode conditionals

Sprint 2 implemented hats WITHOUT training mode blocks (training mode was DISABLED).
Add an "If TRAINING MODE is ENABLED" gate to all five hats (board_scanner, designer,
planner, breakdown_executor, epic_monitor). Each gate uses `human.interact` to
report intent and wait for confirmation before the hat's first state-modifying action.
Per design.md Section 4.1.1 for per-hat specifics.

**Note:** Steps 1 and 2 are tightly coupled — enabling RObot without training mode
conditionals (or vice versa) produces an inconsistent state. Commit together.

### Human-assistant hat instructions

Remove Sprint 2 auto-advance instructions. Replace with HIL interactions:

- **backlog_manager** (design.md Section 4.2.1):
  - `po:triage`: Present epic summary to human via HIL. Wait for "accept to backlog"
    or "reject". On approval → `po:backlog`. On rejection → close issue with comment.
    On timeout → no action, stays in triage, re-presented next cycle.
  - `po:backlog`: Present prioritized backlog. When human says "start this one" →
    `arch:design`.
  - `po:ready`: Report ready epics (informational). When human says "start epic #N" →
    `arch:in-progress`. Remind about stale ready epics.

- **review_gater** (design.md Section 4.2.1):
  - `po:design-review`: Read design doc, present summary + key highlights to human.
    On approval → `arch:plan`. On rejection → append feedback comment, revert to
    `arch:design`.
  - `po:plan-review`: Read story breakdown, present to human. On approval →
    `arch:breakdown`. On rejection → append feedback, revert to `arch:plan`.
  - `po:accept`: Present completed epic for final acceptance. On approval → `done`,
    close issue. On rejection → append feedback, revert to `arch:in-progress`.

### HIL interaction protocol

All `human.interact` gates follow the same pattern: present a summary, ask a
clear question, interpret human response as approve/reject/guidance. Same
protocol across HA backlog_manager, HA review_gater, and architect training
mode gates.

### Human-assistant `invariants/always-confirm.md`

Restore full enforcement: "You MUST confirm with the human before any state-modifying
action." (Sprint 2 had this suspended.)

**Test (Ralph verifies — file inspection only):**
- Both agents' PROMPT.md contains `TRAINING MODE: ENABLED`.
- All five architect hats contain "If TRAINING MODE is ENABLED" conditional with
  `human.interact`.
- HA backlog_manager instructions present to human, handle approve/reject/timeout —
  no auto-advance instructions remaining.
- HA review_gater instructions present artifacts to human, handle approve/reject with
  feedback comments and status reversion.
- `invariants/always-confirm.md` is restored (not suspended).
- MUST NOT launch agents or connect to Telegram.

**Manual verification (operator, later):** Launch both agents with Telegram tokens.
Verify board scanner reports and waits. Approve triage via Telegram. Approve design
review. Verify all gates require human confirmation.

---

## Step 3: Rejection Loops

**Objective:** Human can reject designs and plans with feedback. The architect reads
feedback and produces revised artifacts.

**Implementation:**

### Review gater rejection flow

Already specified in Step 2's review_gater instructions. Key behaviors:

- On design rejection: HA acquires lock, appends comment with human's feedback
  (using standard comment format `### @human-assistant — <timestamp>`), transitions
  `po:design-review` back to `arch:design`, releases lock, pushes.
- On plan rejection: same pattern, reverts `po:plan-review` to `arch:plan`.
- On acceptance rejection: reverts `po:accept` to `arch:in-progress`.

### Architect revision behavior — hat instruction updates required

Sprint 2's architect hats do NOT include rejection-awareness. Update:

- **Designer**: Add rejection-feedback scanning. On re-dispatch to `arch:design`,
  read comments for feedback, incorporate into revised design (overwrite previous).
- **Planner**: Same pattern — scan for feedback, produce revised breakdown as a
  new comment (append-only).
- **Breakdown_executor**: Read the LATEST breakdown comment (not the first) when
  multiple exist after a revision cycle.

### Scenarios to verify

Per design.md Sections 3.1 (Scenario B and C):

**Design rejection loop:**
1. Architect produces design → `po:design-review`
2. Human rejects via HA: "missing error handling"
3. HA appends feedback comment, reverts to `arch:design`
4. Architect detects `arch:design` again, reads rejection feedback
5. Architect produces revised design (overwrites previous)
6. Human approves → lifecycle continues

**Plan rejection loop:**
1. Architect proposes breakdown → `po:plan-review`
2. Human rejects via HA: "stories too large, split further"
3. HA appends feedback comment, reverts to `arch:plan`
4. Architect detects `arch:plan` again, reads rejection feedback
5. Architect produces revised breakdown (new comment)
6. Human approves → lifecycle continues

**Test (Ralph verifies — file inspection only):**
- Designer hat instructions include rejection-feedback scanning logic.
- Planner hat instructions include rejection-feedback scanning and append-only
  revision behavior.
- Breakdown_executor hat instructions read the LATEST breakdown comment.
- MUST NOT launch agents or simulate rejection flows.

**Manual verification (operator, later):**
- Per design.md Section 8.5: full design rejection loop via Telegram.
- Per design.md Section 8.6: full plan rejection loop via Telegram.
- Verify feedback comments use standard format.
- Verify architect reads and incorporates specific feedback.
- Verify revised artifacts address the feedback.

**Integration:** Rejection loops complete the review gate model. Quality improves
through iterative human feedback.

---

## Step 4: Documentation — `docs/`

**Objective:** Complete the operator documentation with Telegram setup, training mode
operations, and a full operational guide.

**Implementation:**

### New: `docs/telegram-setup.md` — Telegram Setup

- Creating Telegram bots (BotFather, one bot per member)
- Token management (one token per member, passed to `just launch`)
- `just launch <member> --telegram-bot-token <TOKEN>` — how it works
- Verifying Telegram connectivity (Ralph RObot startup, chat ID auto-detection)
- Troubleshooting (bot not responding, token errors, chat ID issues)

### New: `docs/training-mode.md` — Training Mode Operations

- What training mode is (human gates all state-modifying actions)
- How it works per hat (board scanner reports, designer/planner ask confirmation,
  review gater presents artifacts)
- Telegram interaction patterns (approve, reject with feedback, provide guidance)
- Rejection loops (design rejection, plan rejection — what happens, what to say)
- Monitoring agents (poll-log.txt, `ralph tools interact`, bot commands)

### New: `docs/operations.md` — Running a Team (End-to-End)

- Full setup walkthrough: init → add members → create workspaces → launch
- Day-to-day operations: creating epics, monitoring the board, reviewing artifacts
- Updating team configuration (knowledge, invariants, process changes, `just sync`)
- Stopping and restarting agents
- Error recovery (stale locks, crashed agents, failed processing)
- Common workflows: add a new epic, review a design, activate an epic

### Update existing pages

- `docs/getting-started.md` — add Telegram setup as a prerequisite for launch,
  update launch command with `--telegram-bot-token`
- `docs/workspace-commands.md` — update `just launch` with mandatory token arg
- `docs/epic-lifecycle.md` — add rejection loop diagrams, HIL interaction points
- `docs/member-roles.md` — add Telegram bot per member, training mode behavior

**Test:** All pages render as valid markdown. Full walkthrough is reproducible by
following `docs/operations.md` step by step.

**Integration:** Complete operator documentation. A new user can set up and run a
full two-agent team by following the docs.

---

## Step 5: Manual Integration and Stress Test Plan

**Objective:** Document the full integration test plan for the operator to run
manually with live Telegram bots. Ralph MUST NOT execute any of these tests.

**Implementation:**

Ralph produces a `test-plan.md` file (or section in the sprint plan) that the
operator follows manually. This is a documentation deliverable, not an automated
test step.

### Full integration test (operator runs manually)

Execute design.md Section 8.2 — all 19 steps with Telegram interactions.
Steps marked **(M)** require manual Telegram interaction with separate bots:

1. **(A)** Generate team repo with `just init`
2. **(A)** Deploy synthetic fixtures
3. **(A)** Add both members
4. **(A)** Create both workspaces
5. **(A)** Verify workspace layout
6. **(A)** Seed synthetic epic at `po:triage`
7. **(A)** Verify board shows epic
8. **(M)** Launch HA, verify it detects epic and presents to human
9. **(M)** Approve triage → backlog → activate (via HA Telegram bot)
10. **(M)** Launch architect, verify it detects `arch:design` and reports intent
    (via architect Telegram bot). Confirm.
11. **(A)** Verify design doc with knowledge propagation markers
12. **(A)** Verify design doc invariant compliance
13. **(M)** Approve design via HA → verify plan proposal appears
14. **(M)** Approve plan → verify story issues created
15. **(A)** Verify epic at `po:ready`
16. **(M)** Tell HA "start epic #1" → verify `arch:in-progress`
17. **(A)** Verify architect fast-forwards to `po:accept`
18. **(M)** Approve acceptance via HA → verify `done`
19. **(M)** Day-2 test: edit knowledge files, push, sync, verify propagation

### Rejection loop tests (operator runs manually)

- **(M)** Design rejection: reject design with feedback, verify architect revises,
  approve revision (design.md Section 8.5)
- **(M)** Plan rejection: reject plan with feedback, verify architect revises,
  approve revision (design.md Section 8.6)

### Concurrent operations test (design.md Section 8.7)

- Launch both agents simultaneously
- Seed epic at `po:triage`
- Approve through full lifecycle via Telegram
- Verify: no lock collisions, no lost updates, no duplicate processing
- Verify: clean poll-log.txt on both agents

### Push-conflict test (design.md Section 8.8)

- Launch both agents simultaneously
- Seed two epics at different statuses (one `po:triage`, one `arch:design`)
- Trigger near-simultaneous pushes
- Verify: both succeed (one directly, one via pull-rebase-retry)
- Verify: no data loss

### Crash-during-lock test (design.md Section 8.9)

- Create lock file simulating crashed architect (old timestamp)
- Start HA → verify stale lock cleanup
- Start architect → verify self-cleanup on startup
- Verify subsequent processing is normal

### Knowledge propagation test (design.md Section 8.10)

- Verify design doc contains grep-able markers:
  - Issue-number commits (team knowledge)
  - Reconciler pattern (project knowledge)
  - Composition patterns (member knowledge)
- Verify all required sections per `design-quality.md`

**Test (Ralph verifies):** The test plan document exists and covers all design.md
Section 8 scenarios. Ralph MUST NOT execute any test that launches agents or
connects to Telegram.

**Demo (operator runs):** Full M2 demo. Both agents with separate Telegram bots.
Human approves triage, reviews design (rejects once, approves revision), approves
plan, activates epic, accepts completion. Rejection loop verified. Concurrent
operations clean. Complete lifecycle with human in the loop.
