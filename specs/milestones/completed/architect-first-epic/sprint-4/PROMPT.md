# Sprint 4: Automated HIL Telegram Tests

## Objective

Automate the M2 HIL test scenarios (design.md Section 8) using tg-mock and
fixture-driven Rust integration tests. No real Telegram, CI-ready.

## Prerequisites

Sprints 1-3 complete. Both agents have training mode enabled, RObot configured,
rejection loops implemented. Docker available. Rust toolchain available.

## Deviations

None. This sprint implements testing infrastructure only — no changes to existing
skeletons, agent configs, or prior sprint artifacts.

## Key References

- Sprint design: `specs/milestone-2-architect-first-epic/sprint-4/design.md`
- Sprint plan: `specs/milestone-2-architect-first-epic/sprint-4/plan.md`
- Mock server research: `specs/milestone-2-architect-first-epic/sprint-4/research/mock-telegram-server.md`
- M2 design (test scenarios): `specs/milestone-2-architect-first-epic/design.md` Section 8
- Ralph Telegram docs: `/opt/workspace/ralph-orchestrator/docs/guide/telegram.md`
- tg-mock: https://github.com/watzon/tg-mock
- Existing fixtures: `specs/milestone-2-architect-first-epic/fixtures/`
- Design principles: `specs/design-principles.md`

## Requirements

1. **Ralph api_url validation** — Ralph's `RALPH_TELEGRAM_API_URL` feature is
   untested. Validate the full sendMessage→getUpdates round-trip through tg-mock
   before building on it. If api_url has bugs, fix in Ralph first. Document
   findings in `research/api-url-validation.md`.

2. **tg-mock lifecycle recipes** — Justfile recipes to start and stop the tg-mock
   Docker container (`ghcr.io/watzon/tg-mock:latest`). Per design.md Justfile
   Recipes section.

3. **Rust test crate** — a test crate at `tests/hil/` with a typed client for
   tg-mock's `/__control/` API, setup helpers that call `just` recipes via
   `std::process::Command`, and a fixture-driven response loop. Per design.md
   Components section.

4. **YAML fixture format** — test scenarios defined as data, not code. Each
   fixture MUST specify response rules (match pattern, reply text, optional
   `times` counter) and expected outcomes. Per design.md Fixtures section.

5. **Lifecycle test** — fixture + test covering full epic triage→done with both
   agents. MUST verify knowledge propagation markers in the design doc (all 3
   scopes). Covers design.md Section 8 scenarios #1, #4, #7.

6. **Rejection loop tests** — fixtures + tests for design rejection and plan
   rejection. Each MUST reject once with specific feedback, verify architect
   revises, then approve. Covers design.md Section 8 scenarios #2, #3.

7. **Edge case tests** — fixtures + tests for push-conflict (two epics
   simultaneously) and crash-recovery (stale lock cleanup). Covers design.md
   Section 8 scenarios #5, #6.

8. **Test runner recipe** — a `just test-hil` recipe that starts tg-mock, runs
   all tests, and stops tg-mock. MUST exit non-zero on any failure.

## Acceptance Criteria

- Given Docker running, when `just test-hil` executes, then all tests pass
  and exit code is 0

- Given the lifecycle fixture, when both agents run with scripted approvals,
  then the epic traverses triage→done and the design doc contains knowledge
  markers from all 3 scopes

- Given the design-rejection fixture, when the first design is rejected with
  feedback, then the architect revises and the epic completes on second approval

- Given the plan-rejection fixture, when the first plan is rejected with
  feedback, then the architect revises the breakdown and the epic completes

- Given the two-epics fixture, when both epics are processed simultaneously,
  then both reach done with no data loss and no stale locks

- Given the stale-lock fixture, when agents start with a pre-existing stale
  lock, then the lock is cleaned up and the epic processes normally to done

- Given all tests pass, then zero real Telegram API calls were made — all
  traffic goes to tg-mock on localhost

- (Regression) Given the existing `specs/milestone-2-architect-first-epic/fixtures/`,
  then the test infrastructure MUST reuse them, not duplicate
