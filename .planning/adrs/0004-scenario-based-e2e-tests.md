---
status: accepted
date: 2026-03-09
decision-makers: operator (ahmed), claude
---

# Scenario-Based E2E Tests Over Feature-Fragment Tests

## Problem

Phase 9 UAT revealed that 20 e2e tests — each testing one CLI command in isolation — missed obvious integration gaps. `bm teams show` didn't display bridge info, `bm bridge identity add` didn't prompt for tokens, and the full bridge lifecycle was never tested end-to-end. Each test was written when its feature shipped, not as part of a user journey. How should e2e tests be organized to prevent this class of gap?

## Constraints

* Tests must model what operators actually do — sequences of commands (init → hire → sync → start), not isolated commands
* Must include runtime verification (member healthy, bridge functional) — not just "files were generated"
* Must not exhaust GitHub API rate limits — shared setup across cases, not duplicated per test
* AI coding agents default to isolated feature tests (their plans decompose by task, not by journey) — the format must resist this tendency
* The `GithubSuite` pattern with `.setup()` and `.case()` is the existing test infrastructure

## Decision

Organize e2e tests as complete operator journeys, one per profile × bridge variation. Each scenario is a sequence of cases that build on shared state: init → hire → configure bridge → sync → start → verify → stop → cleanup. Cases within a scenario share setup (one GitHub repo, one team) and run sequentially.

Coverage is enforced by `invariants/e2e-scenario-coverage.md`: every scenario must include member start, health check, bridge verification (when applicable), and clean shutdown. New features must be tested as steps within scenarios, not as isolated fragments.

## Rejected Alternatives

### Feature-fragment tests (one test per CLI command)

Rejected because: inter-feature gaps are invisible when each feature is tested alone.

* Each of 20 tests duplicates setup (create team, hire member, sync) — wastes GitHub API calls
* Encourages skipping runtime steps ("this test is just about sync, no need to start members")
* AI agents default to this pattern since plans decompose by task, not by journey

### Hybrid (scenarios for happy paths, fragments for edge cases)

Rejected because: ambiguity about what's a "happy path" vs "edge case" causes fragments to creep back in.

* Reasonable in theory but requires clear rules that AI agents don't follow consistently

## Consequences

* Each scenario is a complete validation of the operator experience — if it passes, the full flow works
* Failure isolation is coarser — a failure at step 7 requires reading output to find which step broke
* Cases within a scenario can share state (scenario step 2 builds on step 1), reducing API calls and runtime
* The `GithubSuite` builder pattern already supports sub-cases within a shared setup

## Anti-patterns

* **Do NOT** write isolated e2e tests for individual CLI commands — they miss inter-feature gaps. Every feature must be tested within an operator journey scenario.
* **Do NOT** skip runtime verification in scenarios — "sync created the files" is not enough. The scenario must start the member, verify it's healthy, and verify bridge connectivity.
* **Do NOT** stop a member mid-scenario unless the next case expects it stopped — cases depend on state continuity. A case that stops the member breaks all subsequent cases that expect it running.
* **Do NOT** create new e2e test files for individual features — add a case to the appropriate scenario instead. The invariant `e2e-scenario-coverage.md` enforces this.
