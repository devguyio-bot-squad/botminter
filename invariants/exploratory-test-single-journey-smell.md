# Exploratory Test Single Journey Smell

A single happy path per exploratory test phase is a smell. Every phase MUST cover multiple distinct user journeys.

## Rule

Every exploratory test phase MUST enumerate and test multiple distinct user journeys — not one golden flow with edge cases as afterthoughts.

A "user journey" is any complete path a real user might take through the feature. These are NOT edge cases — they are first-class scenarios. The same principle applies to any feature — bridge provisioning, workspace sync, brain autonomy, or anything else. For illustration, consider autonomous agent interaction, where distinct journeys include:

- User asks a single question and gets a response
- User has a multi-turn conversation (question → response → follow-up → response)
- User interacts through a different member (cross-member messaging)
- User comes back after a restart and resumes interaction
- User sends malformed or unexpected input
- Two users interact with the same member concurrently
- User sends a message while the bridge is recovering

Each of these is a separate happy path, not a variation of one. Any feature with comparable richness — bridge lifecycle, workspace provisioning, member coordination — will have its own equivalent set of distinct journeys.

**The smell test:** If a phase has exactly one integrated end-to-end test, it is almost certainly incomplete. Ask: "What other ways would a real user interact with this feature?" If the answer is more than zero, the phase needs more journeys.

A phase MUST NOT be considered complete until:
1. The distinct user journeys are enumerated in PLAN.md
2. Each journey has at least one integrated test in the Justfile
3. The journeys span interaction variations, lifecycle variations, error handling, and recovery

## Applies To

- All exploratory test phases in `crates/bm/tests/exploratory/`
- Phase planning (PLAN.md) — journeys must be enumerated before test cases are written
- Phase review — reviewers MUST reject phases with a single integrated journey

Does **NOT** apply to:

- Unit tests — isolation is the point
- Integration tests — module boundaries, not user journeys
- E2E tests — governed by `e2e-scenario-coverage.md`
- Phases that test genuinely single-path operations (e.g., `bm init` has one journey: run the wizard). Use judgment — but autonomous interaction, bridge messaging, and lifecycle management are never single-path.

## Examples

This invariant applies to any feature in any phase. The following examples illustrate the pattern — substitute your own feature and subsystems.

**EXAMPLE 1 — Compliant:** If a phase tests a feature where users interact with autonomous members via a chat bridge, the plan would enumerate distinct journeys upfront and each would have integrated tests:
```
## User Journeys
1. Single question: start → send question → get response → stop
2. Multi-turn: start → question → response → follow-up → response → stop
3. Cross-member: start → alice messages → bob sees it → bob replies → alice sees reply → stop
4. Recovery: start → stop → start → send message → get response (works after restart)
5. Malformed input: start → send garbage → survives → send valid → get response → stop
6. Bridge failure: start → send message → kill bridge → restart bridge → send message → get response

## Test Cases
X1: ... (implements journey 1)
X2-X4: ... (implements journey 2)
...
```

**EXAMPLE 2 — Violating:** If a phase tests the same feature but has a single integrated journey buried among 15 subsystem tests, it violates this invariant — even though the test count looks thorough. A reviewer sees 15 tests and approves, but only one tests what the user actually experiences:
```
## Tests
X1-X5: Chat API messaging (send, receive, history, multi-user)
X6-X10: Member lifecycle (start, PID check, stop, restart)
X11: Send message while member running → get response    ← the only integrated journey
X12-X15: Internal state edge cases (files, config, logs)
```

**EXAMPLE 3 — Violating:** If a phase enumerates journeys in the plan but only implements one as an integrated flow, it violates this invariant. Listing journeys without testing them end-to-end is not compliance:
```
## User Journeys
1. Single question
2. Multi-turn conversation
3. Cross-member interaction

## Tests
X1: start → send message → get response → stop          ← journey 1: integrated
X2: send 3 messages to room (no member running)          ← journey 2: NOT integrated
X3: alice sends, bob polls history                        ← journey 3: NOT integrated
```

**EXAMPLE 4 — Compliant:** If a phase tests workspace sync (a different feature), the same rule applies. Distinct journeys would include: first sync, re-sync after file deletion, re-sync after member addition, sync with stale workspace marker, sync with bridge changes. Each needs an integrated test through the user path (`bm teams sync` → verify observable outcome), not just filesystem checks.

## Rationale

This is the most recurring review failure in the project. Builders write one integrated happy path test, then fill remaining test cases with subsystem-level checks (API works, process starts, files can be written). Reviewers approve because "the happy path is covered" and the test count looks thorough. But real users don't follow one path — they ask questions, have conversations, come back after restarts, send unexpected input, interact through different members. Treating these as "edge cases" or "variations" instead of first-class journeys leaves the feature's actual value proposition untested. This invariant makes the pattern a hard reject — for any feature, not just chat.
