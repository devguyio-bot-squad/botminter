# Reviewer Checklist: Exploratory Tests

When reviewing exploratory test changes (PLAN.md, Justfile, or new phases in `crates/bm/tests/exploratory/`), apply this checklist **before** evaluating technical correctness.

## Intent Alignment (check first)

- [ ] **User perspective:** Does every test case simulate what a real user would do? Ask: "Would an operator run this command in production?" If not, it belongs in unit/integration tests.
- [ ] **No internal commands:** Are all commands user-facing (`bm start`, `bm stop`, `bm status`, `bm teams sync`)? Flag any hidden/internal commands (`bm brain-run`, `bm daemon-tick`).
- [ ] **No state bypass:** Does the test use the real delivery path? Flag direct file writes to internal state (`.ralph/events-*.jsonl`, `state.json`) when the user path is through a bridge or CLI.

## Integration Depth (check second)

- [ ] **Multiple user journeys exist:** Real features have many valid paths, not one. A single integrated journey is a smell — enumerate the distinct ways a user interacts with the feature (single action, multi-turn, cross-member, recovery, error input, lifecycle variations).
- [ ] **Each journey is integrated:** Every identified user journey MUST cross subsystem boundaries end-to-end. Proving "A works" and "B works" separately does not prove "A + B work together."
- [ ] **Edge cases through user path:** Are error conditions tested through the user path, not by injecting internal state?
- [ ] **Recovery tested via user path:** Are recovery scenarios (crash, restart, bridge failure) verified by the user's experience (send message, check response), not by inspecting internal files?
- [ ] **Lifecycle variations covered:** First start, restart after stop, restart after crash, concurrent members — each is a distinct journey.

## Technical Correctness (check last)

- [ ] Tests pass (`just exploratory-test`)
- [ ] PLAN.md and Justfile are consistent (test numbers match, descriptions align)
- [ ] No hardcoded values that break across environments (ports, org names, repo names should come from variables)
- [ ] Cleanup phase handles all artifacts created by the new tests

## Common Review Mistakes

These are patterns where a reviewer approves technically correct tests that miss the point:

1. **Subsystem isolation approval:** Tests prove each piece works independently, all pass, code is clean — but no integrated test exists. The reviewer evaluates test quality without questioning test coverage of the user journey.

2. **Internal command acceptance:** A test invokes `bm brain-run --help` and it passes. The reviewer checks "does it work?" instead of "should this be here?"

3. **File injection acceptance:** A test writes events directly to `.ralph/events.jsonl` and the brain processes them. The reviewer sees correct behavior without questioning whether the delivery path matches production.

4. **Unit test counting:** A test runs `cargo test` and asserts pass counts. The reviewer sees verification without questioning whether this belongs in an exploratory suite.

5. **Single happy path acceptance:** A phase has one integrated journey ("start → send message → get response → stop") and the reviewer approves because "the happy path is covered." Real features have many valid user paths — a single journey is a smell, not sufficient coverage.

## Reference Invariants

- `invariants/exploratory-test-scope.md` — what commands are allowed
- `invariants/exploratory-test-user-journey.md` — integration depth (subsystems must be combined)
- `invariants/exploratory-test-single-journey-smell.md` — breadth (single happy path is a smell)
- `invariants/e2e-scenario-coverage.md` — E2E test structure (separate concern)
