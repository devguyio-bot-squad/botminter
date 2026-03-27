# Exploratory Test User Journey

Each exploratory test phase MUST validate the integrated user journey, not just individual subsystems in isolation.

## Rule

Each exploratory test phase MUST include integrated tests that exercise the full path from user action to observable outcome through the entire stack. Proving that subsystems work independently is necessary but not sufficient — the phase MUST combine them into the journeys users actually experience.

Specifically:

- If a feature involves multiple subsystems (e.g., bridge messaging + brain processing), tests MUST exercise them together: user action → subsystem A → subsystem B → observable result.
- The observable outcome MUST be something the user can see or verify (message in a chat room, status in CLI output, file in workspace) — not an internal state change that requires inspecting implementation details.
- Tests that prove each subsystem works in isolation (e.g., "Matrix sends messages" and "brain starts") without combining them into an integrated flow do NOT satisfy this invariant.

See also `exploratory-test-single-journey-smell.md` — a single integrated journey is not enough; this invariant requires integration, that one requires breadth.

## Applies To

- All test phases in `crates/bm/tests/exploratory/` (PLAN.md and Justfile)
- Phase design: when adding a new phase, the plan MUST identify integrated journeys before listing individual test cases

Does **NOT** apply to:

- Unit tests — these intentionally test components in isolation
- Integration tests — these test module boundaries, not user journeys
- E2E tests — governed by `e2e-scenario-coverage.md`
- Individual test cases within a phase — not every case needs to be integrated, but the phase as a whole MUST include integrated journeys

## Examples

**Compliant:**
```
# Phase H: Brain autonomy — integrated
bm start → send message via Matrix → poll for brain response → assert response → bm stop
```
One test crosses all subsystems: CLI (start/stop), bridge (Matrix), brain (response).

**Violating:**
```
# Phase H: Brain autonomy — subsystems tested separately
## Section 1: Matrix messaging
curl send message → assert delivered

## Section 2: Brain lifecycle
bm start → assert PID alive → bm stop
```
Each subsystem passes independently, but no test verifies that a message sent while the brain is running produces an autonomous response.

**Violating:**
```
# Phase H: Brain autonomy — internal state instead of observable outcome
bm start → write event to .ralph/events.jsonl → check state.json for brain_mode=true
```
Tests internal state, not what the user sees. The user sends a Matrix message and expects a response — not a JSON field in a state file.

## Rationale

Phase H.8 exploratory tests validated Matrix API messaging and brain lifecycle as separate sections. All 28 tests passed. But no test sent a message to a running brain member and verified a response — the entire point of the "chat-first member" milestone. A reviewer approved the tests because each subsystem was thoroughly validated. It required human feedback to identify that proving "A works" and "B works" independently does not prove "A + B work together." This invariant ensures every exploratory phase includes tests that cross subsystem boundaries to validate the integrated user experience.
