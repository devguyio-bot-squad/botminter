# Event Routing in Ralph Orchestrator

## How the Event Graph Works

Ralph hats communicate through events. Each hat declares:
- **triggers**: events it listens for (input)
- **publishes**: events it can emit (output)

When a hat publishes an event, Ralph routes it to the hat whose
triggers include that event. This creates a directed graph.

## Branching

A hat can publish one of several events depending on its outcome.
For example, `dev_implement-review` publishes one of:
- `dev.implement.red` (re-dispatch to red/test-writing phase)
- `dev.implement.green` (re-dispatch to green/implementation phase)
- `dev.implement.refactor` (re-dispatch to refactor/cleanup phase)
- `dev.implement.failed` (terminal failure)

These are routing targets, not verdicts. When the reviewer rejects work,
it re-dispatches to the specific TDD phase that needs rework.

The `ro.sh chain` command shows all possible branches as a tree.
At runtime, only one branch is taken per dispatch.

## Cycles

Review hats create intentional cycles. The review hat can reject
work and re-dispatch to an earlier phase (red → green → refactor →
review → red again). This is by design — TDD cycles iterate until
quality passes.

The ro-loop skill detects cycles via dispatch count: if the same
hat is dispatched 3+ times, the loop stops and reports a likely
stuck rejection cycle to the human.

## Fan-Out

Two hats can run in parallel when they are **independent** — neither
publishes an event that the other triggers on (directly or transitively).

Use `ro.sh deps <hat1> <hat2>` to check. The command reports:
- `independent` — safe to fan out
- `sequential: <reason>` — must run in order, with the dependency chain

**File-scope consideration:** Even if two hats are event-independent,
they may conflict if they modify the same files. The orchestrator
(the LLM running ro-loop) should also consider file scope when
deciding to parallelize. The `deps` command checks event dependencies
only — file-scope judgment is left to the orchestrator.

## Terminal Events

Events with no handler are terminal. Common patterns:
- `.failed` events (e.g., `dev.implement.failed`) — no hat handles
  these; the board-scanner's 3-strike escalation applies
- Human gate transitions — the hat sets a `human:*` status, which
  requires human response before the pipeline can continue

## Relationship to ralph run

In autonomous mode (`ralph run`), Ralph's native event loop handles
all routing. The `ralph emit` command publishes events, and the
HatRegistry resolves them to hats. The ro-loop skill is NOT used
in autonomous mode.

In interactive mode (this skill), the human supervises the loop.
The LLM acts as the event loop, using `ro.sh` to resolve routing
and spawning sub-agents for each hat. The human can intervene at
any point — approving, rejecting, or redirecting the pipeline.
