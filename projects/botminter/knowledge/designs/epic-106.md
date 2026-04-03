# Design: Transition BotMinter to Fully Agentic SDLC

**Epic:** #106
**Author:** bob (superman)
**Date:** 2026-04-04
**Status:** Draft
**Reference:** [OpenAI Harness Engineering (Feb 2026)](https://openai.com/index/harness-engineering/)

---

## 1. Overview

### Problem

BotMinter's scrum-compact profile provides a working agentic SDLC: 18 hats, board-driven dispatch, knowledge/invariant scoping, human review gates, and rejection loops. However, six capability gaps prevent the transition from supervised (Tier 2) to fully agentic (Tier 3) operation.

OpenAI's Harness Engineering team articulated the principle: *"When something failed, the fix was almost never 'try harder.' Human engineers always asked: 'what capability is missing, and how do we make it legible and enforceable for the agent?'"*

Each gap is a missing capability in BotMinter's profile system. Closing them makes the framework better for all teams, not just the dogfooding team.

### Scope

This epic adds six features to BotMinter, delivered primarily as scrum-compact profile enhancements with supporting CLI and manifest changes where needed. All features are opt-in or backward-compatible with existing team setups.

### What This Epic Is NOT

- Not ad-hoc changes to any specific team's configuration
- Not changes to Ralph Orchestrator (upstream dependency)
- Not a new profile — features enhance the existing scrum-compact profile

---

## 2. Architecture

### 2.1 BotMinter Product Model

BotMinter delivers conventions-as-code through **profiles**. A profile packages: a manifest (`botminter.yml`), role definitions with hats, knowledge, invariants, skills, workflows, and directory templates. When an operator runs `bm init` or `bm hire`, the profile content is extracted into the operator's team repo. Agents then consume these conventions at runtime.

This design introduces six features that extend the profile's capabilities. Each feature modifies one or more of these profile layers:

| Profile Layer | What It Contains | How Agents Consume It |
|---------------|-----------------|----------------------|
| **Manifest** (`botminter.yml`) | Schema fields — roles, statuses, labels, bridges | CLI parses at extraction time; agents read runtime config |
| **Role definitions** (`roles/<role>/ralph.yml`) | Hat definitions with triggers, instructions, backpressure | Ralph activates hats based on events |
| **Invariants** (`invariants/`) | Mandatory constraints (prose + executable) | Hats verify compliance before transitions |
| **Knowledge** (`knowledge/`) | Advisory context — conventions, designs, references | Hats load on-demand as working context |
| **Skills** (`coding-agent/skills/`) | Composable agent capabilities | Ralph loads skills into hat context |
| **Directory templates** | Scaffolded directories for plans, metrics, etc. | Extracted by `bm init` for team use |

### 2.2 Development Context

BotMinter evolved through four methodology phases (Ralph-Orchestrated, GSD Framework, Agent SOP/Hat System, A-Team/Dogfooding). Planning artifacts exist across multiple structures reflecting this evolution:

- 11 ADRs in the project repo (7 accepted, 3 proposed) — following ADR-0001 format
- GSD phase plans and milestone plans from Phase 2
- Feature specs from Phase 3
- Design docs and team agreements from Phase 4

This fragmentation is acknowledged. Feature 4 (Plans as First-Class Artifacts) establishes canonical conventions to resolve it going forward.

### 2.3 The Six Capability Gaps

| Gap | Current State | BotMinter Feature |
|-----|--------------|-------------------|
| Invariants are prose-only | Agents are *instructed* to follow markdown invariants; nothing mechanically prevents violations | **Executable Invariant Checks** — shell-script checks with agent-readable output |
| No application legibility | Bug investigation relies on code reading; no structured test output or runtime observation | **Application Legibility** — structured output conventions and optional dev-environment bootstrapping |
| No automated cleanup | Stale docs, dead code, and quality drift go undetected | **Garbage Collection** — gardener hat with golden principles and quality scoring |
| Scattered planning artifacts | Designs, specs, plans across multiple directories and formats | **Plans as First-Class Artifacts** — canonical plan conventions and directory templates |
| Three hard-coded human gates | Agent cannot proceed without explicit approval at design review, plan review, and acceptance | **Graduated Autonomy** — configurable autonomy tiers in the profile manifest |
| No metrics | No cycle-time tracking, rejection rates, or data-driven feedback | **Metrics and Feedback Loops** — transition logging and automated reporting |

### 2.4 Existing ADR Relevance

| ADR | Title | How It Informs This Design |
|-----|-------|---------------------------|
| 0001 | ADR Format (Spotify-style) | New decisions during implementation follow this format |
| 0002 | Shell Script Bridge with YAML Manifest | Pattern reference for executable check scripts (shell + YAML config) |
| 0004 | Scenario-Based E2E Tests | Test approach for new features |
| 0005 | E2E Test Environment and Isolation | Isolation model for dev-boot and observability |
| 0006 | Directory Modules Only | Architecture layer checks must respect this convention |
| 0007 | Domain Modules and Command Layering | Defines the architecture layers that checks validate |
| 0008 | Formation as Deployment Strategy | Dev-boot aligns with the formation abstraction |
| 0009 | Exploratory Integration Tests | Gardener checks complement (not replace) exploratory tests |

---

## 3. Components and Interfaces

### 3.1 Feature 1: Executable Invariant Checks

#### Problem

Invariants are markdown documents that instruct agents what to do. Nothing mechanically verifies compliance. Violations can reach code review undetected.

#### Feature Description

BotMinter's invariant system gains an executable layer. Alongside existing prose invariants (`.md` files), profiles can include shell-script checks in a `checks/` subdirectory. These checks run automatically during code review and verification, catching violations before they propagate.

#### Design

**Check Script Contract:**

Every check script follows a standard interface:
- **Input:** Runs in the project repository working directory
- **Exit code:** 0 = pass, non-zero = failure
- **Failure output** (agent-readable remediation, following Harness Engineering's pattern):

```
VIOLATION: <what was detected>
RULE: <which invariant>
REMEDIATION: <what the agent should do>
REFERENCE: <path to the governing invariant>
```

This contract mirrors ADR-0002's pattern: declarative scripts with structured output, not arbitrary executables.

**Profile Enhancement:**

The scrum-compact profile's `invariants/` directory gains a `checks/` subdirectory containing baseline check scripts (e.g., test coverage thresholds, naming conventions). Teams can add project-scoped checks for project-specific rules like architecture layer validation (per ADR-0007's domain-command layering).

**Hat Integration:**

Two existing hats gain check-running steps in their instructions:
- `dev_code_reviewer`: runs all applicable checks before reviewing; rejects to `dev:implement` if any fail
- `qe_verifier`: runs checks as part of verification; rejects if any fail

Checks are discovered by directory scan (convention-over-configuration) — adding a new script to `checks/` automatically includes it in future reviews.

**Relationship to Prose Invariants:**

Prose invariants remain as human-readable reference documentation. Executable checks are the mechanical enforcement layer. Both coexist — hats read prose for context and run scripts for enforcement.

#### Acceptance Criteria

- **Given** a code change violating an architecture layer rule,
  **When** `dev_code_reviewer` runs invariant checks,
  **Then** the check fails with structured remediation output and the story is rejected to `dev:implement`

- **Given** a code change passing all checks,
  **When** `dev_code_reviewer` runs invariant checks,
  **Then** all checks pass and review proceeds normally

- **Given** a new check script added to the checks directory,
  **When** subsequent code reviews run,
  **Then** the new check is automatically discovered and executed

---

### 3.2 Feature 2: Application Legibility

#### Problem

QE investigates bugs by reading source code. There is no structured test output for agents to parse, no dev-environment bootstrapping for runtime observation, and no way for agents to interact with a running application.

#### Feature Description

Legibility is introduced in phases of increasing complexity. Phase A is a profile-level convention. Phases B through D are optional, project-specific capabilities.

#### Design

**Phase A: Structured Test Output (profile convention)**

The profile defines a test output format convention requiring structured JSON output with standard fields: test name, status, duration, error message, and file path. Hats (`qe_investigator`, `dev_implementer`) parse this output to navigate directly to failures instead of scanning raw console output. Enforcement is via the code reviewer hat and a check script (Feature 1).

**Phase B: Dev-Environment Bootstrapping (optional, per-project)**

Projects with a runnable application can define boot/teardown configuration: a boot script, health check endpoint, teardown script, and isolation mode. This aligns with ADR-0008 (Formation as Deployment Strategy) — dev-boot uses the formation abstraction for local deployment. The `dev_implementer` and `qe_verifier` hats gain instructions to boot the app when this configuration exists.

**Phase C: Observability Stack (deferred)**

Ephemeral per-worktree observability (logs, metrics, traces). Deferred until Phase B is validated.

**Phase D: UI Introspection (deferred)**

Browser automation for web projects. Deferred until an applicable project exists.

#### Acceptance Criteria

- **Given** a test suite runs during implementation,
  **When** tests complete,
  **Then** output is structured JSON that agents can parse for specific failure details

- **Given** a project with dev-environment bootstrapping configured,
  **When** `dev_implementer` works on a story,
  **Then** the application boots in isolation and the agent can validate behavior at runtime

---

### 3.3 Feature 3: Garbage Collection

#### Problem

No automated detection of stale documentation, duplicated code, dead code, or quality drift. Entropy accumulates silently until a human notices.

#### Feature Description

The scrum-compact profile gains a new `arch_gardener` hat that performs periodic codebase cleanup and quality assessment, guided by a configurable set of golden principles.

#### Design

**Gardener Hat:**

A new hat added to the superman role definition. It is activated by a `gardener.scan` event (dispatched by the board scanner on a configurable schedule). The hat:

1. Scans the codebase against golden principles — a YAML-defined set of quality rules (e.g., "prefer shared utilities over duplicated helpers," "remove dead code," "use consistent error handling patterns")
2. Runs all executable invariant checks (Feature 1) and notes new violations
3. Checks documentation freshness by comparing git log dates against knowledge file dates
4. Produces a quality score report with per-domain grades (A-F), test coverage, invariant compliance, and documentation freshness
5. Opens targeted refactoring or fix-up issues for drift items

**Golden Principles:**

A YAML configuration file shipped in the profile's invariants directory, defining quality rules with detection heuristics and remediation guidance. Teams can override or extend with project-scoped principles.

```yaml
principles:
  - name: shared-utilities-over-hand-rolled
    description: "Prefer shared utility packages over hand-rolled helpers"
    detection: "Find functions with >80% similarity across modules"
    remediation: "Extract to shared utility, update all call sites"
```

**Triggering:**

The board scanner dispatches `gardener.scan` on a configurable schedule (e.g., after every N scan cycles, or when explicitly invoked). This is profile configuration, not a Ralph Orchestrator change — the scanner skill's instructions are part of the profile.

#### Acceptance Criteria

- **Given** the gardener hat runs its scan,
  **When** it detects duplicated utility code,
  **Then** it opens a refactoring issue describing the duplication and remediation

- **Given** a knowledge file references a renamed or deleted function,
  **When** the documentation freshness scan runs,
  **Then** a fix-up issue is opened

- **Given** the gardener completes its scan,
  **When** it updates the quality score,
  **Then** the score reflects current coverage, invariant compliance, and documentation freshness

---

### 3.4 Feature 4: Plans as First-Class Artifacts

#### Problem

Planning artifacts are fragmented across multiple directories and formats reflecting BotMinter's evolution through different methodologies. Story breakdowns exist only in issue comments. No execution plans track epic progress as a living document.

#### Feature Description

The profile gains execution plan conventions and directory templates that establish a canonical home for planning artifacts. This brings structure to the planning landscape without invalidating existing artifacts.

#### Design

**Execution Plan Convention:**

When the `arch_planner` hat produces a story breakdown, it also creates an execution plan — a living document that tracks the epic's progress through implementation. The plan captures the story list, key decisions made during execution, and progress milestones.

```markdown
# Execution Plan: Epic #<number> -- <title>

## Status: In Progress | Completed

## Stories
| # | Title | Status | Completed |
|---|-------|--------|-----------|

## Decision Log
| Date | Decision | Rationale |
|------|----------|-----------|

## Progress Notes
- <date>: <event>
```

**Hat Integration:**

- `arch_planner`: creates the execution plan when a story breakdown is approved
- `arch_monitor`: updates plan progress as stories complete
- On epic completion, the plan moves from `active` to `completed` status

**Artifact Organization Convention:**

The profile establishes clear conventions for where different artifact types belong:

| Artifact Type | Belongs To | Rationale |
|---------------|-----------|-----------|
| Knowledge (conventions, references) | Knowledge layer | Advisory context loaded on-demand by hats |
| Design docs | Knowledge layer | Designs are implementation context for hats |
| Execution plans | Plans directory | Living documents that track execution state |
| Invariants (constraints) | Invariant layer | Machine-readable and prose constraints |
| ADRs (architecture decisions) | Project repository | ADRs document codebase decisions, live with the code (per ADR-0001) |
| Team agreements | Agreements directory | Team-level governance records |

ADRs remain in the project repository following the established ADR-0001 format. They document *codebase* decisions and belong with the code they govern, not in the team configuration layer.

#### Acceptance Criteria

- **Given** `arch_planner` produces a story breakdown,
  **When** the breakdown is approved,
  **Then** an execution plan exists with the story list and initial status

- **Given** a story reaches `done`,
  **When** `arch_monitor` scans the epic,
  **Then** the execution plan's story table is updated

- **Given** all stories in an epic reach `done`,
  **When** the epic is accepted,
  **Then** the plan is archived as completed

---

### 3.5 Feature 5: Graduated Autonomy

#### Problem

The scrum-compact profile hard-codes three human gates: `po:design-review`, `po:plan-review`, and `po:accept`. Agents cannot proceed without explicit human approval at each gate, regardless of the team's confidence in the agent's work quality.

#### Feature Description

The `botminter.yml` profile manifest gains an `autonomy` configuration field that lets operators choose how many human gates to enforce. Three tiers are defined, with `supervised` (current behavior) as the default. The `bm` CLI and profile extraction pipeline interpret this setting; hat instructions adapt their behavior accordingly.

#### Design

**Manifest Schema Extension:**

```yaml
# botminter.yml (new top-level field)
autonomy:
  default: supervised
  tiers:
    supervised:
      description: "3 human gates: design-review, plan-review, accept"
      human_gates: [po:design-review, po:plan-review, po:accept]
    guided:
      description: "1 human gate: accept only"
      human_gates: [po:accept]
      auto_advance_after: lead_review
    autonomous:
      description: "0 human gates, async notification"
      human_gates: []
      notification: true
```

**CLI Implementation:**

The `ProfileManifest` struct in `manifest.rs` gains an `autonomy` field. During extraction, the autonomy setting is written to a runtime configuration file that hats can read. The field is optional and defaults to `supervised`, preserving current behavior for existing teams.

**Hat Behavior Adaptation:**

The `po_reviewer` hat instructions check the active autonomy tier:
- **Supervised:** Current behavior — post review request, wait for human comment
- **Guided:** Auto-advance design and plan reviews after lead approval. Wait for human only at `po:accept`. Post notification comment on each auto-advance.
- **Autonomous:** Auto-advance all gates. Post notification comments. Human retains override capability.

**Override Mechanism:**

- Human comments `Rejected: <feedback>` on any issue to revert an auto-advance
- Human comments `Hold` to pause auto-advances for that specific issue
- Changing the autonomy tier requires a profile re-sync — agents cannot modify the setting at runtime

**Why `botminter.yml`, not `ralph.yml`:** Ralph Orchestrator is an upstream dependency. Its `ralph.yml` defines the event loop, hats, skills, and iteration config. Autonomy is a BotMinter-level policy that determines how hats behave — it's a profile concern, not an orchestrator concern. The profile's hat instructions read the autonomy setting and adjust their behavior. Ralph just runs whatever instructions the hat provides.

#### Acceptance Criteria

- **Given** an operator configures `autonomy: guided`,
  **When** lead review approves a design doc,
  **Then** `po:design-review` auto-advances with a notification comment (no human wait)

- **Given** `autonomy: guided`,
  **When** an epic reaches `po:accept`,
  **Then** the agent waits for human comment (same as supervised at this gate)

- **Given** a human comments `Rejected: <feedback>` on an auto-advanced issue,
  **When** the agent scans the issue,
  **Then** the status reverts and feedback is processed as a rejection

---

### 3.6 Feature 6: Metrics and Feedback Loops

#### Problem

No cycle-time tracking, rejection rate data, or quantitative feedback. The existing `poll-log.txt` audit log provides traceability but no analytics. Retrospectives lack data-driven input.

#### Feature Description

The profile gains metrics infrastructure: transition logging in the board scanner skill, derived metrics computation, and weekly automated reports. This creates a data foundation for measuring improvement and driving retrospectives.

#### Design

**Transition Logging:**

The board scanner skill (auto-injected into coordinator prompts) gains an instruction to append a JSONL entry after each status transition:

```jsonl
{"issue":106,"type":"Epic","from":"po:triage","to":"po:backlog","ts":"2026-04-03T15:00:00Z","hat":"po_backlog"}
```

This is an additive change to the scanner skill's instructions. The skill already posts comments and logs to `poll-log.txt`; JSONL append is minimal additional work.

**Derived Metrics:**

| Metric | Measures | Target |
|--------|----------|--------|
| Design cycle time | Duration from `arch:design` to `po:ready` | Trending down |
| Implementation cycle time | Duration from `dev:implement` to `qe:verify` | Trending down |
| Human gate wait time | Time spent in review statuses | < 4 hours median |
| Rejection rate per gate | Percentage of rejections at each gate | < 15% |
| First-pass rate | Stories reaching `done` without rejection | > 70% |
| Throughput | Issues completed per week | Trending up |

**Weekly Reports:**

A `cw_writer` hat task (or gardener hat extension) generates a weekly summary from the transition log. The existing `retrospective` skill receives metrics as input for data-driven retros.

#### Acceptance Criteria

- **Given** the board scanner transitions an issue's status,
  **When** the transition completes,
  **Then** a JSONL entry is appended to the transition log

- **Given** a week of transition data exists,
  **When** the weekly report generator runs,
  **Then** a report shows cycle times, rejection rates, and throughput trends

- **Given** the retrospective skill is invoked,
  **When** metrics data is available,
  **Then** the retro includes data-driven observations alongside qualitative input

---

## 4. Data Models

### 4.1 Architecture Layer Definition (project-scoped YAML)

```yaml
layers:
  - name: types
    allowed_imports: []
  - name: config
    allowed_imports: [types]
  - name: domain
    allowed_imports: [types, config]
  - name: command
    allowed_imports: [types, config, domain]
cross_cutting:
  - name: providers
    accessible_from: [domain, command]
```

Derived from ADR-0007's domain-command layering convention.

### 4.2 Golden Principles (profile-level YAML)

```yaml
principles:
  - name: string           # Identifier
    description: string     # Human-readable explanation
    detection: string       # How to find violations
    remediation: string     # How to fix violations
```

### 4.3 Autonomy Configuration (in `botminter.yml`)

```yaml
autonomy:
  default: supervised | guided | autonomous
  tiers:
    <tier-name>:
      description: string
      human_gates: [status-name, ...]
      auto_advance_after: string   # optional
      notification: boolean        # optional
```

### 4.4 Transition Log Entry (JSONL)

```json
{
  "issue": "integer",
  "type": "Epic | Task | Bug",
  "from": "string (status)",
  "to": "string (status)",
  "ts": "ISO 8601 UTC",
  "hat": "string (hat name)"
}
```

### 4.5 Execution Plan (Markdown)

```markdown
# Execution Plan: Epic #<number> -- <title>
## Status: In Progress | Completed
## Stories
| # | Title | Status | Completed |
## Decision Log
| Date | Decision | Rationale |
## Progress Notes
- <date>: <event>
```

---

## 5. Error Handling

### 5.1 Check Script Failures

- Script *crashes* (unexpected error, not a check failure) are logged as warnings; review continues with remaining checks
- Three consecutive crashes from the same script flag it for human attention
- A crashed check does not block review — only explicit check *failures* (exit 1 with VIOLATION output) block

### 5.2 Dev-Environment Bootstrap Failures

- Boot script failure or health check timeout causes the agent to proceed without the running application (degraded mode)
- Warning comment posted on the issue explaining the fallback

### 5.3 Auto-Advance Failures (Graduated Autonomy)

- Status transition error during auto-advance falls back to supervised behavior for that gate
- Comment posted explaining the fallback; retried on next scan cycle

### 5.4 Gardener Failures

- Scan failure triggers retry on next cycle, maximum 3 retries before flagging for human attention
- Gardener failures never block other work — gardening is a background maintenance activity

### 5.5 Metrics Failures

- JSONL write failure logged but does not block the status transition (metrics are observational, not transactional)
- Report generation failure results in a warning; historical data is preserved for the next attempt

---

## 6. Impact on Existing System

### 6.1 Profile Changes

| Component | Change | Risk |
|-----------|--------|------|
| `invariants/` | Add `checks/` subdirectory and baseline scripts | Low — additive directory |
| `invariants/` | Add `golden-principles.yml` | Low — new file |
| `botminter.yml` | Add `autonomy` field | Low — optional, defaults to `supervised` |
| Role `ralph.yml` | Add `arch_gardener` hat definition | Medium — new hat, tested in isolation |
| Role `ralph.yml` | Update `dev_code_reviewer` and `qe_verifier` hat instructions | Medium — adds check-running step |
| Board scanner skill | Add transition JSONL logging instruction | Low — additive |
| Directory templates | Add `plans/` and `metrics/` scaffolding | Low — empty directories at extraction |

### 6.2 CLI/Codebase Changes

| Component | Change | Risk |
|-----------|--------|------|
| `manifest.rs` | Parse optional `autonomy` field from `botminter.yml` | Low — new optional field |
| `extraction.rs` | Extract new directories (`plans/`, `metrics/`, `checks/`) | Low — additive extraction logic |
| Potential `bm check` command | Run invariant checks on demand | Low — new command, no side effects |

### 6.3 Backward Compatibility

- All features are additive or opt-in
- Default `autonomy: supervised` preserves current three-gate behavior
- Existing teams gain new directories on next profile extraction
- No behavioral changes unless explicitly configured
- Prose invariants remain alongside executable checks
- Existing ADRs are referenced, not modified

### 6.4 What Does NOT Change

- Ralph Orchestrator (`ralph.yml` schema, event system, hat dispatch)
- GitHub Projects v2 integration (same statuses, same board structure)
- Human review gate behavior (unless autonomy tier is explicitly changed)
- Knowledge hierarchy and resolution order
- Existing 11 ADRs in the project repository
- Profile extraction mechanics (`bm init`, `bm hire`)

---

## 7. Security Considerations

### 7.1 Graduated Autonomy

- `autonomous` mode removes all human gates — operators should validate agent review quality at `guided` tier before upgrading
- Override mechanism (`Rejected:` comment) provides an emergency brake at any tier
- Audit trail via notification comments on every auto-advance
- Autonomy setting requires profile re-sync to change — agents cannot escalate their own autonomy at runtime
- Recommendation: validate `guided` on low-risk work before upgrading to `autonomous`

### 7.2 Executable Invariant Checks

- Check scripts execute in the agent's context — must not introduce command injection vulnerabilities
- Scripts are read-only analyzers during the check phase (analyze, not modify)
- Only the gardener hat's cleanup PRs modify code, and those go through the normal code review pipeline
- Check scripts are version-controlled in the profile or team repo — no arbitrary execution
- Following ADR-0002's pattern: scripts are declarative with structured output, not unconstrained executables

### 7.3 Metrics Data

- Transition logs contain issue numbers, status names, and timestamps — no sensitive data
- Stored with the same access control as other team artifacts
- No PII or credentials in the metrics pipeline

### 7.4 Observability Stack (Phase C, deferred)

- Per-worktree, ephemeral — torn down after the task completes
- Operates on test/dev data only, not production data
- Local-only, not network-exposed

---

## 8. Implementation Order

| Phase | Feature | Risk | Value |
|-------|---------|------|-------|
| 1 | Executable Invariant Checks | Low | High — immediate quality enforcement |
| 2 | Plans as First-Class Artifacts | Low | Medium — resolves artifact fragmentation |
| 3 | Garbage Collection | Low | High — automated quality maintenance |
| 4 | Metrics and Feedback Loops | Low | Medium — enables data-driven improvement |
| 5 | Graduated Autonomy | Medium | High — reduces human bottleneck |
| 6 | Application Legibility (Phases A-D) | Medium-High | High — runtime observation |

**Rationale:** Enforcement and artifact structure first (low risk, immediate value). Autonomy only after quality infrastructure is proven reliable. Observability last (highest complexity, requires per-project configuration).

Each implementation phase produces an ADR (following ADR-0001 format) documenting the decisions made.

---

## 9. Success Criteria

| Metric | Current | Target |
|--------|---------|--------|
| Invariant violations reaching code review | Untracked | Zero (caught by executable checks) |
| Human gate wait time | Unknown | < 4 hours median |
| Rejection rate at code review | Unknown | < 15% |
| Stale knowledge docs | Unknown | Detected within 1 week |
| Autonomy tier | Supervised only | Guided on the dogfooding project |
| Cycle time trend | Untracked | Measured and trending down |
| First-pass success rate | Unknown | > 70% |

---

## 10. References

- [OpenAI Harness Engineering (Feb 2026)](https://openai.com/index/harness-engineering/)
- BotMinter ADR-0001: ADR Format (`.planning/adrs/0001-adr-process.md`)
- BotMinter ADR-0002: Shell Script Bridge pattern (`.planning/adrs/0002-bridge-abstraction.md`)
- BotMinter ADR-0007: Domain Modules and Command Layering (`.planning/adrs/0007-domain-command-layering.md`)
- BotMinter ADR-0008: Formation as Deployment Strategy (`.planning/adrs/0008-team-runtime-architecture.md`)
- BotMinter `profiles/scrum-compact/botminter.yml` — profile manifest
- BotMinter `PROCESS.md` — status graph and workflow conventions
