# Summary — Spike: Autonomous Ralph Runner (M1.5)

> Overview of all artifacts produced during this milestone's planning phase.

---

## Objective

Validate that Ralph can run autonomously in a persistent loop — picking up work items, advancing them through multiple phases (propose → finish), self-clearing state between phases, and idling when no work remains. The validated pattern feeds directly into M2's team member configurations.

## Artifacts

| File | Purpose |
|------|---------|
| `rough-idea.md` | Original problem statement and three core questions (work discovery, loop lifetime, state hygiene) |
| `requirements.md` | 12 Q&A entries covering scratchpad reset, event flow, prototype scope, success criteria, and the Proposal A vs B decision |
| `research/continuous-agent-design.md` | Deep analysis of Ralph internals, design tensions, and two proposals (outer loop vs persistent mode). Chose Proposal B. |
| `design.md` | Standalone design: event flow, ralph.yml config, PROMPT.md template, work item model, self-clear sequence, observability plan, acceptance criteria |
| `plan.md` | 8-step implementation plan from scaffolding through execution, verification, findings, and artifact preservation |
| `summary.md` | This file |
| `PROMPT.md` | Implementation prompt for Ralph to execute the spike autonomously via plan.md steps 1–8 |

## Scope

**Does NOT exercise:** `.github-sim/` format, Telegram/HIL, multi-agent coordination, write-locks, or knowledge/invariant scoping. These are deferred to M2.

## Key Design Decisions

1. **Proposal B (persistent mode)** over Proposal A (outer loop) — validates the persistent-mode mechanics needed for the hybrid batch model in M2.
2. **Self-clear via file overwrite** — board scanner overwrites scratchpad and deletes tasks.jsonl before each dispatch. No CLI command needed.
3. **Directory-based state model** — `work/` → `proposed/` → `done/` mirrors M2's status-label transitions without coupling to `.github-sim/` format.
4. **Three hats** — board_scanner (dispatch + self-clear), proposal_writer (phase 1), finisher (phase 2). Work hats publish `board.rescan`; only board_scanner publishes `LOOP_COMPLETE`.
5. **Plain text work items** — 3 trivial tasks, enough to prove first pickup, clean handoff, repeatability, and idle behavior.

## Success Criteria (11 total)

- **Must pass (8):** Multi-phase processing, two-hat dispatch, work hats publish board.rescan, LOOP_COMPLETE only on idle, persistent mode keeps alive, idle on empty board, no crashes, priority ordering.
- **Important but non-blocking (3):** Self-clear between phases, no context pollution, memories persist.

## Next Steps

1. **Implement the spike** — follow `plan.md` steps 1–8 to scaffold, configure, run, verify, and document findings.
2. **Feed findings into M2** — the validated ralph.yml + PROMPT.md pattern becomes the template for architect and human-assistant member configurations.
3. **Address open tradeoffs in M2** — `cooldown_delay_seconds` global scope, no per-issue cost isolation, error propagation between items.
