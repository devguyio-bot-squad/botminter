# Spike: Autonomous Ralph Runner (M1.5)

Implement the M1.5 spike — a minimal prototype proving Ralph can run autonomously in a persistent loop, processing work items through a two-phase pipeline (propose → finish) with self-clearing state between phases.

## Objective

Execute `specs/milestone-1.5-autonomous-ralph/plan.md` steps 1–8 in order. Each step has explicit test requirements — verify them before moving to the next step.

## Key Requirements

- Scaffold `specs/milestone-1.5-autonomous-ralph/artifacts/` with 3 work items, ralph.yml, and PROMPT.md per the design
- ralph.yml uses `persistent: true`, three hats (board_scanner, proposal_writer, finisher), and the event flow from `design.md` section 4.1
- PROMPT.md uses the role identity template from `design.md` section 4.2
- Run with `RALPH_DIAGNOSTICS=1 ralph run -v --no-tui -P PROMPT.md`
- Verify all 11 acceptance criteria after the run (full list in `plan.md` step 6)
- Write `findings.md` with pass/fail results, observations, and M2 implications
- Commit all artifacts (source files, runtime output, diagnostics)

## Acceptance Criteria

- **Given** Ralph starts with `persistent: true` and 3 work items in `work/`, **when** the run completes, **then** all 3 items are in `done/`, `work/` and `proposed/` are empty
- **Given** items exist in both `work/` and `proposed/`, **when** the board scanner fires, **then** it prioritizes `work/` over `proposed/`
- **Given** all items are in `done/`, **when** the board scanner fires, **then** it publishes `LOOP_COMPLETE` and persistent mode keeps the loop alive via `task.resume`
- **Given** the board scanner dispatches to a work hat, **when** it self-clears, **then** scratchpad is overwritten and tasks.jsonl is deleted before the event is published
- **Given** `RALPH_DIAGNOSTICS=1` is set, **when** the run completes, **then** `.ralph/diagnostics/` contains orchestration, agent-output, and trace files spanning all phases

## Reference

- Design: `specs/milestone-1.5-autonomous-ralph/design.md`
- Plan: `specs/milestone-1.5-autonomous-ralph/plan.md`
- Requirements: `specs/milestone-1.5-autonomous-ralph/requirements.md`

## Launch Command (Step 5)

From inside the scaffolded `artifacts/` directory:

```bash
cd specs/milestone-1.5-autonomous-ralph/artifacts
RALPH_DIAGNOSTICS=1 ralph run -v --no-tui -P PROMPT.md 2>&1 | tee ralph-output.log
```
