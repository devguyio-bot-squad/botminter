# Sprint 2: Two Agents, Full Lifecycle — Autonomous Coordination

## Objective

Add remaining architect hats (planner, breakdown_executor, epic_monitor), evolve the
human-assistant to a three-hat model (board_scanner, backlog_manager, review_gater),
add team-level skills, and validate that both agents coordinate through the full epic
lifecycle autonomously (no HIL — review gates auto-advance).

## Sprint 2 Deviations from Design

These are intentional scope decisions. See sprint plan for full rationale.

- **Training mode: DISABLED** — no HIL channel. Agent acts autonomously. Re-enabled Sprint 3.
- **RObot: disabled** — no Telegram bots. Sprint 3.
- **Telegram deferred** — no `--telegram-bot-token`. Sprint 3.
- **Auto-advance gates** — backlog_manager and review_gater auto-advance all gates
  instead of waiting for human input. Sprint 3 restores HIL gates.
- **Project name `hypershift` hardcoded** — single-project assumption for M2.

## Key References

- Design: `specs/milestone-2-architect-first-epic/design.md`
- Sprint plan: `specs/milestone-2-architect-first-epic/sprint-2/plan.md`
- Sprint 1 artifacts: architect skeleton with board_scanner + designer

## Requirements

1. **Architect hats** — add planner, breakdown_executor, epic_monitor to architect
   ralph.yml. Update board_scanner dispatch and priority ordering. Update architect
   PROMPT.md and CLAUDE.md for the full five-hat model. (design.md Section 4.1.1)

2. **Human-assistant** — rewrite ralph.yml, PROMPT.md, CLAUDE.md for the three-hat
   model with Sprint 2 auto-advance behavior. (design.md Section 4.2.1)

3. **PROCESS.md** — replace M1 "submodule" references in Communication Protocols with
   `.botminter/` model. (deferred from Sprint 1, design review Finding 49)

4. **Team-level skills** — create `create-epic` and `board` in
   `skeletons/profiles/rh-scrum/agent/skills/`. (design.md Section 4.5)

5. **Documentation** — update getting-started and epic-lifecycle; create member-roles
   and skills docs. (see sprint plan Step 4)

## Acceptance Criteria

- Given an epic at `status/arch:plan`, when the architect scans, then the planner
  proposes a story breakdown as a comment and transitions to `status/po:plan-review`

- Given an epic at `status/arch:breakdown`, when the architect scans, then story
  issues are created with `kind/story`, `parent` link, `status/dev:ready`, and
  Given-When-Then acceptance criteria

- Given an epic at `status/arch:in-progress`, when the architect scans, then the
  epic_monitor fast-forwards to `status/po:accept` with a comment

- Given a `status/po:triage` epic, when the HA scans, then it auto-advances directly
  to `status/arch:design` with a comment

- Given a `status/po:design-review` epic, when the HA scans, then it auto-advances
  to `status/arch:plan` with an approval comment

- Given a `status/po:plan-review` epic, when the HA scans, then it auto-advances
  to `status/arch:breakdown` with an approval comment

- Given a `status/po:ready` epic, when the HA scans, then it auto-advances
  to `status/arch:in-progress` with a comment

- Given a `status/po:accept` epic, when the HA scans, then it auto-advances
  to `status/done` and closes the issue

- Given a stale lock older than `stale_lock_threshold_minutes`, when the HA board
  scanner runs, then the stale lock is deleted, committed, and pushed

- Given both agents running concurrently with a synthetic epic at `status/po:triage`,
  when the lifecycle completes, then the epic reaches `status/done` with
  `state: closed`, story issues exist, no lock collisions, and clean poll-log.txt

- Given `create-epic` is invoked, then an issue is created with next available
  number, `kind/epic`, `status/po:triage`, using the invoking member's role prefix
  in the lock ID

- Given `board` is invoked, then all issues are displayed grouped by status with
  epic-to-story relationships
