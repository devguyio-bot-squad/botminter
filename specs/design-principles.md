# Design Principles for Member Skeletons and Sprint PROMPTs

> Learnings from M1.5 spike, M2 design, M2 design review, and M2 sprint execution.
> Reference when building or reviewing member skeletons (`ralph.yml`, `PROMPT.md`,
> `CLAUDE.md`, hat instructions) and sprint-level PROMPT.md files.

---

## 1. Separation of Concerns: PROMPT.md vs CLAUDE.md vs Hat Instructions

| Layer | Purpose | Examples |
|-------|---------|---------|
| **PROMPT.md** | Role identity + cross-hat behavioral rules | "You are the architect", write-lock protocol, sync-before-scan/push-after-write, training mode declaration |
| **CLAUDE.md** | Role context — workspace model, codebase access, team references, invariant file locations | Fork chain, CWD model, pointers to `.botminter/CLAUDE.md` and `PROCESS.md`, `# INVARIANTS` section |
| **Hat instructions** (ralph.yml) | Operational details for each hat | Event publishing, self-clear, poll-log, `### Knowledge` paths, `### Backpressure` gates, dispatch logic |
| **`core.guardrails`** (ralph.yml) | Universal behavioral rules injected into every hat (numbered 999+) | Lock discipline, invariant compliance, cross-cutting rules |

**Rules:**
- PROMPT.md and CLAUDE.md MUST NOT prompt about hats. Ralph handles hat prompting.
- Cross-hat concerns (apply to all hats) MUST be in PROMPT.md.
- Hat-specific concerns (apply to one hat) MUST be in that hat's `instructions:` block in ralph.yml.
- Knowledge paths MUST be in each hat's `### Knowledge` section — MUST NOT appear in PROMPT.md or CLAUDE.md. The agent references these as needed, not upfront.
- Generic invariants (file-based, team/project/member scope) MUST be in CLAUDE.md's `# INVARIANTS` section. Claude Code injects CLAUDE.md natively into every hat.
- Hat-specific quality gates MUST be `### Backpressure` in each hat's instructions. MUST NOT prescribe how — define gates that reject incomplete work.
- Universal guardrails MUST be in `core.guardrails` in ralph.yml. Ralph injects these into every hat prompt as `### GUARDRAILS`.

## 2. Validated Ralph Runtime Pattern (M1.5)

These behaviors were validated in the M1.5 spike and MUST be adopted in all member skeletons:

- ralph.yml MUST NOT set `starting_event` — hardcoding a starting event shortcuts the orchestrator hat. All designs MUST have an orchestrator hat (board scanner / coordinator) as the entry point. The orchestrator reads state (board, files) and dispatches to the correct work hat. Without it, the loop bypasses state-aware routing and breaks rejection loops, round-robin, and priority dispatch.
- `persistent: true` MUST be set — keeps the agent alive. LOOP_COMPLETE is suppressed; `task.resume` is injected automatically and routed by the coordinator back to `board.scan`.
- `task.resume` is reserved — MUST NOT appear in hat triggers. The coordinator handles it.
- The board scanner MUST publish LOOP_COMPLETE when no work is found.
- Every hat MUST have a `default_publishes`. Prevents fallback exhaustion. Board scanner defaults to `LOOP_COMPLETE`; work hats default to `LOOP_COMPLETE`.
- The board scanner MUST overwrite the scratchpad and delete `tasks.jsonl` at the start of every cycle. Prevents context pollution between issues.
- Hats MUST verify the issue is not already at the target output status before dispatching (idempotent dispatch). Prevents duplicate processing.
- Each event MUST appear in exactly one hat's `triggers` list. If the same event appears in multiple hats' triggers, routing is ambiguous and some hats become unreachable. This is a Ralph runtime constraint — not a convention, an error.
- Rejection routing MUST rely on the hatless Ralph orchestrator, NOT on hat subscriptions. When a review hat publishes a rejection event (e.g., `lead.rejected`) and no hat subscribes, the hatless Ralph orchestrator examines the context — it determines which work hat's output was rejected and routes directly back to that hat. For example: `arch_designer` → `lead_reviewer` → `lead.rejected` (unmatched) → hatless Ralph sees the reviewer rejected the designer's work → activates `arch_designer` directly. No board scanner re-scan needed for rejections. Work hats MUST NOT subscribe to rejection events when multiple work hats share the same reviewer.
- The board scanner MUST trigger ONLY on `board.scan`. It MUST NOT subscribe to approval or rejection events.
- `LOOP_COMPLETE` MUST NOT appear in a hat's `publishes` list in ralph.yml. It MUST only be referenced in the hat's `instructions` block (e.g., "Publish LOOP_COMPLETE"). The `default_publishes: LOOP_COMPLETE` safety net is separate — it is a Ralph runtime fallback, not a hat declaration.

### 2.1. Two Execution Models

| Model | `persistent` | Entry point | Exit condition | Restart responsibility |
|-------|-------------|-------------|---------------|----------------------|
| **Poll-based** | `true` | Board scanner on `board.scan` | Never (LOOP_COMPLETE → `task.resume` → `board.scan`) | Self (Ralph runtime) |
| **Event-triggered** | `false` | Board scanner on `board.scan` | LOOP_COMPLETE after no work found | External (daemon) |

**Poll-based (existing):** Board scanner runs in a loop. On LOOP_COMPLETE, Ralph injects `task.resume` which routes back to `board.scan`. The member stays alive indefinitely. `cooldown_delay_seconds` controls the scan interval.

**Event-triggered (new):** Board scanner runs once (or until no work remains). It MUST scan ALL matching issues — not just the first one. When no work is found, it publishes LOOP_COMPLETE and Ralph exits. The daemon handles the next restart.

The execution model is controlled by the `event_loop.persistent` field in `ralph.yml`:

```yaml
event_loop:
  persistent: false              # event-triggered mode
  max_iterations: 200            # safety limit per invocation
```

The board scanner MUST process ALL matching work items in priority order before publishing LOOP_COMPLETE. The key invariant: **the board scanner is always the universal entry point, regardless of execution model.**

## 3. Human-in-the-Loop (HIL)

- Hat instructions SHOULD say "present/report via `human.interact`" — MUST NOT write explicit two-pass logic. Ralph handles blocking and response delivery transparently. The next time the hat fires, the response will be in context.
- Training mode MUST be universal — all team members, not just the human-assistant. Like a TRACE log level.
- Training mode MUST be a PROMPT.md toggle — declared as `## !IMPORTANT — OPERATING MODE` with `TRAINING MODE: ENABLED`. Hat instructions MUST check this conditionally: `If TRAINING MODE is ENABLED (see PROMPT.md)`. Future: toggle to DISABLED or AUTONOMOUS.
- Training mode ⊃ poll-log — training mode and poll-log are two observability knobs. Training mode is a superset that includes human reporting on top of operational logging. Poll-log MUST always run; training mode adds `human.interact` confirmation when enabled.

## 4. Operational Logging

- Poll-log MUST NOT be gated by training mode. It's operational logging for debugging, valuable in all modes.
- Poll-log MUST live at workspace root (`poll-log.txt`) and MUST be gitignored via `.git/info/exclude`.

## 5. Cooldown

- ralph.yml MUST NOT include `cooldown_delay_seconds`. Agent processing time provides natural throttling. No artificial delay between phase transitions.

## 6. Workspace Model

- Agent CWD MUST be the project repo. Team repo MUST be cloned into `.botminter/` inside it.
- All roles MUST use the same model — including non-code-working roles (human-assistant). Consistency over efficiency for tooling simplicity.
- `create-workspace` MUST write a `.botminter/.member` marker, read by `just sync` to identify which member the workspace belongs to.
- PROMPT.md and CLAUDE.md MUST be symlinks (auto-update on pull).
- ralph.yml and settings.local.json MUST be copies (require `just sync` + restart).
- Skills MUST be read directly via `skills.dirs` from `.botminter/` paths — no assembly needed.
- Agents MUST be symlinked into `.claude/agents/` from all `agent/agents/` layers.
- `.botminter/` MUST be excluded via both `.git/info/exclude` (local) and `.gitignore` (project-level). `just sync` MUST verify and repair `.git/info/exclude` if missing.
- Nested Ralph is a dev-only concern — when Ralph develops botminter and launches a team member's Ralph (Claude-inside-Claude), `CLAUDECODE` env var MUST be unset. This goes in the generator repo's `just dev-launch`, MUST NOT be in the production `just launch`.

## 7. Knowledge, Invariants, and Backpressure

Three-tier model for guiding and constraining agent behavior:

| Tier | Mechanism | Scope | Where configured |
|------|-----------|-------|------------------|
| **Knowledge** | `### Knowledge` in hat instructions | Per hat | Hat's `instructions:` block lists directories. Agent consults as needed — never reads all upfront. |
| **Invariants** | `# INVARIANTS` in CLAUDE.md | All hats | CLAUDE.md points to invariant files at team, project, and member scopes. Claude Code injects natively. |
| **Backpressure** | `### Backpressure` in hat instructions | Per hat | Quality gates that must pass before the hat can transition status. Don't prescribe how — define what success looks like. |
| **Guardrails** | `core.guardrails` in ralph.yml | All hats | Universal rules injected as numbered `### GUARDRAILS` (999+) by Ralph into every hat prompt. |

**Rules:**
- Knowledge MUST be lazy — list the directories, let the agent decide what's relevant. MUST NOT force upfront reads.
- Invariants are deep context — file-based, detailed, accumulated across scopes. Suitable for complex policies.
- Backpressure MUST be per-hat gates — short, verifiable conditions that block status transitions. MUST NOT prescribe how — define gates that reject incomplete work.
- Guardrails MUST be universal rules — behavioral constraints that apply regardless of which hat is active. MUST be short and actionable.

## 8. Reference Hat Examples

These examples show the canonical hat structure. All hats follow the same pattern.

### 8a. Work hat with knowledge + backpressure (Designer)

````yaml
designer:
  name: Designer
  description: Produces a design doc for an epic in status/arch:design.
  triggers:
    - arch.design
  default_publishes: LOOP_COMPLETE
  instructions: |
    ## Designer

    You are the architect's designer hat. Produce a design document for the epic.

    ### Knowledge

    Reference these when producing designs — consult as needed, not all upfront:
    - Team: `.botminter/knowledge/`
    - Project: `.botminter/projects/<project>/knowledge/`
    - Member: `.botminter/team/architect/knowledge/`
    - Hat: `.botminter/team/architect/hats/designer/knowledge/`

    ### Workflow:

    1. Read the epic issue from `.botminter/.github-sim/issues/<number>.md`.
    2. Consult relevant knowledge as needed for the design.
    3. **If TRAINING MODE is ENABLED** (see PROMPT.md): Report via `human.interact` and wait.
    4. Acquire the write-lock on the issue.
    5. Produce the design doc at `.botminter/projects/<project>/knowledge/designs/epic-<number>.md`.
    6. Append a comment to the epic linking to the design doc.
    7. Update status: `status/arch:design` → `status/po:design-review`.
    8. Commit, push, release the lock.
    9. Publish LOOP_COMPLETE.

    ### Backpressure

    Before transitioning to `status/po:design-review`, verify:
    - Design doc has a Security Considerations section
    - Design doc has acceptance criteria (Given-When-Then)
    - Design doc references applicable project knowledge

    ### On Failure
    Append a comment: `Processing failed: <reason>. Attempt N/3.`
    Publish LOOP_COMPLETE.
````

### 8b. Board scanner hat (dispatch + self-clear)

````yaml
board_scanner:
  name: Board Scanner
  description: Scans for status/arch:* labels, dispatches to appropriate hat.
  triggers:
    - board.scan
  publishes:
    - arch.design
    - arch.plan
    - arch.breakdown
  default_publishes: LOOP_COMPLETE
  instructions: |
    ## Board Scanner

    ### Self-clear
    Overwrite scratchpad. Delete tasks.jsonl.

    ### Sync
    Run `just -f .botminter/Justfile sync`.

    ### Scan
    Read `.botminter/.github-sim/issues/`. Find issues with `status/arch:*` labels.
    Priority: `arch:breakdown > arch:plan > arch:design > arch:in-progress`.

    ### Dispatch
    For the highest-priority issue found:
    - Verify issue is not already at the target output status (idempotent dispatch).
    - Log to poll-log.txt: `[ISO-timestamp] Board scan: found #N at status/arch:<phase>`.
    - Publish the matching event (e.g., `arch.design`).

    If no work found: publish `LOOP_COMPLETE`.
````

### 8c. Work hat without knowledge (Breakdown Executor)

````yaml
breakdown_executor:
  name: Breakdown Executor
  description: Creates story issues from the approved breakdown.
  triggers:
    - arch.breakdown
  default_publishes: LOOP_COMPLETE
  instructions: |
    ## Breakdown Executor

    Create story issues from the PO-approved breakdown.

    ### Workflow:

    1. Read the epic issue and the approved breakdown comment.
    2. **If TRAINING MODE is ENABLED** (see PROMPT.md): Report via `human.interact` and wait.
    3. Acquire the write-lock on the epic.
    4. For each story: create `.botminter/.github-sim/issues/<number>.md` with
       `kind/story`, `status/dev:ready`, `parent: <epic>`.
    5. Append a comment to the epic listing created story numbers.
    6. Update status: `status/arch:breakdown` → `status/po:ready`.
    7. Commit all in one commit, push, release the lock.
    8. Publish LOOP_COMPLETE.

    ### Backpressure

    Before transitioning to `status/po:ready`, verify:
    - Each story issue has acceptance criteria in Given-When-Then format
    - Each story has `kind/story`, `parent`, and `status/dev:ready` labels
    - Epic comment lists all created story numbers

    ### On Failure
    Append a comment: `Processing failed: <reason>. Attempt N/3.`
    Publish LOOP_COMPLETE.
````

**Key patterns to note:**
- `### Knowledge` appears only in hats that produce artifacts requiring context (designer, planner). Hats that work from already-approved content (breakdown_executor) don't need it.
- `### Backpressure` appears in every work hat. It defines verifiable conditions, not prescriptive steps.
- Board scanner has no `### Knowledge` or `### Backpressure` — it does self-clear, sync, scan, and dispatch.
- Board scanner MUST trigger ONLY on `board.scan`. All other events (approval, rejection) go unmatched and the hatless Ralph orchestrator restarts the cycle via `persistent: true`.
- Every hat has `default_publishes` and `### On Failure`.
- `LOOP_COMPLETE` MUST NOT appear in a hat's `publishes` list — only referenced in the hat's `instructions` block. `default_publishes: LOOP_COMPLETE` is the safety net.
- Work hats publish `LOOP_COMPLETE` via instructions when done — the persistent loop handles restarting via `board.scan`.

## 9. Write-Lock Protocol

- Only writes require locks — any agent MAY read at any time without a lock. Locks MUST protect writes.
- Each agent MUST watch a different status prefix (`arch:*` vs `po:*`). Two agents MUST NOT modify the same issue simultaneously.
- Agents MUST lock late — think first, then acquire lock → write → push → release. Minimize lock hold time.
- Push failures are self-healing — Git push conflicts are detected naturally. Claude recovers without explicit instructions. Hat instructions SHOULD NOT encode push-failure recovery.

## 10. Hat-Level Skills

- **Deferred to post-POC** — Ralph has the infrastructure for hat-scoped skill filtering (`hats:` frontmatter in SKILL.md, `SkillRegistry::is_visible()`, unit tests), but the EventLoop doesn't pass the active hat ID at runtime — it calls `build_index(None)`. All skills are visible to all hats regardless of frontmatter.
- **Hat-level knowledge directories remain** — these are instruction-driven (the hat tells the agent where to look), so they work without runtime filtering.
- **When Ralph wires up hat filtering** — add hat-level skill directories to member skeletons and `skills.dirs`.

## 11. Sprint PROMPT.md Design

Sprint PROMPTs are task-level prompts that drive Ralph to implement a sprint's worth of
work. They are distinct from member PROMPT.md files (Section 1), which define role identity
and cross-hat behavioral rules.

### Structure

Every sprint PROMPT follows the same skeleton:

```
# Sprint N: Title
## Objective          — 1-2 sentences. What, not how.
## Prerequisites      — What the prior sprint delivered.
## Deviations         — What's intentionally out of scope vs the design (if any).
## Key References     — Pointers to design.md, sprint plan, research, prior artifacts.
## Requirements       — Numbered list. WHAT changes, not HOW to change it.
## Acceptance Criteria — Given-When-Then. The quality gates Ralph must pass.
```

### Rules

- Objective MUST be a sentence, not an essay. Ralph reads the design doc for depth. The
  PROMPT just needs to orient.

- Requirements MUST say WHAT and reference WHERE. Each requirement is 1-3 lines stating
  what needs to change, then a pointer: "Per design.md Section X." MUST NOT duplicate the
  design content into the PROMPT — Ralph will read the referenced section between turns.
  The files on disk are the source of truth; the PROMPT points at them.

- MUST NOT prescribe hat instruction text. Say "add a training mode gate to all five
  hats per design.md Section 4.1.1." MUST NOT paste the instruction blocks into the PROMPT.
  Ralph reads the design doc between turns and writes the instructions itself.

- MUST NOT prescribe implementation steps. Say what the outcome MUST be. Ralph figures
  out the sequence. Acceptance criteria catch incomplete work. The sprint plan provides
  ordering guidance, but Ralph MAY deviate — the PROMPT's acceptance criteria are what
  matter, not the plan's step order.

- Acceptance criteria MUST be the quality gates. Given-When-Then format. These are the
  gates Ralph MUST pass. They MUST be observable and verifiable — if you can't check it by
  reading the resulting files or running the resulting code, it's not a useful gate.

- MUST use RFC 2119 language. Use MUST, MUST NOT, SHOULD, MAY consistently. Ralph treats
  these keywords with appropriate weight. "You MUST confirm with the human" is stronger
  and more reliably followed than "you should probably confirm with the human."

- Deviations from design MUST be explicit. When a sprint intentionally defers or changes
  something from the design, it MUST list rationale and which sprint picks it up. This
  prevents Ralph from implementing deferred work and gives the next sprint clear context.

- Sprints MUST chain via prerequisites. State what the prior sprint delivered. Each Ralph
  session starts with fresh context — no memory of prior sprints. Prerequisites are the
  handoff mechanism. Be explicit: "Sprint 2 complete. Both agents coordinate through the
  full lifecycle autonomously."

- Regression criteria MUST be labeled. If an acceptance criterion tests behavior from a
  prior sprint (not new in this sprint), prefix it with "(Regression)" so the scope is
  clear. This signals Ralph not to re-implement it, just verify it still works.

- Requirements MUST NOT say "add X" when X might partially exist. Requirements MUST
  acknowledge the current state when it's ambiguous. Say "Sprint 2 omitted these" or
  "update the existing recipe" rather than bare "add." Ralph searches the codebase, but
  an accurate requirement prevents wasted iterations discovering something already
  half-built.

### Anti-Patterns

| Anti-pattern | Fix |
|-------------|-----|
| Pasting design.md content into requirements | Reference the section: "Per design.md Section X" |
| Quoting exact hat instruction text to add | Describe the gate/behavior; Ralph writes the instructions |
| Specifying keyword-matching rules for `human.interact` | Describe the interaction pattern; Ralph handles parsing |
| Step-by-step implementation sequence in requirements | State the outcome; let the sprint plan handle ordering |
| Requirements without a design.md reference | Every non-trivial requirement MUST trace to a design section |
| Vague "add X" without stating current state | Acknowledge what exists: "Sprint 2 omitted these" or "update the existing recipe" |
| Casual language for hard constraints | MUST use MUST/MUST NOT for non-negotiable rules, SHOULD for recommendations |
| Setting `starting_event` in ralph.yml | MUST NOT set — all designs MUST route through an orchestrator hat |
| Same event in multiple hats' triggers | Each event MUST appear in exactly one hat's triggers — duplicates cause ambiguous routing |
| Work hats subscribing to shared rejection events | Rejection events from shared reviewers MUST go unmatched — the hatless Ralph orchestrator examines context and routes back to the originating work hat |
| `LOOP_COMPLETE` in a hat's `publishes` list | MUST NOT appear in `publishes` — only referenced in hat instructions. Use `default_publishes: LOOP_COMPLETE` as the safety net |

### Example: Good Requirement

```
4. **Architect hats** — add training mode conditional blocks to all five hats in
   `ralph.yml`. Sprint 2 omitted these (training mode was DISABLED). Each hat
   needs an "If TRAINING MODE is ENABLED" gate that reports intent via
   `human.interact` and waits for confirmation before any state-modifying action.
   Per design.md Section 4.1.1 for per-hat specifics.
```

Four lines: what changed (Sprint 2 omitted them), what's needed (a gate per hat),
how the gate works at a behavioral level (report intent, wait for confirmation), and
where to find the details (design.md Section 4.1.1). Ralph reads 4.1.1 and writes the
hat instructions — no need to spell them out in the PROMPT.

### Example: Bad Requirement

```
4. **Architect hats** — add the following to each hat:
   - board_scanner: Add step: "If TRAINING MODE is ENABLED (see PROMPT.md):
     Report board state to human via `human.interact`. Present detected epics
     and intended dispatch. Wait for confirmation. On approval: proceed with
     dispatch. On timeout: do NOT dispatch, publish `board.rescan`."
   - designer: Add step: "If TRAINING MODE is ENABLED: Report design intent
     to human via `human.interact`. Summarize epic and planned approach..."
   [... 20 more lines of quoted instruction text ...]
```

This prescribes the exact text Ralph should write. It's fragile (the quoted text may
not match the design doc), verbose, and prevents Ralph from adapting the instructions
to the actual codebase state. Ralph reads design.md and writes better instructions
than a PROMPT author can pre-write.

### Example: Good Deviations Section

```
## Sprint 2 Deviations from Design

These are intentional scope decisions. See sprint plan for full rationale.

- **Training mode: DISABLED** — no HIL channel. Agent acts autonomously. Re-enabled Sprint 3.
- **RObot: disabled** — no Telegram bots. Sprint 3.
- **Auto-advance gates** — backlog_manager and review_gater auto-advance all gates
  instead of waiting for human input. Sprint 3 restores HIL gates.
```

Each deviation states what's changed, why, and when it's addressed. This prevents
Ralph from implementing Telegram or HIL during Sprint 2, and gives Sprint 3 a clear
list of what needs to be un-deferred.
