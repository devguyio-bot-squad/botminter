# Gastown vs. Our Toolkit — Deep Comparison

> Comparison of [steveyegge/gastown](https://github.com/steveyegge/gastown) against the end-game vision described in [rough-idea.md](../../master-plan/rough-idea.md) and [design.md](../../master-plan/design.md).
>
> **Source:** Gastown cloned and analyzed at `/tmp/gastown/` on 2026-02-15.

---

## What They Are

| | Gastown | Our Toolkit |
|--|---------|-------------|
| **Core identity** | Runtime orchestration platform (Go daemon + CLI) | Generator/metaprompting system (stamps out team repos) |
| **Deliverable** | A running system you operate via `gt` commands | Team definitions (prompts, configs, process docs) that any runtime can execute |
| **Runtime** | IS the runtime (manages sessions, heartbeats, worktrees) | Runtime is pluggable — Ralph today, could be anything tomorrow |

---

## Agent Model

| | Gastown | Our Toolkit |
|--|---------|-------------|
| **Role types** | **7 hardcoded in Go**: Mayor, Deacon, Boot, Dog, Witness, Refinery, Polecat/Crew. Cannot add new types without forking. | **Zero fixed roles.** Roles are defined entirely by profiles. A profile can define any roles it wants. |
| **Role specialization** | Workers (Polecats) are **generic** — any polecat can do any work. Specialization is in the formula, not the agent. | Agents are **deeply specialized** via prompts, knowledge, invariants. An architect agent thinks fundamentally differently than a QE agent. But this is a profile choice, not a toolkit constraint. |
| **Infrastructure vs domain** | Roles are **infrastructure** (supervisor, merge queue, health monitor). The domain work is generic "execute this formula step." | Roles are **domain-defined by profiles.** The infrastructure is the generator skeleton (Justfiles, coordination mechanisms). |
| **Extensibility** | Closed. Can't add "security-auditor" role without Go changes. Can only swap which AI runtime (Claude/Gemini/Codex) executes each fixed role. | Open. A new profile can define any role. `security-audit` profile with "scanner" and "analyst" roles works without toolkit changes. |

---

## Coordination

| | Gastown | Our Toolkit (end-game) |
|--|---------|-------------|
| **Work discovery** | Hook-based (pinned Bead = personal inbox) | Status-label-based (agents watch for `status/<role>:*`) |
| **Communication** | Mail system (typed messages: POLECAT_DONE, MERGE_READY, HELP, etc.) + Nudge (real-time tmux) | Issue comments + status transitions. End-game: real GitHub issue comments, PR reviews |
| **Message types** | Fixed in Go (7 protocol types). Can't define custom message semantics. | Profile-defined. The comment format, label conventions, and communication protocols are in PROCESS.md — fully customizable per profile. |
| **Workflow definition** | Formulas (TOML). 4 types: workflow (sequential DAG), convoy (parallel fan-out + synthesis), expansion (template-based), aspect (parallel analysis). DAGs only — **no loops, no conditional branching, no state machines.** Crash-recoverable via per-step bead persistence. | Hat event loops (YAML). Support persistent loops, conditional branching, idle/retry patterns. Profile-defined, not hardcoded. No crash recovery yet. |
| **Concurrency** | Built-in. Go daemon manages 5-30 concurrent polecats with worktree isolation. | Profile-defined coordination conventions + toolkit mechanisms (write-locks for POC, real GitHub for end-game). |

---

## Knowledge & Context

| | Gastown | Our Toolkit |
|--|---------|-------------|
| **Model** | **Single layer, session-injected.** `gt prime` injects context at session start via hooks. No persistent layered knowledge. | **Four-layer recursive scoping.** Team -> Project -> Member -> Member+Project. Persistent, git-portable, resolution-ordered. |
| **Persistence** | Beads (JSONL ledger) store work history. Context is ephemeral per session, rebuilt fresh. | Knowledge files persist in git. Memories persist in workspace. Both survive restarts and accumulate over time. |
| **Learning** | Agent CVs track work history and performance. But no mechanism for agents to contribute knowledge back to the system. | Explicit feedback loop: agents propose knowledge additions -> PO reviews -> placed at correct scope -> all future launches benefit. |
| **Scope management** | None. All context is flat — town-level CLAUDE.md + role template. | Governance by scope. Team-wide patterns live at team level. Project pitfalls at project level. Member-specific learnings at member level. PO decides scope placement. |

---

## Process & Methodology

| | Gastown | Our Toolkit |
|--|---------|-------------|
| **Process model** | **Fixed.** Polecat receives work -> executes -> self-cleans. Witness spawns/monitors. Refinery merges. No scrum, no kanban, no custom statuses. Issue states: open -> in_progress -> closed. | **Profile-defined.** `rh-scrum` defines kanban with 8+ statuses, epic/story lifecycles, rejection loops, design gates. A different profile could define something entirely different. |
| **Reusability** | One system. Every project gets the same 7 roles, same coordination model. | Three-layer generator. Skeleton is process-agnostic. Profile encodes a methodology. Instance is project-specific. Profiles are reusable across teams within a methodology. |
| **Process evolution** | Not supported. Process changes require Go code changes or formula edits. | Built-in. Team members propose process changes as PRs in the team repo. PO reviews and merges. Retrospectives refine the profile over time. |

---

## Quality & Confidence

| | Gastown | Our Toolkit (end-game, M4) |
|--|---------|-------------|
| **Quality model** | Merge queue (Refinery) runs tests. That's it. No design review, no architectural sign-off, no multi-stage quality pipeline. | Distributed eval framework. Hard-gate invariants + scored advisory evals. Quality = aggregate of independent assessments from multiple specialized agents. |
| **Attribution** | Strong. Every action has an actor. Agent CVs, work history, model A/B testing. | Planned (M4). Currently implicit. |
| **HIL graduation** | Not supported. Agents are always autonomous (polecats) or always human-controlled (crew). | Formal path: training -> supervised -> autonomous, informed by eval/confidence scores. Per-role, configurable. |

---

## What Gastown Has That We Don't

1. **Crash recovery** — Daemon -> Boot -> Deacon supervision chain. Three-tier watchdog. We have "human restarts manually."
2. **NDI (Nondeterministic Idempotence)** — "Any agent can continue any work." Work belongs to Beads, not agents. We tightly couple work to agent sessions.
3. **Merge queue** — Refinery handles test-before-merge, conflict resolution, retry logic. We defer to M5.
4. **Attribution** — Every action attributed to a specific agent. Enables CVs, model evaluation, audit trails. We plan this for M4 but don't have it yet.
5. **Scale** — Designed for 5-30 concurrent workers per project. We're at 2-5.
6. **Agent runtime flexibility** — Can run Claude, Gemini, Codex, Cursor, or custom agents. We're tied to Ralph/Claude.
7. **Durable per-task workflows** — Formulas encode entire work contracts (543-line polecat workflow with 10 steps, exit criteria, failure modes, variable substitution). Each step persists on completion. Our hat event loops are role-level behavior, not per-task contracts.
8. **Structured fan-out** — Convoy formulas spawn parallel analysts (6-leg design, 10-leg code review) with a synthesis step that combines outputs. We have no equivalent pattern yet.

---

## What We Have That Gastown Doesn't

1. **Generator model** — Stamp out entire team configurations. Gastown has zero automation for team setup — everything is manual config files.
2. **Profile-based methodology** — Process is a portable, shareable, versionable artifact. Gastown's process is hardcoded in Go.
3. **Role extensibility** — Any profile can define any roles. Gastown is locked to 7 types.
4. **Recursive knowledge scoping** — Four-level knowledge resolution with governance. Gastown has flat, session-injected context.
5. **Process evolution** — Teams can modify their own process through structured proposals. Gastown can't change its process without code changes.
6. **Deep role specialization** — Agents that embody domain expertise through prompts, knowledge, and invariants. Gastown's workers are generic.
7. **Workflows with loops/state machines** — Hat event loops support persistent polling, conditional branching, idle/retry. Gastown's formulas are DAGs only — no loops, no conditionals. (Though Gastown's formulas have stronger crash recovery via per-step bead persistence.)
8. **HIL graduation** — Formal path from training to autonomous with eval-based promotion.

---

## Deep Dive: Formulas & Molecules

Gastown's workflow system is its most sophisticated subsystem. Understanding it clarifies both what Gastown does well and where it hits structural limits.

### Lifecycle

```
Formula (TOML source) ──── "Ice-9" (template)
    │
    ▼  bd cook
Protomolecule (frozen) ──── Solid (ready to instantiate)
    │
    ├──▶ bd mol pour ──▶ Molecule (persistent, synced) ──▶ bd squash ──▶ Digest
    │
    └──▶ bd mol wisp ──▶ Wisp (ephemeral, not synced) ──┬▶ bd squash ──▶ Digest
                                                        └▶ bd burn ──▶ (gone)
```

Formulas are source templates (like source code). They get "cooked" into protomolecules (frozen templates), then "poured" into molecules (active workflow instances) or wisps (ephemeral instances for patrol loops). Completed molecules are "squashed" into digests (summaries). Wisps can be burned (discarded) since they have no audit value.

### Four Formula Types

| Type | Structure | Use Case | Example |
|------|-----------|----------|---------|
| **workflow** | Sequential steps with `needs` dependencies (DAG) | Multi-step work: implement, test, review, submit | `mol-polecat-work` (10 steps, 543 lines) |
| **convoy** | Parallel legs + synthesis step | Fan-out analysis: multiple agents explore in parallel, one synthesizes | `design` (6 legs: api, data, ux, scale, security, integration) |
| **expansion** | Template-based step generation | Dynamic step creation from templates | — |
| **aspect** | Multi-aspect parallel analysis (like convoy for analysis) | Parallel assessments | — |

### Workflow Formulas (Sequential DAGs)

The core formula type. Steps declare dependencies via `needs`, forming a DAG. The parser validates unique IDs, resolves references, detects cycles (DFS), and supports topological sort (Kahn's algorithm) for execution ordering. Steps can be marked `parallel = true` to run concurrently when they share the same `needs`.

**The canonical polecat workflow** (`mol-polecat-work`, 10 steps):
```
load-context → branch-setup → preflight-tests → implement → self-review
→ run-tests → commit-changes → cleanup-workspace → prepare-for-review → submit-and-exit
```

Each step has detailed prose instructions, variable substitution (`{{issue}}`, `{{base_branch}}`), and exit criteria. The formula encodes the *entire* polecat work contract — from "prime your environment" to "nuke your sandbox and exit."

**Key design decisions in the polecat formula:**
- Self-cleaning model: polecats submit work to a merge queue, then destroy themselves. "Done means gone."
- Pre-flight checks: verify base branch health *before* implementing (don't walk past a broken warp core, but don't fix it either — file a bead and proceed)
- Crash recovery: "A polecat can crash after any step and resume from the last completed step"
- No direct push to main — Refinery merges from the merge queue

### Convoy Formulas (Parallel Fan-Out + Synthesis)

Multiple agents work in parallel on different dimensions of a problem, then a synthesis step combines their outputs. Each leg gets its own polecat.

**Design convoy** (6 parallel legs + synthesis):
- **api** — interface design, CLI ergonomics
- **data** — data model, storage, schema
- **ux** — user experience, discoverability
- **scale** — performance at scale, bottlenecks
- **security** — threat model, attack surface
- **integration** — system fit, compatibility
- **synthesis** — combines all 6 analyses into a unified design doc (depends on all legs)

**Code review convoy** (10 parallel legs + synthesis + presets):
- 7 analysis legs: correctness, performance, security, elegance, resilience, style, smells
- 3 verification legs: wiring (deps added but not used), commit-discipline, test-quality
- **Presets** for selective execution: `gate` (4 legs, fast), `full` (all 10), `security-focused` (4 legs), `refactor` (4 legs)

The convoy model is powerful for structured analysis — it forces multi-perspective evaluation with a synthesis step that identifies conflicts between dimensions.

### Wisps (Ephemeral Molecules)

Wisps are molecules that never sync to git. Used for operational patrol loops that repeat continuously and have no audit value.

**Witness patrol wisp** (`mol-witness-patrol`, 9 steps):
```
inbox-check → process-cleanups → check-refinery → survey-workers
→ check-timer-gates → check-swarm → patrol-cleanup → context-check → loop-or-exit
```

The `loop-or-exit` step either creates a new wisp and starts over (if context is low) or exits for daemon respawn (if context is high). This is the closest Gastown gets to a loop — but it's manual: the last step explicitly squashes the current wisp, creates a new one, and continues from step 1. The loop is *in the step instructions*, not in the formula structure.

Includes cost-saving backoff: `await-signal` with exponential backoff (30s base, 2x multiplier, 5m cap) when no activity is detected.

### Molecule Persistence & Recovery

Each step completion is persisted as a bead (immutable git-tracked record). On crash, a new agent session resumes from the last completed step — this is the NDI (Nondeterministic Idempotence) guarantee. Work belongs to the molecule, not the agent session.

Two instantiation formats:
1. **Old format**: Steps embedded as markdown in the molecule description. Parsed via regex (`## Step: <ref>`, `Needs:`, `Tier:`, `Type:`, `Backoff:`).
2. **New format**: Steps are child issues of the molecule. Dependencies wired via issue-level `DependsOn`.

Variable expansion uses `{{variable}}` templates resolved at instantiation time from context maps.

### Structural Limitations

1. **No loops in the formula structure** — Formulas are DAGs (validated by cycle detection). The witness patrol "loops" by having its last step manually create a new wisp. This is a workaround, not a language feature.
2. **No conditional branching** — No if/else, no skip-step-if, no error routing. Steps execute linearly along the DAG. The only "branching" is in the prose instructions ("if X, do Y").
3. **No state machines** — No named states, no transitions, no event-driven flow. Status is just step completion tracking (open → in_progress → closed).
4. **No cross-molecule coordination** — Molecules are independent. The `WaitsFor: all-children` mechanism exists but is limited to parent-child relationships within bonded molecules.
5. **Fixed to 7 agent types** — Formulas can define *what* work to do, but *who* does it is hardcoded (polecats do work, Witness patrols, Refinery merges). You can't define a formula that creates a new role type.

### Comparison: Formulas vs. Hat Event Loops

| | Gastown Formulas | Our Hat Event Loops |
|--|-----------------|---------------------|
| **Structure** | DAGs (no cycles allowed) | Persistent loops with idle/retry |
| **Branching** | None — linear DAG only | Conditional branching on events |
| **State model** | Step completion tracking | Named statuses with transitions |
| **Looping** | Manual workaround (wisp squash + recreate) | Native — hats poll in a loop |
| **Scope** | Per-work-item (one molecule per task) | Per-role (hat defines ongoing behavior) |
| **Definition** | TOML (parsed by Go, validated at compile time) | YAML + prose (interpreted by LLM at runtime) |
| **Durability** | Strong — each step persisted as a bead, crash-recoverable | Weak — coupled to agent session, lost on crash |
| **Fan-out** | Native (convoy type, parallel legs with synthesis) | Not yet designed |

Formulas excel at *structured, durable, per-task workflows* with crash recovery. Hat event loops excel at *ongoing role behavior* with conditional logic and persistent polling. They solve different problems.

---

## The Fundamental Difference

**Gastown** solves: "How do I run 30 AI workers simultaneously without them stepping on each other?"

**Our toolkit** solves: "How do I define, package, and reproduce an entire team's process, roles, knowledge, and quality standards as a portable, evolvable artifact?"

They are complementary layers. Gastown could theoretically be a runtime backend for teams generated by our toolkit — it handles the session plumbing, we handle the team definition.

---

## Ideas Worth Borrowing

| From Gastown | Applicability |
|-------------|---------------|
| Supervision chain (Daemon -> Boot -> Deacon) | Could inform our launcher evolution — automated restart, health checks |
| NDI (any agent can continue any work) | Could inform how we handle agent session crashes — decouple work from session |
| Attribution / agent CVs | Should inform M4 eval system design — track who did what, performance over time |
| Merge queue (Refinery) | Could inform M5 GitHub integration — automated test-before-merge |
| Beads as durable work ledger | Could inform how we evolve `.github-sim/` — structured, immutable work records |
| Convoy fan-out + synthesis | Powerful pattern for design and code review — multiple parallel analysts with structured synthesis. Could inform how our architect/reviewer roles handle multi-faceted analysis |
| Per-step crash recovery | Formulas persist each step completion as a bead, so agents resume from last completed step after crash. Our hat event loops currently lose all state on crash — worth solving |
| Exponential backoff for patrol loops | Witness patrol uses `await-signal` with 30s/2x/5m backoff to reduce cost during idle periods. Our polling hats (board_scanner) should adopt similar cost-saving patterns |
| Review presets (gate/full/custom) | Code review convoy with configurable leg selection (4-leg gate for fast checks, 10-leg full for major features). Could inform how our reviewer role scales review depth |
| Self-cleaning worker model | Polecats submit to merge queue then destroy themselves — clean separation of work lifecycle from merge lifecycle. Could inform how our dev role hands off to reviewer |
