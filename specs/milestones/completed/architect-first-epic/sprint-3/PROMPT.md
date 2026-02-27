# Sprint 3: Telegram, HIL, Training Mode — Human in the Loop

## Objective

Add Telegram routing (separate bot per member), mandatory `--telegram-bot-token` in
`just launch`, enable training mode, wire up HIL review gates, and implement rejection
loops. The human gates all decisions via Telegram.

## Prerequisites

Sprint 2 complete. Both agents coordinate through the full epic lifecycle autonomously.
The auto-advance behavior in Sprint 2 is replaced with actual HIL interactions.

## Key References

- Design: `specs/milestone-2-architect-first-epic/design.md`
- Sprint plan: `specs/milestone-2-architect-first-epic/sprint-3/plan.md`
- Ralph HIL research: `specs/milestone-2-architect-first-epic/research/ralph-orchestrator-human-in-the-loop.md`
- Sprint 2 artifacts: both agents with auto-advancing gates

## Requirements

1. **`just launch` update** — mandatory `--telegram-bot-token <TOKEN>` argument
   (design.md Section 2.7). Set `RALPH_TELEGRAM_BOT_TOKEN` from arg. Abort with
   clear error if missing. Keep `--dry-run` support.

2. **Both agents' ralph.yml** — enable RObot with nested config:
   ```yaml
   RObot:
     enabled: true
     timeout_seconds: 600
     checkin_interval_seconds: 300
   ```

3. **Both agents' PROMPT.md** — change training mode from DISABLED to ENABLED

4. **Architect hats** — add training mode conditional blocks to all five hats in
   `ralph.yml`. Sprint 2 omitted these (training mode was DISABLED). Each hat
   needs an "If TRAINING MODE is ENABLED" gate that reports intent via
   `human.interact` and waits for confirmation before any state-modifying action.
   Per design.md Section 4.1.1 for per-hat specifics.

5. **Human-assistant hats** — remove Sprint 2 auto-advance instructions:
   - backlog_manager: present to human via HIL, wait for decision (approve/reject for
     triage, "start this one" for backlog, "start epic #N" for ready)
   - review_gater: present artifacts to human via HIL, wait for approval/rejection.
     On rejection: append feedback comment, revert status. On timeout: no action,
     re-present next cycle.
   - Per design.md Section 4.2.1 for full hat instruction content.

6. **`invariants/always-confirm.md`** — restore full enforcement (was suspended in
   Sprint 2)

7. **Rejection loops** — update architect designer, planner, and breakdown_executor
   hats for revision-awareness. Sprint 2 hats don't check for rejection feedback.
   - Designer must scan comments for rejection feedback and revise accordingly
   - Planner must scan comments and produce revised breakdown (new comment)
   - Breakdown_executor must read the LATEST breakdown comment after revisions
   - Per design.md Scenarios B and C (Section 3.1)

8. **HIL interaction protocol** — all `human.interact` gates must present a
   summary and ask a clear question. Human responds with approve/reject +
   optional feedback. Agents interpret and act accordingly. Same protocol across
   all HA gates and architect training mode confirmations.

9. **Documentation** — complete `docs/`:
   - New `docs/telegram-setup.md`: creating bots, token management, troubleshooting
   - New `docs/training-mode.md`: what it is, per-hat behavior, interaction patterns,
     rejection loops, monitoring
   - New `docs/operations.md`: full end-to-end setup walkthrough, day-to-day ops,
     error recovery, common workflows
   - Update `docs/getting-started.md`, `docs/workspace-commands.md`,
     `docs/epic-lifecycle.md`, `docs/member-roles.md` with Telegram/HIL content

## Verification Strategy

This sprint's features depend on live Telegram bots and human interaction. Ralph MUST
NOT attempt to launch agents, connect to Telegram, or run live integration tests.
Verification is split into two tiers:

**Tier 1 — Ralph verifies (file inspection + dry-run):** Ralph confirms the artifacts
are correctly wired by reading the resulting files and running non-interactive checks.

**Tier 2 — Manual verification (operator runs later):** The operator launches agents
with real Telegram tokens, interacts via Telegram, and validates end-to-end behavior.
These criteria are documented here for the operator's test plan, not for Ralph to execute.

## Acceptance Criteria — Tier 1 (Ralph verifies)

- Given `just launch architect` without `--telegram-bot-token`, when the recipe runs,
  then it aborts with a clear error message

- Given `just launch architect --telegram-bot-token test-token --dry-run`, when the
  recipe runs, then it prints state including `RALPH_TELEGRAM_BOT_TOKEN=test-token`
  and does not launch Ralph

- Given both agents' `ralph.yml`, then each contains `RObot.enabled: true`,
  `RObot.timeout_seconds: 600`, `RObot.checkin_interval_seconds: 300`

- Given both agents' PROMPT.md, then each contains `TRAINING MODE: ENABLED`

- Given the architect's `ralph.yml`, then all five hats (board_scanner, designer,
  planner, breakdown_executor, epic_monitor) contain an "If TRAINING MODE is ENABLED"
  conditional block that uses `human.interact`

- Given the human-assistant's `ralph.yml`, then backlog_manager and review_gater
  instructions present artifacts to the human via `human.interact` and handle
  approve/reject/timeout — with no Sprint 2 auto-advance instructions remaining

- Given the architect's designer hat instructions, then they include
  rejection-feedback scanning (read comments for feedback on re-dispatch)

- Given the architect's planner hat instructions, then they include
  rejection-feedback scanning and produce revised breakdowns as new comments

- Given the architect's breakdown_executor hat instructions, then they read the
  LATEST breakdown comment when multiple exist

- Given `invariants/always-confirm.md`, then full enforcement is restored (not
  suspended)

- Given `docs/telegram-setup.md`, `docs/training-mode.md`, and `docs/operations.md`,
  then each exists and renders as valid markdown

- (Regression) Given the HA board scanner hat instructions, then stale lock cleanup
  behavior from Sprint 2 is preserved

## Acceptance Criteria — Tier 2 (manual verification by operator)

These require live Telegram bots. The operator runs these after Sprint 3 implementation.

- Given `just launch architect --telegram-bot-token <TOKEN>`, when Ralph starts, then
  the architect communicates via its own Telegram bot (separate from the HA's bot)

- Given training mode enabled, when the architect board scanner detects work, then it
  reports board state to the human via Telegram and waits for confirmation before
  dispatching

- Given `status/po:triage`, when the HA backlog_manager fires, then it presents the
  epic to the human via Telegram and waits for approval before advancing

- Given `status/po:design-review`, when the HA review_gater fires, then it presents
  the design summary to the human and waits for approve/reject

- Given the human rejects a design with feedback "missing error handling", when the
  review_gater processes rejection, then a feedback comment is appended, the epic
  reverts to `status/arch:design`, and the architect subsequently reads the feedback
  and produces a revised design that addresses the concern

- Given the human rejects a plan with feedback "stories too large", when the
  review_gater processes rejection, then a feedback comment is appended, the epic
  reverts to `status/arch:plan`, and the architect produces a revised breakdown
  as a new comment

- Given both agents running with separate Telegram bots and a synthetic epic at
  `status/po:triage`, when the human approves through the full lifecycle via Telegram,
  then the epic reaches `status/done` with no lock collisions or data loss

- Given an agent crashes while holding a lock, when the HA board scanner runs, then
  the stale lock is cleaned up within one scan cycle and subsequent processing proceeds
  normally
