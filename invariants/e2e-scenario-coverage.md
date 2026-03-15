# E2E Scenario Coverage

E2E tests MUST be organized as complete user scenarios, not isolated feature fragments. Any change to a happy path feature MUST update the corresponding scenario test.

## Rule

### Scenario structure

E2E tests MUST model complete operator journeys — the full sequence of commands an operator runs to achieve a goal. Each profile × bridge variation MUST have one happy path scenario covering the entire lifecycle.

Tests MUST NOT be created as isolated single-feature fragments (e.g., "test sync alone", "test identity add alone") when the feature is part of a happy path journey. Instead, the feature MUST be tested as a step within the appropriate scenario.

### Scenarios MUST verify runtime, not just setup

Every scenario MUST include member launch and runtime verification — not just CLI setup commands. The operator journey does not end at `bm teams sync`. It ends when members are running, healthy, and connected to their services.

Specifically, each scenario MUST verify:

1. **Member starts successfully** — Ralph orchestrator process launches without errors
2. **Member is healthy** — `bm status` shows the member as running (not crashed, not errored)
3. **Bridge is functional (when configured)** — if a bridge is part of the scenario, the test MUST verify that the bridge service is reachable and the member can communicate through it (e.g., tg-mock receives API calls from the member process)
4. **Clean shutdown** — `bm stop` terminates all members and bridge services

Tests that only verify file structure, config generation, or CLI output without starting members are setup tests, not scenario tests. Setup tests are valuable but they do NOT satisfy this invariant.

### Mandatory update on happy path changes

Any code change that adds, removes, or modifies behavior in a happy path flow MUST update the corresponding scenario test to cover the new behavior. This includes:

- Adding a new CLI command that operators use during setup or operation
- Changing the output of an existing command (new fields, changed format)
- Adding a new step to the init/hire/sync/start flow
- Adding a new flag that affects default behavior

A PR that adds a happy path feature without updating the scenario test is incomplete.

### Current scenarios

| Scenario | Profile | Bridge | Journey |
|----------|---------|--------|---------|
| Fresh start | scrum-compact | telegram | init --bridge → hire all → project add → sync teams → sync projects → **start → status (healthy) → bridge functional (tg-mock receives calls) → stop (clean)** |
| Existing team | scrum-compact | telegram | clone existing → hire → sync → **start → status (healthy) → bridge functional → stop (clean)** |
| Daemon mode | scrum-compact | telegram | init → hire → **daemon start → GH events → members auto-launch → status (members healthy, bridge connected) → daemon stop (all processes terminated)** |

Sub-scenarios MAY share setup by building on earlier scenarios (e.g., daemon mode reuses fresh start setup but replaces `bm start` with `bm daemon start`).

## Applies To

- All E2E tests in `crates/bm/tests/e2e/`
- Any code change to `crates/bm/src/commands/` that affects operator-facing CLI behavior
- Any change to profiles that affects the init/hire/sync/start flow

**Does NOT apply to:**

- Unit tests and integration tests (these test internals and variations)
- Conformance tests (these validate bridge spec compliance)
- One-off regression tests for bugs that don't affect the happy path

## Examples

**Compliant:**
```
Feature: "bm teams show displays bridge info"
Action: Add "Bridge: Telegram [external]" line to teams show output
Test update: Add assertion to Scenario 1 after the hire step:
  - bm teams show → assert stdout contains "Bridge:"
```

**Compliant:**
```
Feature: "bm bridge identity add prompts for token"
Action: Add interactive prompt before recipe invocation
Test update: Scenario 1 already has identity add step with env var token.
  Non-interactive path covered. Interactive path verified by UAT.
```

**Violating:**
```
Feature: "bm bridge room create"
Action: Add room creation command
Test: Create isolated e2e_bridge_room_create test that sets up
  a team from scratch just to test room creation
Problem: Room creation is part of the sync flow. It should be
  a step in Scenario 1, not a standalone test.
```

**Violating:**
```
Feature: "bm teams show displays bridge info"
Action: Add bridge display to teams show
Test: No e2e test updated. Only unit test added for the display function.
Problem: Happy path changed without scenario coverage.
```

**Violating:**
```
Scenario test: "Fresh start with bridge"
Journey: init → hire → sync → verify workspace files → done
Problem: Scenario ends at sync. Never starts members, never
  verifies Ralph launches healthy, never checks bridge receives
  messages. This is a setup test, not a scenario test.
```

**Violating:**
```
Feature: "Add Rocket.Chat bridge"
Test: New e2e test verifies bm bridge start launches Podman pod
  and bm bridge stop tears it down. Does not start any members.
Problem: Bridge lifecycle alone is not a scenario. The test must
  verify a member starts, connects through the bridge, and the
  bridge receives the member's API calls.
```

## Rationale

Phase 9 UAT revealed that 20 fragmented e2e tests — each testing one CLI command in isolation — missed obvious gaps like `bm teams show` not displaying bridge info. The tests were written incrementally as features shipped, not designed as user journeys. This led to duplicated setup code, excessive GitHub API usage, and gaps that only a human running the full sequence would catch. Scenario-based tests model what operators actually do, make gaps immediately visible, and prevent feature additions from shipping without journey-level validation.
