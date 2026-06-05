---
name: ro-loop
description: >-
  Ralph Orchestrator interactive loop — dispatches hats from ralph.yml
  as sub-agents in human-supervised sessions. Use when asked to "process
  an issue", "dispatch hats", "run the hat pipeline", "orchestrate
  issue N", or during bm chat sessions that need hat-based workflow.
  Parses ralph.yml via yq scripts — never reads the YAML directly.
compatibility: Requires yq v4+ and jq. For interactive/chat sessions, not autonomous ralph run.
metadata:
  author: botminter
  version: 1.0.0
  category: orchestration
  tags: [ralph, orchestrator, hats, events, dispatch]
---

# ro-loop

Interactive Ralph Orchestrator loop. Dispatches hats from `ralph.yml` as
sub-agents, routing events between them until the pipeline completes or
hits a human gate.

**Scope:** This skill is for human-supervised sessions (e.g., `bm chat`,
interactive Claude Code). It does NOT replace Ralph's native `ralph run`
event loop which handles autonomous operation.

## Dispatch-First Principle (RFC 2119)

The key words "MUST", "MUST NOT", "SHALL", "SHALL NOT", and "SHOULD"
in this document are to be interpreted as described in RFC 2119.

The orchestrator is a **router**, not a researcher.

1. You MUST resolve the issue's project board status to a hat and
   dispatch immediately.
2. You MUST NOT use issue body, comments, or PR state to determine
   routing. Routing MUST be determined solely by project board status.
3. You MUST NOT investigate, explore, or gather context before
   dispatching — that is the hat's job.
4. You SHALL only investigate routing when the status is unknown,
   absent, or does not resolve to a hat via `resolve-status`.
5. You MUST extract hat instructions using `ro.sh hat <id> instructions`,
   not from memory or prior conversation context.

The orchestrator still reads minimal issue metadata (number, project)
to pass context to the hat sub-agent — only the **routing decision**
is constrained.

## Instructions

### Step 1: Orient

Understand the routing topology and session guardrails.

```bash
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh graph
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh config
```

The graph shows every hat's triggers, publishes, and downstream hats.
The config provides guardrails, skill dirs, and event loop settings.
Cache both — they don't change within a session.

### Step 2: Resolve Entry Point

Determine the hat to dispatch. There are three entry paths — use
whichever matches the information you have. In ALL cases, proceed
IMMEDIATELY to Step 3 after resolving the hat.

**A. User provides status directly** (e.g., "it's at eng:dev:implement"):

```bash
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh resolve-status <status>
```

**B. User provides issue number only** (e.g., "process issue #67"):

Load the `github-project` skill and use its **query-issues** operation
with `--type project-status --issue <N>` to get the board status. Then:

```bash
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh resolve-status <status>
```

**C. User provides event directly** (e.g., "dispatch dev.implement"):

```bash
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh resolve <event>
```

If no hat handles the status or event, report to the human and stop.

### Step 3: Dispatch Hat

Get the hat's instructions and spawn a sub-agent.

```bash
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh hat <id> instructions
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh hat <id> publishes
```

Spawn a sub-agent using the Agent tool with this template:

```text
You are the `{hat_name}` hat.

{hat_instructions}

## Context
- Issue: #{issue_number}
- Project: {project}
- Working directory: {cwd}

## Guardrails
999. {guardrail_1_from_config}
1000. {guardrail_2_from_config}
...

## When you finish
Report what you did and which event to publish next.
You may ONLY publish one of: {publishes_list}
If you cannot complete your work, publish the .failed event.

## Constraints
- Do NOT read ralph.yml directly
- Do NOT transition GitHub status or post comments unless
  your hat instructions explicitly say to
```

Guardrails are extracted from `ro.sh config | jq '.core.guardrails'` and
formatted as numbered items starting at 999.

**You are an orchestrator, not a participant.** You MUST NOT pre-read
artifacts, interpret feedback, inject your own analysis, or add
context beyond what the template above specifies. The hat's instructions
tell it how to gather its own context — that is the hat's job, not yours.

### Step 4: Route Next Event

After the sub-agent completes:

1. Extract the published event from the sub-agent's response
2. Validate it against the hat's publishes list — reject invalid events
3. Run `ro.sh resolve <event>` to find the next hat
4. If the event has no handler, the pipeline has terminated

**Do NOT interpret the sub-agent's work product.** Your job is to extract
the published event and route it. Do not summarize findings, analyze
feedback, or carry context from one hat into the next hat's dispatch.

**Fan-out check:** If you have multiple independent hats ready to
dispatch (e.g., from separate branches), check before parallelizing:

```bash
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh deps <hat1> <hat2>
```

- `independent` — safe to dispatch in parallel (use multiple Agent calls
  in one message)
- `sequential: <reason>` — must dispatch in order

### Step 5: Repeat or Terminate

Continue the dispatch-route cycle until one of:

- **Terminal event:** No hat handles the published event (e.g., `.failed`
  events with no handler, or events that map to sentinel/human gates)
- **Human gate:** Hat transitions issue to a `human:*` status — stop
  and report to the human
- **Circuit breaker:** 50 hat dispatches in a single session — stop and
  report. This prevents runaway loops from review→re-dispatch cycles
- **Cycle limit:** Same hat dispatched 3+ times — likely a stuck
  rejection loop. Stop and report

## Diagnostic Commands

These commands help debug routing issues without dispatching anything:

```bash
# What hats exist?
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh hats

# What's the full path from an event?
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh chain <event>

# Can these hats run in parallel?
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh deps <hat1> <hat2>
```

## Error Handling

| Situation | Action |
|-----------|--------|
| Sub-agent doesn't report an event | Prompt it: "Which event from {publishes_list} do you publish?" |
| Sub-agent reports invalid event | Reject, show valid options, ask again |
| Sub-agent fails 3 times on same hat | Stop loop, report failure to human |
| Cycle detected (hat dispatched 3+ times) | Stop loop, report stuck rejection cycle |
| `ro.sh resolve` exits 1 | Event is terminal — pipeline complete or needs human |

## Examples

### Example 1: User gives status — dispatch immediately

User says: "process issue #67, it's at human:po:plan-review"

```text
1. ro.sh resolve-status human:po:plan-review → po_gate
   (NO reading issue body, comments, or PR state)
2. ro.sh hat po_gate instructions → extract hat instructions
3. Dispatch po_gate sub-agent with hat instructions + context
4. Sub-agent completes → transitions status → no event published
5. Pipeline continues from new status...
```

### Example 2: User gives issue number only — query status first

User says: "process issue #42"

```text
1. query-issues.sh --type project-status --issue 42
   → {"number":42, "title":"...", "status":"eng:dev:implement", ...}
2. ro.sh resolve-status eng:dev:implement → dev_implement-plan
3. Dispatch dev_implement-plan sub-agent immediately
4. Continue TDD cycle from published events...
```

### Example 3: Mid-pipeline event routing

After a hat publishes `dev.implement.red`:

```text
1. ro.sh resolve dev.implement.red → dev_implement-red
2. Dispatch dev_implement-red sub-agent immediately
3. Continue from published events...
```

### Example 4: Inspect the routing graph

User says: "show me how dev events route"

```text
1. ro.sh chain dev.implement
   → shows tree: plan → red → green → refactor → review → (branches
     back to red/green/refactor for rejection, or terminates)
```

## Troubleshooting

### yq not found

```bash
curl -L https://github.com/mikefarah/yq/releases/download/v4.53.2/yq_linux_amd64.tar.gz \
  | tar xz && mv yq_linux_amd64 /usr/local/bin/yq
```

### Hat not found for event

Run `ro.sh graph` to see all routes. Common causes:
- Event is terminal (`.failed` events have no handler by design)
- Event needs board-scanner dispatch first (status-to-event mapping)
- Typo in event name — check against `ro.sh hat <id> publishes`

### ralph.yml not found

Set `RALPH_FILE` env var or use `--file <path>`:

```bash
bash ${CLAUDE_SKILL_DIR}/scripts/ro.sh --file /path/to/ralph.yml hats
```
