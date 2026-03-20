# Comparative Analysis: BotMinter vs Symphony

## Scoped Context, Knowledge, Invariants, Roles & Hats

**Date:** 2026-03-18
**Scope:** Architectural comparison of agent orchestration approaches
**Projects:**
- **BotMinter** — Rust CLI (`bm`) for managing GitOps-style teams of coding agents (pre-alpha, Apache 2.0)
- **Symphony** — OpenAI's Elixir/OTP orchestrator that dispatches Linear issues to Codex agents (Apache 2.0)

---

## 1. Executive Summary

Symphony and BotMinter both orchestrate autonomous coding agents, but they occupy fundamentally different positions in the design space. Symphony is a **dispatch engine** — it efficiently farms Linear issues to identical Codex instances with retries, concurrency control, and observability. BotMinter is a **team simulation framework** — it models how a real software team works, with specialized roles, institutional knowledge, constitutional constraints, and conventions that accumulate across organizational boundaries.

They answer different questions:

- **Symphony:** *"How do I run many agents efficiently?"*
- **BotMinter:** *"How do I make agents work together like a high-functioning team?"*

This report examines the architectural differences across five dimensions where BotMinter's design is most distinct: scoped context, knowledge, invariants, roles, and hats.

---

## 2. Scoped Context

### 2.1 BotMinter: Four-Level Additive Scoping

BotMinter's primary architectural differentiator is its layered context model. Knowledge and constraints resolve additively at four levels:

```
team-wide                All agents, all projects
  +-- project-wide        All agents on this project
      +-- member-wide     This agent, all projects
          +-- member+project   This agent, this project
```

An optional fifth level — **hat-level** knowledge — is managed by the Knowledge Manager skill, scoped at `team/<member>/hats/<hat_key>/knowledge/`.

On disk, this maps to directory hierarchies:

```
team-repo/
  knowledge/              # team-wide
  invariants/             # team-wide
  projects/<project>/
    knowledge/            # project-wide
    invariants/           # project-wide
  members/<member>/
    knowledge/            # member-wide
    invariants/           # member-wide
    projects/<project>/
      knowledge/          # member+project
```

Context **accumulates** as scope narrows — broader context is never lost, only augmented. When ambiguity arises, broader scope is preferred; it is easier to narrow later than to discover scattered knowledge.

### 2.2 Symphony: Flat, Single-Scope

Symphony provides a single `WORKFLOW.md` file that governs all agent behavior. This file contains YAML frontmatter (configuration) and a Markdown body (prompt template). All agents receive the same template, rendered with issue-specific variables (title, description, labels, etc.) via Liquid-compatible templating.

There is no mechanism to vary context by project, agent identity, or role. All Codex instances are peers receiving the same instructions.

### 2.3 Comparison

| Dimension | BotMinter | Symphony |
|-----------|-----------|----------|
| Scoping model | 4-level additive hierarchy (+ optional hat-level) | Flat, single file |
| Context resolution | Accumulative — broader context inherited, narrower context augments | Uniform — one template for all agents |
| Customization granularity | Per-member, per-project, per-member-on-project | Per-issue (template variables only) |
| Storage format | Git-backed directory hierarchy of Markdown files | YAML frontmatter + Markdown body in one file |
| Agent self-service | Agents can create/update knowledge via Knowledge Manager skill | Agents cannot modify their own context |

### 2.4 Assessment

BotMinter treats context as a first-class architectural concern with compositional layering. Symphony treats context as a configuration input. BotMinter's approach is more sophisticated for multi-agent, multi-project scenarios; Symphony's is simpler and sufficient for its "one workflow, many issues" model.

---

## 3. Knowledge

### 3.1 BotMinter: Managed, Scoped, Agent-Accessible

Knowledge in BotMinter is **informational context** — conventions, how-to guides, and protocols that agents SHOULD reference. It is distinct from invariants (Section 4) in that knowledge is guidance, not mandate.

**Team-level knowledge files observed:**

| File | Content |
|------|---------|
| `nono_trust_verification_and_instruction_files.md` | Nono security tool's cryptographic verification of instruction files (ECDSA P-256 signing, `.bundle` sidecars, Sigstore for CI/CD) |
| `ralph_orchestrator_events_evidence_and_confession.md` | Ralph's backpressure evidence system and Confession phase (gated events, coverage thresholds, thrashing detection) |
| `ralph_orchestrator_robot_human_in_the_loop.md` | Three communication channels: blocking questions, non-blocking notifications, proactive human guidance |

**Profile-level knowledge files (shared by `scrum` and `scrum-compact`):**

| File | Content |
|------|---------|
| `commit-convention.md` | Conventional commits format: `<type>(<scope>): <subject>` with `Ref: #<issue>` |
| `communication-protocols.md` | GitHub issues as coordination fabric; comment attribution with role headers; human-in-the-loop via GitHub comments and Telegram |
| `pr-standards.md` | Standard PR format (Summary, Related Issues, Changes, Testing) |

Knowledge is managed through dedicated CLI commands (`bm knowledge list/show/interactive`) and a Knowledge Manager skill that allows agents to create, update, and organize knowledge files during runtime — enabling **institutional learning** over time.

### 3.2 Symphony: Implicit in Prompt Templates and Skills

Symphony has no formal "knowledge" concept. Context is embedded in two places:

1. **WORKFLOW.md** — the prompt template body contains all instructions and conventions
2. **`.codex/skills/`** — six skill definitions (commit, push, land, debug, linear, pull) containing SKILL.md files with procedural instructions

These are static, repo-level resources applied uniformly. There is no management tooling, no scoping, and no agent self-service.

### 3.3 Comparison

| Dimension | BotMinter | Symphony |
|-----------|-----------|----------|
| Formalization | Explicit concept with dedicated file format and directory structure | Implicit — embedded in prompts and skill files |
| Scoping | 4-level hierarchy | Global (repo-level) |
| Management tooling | `bm knowledge list/show/interactive` CLI commands | Manual file editing |
| Agent self-service | Agents can create/update knowledge via Knowledge Manager skill | No — agents cannot modify context |
| Lifecycle | Knowledge evolves as agents learn and conventions change | Static until a human edits configuration |

### 3.4 Assessment

BotMinter elevates knowledge to a managed, scoped, agent-accessible resource with a lifecycle. Symphony embeds knowledge implicitly in prompt templates and Codex skills with no formal management or evolution mechanism.

---

## 4. Invariants

### 4.1 BotMinter: Constitutional Constraints

Invariants in BotMinter are **hard constraints that MUST be satisfied** — they are not suggestions, and violations are treated as bugs. They use RFC 2119 language (MUST, MUST NOT, SHOULD) and follow a structured format with five required sections: Title+summary, Rule, Applies To, Examples, and Rationale.

**Team-level invariants observed:**

| Invariant | Rule |
|-----------|------|
| `flaky-tests.md` | All flaky tests MUST be root-caused and fixed immediately. Never use `#[ignore]`. After 10 minutes of debugging, track in `crates/bm/tests/README.md`. |
| `gh-api-e2e.md` | Any code constructing payloads for external APIs MUST have an E2E test hitting the real API. Motivated by a GraphQL escaping bug that passed 12 unit tests. |
| `no-hardcoded-profiles.md` | Code and tests MUST NOT hardcode profile names, role names, status values, or label names. Tests must use `list_profiles()` dynamically. |
| `test-path-isolation.md` | Tests MUST use temporary directories for any paths under the user's home. Never use real user directories. |
| `cli-idempotency.md` | All state-mutating CLI commands MUST be idempotent. Detect existing resources and skip/update gracefully. |
| `e2e-scenario-coverage.md` | E2E tests MUST model complete operator journeys, not isolated feature fragments. Every scenario MUST verify runtime. |
| `profiles_updated.md` | Profile files MUST use paths consistent with the workspace model. Workspace model changes require updating all profile files in the same changeset. |

**Profile-level invariants (shared by both profiles):**

| Invariant | Rule |
|-----------|------|
| `code-review-required.md` | All code changes require review (peer review in `scrum`, self-review via `dev_code_reviewer` hat in `scrum-compact`) |
| `test-coverage.md` | All stories must have test coverage before done |

Invariants follow the same 4-level scoping as knowledge (team, project, member, member+project).

**Enforcement is two-layered:**

1. **Mechanical backpressure** — Ralph's evidence system validates required checks before accepting completion events. Hard-coded thresholds: test pass, lint clean, coverage >= 80%, mutation score >= 70%, complexity <= 10.0. Failing evidence causes event rewriting (e.g., `build.done` becomes `build.blocked`).

2. **Semantic self-assessment (Confession phase)** — A Confessor hat independently finds issues in the Builder's work, then a Confession Handler verifies claims. Acceptance requires confidence >= 80%. This catches quality issues that mechanical checks cannot.

3. **Post-commit audit** — The Builder hat is instructed to spin up sub-agents for invariant auditing after every commit, checking the changeset against all applicable invariants.

Thrashing detection (3 consecutive `build.blocked` events) triggers automatic abandonment — preventing infinite retry loops.

### 4.2 Symphony: Code-Level Constraints

Symphony has no formal invariant concept. Constraints exist as hardcoded Elixir logic:

- `max_turns` (default 20) — maximum Codex turns per issue
- Concurrency limits — maximum simultaneous agents
- State-based reconciliation — agents stop when issues move to terminal states
- Exponential backoff on transient failures
- Retry budgets

These are implementation details, not documented constitutional constraints. There is no self-assessment layer, no semantic quality gate, and no mechanism for agents to audit their own work against declared constraints.

### 4.3 Comparison

| Dimension | BotMinter | Symphony |
|-----------|-----------|----------|
| Formalization | Explicit Markdown files with structured format (5 required sections) | Implicit — hardcoded in Elixir |
| Scoping | 4-level hierarchy matching knowledge scoping | N/A — code-level only |
| Language | RFC 2119 (MUST/MUST NOT/SHOULD) | N/A |
| Enforcement layers | Three: mechanical backpressure + Confession phase + post-commit audit | One: code-level checks (concurrency, retries, state) |
| Semantic self-assessment | Yes — Confessor hat + Confession Handler with 80% confidence threshold | No |
| Thrashing protection | Yes — 3 consecutive `build.blocked` triggers abandonment | Yes — retry budgets with exponential backoff |
| Agent awareness | Agents explicitly audit invariants after every commit | Agents have no awareness of system constraints |
| Evolvability | New invariants added as files; agents can propose new ones | Requires code changes |

### 4.4 Assessment

BotMinter's invariant system is a standout feature. It creates a constitution for agent behavior that is scoped, documented, auditable, and enforced both mechanically and semantically. Symphony's constraints are implicit in code — effective but not introspectable by agents or extensible without code changes.

---

## 5. Roles and Hats

### 5.1 BotMinter: Event-Driven Role Switching

**Roles** define team positions. In the `scrum` profile, roles include architect, dev, QE, PO, team-manager, and human-assistant — each backed by a separate agent instance. In the `scrum-compact` profile, a single agent fills all roles by switching **hats**.

**Hats** are Ralph Orchestrator's mechanism for behavioral context switching within a single agent. Each hat is defined by:

```yaml
name: dev_implementer
description: "Implements the story..."
triggers:
  - arch_design.done
publishes:
  - build.done
default_publishes:
  - build.blocked
instructions: |
  # Implementation instructions...
```

Five hat archetypes govern different behavioral modes:

| Archetype | Purpose | Example |
|-----------|---------|---------|
| **Scanner** | Entry point — polls GitHub Projects v2, dispatches work | `board_scanner` |
| **Worker** | Produces artifacts (code, designs, tests) | `arch_designer`, `dev_implementer`, `dev_tester` |
| **Reviewer** | Decoupled quality gate — approves or rejects | `dev_code_reviewer`, `arch_reviewer` |
| **Gater** | Supervised mode — presents to human for approval | `supervised_gate` |
| **Monitor** | Watches progress, auto-advances when conditions met | `progress_monitor` |

Hat transitions are event-driven. The flow for a typical work item:

```
board_scanner (scans board)
  --> arch_designer (designs architecture)
    --> arch_reviewer (reviews design)
      --> dev_implementer (writes code)
        --> dev_code_reviewer (reviews code)
          --> dev_tester (writes tests)
            --> board_scanner (advances status, picks next item)
```

Events like `build.done`, `review.approved`, `review.rejected`, and `verify.passed` drive transitions. Rejection events loop back to earlier hats for rework.

**Profiles** package roles into methodologies:

- **`scrum-compact`**: One agent, all hats. A "superman" model where a single Ralph instance cycles through scanner, worker, reviewer, and tester hats.
- **`scrum`**: Multiple agents with dedicated roles. Each member runs its own Ralph instance with role-specific hats.

### 5.2 Symphony: Homogeneous Worker Pool

Symphony has no role concept. All agents are identical Codex instances that receive the same prompt template rendered with issue-specific variables. The Orchestrator dispatches uniformly — any available agent slot can pick up any issue.

There is no behavioral differentiation between agents, no event-driven state machine, and no role switching. Each agent runs independently on its assigned issue for up to `max_turns`, then completes or fails.

The closest analog to role specialization is the set of Codex skills (`.codex/skills/`) — but these are available to all agents equally, not assigned by role.

### 5.3 Comparison

| Dimension | BotMinter | Symphony |
|-----------|-----------|----------|
| Role model | Formally defined team positions with distinct responsibilities | No roles — all agents are identical |
| Behavioral switching | Event-driven hat transitions within a single agent | No switching — one prompt per issue |
| Hat archetypes | 5 types: Scanner, Worker, Reviewer, Gater, Monitor | N/A |
| Coordination model | Decentralized — emergent from events, labels, and shared conventions | Centralized — Orchestrator GenServer dispatches |
| Process methodology | Pluggable via profiles (`scrum`, `scrum-compact`) | Single methodology defined in WORKFLOW.md |
| Quality gates | Reviewer hats + Gater hats + Confession phase | None between agent runs — success/fail only |
| Multi-agent topology | Supports both single-agent-many-hats and many-agents-dedicated-roles | Many identical agents, no topology |

### 5.4 Assessment

BotMinter has a rich role/hat system that enables sophisticated multi-persona, event-driven agent behavior — even within a single agent instance. Symphony deliberately avoids this complexity; its agents are homogeneous workers dispatched by a central coordinator. The tradeoff is clear: BotMinter models real team dynamics at the cost of configuration complexity; Symphony optimizes for operational simplicity at the cost of behavioral richness.

---

## 6. Architectural Philosophy

The differences above reflect fundamentally different design philosophies:

| Aspect | BotMinter | Symphony |
|--------|-----------|----------|
| Mental model | A team of specialists with process, roles, and institutional knowledge | A pool of identical workers pulling from a task queue |
| Coordination | Decentralized — emergent from shared conventions and event-driven state machines | Centralized — one Orchestrator GenServer controls everything |
| Agent identity | Persistent — agents have identity, memory, knowledge, and role-specific behavior | Ephemeral — spun up per issue, torn down after |
| Quality assurance | Multi-layered: mechanical backpressure, Confession phase, Reviewer hat, optional human Gater | Single layer: Codex runs, success or failure, retry on transient errors |
| Self-improvement | Agents can create knowledge and invariants, evolving team conventions over time | Static configuration — no self-improvement |
| Issue tracker | GitHub Issues + Projects v2 (decentralized coordination fabric) | Linear (centralized task queue) |
| Agent runtime | Ralph Orchestrator (event loop with hats, backpressure, human-in-the-loop) | Codex app-server (JSON-RPC over stdio) |
| Implementation | Rust CLI with embedded profiles | Elixir/OTP with Phoenix LiveView |
| Complexity | High — sophisticated but more to configure and understand | Low — simple, opinionated, effective for its use case |

---

## 7. Conclusions

### 7.1 Where Symphony Excels

- **Operational simplicity** — one config file, one agent type, one poll loop
- **Fault tolerance** — OTP supervision trees provide battle-tested resilience
- **Observability** — terminal dashboard + Phoenix LiveView + JSON API out of the box
- **Time to value** — configure a WORKFLOW.md, point at Linear, and agents start working

### 7.2 Where BotMinter Excels

- **Context management** — 4-level scoped knowledge prevents context pollution and enables per-agent, per-project customization
- **Quality enforcement** — invariants as constitutional constraints with mechanical + semantic enforcement
- **Team modeling** — roles and hats enable realistic multi-persona workflows with quality gates between phases
- **Institutional learning** — agents can create and update knowledge, building team conventions over time
- **Process flexibility** — pluggable profiles allow different methodologies (scrum, scrum-compact, future custom profiles)

### 7.3 Complementary, Not Competing

These projects are not direct competitors. Symphony optimizes for **throughput** — processing a backlog of independent issues efficiently. BotMinter optimizes for **process fidelity** — ensuring agents work together with the conventions, constraints, and quality gates that real teams need. An organization might use Symphony for high-volume, independent task processing while using BotMinter for complex, multi-phase features requiring architectural review, code review, and testing as distinct stages.

---

*Report generated from source analysis of both repositories. Symphony analysis based on commit `1f86bac` (main). BotMinter analysis based on current working tree at `/home/sandboxed/workspace/botminter/`.*
