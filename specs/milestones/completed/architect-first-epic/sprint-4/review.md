# Sprint 4 Review: Plan + PROMPT

> Combined findings from two independent reviews of plan.md and PROMPT.md.
> Reviewed against design-principles.md Section 11 and M2 design.md Section 8.

---

## HIGH Findings

### H1. Incorrect M2 design.md scenario references in PROMPT

**Affects:** PROMPT.md Requirements 5, 6, 7

The PROMPT says "Covers design.md Section 8 scenarios #1, #4, #7" (and #2, #3,
#5, #6). These numbered scenarios don't exist in the M2 design.md. Section 8 uses
subsection numbers with descriptive titles:

| PROMPT says | Intended meaning | Correct reference |
|---|---|---|
| scenarios #1, #4, #7 | Lifecycle, concurrent ops, knowledge propagation | Sections 8.2, 8.7, 8.10 |
| scenarios #2, #3 | Design/plan rejection | Sections 8.5, 8.6 |
| scenarios #5, #6 | Push-conflict, crash-recovery | Sections 8.8, 8.9 |

Ralph will search for "scenario #1" in the design doc, not find it, and waste
iterations resolving the ambiguity.

**Fix:** Replace all "scenarios #N" with actual subsection references.

---

### H2. Missing team-level knowledge marker in approve-all fixture

**Affects:** plan.md Step 3 (approve-all.yml)

The fixture's `design_doc_contains` checks for `reconciler` (project scope) and
`composition` (member scope) but omits the team-level marker. M2 design Section 8.10
explicitly requires verification of all three scopes:

> - Contains reference to issue-number-based commits (team-level knowledge propagation)
> - Contains reference to reconciler pattern (project-level knowledge propagation)
> - Contains composition-based design patterns (member-level knowledge propagation)

The team-level fixture (`commit-convention.md`) seeds: "All commits must reference
an issue number using `Ref: #<number>`." The design doc should mention this, and
the fixture should check for it (e.g., grep for `issue` or `Ref:`).

Without this check, the test can pass while team-level knowledge propagation is broken.

**Fix:** Add a third entry to `design_doc_contains` for the team-level marker.

---

### H3. `RALPH_TELEGRAM_API_URL` not plumbed through `just launch`

**Affects:** plan.md Step 2 (setup.rs `start_agent`), design.md Architecture

The plan says agents connect to tg-mock via `RALPH_TELEGRAM_API_URL=http://localhost:8081`.
But the existing `just launch` recipe (Justfile lines 236-335) only accepts
`--telegram-bot-token`. It has no `--telegram-api-url` flag and does not pass
`RALPH_TELEGRAM_API_URL` through to the Ralph process.

The `start_agent()` function signature is `start_agent(workspace, token)` — no
api_url parameter.

Three options:
- **(a)** Add `--telegram-api-url` flag to `just launch` — but the PROMPT says
  "do NOT modify existing skeleton files"
- **(b)** Have `start_agent()` bypass `just launch` and invoke `ralph run` directly
  with both env vars — loses the workspace discovery/sync that `just launch` does
- **(c)** Export `RALPH_TELEGRAM_API_URL` in the shell environment before calling
  `just launch` — the recipe calls `ralph run` in the same env, so the var should
  pass through. This works because the Justfile line 334 runs:
  `RALPH_TELEGRAM_BOT_TOKEN="$RALPH_TELEGRAM_BOT_TOKEN" CLAUDECODE= ralph run -p PROMPT.md`
  — it only overrides two vars, so `RALPH_TELEGRAM_API_URL` from the parent env
  would survive.

Option (c) is simplest and requires no Justfile changes. But neither the plan
nor the design discusses this.

**Fix:** Explicitly state that `start_agent()` exports `RALPH_TELEGRAM_API_URL`
in the process environment and calls `just launch` normally. The env var passes
through to `ralph run` because the Justfile recipe doesn't override it. Add a
note that this depends on `just launch` not clearing the environment.

---

### H4. No timing budget or timeout guidance

**Affects:** plan.md Steps 3-5, design.md Error Handling

Each hat activation is a Claude Code invocation — 30-60 seconds minimum per hat.
The full lifecycle traverses ~8-10 hat activations across both agents (board scanner
dispatches + work hat executions + re-scans). Realistic estimates:

| Scenario | Hat activations | Estimated duration |
|----------|----------------|-------------------|
| Lifecycle (approve-all) | ~16-20 (both agents) | 15-25 min |
| Design rejection | ~20-24 (extra rejection cycle) | 20-30 min |
| Plan rejection | ~20-24 | 20-30 min |
| Push-conflict (two epics) | ~30-40 | 30-45 min |
| Crash-recovery | ~16-20 | 15-25 min |
| **Full suite** | | **~2-3 hours** |

The plan says `wait_for_status` has a "configurable timeout" and agents use "bounded
`max_iterations` and `max_runtime_seconds`" but provides no values. Without guidance:
- Too-short timeouts → false failures
- Too-long timeouts → silent hangs on real failures
- No per-test timeout → a broken test blocks the entire suite

**Fix:** Add a timing section to the plan with: recommended `wait_for_status`
timeout per scenario (e.g., 30 min for lifecycle, 45 min for rejection), recommended
`max_runtime_seconds` for agent ralph.yml (e.g., 3600), poll interval for tg-mock
requests (e.g., 5s), and overall `cargo test` timeout.

---

## MEDIUM Findings

### M1. Requirement 1 prescribes implementation steps (WHAT-not-HOW violation)

**Affects:** PROMPT.md Requirement 1

> "Validate the full sendMessage→getUpdates round-trip through tg-mock before
> building on it. If api_url has bugs, fix in Ralph first. Document findings
> in research/api-url-validation.md."

This tells Ralph what to do in what order ("before building on it"), where to do
it ("fix in Ralph first" — a different repo), and where to write it up. Per
design-principles Section 11: "Don't prescribe implementation steps."

**Fix:** Rewrite as: "Ralph's `RALPH_TELEGRAM_API_URL` MUST support full
sendMessage/getUpdates round-trip through tg-mock. Per design.md Error Handling
section."

---

### M2. Requirement 3 leaks implementation detail (WHAT-not-HOW violation)

**Affects:** PROMPT.md Requirement 3

> "a typed client for tg-mock's `/__control/` API, setup helpers that call `just`
> recipes via `std::process::Command`, and a fixture-driven response loop"

Enumerates modules and implementation choices (`std::process::Command`). The PROMPT
should state WHAT the crate does, not how it's structured.

**Fix:** Simplify to: "A Rust test crate at `tests/hil/` that interacts with
tg-mock's control API, manages agent lifecycles, and runs fixture-driven test
scenarios. Per design.md Components section."

---

### M3. Missing RFC 2119 language in Requirements 1, 2, 3, 8

**Affects:** PROMPT.md

Requirements 4-6 use MUST. Requirements 1-3 and 8 use casual language. Ralph
treats MUST as a hard constraint and casual language as a suggestion.

**Fix:** Add MUST to each requirement's main verb.

---

### M4. Requirement 5 conflates three M2 test scenarios without clear mapping

**Affects:** PROMPT.md Requirement 5

The lifecycle test is claimed to cover three distinct M2 design sections (8.2, 8.7,
8.10) in a single fixture. But concurrent operations (8.7) requires verifying
poll-log cleanliness and no duplicate dispatches — the plan's `Expectations` struct
has no field for this. Knowledge propagation (8.10) is partially covered but missing
the team-level marker (H2).

**Fix:** Either split into separate requirements with distinct acceptance criteria,
or explicitly state that the lifecycle fixture covers all three aspects and explain
why (both agents running = concurrent ops; design doc markers = knowledge propagation).
Add poll-log verification to expectations if concurrent ops is covered here.

---

### M5. `src/tests/` is non-standard Rust project layout

**Affects:** plan.md Step 3, design.md Components

The plan places tests under `tests/hil/src/tests/lifecycle.rs`. In Rust, integration
tests go in `tests/` at the crate root (i.e., `tests/hil/tests/lifecycle.rs`), not
inside `src/`. Files under `src/` compile as library modules, not test binaries.

Steps 3 and 4 are also inconsistent — Step 3 uses `src/tests/lifecycle.rs` while
Step 4 uses `tests/design_rejection.rs`.

**Fix:** Use standard Rust layout: shared code in `src/lib.rs` + `src/mock_client.rs` +
`src/setup.rs`, integration tests in `tests/*.rs`.

---

### M6. Response loop matching ambiguous for multi-agent scenarios

**Affects:** plan.md Step 3 (scripted-human / run_scenario), design.md Components

The fixture response rules have `match` (regex), `reply`, `context`, and `times` —
but no `token` or `agent` field. Both agents send messages to tg-mock. A catch-all
`match: ".*"` fires on any message from either agent. The human-assistant sends HIL
questions; the architect sends status updates. Injecting "approved" into the architect's
chat in response to a status update is nonsensical.

In practice, only the human-assistant's messages require human replies (the architect
sends `human.interact` only for training mode confirmations). But the plan doesn't
clarify this.

**Fix:** Either add a `token` field to `ResponseRule` for per-agent scoping, or
document that the response loop only processes messages from specific tokens. The
plan and design should explain the multi-agent message routing strategy.

---

### M7. No diagnostic output on test failure

**Affects:** plan.md Steps 3-5

When a test fails after 20+ minutes, the developer needs: agent stdout/stderr,
tg-mock request history, issue file contents, poll-log.txt. The plan's assertions
are minimal (`assert!(result.is_ok())`). No discussion of capturing or surfacing
diagnostics.

**Fix:** Add a diagnostics section. At minimum: capture agent output to files,
dump tg-mock request log on failure, dump final issue status and poll-log.

---

### M8. `two-epics.yml` missing multi-epic assertions

**Affects:** plan.md Step 5

The fixture's expectations check `epic_status: "status/done"` for the primary epic
only. The `Expectations` struct has no field for verifying the second epic also
reached done. M2 design Section 8.8 requires: "Verify no data loss — both status
transitions are reflected in the final state."

**Fix:** Add `extra_epic_expectations` or a list-based assertions field.

---

### M9. No CI cost / API key discussion

**Affects:** plan.md, design.md

Each test launches real Ralph instances that invoke Claude. The full suite is
estimated at 2-3 hours of Claude API usage. The plan says "CI-ready" but does
not discuss: API key management in CI, cost per run, or strategies for reducing
cost (cheaper models, shared lifecycle runs).

**Fix:** Add a cost/CI section noting that `ANTHROPIC_API_KEY` must be available,
estimating cost per run, and suggesting model choice in test ralph.yml configs.

---

## LOW Findings

### L1. Stale lock content format may not match Ralph's expectations

**Affects:** plan.md Step 5 (stale-lock.yml)

The fixture uses `content: "crashed-agent:fake-loop-id 2026-01-01T00:00:00Z"`.
M2 design Section 8.9 uses `architect:loop-crashed 2026-02-16T10:00:00Z`. If
Ralph validates the role name in the lock file, `crashed-agent` may not be
recognized. Use a realistic role name like `architect`.

---

### L2. Fixture reuse requirement not explicitly addressed in plan

**Affects:** plan.md Step 2

The PROMPT has a regression criterion: "the test infrastructure MUST reuse
[existing fixtures], not duplicate." The plan's `setup_team_repo()` should
explicitly call `bash specs/milestone-2-architect-first-epic/fixtures/deploy.sh`.

---

### L3. Docker image tag not pinned

**Affects:** plan.md Step 1, design.md

Uses `ghcr.io/watzon/tg-mock` without a version tag. For reproducible tests,
pin to a specific tag or digest.

---

### L4. `test-server-down` not called on test failure

**Affects:** plan.md Step 2 (Justfile recipe)

```just
test-hil: test-server-up
    cd tests/hil && cargo test -- --test-threads=1
    just test-server-down
```

If `cargo test` fails, Just stops — `test-server-down` never runs. The container
stays running, causing port conflicts on next run.

**Fix:** Use: `cargo test ...; status=$?; just test-server-down; exit $status`
Or have `test-server-up` remove any existing container first.

---

### L5. M2 design Section 8.3 (Lock Contention) not covered

**Affects:** PROMPT.md Deviations section

Section 8.3 tests: "create a lock, verify agent skips the issue, delete lock,
verify agent picks it up." This is distinct from crash-recovery (8.9) which tests
stale lock cleanup. Neither the plan nor the PROMPT mentions 8.3. If intentionally
omitted, it should appear in PROMPT Deviations with rationale.

---

### L6. Key References includes absolute environment-specific path

**Affects:** PROMPT.md

`/opt/workspace/ralph-orchestrator/docs/guide/telegram.md` is environment-dependent.
Note this dependency or use a relative reference.

---

### L7. Requirement 4 partially duplicates design content

**Affects:** PROMPT.md

> "Each fixture MUST specify response rules (match pattern, reply text, optional
> `times` counter) and expected outcomes."

The parenthetical field list is design-level detail. Reference the design section
instead of enumerating fields.

---

## Summary

| ID | Finding | Severity |
|----|---------|----------|
| H1 | Incorrect M2 design scenario references | HIGH |
| H2 | Missing team-level knowledge marker | HIGH |
| H3 | api_url not plumbed through just launch | HIGH |
| H4 | No timing budget | HIGH |
| M1 | Requirement 1 prescribes steps | MEDIUM |
| M2 | Requirement 3 leaks implementation | MEDIUM |
| M3 | Missing RFC 2119 language | MEDIUM |
| M4 | Conflated scenarios in Requirement 5 | MEDIUM |
| M5 | Non-standard Rust test layout | MEDIUM |
| M6 | Response loop multi-agent ambiguity | MEDIUM |
| M7 | No failure diagnostics | MEDIUM |
| M8 | Missing multi-epic assertions | MEDIUM |
| M9 | No CI cost discussion | MEDIUM |
| L1 | Stale lock format mismatch | LOW |
| L2 | Fixture reuse not explicit | LOW |
| L3 | Docker image not pinned | LOW |
| L4 | Cleanup not called on failure | LOW |
| L5 | Section 8.3 not covered | LOW |
| L6 | Absolute path in references | LOW |
| L7 | Requirement 4 duplicates design | LOW |
