# Scenario-Based E2E Tests Over Feature-Fragment Tests

---
status: accepted
date: 2026-03-09
decision-makers: operator (ahmed), claude
---

## Context and Problem Statement

Phase 9 UAT revealed that 20 e2e tests — each testing one CLI command in isolation — missed obvious integration gaps. `bm teams show` didn't display bridge info, `bm bridge identity add` didn't prompt for tokens, and the full bridge lifecycle was never tested end-to-end. Each test was written when its feature shipped, not as part of a user journey. How should e2e tests be organized to prevent this class of gap?

## Decision Drivers

* Operators run sequences of commands (init → hire → sync → start), not individual commands in isolation
* Feature-fragment tests duplicate setup code across 20 tests (create team, hire member, sync) consuming excessive GitHub API rate limit
* Gaps between features (e.g., "sync writes config but show doesn't display it") are invisible when each feature is tested alone
* AI coding agents tend to write isolated feature tests because their plans decompose by implementation task, not by user journey
* The operator journey does not end at setup — members must start, run healthy, and connect to bridges

## Considered Options

* **Scenario-based tests** — organize e2e tests as complete operator journeys (init through stop), one per profile × bridge variation
* **Feature-fragment tests** — one e2e test per CLI command or feature (current approach)
* **Hybrid** — scenarios for happy paths, fragments for edge cases

## Decision Outcome

Chosen option: "Scenario-based tests", because it models what operators actually do, makes gaps immediately visible when a scenario step is missing, and forces feature additions to update the journey test.

### Consequences

* Good, because each scenario is a complete validation of the operator experience — if it passes, the full flow works
* Good, because sub-scenarios can share setup (scenario 2 builds on scenario 1's state), reducing API calls and test time
* Good, because the invariant (`invariants/e2e-scenario-coverage.md`) prevents feature additions from shipping without scenario coverage
* Good, because runtime verification (member healthy, bridge functional) is mandatory — tests can't stop at "files were generated"
* Bad, because failure isolation is coarser — a failure at step 7 of a 10-step scenario requires reading output to find which step broke
* Neutral, because the existing `GithubSuite` pattern with `.setup()` and `.case()` already supports this structure

### Confirmation

Compliance is enforced by `invariants/e2e-scenario-coverage.md` which requires:
1. Every scenario includes member start, health check, bridge verification (when applicable), and clean shutdown
2. Any happy path feature change updates the corresponding scenario test
3. New features are tested as steps within scenarios, not as isolated fragments

The ADR README index must list this ADR.

## Pros and Cons of the Options

### Scenario-based tests

* Good, because mirrors the actual operator experience end-to-end
* Good, because gaps are immediately visible (missing step = missing assertion in the journey)
* Good, because shared setup reduces GitHub API calls and test runtime
* Good, because forces runtime verification (not just config generation)
* Bad, because failure messages point to a step within a long scenario, not a named test
* Bad, because adding a step to a scenario requires re-running the entire journey

### Feature-fragment tests

* Good, because failure isolation is precise (test name = feature)
* Good, because easy to write incrementally as features ship
* Bad, because duplicated setup across 20+ tests (each creates a team, hires, syncs)
* Bad, because inter-feature gaps are invisible (each feature works alone, but the sequence has holes)
* Bad, because encourages skipping runtime steps ("this test is just about sync, no need to start members")
* Bad, because AI agents default to this pattern since plans decompose by task, not by journey

### Hybrid

* Good, because scenarios cover happy paths while fragments cover edge cases
* Good, because edge case tests (error handling, invalid input) don't need full journey setup
* Neutral, because requires clear rules about what's a "happy path" vs "edge case" — ambiguity leads to fragments creeping back in

## More Information

The existing 20 e2e tests should be refactored into 3 scenarios (Fresh Start, Existing Team, Daemon Mode) as resources allow. New features MUST follow the scenario pattern per the invariant. The `GithubSuite` builder pattern already supports sub-cases within a shared setup, making the refactor straightforward.

See also:
- `invariants/e2e-scenario-coverage.md` — the constitutional constraint
- `invariants/gh-api-e2e.md` — complementary invariant for API interaction testing
- `.planning/debug/resolved/uat6-sync-bridge-issues.md` — the Phase 9 gap that motivated this decision
