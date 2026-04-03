# Design: Transition BotMinter to Fully Agentic SDLC

**Epic:** #106
**Author:** bob (superman)
**Date:** 2026-04-03
**Status:** Draft
**Reference:** [OpenAI Harness Engineering (Feb 2026)](https://openai.com/index/harness-engineering/)

---

## 1. Overview

Transition BotMinter from its current supervised agentic SDLC (Tier 2 — AI-Driven) to a fully agentic SDLC (Tier 3 — AI-Autonomous), inspired by OpenAI's Harness Engineering methodology.

OpenAI's Harness team shipped ~1M lines of production code in 5 months with **zero manually-written code** using 3 engineers (later 7). Their key insight:

> *"When something failed, the fix was almost never 'try harder.' Human engineers always asked: 'what capability is missing, and how do we make it legible and enforceable for the agent?'"*

BotMinter already has strong structural alignment (~60%) with validated Harness patterns. This epic closes the remaining 6 gaps through a phased approach.

---

## 2. Architecture: Current State vs Target State

### 2.1 What We Already Have (Validated by Harness)

| Harness Pattern | BotMinter Equivalent | How It Maps |
|---|---|---|
| Agent specialization (code, review, test, deploy) | 18 hats (PO, arch, dev, QE, SRE, CW, lead) | Harness uses Codex agents with different roles; BotMinter uses hat-switching on a single superman agent. Same separation of concerns. |
| Agent-to-agent review ("pushed almost all review to agent-to-agent") | `lead_reviewer` → `dev_code_reviewer` → `qe_verifier` chain | Harness's agent review loop matches our multi-hat review pipeline. |
| AGENTS.md as table of contents (~100 lines, pointers to deeper docs) | CLAUDE.md as entry point → `team/knowledge/`, invariants, hat knowledge | Both use a small entry file that points to structured deeper knowledge. |
| Structured docs/ as system of record | `team/knowledge/`, `team/projects/<project>/knowledge/`, hat-level knowledge | Harness: `docs/{design-docs,exec-plans,product-specs,references}`. BotMinter: `team/knowledge/` with project/member/hat scoping. Same progressive disclosure pattern. |
| Architectural constraints enforced via rules | `team/invariants/`, project invariants, member invariants | Both define rules. Difference: Harness enforces mechanically; BotMinter enforces via prose. |
| Plans as first-class artifacts (exec-plans with progress logs) | Design docs in `team/projects/<project>/knowledge/designs/` | Harness versions active/completed plans with decision logs. BotMinter stores designs but not execution plans. |
| Feedback loops (review → reject → revise → re-review) | Rejection loops at every gate with comment-based feedback | Both iterate via structured feedback. |
| Declarative workflow (status-driven dispatch) | Board scanner + status graph + hat dispatch | Both use a status-driven orchestrator that dispatches work based on current state. |
| "Ralph Wiggum Loop" (agent reviews own work, iterates until all reviewers satisfied) | Self-review chain: `dev_implementer` → `dev_code_reviewer` → `qe_verifier` | Same pattern: agent writes, agent reviews, agent iterates. |
| "Boring" technology preference (composable, stable APIs) | Project invariants guide technology choices | Both favor technologies agents can model well. |
| Repository as single source of truth ("if it isn't in the repo, it doesn't exist") | `team/` repo as control plane + project repos for code | Both reject external context (Slack, docs, heads) in favor of versioned artifacts. |

### 2.2 The Six Gaps

```
Current State                          Target State
-----------                            -----------
Prose invariants (honor system)   -->  Mechanical enforcement (CI lints + structural tests)
No telemetry access               -->  Per-worktree observability (logs/metrics/traces)
No automated cleanup              -->  Garbage collection (gardener hat + golden principles)
Designs only, no exec plans       -->  Plans as first-class artifacts (active/completed/debt)
3 hard human gates                -->  Graduated autonomy (supervised/guided/autonomous)
No metrics                        -->  Cycle-time + quality tracking + data-driven retros
```

---

## 3. Components and Interfaces

### 3.1 Gap 1: Mechanical Enforcement

#### Problem
Invariants are markdown documents (`team/invariants/`, `team/projects/<project>/invariants/`). Agents are *instructed* to follow them. Nothing prevents violations from reaching code review.

Harness's approach: *"Custom linters and structural tests enforce layered architecture. Lint error messages are written for agents — they inject remediation instructions into context."*

#### Design

**3.1.1 Architecture Layers Definition**

Each project defines its allowed dependency layers in a machine-readable YAML file:

```yaml
# team/projects/botminter/invariants/architecture-layers.yml
layers:
  - name: types
    description: "Pure types/interfaces — no imports from other layers"
    allowed_imports: []
  - name: config
    description: "Configuration — imports types only"
    allowed_imports: [types]
  - name: repository
    description: "Data access — imports types, config"
    allowed_imports: [types, config]
  - name: service
    description: "Business logic — imports types, config, repo"
    allowed_imports: [types, config, repository]
  - name: runtime
    description: "Application bootstrap — imports all above"
    allowed_imports: [types, config, repository, service]
  - name: ui
    description: "Presentation — imports all above"
    allowed_imports: [types, config, repository, service, runtime]

cross_cutting:
  - name: providers
    description: "Auth, connectors, telemetry, feature flags — enter through single interface"
    accessible_from: [service, runtime, ui]
```

**3.1.2 Executable Invariant Checks**

Convert the top prose invariants into executable scripts:

```
team/projects/<project>/invariants/
  architecture-layers.yml          # Layer definitions (new)
  checks/                          # Executable checks (new)
    check-architecture-layers.sh   # Validates import directions
    check-structured-logging.sh    # Enforces structured log format
    check-naming-conventions.sh    # Validates naming patterns
    check-file-size-limits.sh      # Flags oversized files
    check-test-coverage.sh         # Minimum coverage thresholds
  design-quality.md                # Existing (remains as reference)
```

Each check script:
- Exits 0 on pass, 1 on failure
- On failure, prints **agent-readable remediation instructions** (not just "failed")
- Example error output:
  ```
  VIOLATION: service/auth.rs imports ui/components.rs
  LAYER RULE: 'service' layer must not import from 'ui' layer
  REMEDIATION: Move the shared type to 'types/' layer, then import from there.
  See: team/projects/botminter/invariants/architecture-layers.yml
  ```

**3.1.3 Integration Points**

- `dev_code_reviewer` hat runs all `checks/` scripts before reviewing
- `qe_verifier` hat runs checks as part of verification
- If any check fails, auto-reject back to `dev:implement` with the error output as feedback
- CI pipeline runs checks on every PR

**3.1.4 Custom Lint Error Messages**

Following Harness: *"Because the lints are custom, we write the error messages to inject remediation instructions into agent context."*

Every lint/check error message follows this template:
```
VIOLATION: <what happened>
RULE: <which rule was violated>
REMEDIATION: <exactly what to do to fix it>
REFERENCE: <path to the invariant/knowledge file>
```

#### Acceptance Criteria

- **Given** a code change that violates a defined architecture layer rule
  **When** the `dev_code_reviewer` hat runs invariant checks
  **Then** the check fails with an agent-readable remediation message and the story is rejected back to `dev:implement`

- **Given** a code change that passes all invariant checks
  **When** the `dev_code_reviewer` hat runs invariant checks
  **Then** all checks pass and review proceeds normally

- **Given** a new invariant is added to `checks/`
  **When** subsequent code reviews run
  **Then** the new check is automatically included without hat modifications

---

### 3.2 Gap 2: Application Legibility

#### Problem
QE investigates bugs by reading code and issue descriptions. No telemetry, no UI introspection. Agents can't observe runtime behavior.

Harness's approach: *"We made the app bootable per git worktree. We wired Chrome DevTools Protocol into the agent runtime. Logs, metrics, and traces are exposed via a local observability stack that's ephemeral for any given worktree."*

#### Design (Phased)

**Phase A: Structured Test Output (All projects)**

```
team/projects/<project>/invariants/
  test-output-format.yml           # Required test output structure
```

```yaml
# test-output-format.yml
requirements:
  - format: structured_json        # Tests must output parseable results
  - include:
    - test_name
    - status: [pass, fail, skip]
    - duration_ms
    - error_message                 # On failure
    - file_path                     # Source location
    - coverage_delta                # Optional
```

- `qe_investigator` and `dev_implementer` parse structured test output
- Test failures include stack traces and file locations agents can navigate
- Coverage reports available as data, not just pass/fail

**Phase B: Per-Worktree App Boot (Project-specific)**

- Projects that have a runnable app define a `dev-boot.sh` script
- Script boots the app in isolation (unique port, ephemeral DB)
- `sre_setup` hat provisions the environment
- `dev_implementer` can boot the app to validate behavior

```yaml
# team/projects/<project>/knowledge/dev-environment.yml
boot:
  script: ./scripts/dev-boot.sh
  health_check: http://localhost:${PORT}/health
  teardown: ./scripts/dev-teardown.sh
  isolation: worktree              # Each worktree gets its own instance
```

**Phase C: Observability Stack (Project-specific)**

Ephemeral per-worktree observability following Harness's architecture:

```
App → Vector (log/metric/trace collector)
       ├→ Victoria Logs  (queryable via LogQL)
       ├→ Victoria Metrics (queryable via PromQL)
       └→ Victoria Traces (queryable via TraceQL)
```

- Stack spins up with `dev-boot.sh`, tears down with worktree
- Agents query via standard APIs (LogQL, PromQL, TraceQL)
- Enables prompts like: "ensure no span exceeds 2 seconds" or "find the error causing this bug"

**Phase D: UI Introspection (Web projects only)**

- Chrome DevTools Protocol integration
- Agent capabilities: DOM snapshots, screenshots, navigation
- `qe_verifier` can validate UI state and take before/after screenshots

#### Acceptance Criteria

- **Given** a test suite runs during `dev:implement`
  **When** tests complete
  **Then** output is structured JSON that agents can parse for specific failure details

- **Given** a project with `dev-environment.yml` configured
  **When** `dev_implementer` works in a worktree
  **Then** the app boots in isolation and the agent can validate behavior against it

- **Given** a worktree with observability stack running
  **When** the agent queries logs for a specific error pattern
  **Then** matching log entries are returned with timestamps and context

---

### 3.3 Gap 3: Garbage Collection (Entropy Management)

#### Problem
No automated cleanup. No quality grading. No stale-doc detection. Over time, agent-generated patterns drift.

Harness's experience: *"Our team used to spend every Friday (20% of the week) cleaning up 'AI slop.' Instead, we started encoding 'golden principles' and built a recurring cleanup process. Technical debt is like a high-interest loan."*

#### Design

**3.3.1 Quality Scoring**

```markdown
# team/projects/<project>/knowledge/QUALITY_SCORE.md

## Quality Assessment — 2026-04-03

| Domain | Test Coverage | Invariant Compliance | Code Patterns | Doc Freshness | Grade |
|--------|--------------|---------------------|---------------|---------------|-------|
| Auth   | 85%          | Pass                | Clean         | Current       | A     |
| API    | 72%          | Pass                | 2 drift items | Stale (14d)   | B     |
| UI     | 45%          | 1 violation         | 5 drift items | Stale (30d)   | C     |

**Overall: B-**

### Drift Items
- [ ] API: duplicated validation helper in 3 locations
- [ ] UI: inconsistent error handling pattern
- [ ] API docs: doesn't reflect new endpoint added in #98
```

Updated weekly by the gardener process.

**3.3.2 Golden Principles**

```yaml
# team/projects/<project>/invariants/golden-principles.yml
principles:
  - name: shared-utilities-over-hand-rolled
    description: "Prefer shared utility packages over hand-rolled helpers"
    detection: "Find functions with >80% similarity across different modules"
    remediation: "Extract to shared utility, update all call sites"

  - name: validated-boundaries
    description: "Validate data at boundaries, not YOLO-style deep in logic"
    detection: "Find parse/deserialize calls without validation"
    remediation: "Add boundary validation using typed schemas"

  - name: no-dead-code
    description: "Remove unused functions, imports, and variables"
    detection: "Static analysis for unreachable code"
    remediation: "Delete the dead code"

  - name: consistent-error-handling
    description: "Use the project's error handling pattern consistently"
    detection: "Find error handling that doesn't match project pattern"
    remediation: "Refactor to use standard error pattern"
```

**3.3.3 Gardener Hat (`arch_gardener`)**

New hat added to superman's hat roster:

```yaml
# In ralph.yml hats section
arch_gardener:
  purpose: "Periodic codebase cleanup and quality assessment"
  trigger: "Recurring schedule (weekly) or manual invocation"
  workflow:
    1. Scan codebase against golden principles
    2. Run invariant checks, note any new violations
    3. Check doc freshness (modified date vs code changes)
    4. Update QUALITY_SCORE.md
    5. Open targeted refactoring PRs for drift items (small, reviewable in <1 min)
    6. Open fix-up PRs for stale docs
```

**3.3.4 Doc-Gardening**

Integrated into the gardener hat:
- Compare design docs against actual implementation
- Flag knowledge files that reference deleted/renamed code
- Check that PROCESS.md reflects current status graph
- Open fix-up issues or PRs for stale content

#### Acceptance Criteria

- **Given** the gardener hat runs its weekly scan
  **When** it detects duplicated utility code across modules
  **Then** it opens a refactoring PR to extract the shared utility

- **Given** a design doc references a function that was renamed
  **When** the doc-gardening scan runs
  **Then** a fix-up PR is opened updating the reference

- **Given** the gardener hat completes its scan
  **When** it updates QUALITY_SCORE.md
  **Then** the score reflects current test coverage, invariant compliance, and doc freshness

---

### 3.4 Gap 4: Plans as First-Class Artifacts

#### Problem
Design docs exist as files. Story breakdowns exist only in issue comments. No execution plans with progress tracking. No tech-debt tracker.

Harness's approach: *"Plans are treated as first-class artifacts. Complex work is captured in execution plans with progress and decision logs that are checked into the repository. Active plans, completed plans, and known technical debt are all versioned and co-located."*

#### Design

**3.4.1 Knowledge Directory Extension**

```
team/projects/<project>/knowledge/
  designs/                          # ✅ Already exists
    epic-106.md                     # This document
  plans/                            # 🆕
    active/
      epic-106-plan.md              # Execution plan with progress log
    completed/
      epic-24-plan.md               # Archived after epic completion
  generated/                        # 🆕
    board-snapshot.md                # Auto-generated board state
    architecture-map.md             # Auto-generated from code analysis
  product-specs/                    # 🆕
    index.md                        # Product requirements catalog
  references/                       # 🆕
    harness-engineering.md          # External reference docs
  QUALITY_SCORE.md                  # 🆕 (from Gap 3)
  tech-debt-tracker.md              # 🆕
```

**3.4.2 Execution Plan Format**

```markdown
# Execution Plan: Epic #106 — Agentic SDLC Transition

## Status: In Progress

## Stories
| # | Title | Status | Completed |
|---|-------|--------|-----------|
| 1 | Mechanical enforcement | dev:implement | — |
| 2 | Plans structure | qe:test-design | — |

## Decision Log
| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-04-03 | Start with enforcement | Lowest risk, highest impact |
| 2026-04-05 | Skip Phase D for now | No web UI project yet |

## Progress Notes
- 2026-04-03: Epic created, design doc produced
- 2026-04-04: Design approved, planning started
```

**3.4.3 Tech Debt Tracker**

```markdown
# Tech Debt Tracker

## Active Debt

| ID | Description | Source | Priority | Effort |
|----|-------------|--------|----------|--------|
| TD-001 | Duplicated validation in API module | Epic #98 | Medium | 2h |
| TD-002 | Missing error handling in auth flow | Bug #105 | High | 4h |

## Resolved
| ID | Description | Resolved By | Date |
|----|-------------|-------------|------|
```

**3.4.4 Hat Integration**

- `arch_planner` writes execution plans to `plans/active/`, not just issue comments
- `arch_monitor` updates progress in the plan file as stories complete
- On epic completion, `arch_monitor` moves plan to `plans/completed/`
- `dev_implementer` logs discovered tech debt to `tech-debt-tracker.md`
- `arch_gardener` processes the debt tracker periodically

#### Acceptance Criteria

- **Given** the `arch_planner` hat produces a story breakdown
  **When** the breakdown is approved
  **Then** an execution plan file exists in `plans/active/` with story list and status

- **Given** a story reaches `done` status
  **When** `arch_monitor` scans the epic
  **Then** the execution plan's story table is updated to reflect completion

- **Given** all stories in an epic reach `done`
  **When** the epic is accepted
  **Then** the plan file moves from `plans/active/` to `plans/completed/`

---

### 3.5 Gap 5: Graduated Autonomy

#### Problem
3 hard human gates (`po:design-review`, `po:plan-review`, `po:accept`). Agent cannot proceed without explicit human approval. This is the biggest bottleneck.

Harness's evolution: *"Humans may review pull requests, but aren't required to. Over time, we've pushed almost all review effort towards being handled agent-to-agent."*

#### Design

**3.5.1 Trust Tiers**

```yaml
# ralph.yml — per-project autonomy configuration
projects:
  botminter:
    autonomy: supervised    # supervised | guided | autonomous

# Tier definitions:
# supervised (current default):
#   - 3 human gates: po:design-review, po:plan-review, po:accept
#   - Agent waits for human comment at each gate
#   - Human must explicitly approve or reject
#
# guided:
#   - 1 human gate: po:accept only
#   - lead_reviewer approval auto-advances design and plan reviews
#   - po:design-review → auto-advance after lead:design-review passes
#   - po:plan-review → auto-advance after lead:plan-review passes
#   - Human gets async notification of auto-advances
#   - Human can still intervene by commenting on any issue
#
# autonomous:
#   - 0 human gates (async notification only)
#   - All gates auto-advance after agent review passes
#   - Human reviews async via board/notifications
#   - Human can intervene at any time by commenting
```

**3.5.2 `po_reviewer` Hat Modification**

The `po_reviewer` hat checks the project's autonomy tier before gating:

```
# Pseudocode for po_reviewer decision
if autonomy == "supervised":
    # Current behavior: post review request, wait for human comment
    post_review_request()
    wait_for_human_response()

elif autonomy == "guided":
    if gate == "po:accept":
        # Still requires human
        post_review_request()
        wait_for_human_response()
    else:
        # Auto-advance after lead review passed
        post_notification_comment("Auto-approved (guided mode). Lead review passed.")
        auto_advance()

elif autonomy == "autonomous":
    # Auto-advance all gates
    post_notification_comment("Auto-approved (autonomous mode). Agent review passed.")
    auto_advance()
```

**3.5.3 Notification Comments**

When auto-advancing, the agent posts a notification comment so the human has an audit trail:

```markdown
### 📝 po — 2026-04-05T10:30:00Z

**Auto-approved (guided mode)**

Design review auto-advanced after lead review passed.
Lead review: approved by 👑 lead at 2026-04-05T10:25:00Z

To override: comment `Rejected: <feedback>` to revert.
```

**3.5.4 Override Mechanism**

Even in `guided` or `autonomous` mode, the human can intervene at any time:
- Comment `Rejected: <feedback>` on any issue to revert the most recent auto-advance
- Comment `Hold` to pause auto-advances for a specific issue
- Change the `autonomy` setting in `ralph.yml` to downgrade the tier

#### Acceptance Criteria

- **Given** a project configured with `autonomy: guided`
  **When** lead review approves a design doc
  **Then** `po:design-review` auto-advances to `arch:plan` with a notification comment

- **Given** a project configured with `autonomy: guided`
  **When** an epic reaches `po:accept`
  **Then** the agent posts a review request and waits for human comment (same as supervised)

- **Given** a project configured with `autonomy: autonomous`
  **When** any gate is reached
  **Then** the agent auto-advances with a notification comment and the human can override

- **Given** a human comments `Rejected: <feedback>` on an auto-advanced issue
  **When** the agent scans the issue
  **Then** the status reverts and the feedback is processed

---

### 3.6 Gap 6: Metrics and Feedback Loops

#### Problem
No cycle-time tracking. No quality metrics. No way to know if the process is improving. `poll-log.txt` exists for board scan audit but provides no analytics.

Harness's approach: They track quality grades per domain over time and measure throughput (3.5 PRs/engineer/day).

#### Design

**3.6.1 Transition Timestamps**

Board scanner logs every status transition to a JSONL file:

```jsonl
{"issue":106,"type":"Epic","from":"po:triage","to":"po:backlog","ts":"2026-04-03T15:00:00Z","hat":"po_backlog"}
{"issue":107,"type":"Task","from":"qe:test-design","to":"dev:implement","ts":"2026-04-03T15:05:00Z","hat":"qe_test_designer"}
```

Location: `team/metrics/transitions.jsonl` (append-only)

**3.6.2 Derived Metrics**

Computed from the transition log:

| Metric | What It Measures | Target |
|--------|-----------------|--------|
| Design cycle time | `arch:design` to `po:ready` | Trending down |
| Implementation cycle time | `dev:implement` to `qe:verify` | Trending down |
| Human gate wait time | Time in `po:design-review`, `po:plan-review`, `po:accept` | < 4 hours |
| Rejection rate per gate | % of times each gate rejects | < 15% |
| First-pass rate | % of stories that reach `done` without any rejection | > 70% |
| Throughput | Issues completed per day/week | Trending up |
| Bug escape rate | Bugs filed within 30 days of epic completion | Trending down |

**3.6.3 Weekly Quality Report**

Auto-generated, stored in `team/metrics/`:

```markdown
# Weekly Report — 2026-04-07

## Throughput
- Issues completed: 12
- PRs merged: 8
- Epics advanced: 2

## Cycle Times (median)
- Design → Ready: 3.2 days (prev: 4.1 days, -22%)
- Implement → Verify: 1.1 days (prev: 1.5 days, -27%)

## Gates
- Human gate wait time (median): 6.2 hours
- Rejection rate: 18% (target: <15%)
  - Code review: 25% (↑ — investigate)
  - QE verify: 12% (✓)

## Quality
- Invariant violations caught pre-review: 4
- Invariant violations reaching review: 0 (✓)
- Test coverage delta: +3.2%

## Action Items
- Code review rejection rate is above target. Review top rejection reasons.
```

**3.6.4 Retrospective Integration**

The existing `retrospective` skill receives metrics as input:
- Instead of "what went well?" → "design review took 3x longer on epic #24 than #18 — here's why"
- Data-driven action items backed by measured trends
- Retro outputs stored in `team/agreements/retros/` per existing convention

**3.6.5 Board Scanner Integration**

The board scanner (`board-scanner` skill) adds a single line per transition:

```bash
# In board scanner, after each status transition
echo '{"issue":'$ISSUE',"type":"'$TYPE'","from":"'$FROM'","to":"'$TO'","ts":"'$(date -u +%FT%TZ)'","hat":"'$HAT'"}' \
  >> team/metrics/transitions.jsonl
```

#### Acceptance Criteria

- **Given** the board scanner transitions an issue's status
  **When** the transition completes
  **Then** a JSONL entry is appended to `team/metrics/transitions.jsonl`

- **Given** a week of transition data exists
  **When** the weekly report generator runs
  **Then** a report is produced with cycle times, rejection rates, and throughput

- **Given** the retrospective skill is invoked
  **When** metrics data is available
  **Then** the retro includes data-driven observations and specific action items

---

## 4. Data Models

### 4.1 Architecture Layers Schema
```yaml
# YAML schema for architecture-layers.yml
type: object
properties:
  layers:
    type: array
    items:
      type: object
      properties:
        name: { type: string }
        description: { type: string }
        allowed_imports: { type: array, items: { type: string } }
  cross_cutting:
    type: array
    items:
      type: object
      properties:
        name: { type: string }
        description: { type: string }
        accessible_from: { type: array, items: { type: string } }
```

### 4.2 Golden Principles Schema
```yaml
# YAML schema for golden-principles.yml
type: object
properties:
  principles:
    type: array
    items:
      type: object
      properties:
        name: { type: string }
        description: { type: string }
        detection: { type: string }
        remediation: { type: string }
```

### 4.3 Trust Tier Schema
```yaml
# In ralph.yml
projects:
  <project-name>:
    autonomy:
      type: string
      enum: [supervised, guided, autonomous]
      default: supervised
```

### 4.4 Transition Log Entry
```json
{
  "issue": "integer — issue number",
  "type": "string — Epic|Task|Bug",
  "from": "string — previous status",
  "to": "string — new status",
  "ts": "string — ISO 8601 UTC timestamp",
  "hat": "string — hat that performed the transition"
}
```

---

## 5. Error Handling

### 5.1 Mechanical Enforcement Failures
- If a check script crashes (not just fails), the `dev_code_reviewer` hat reports the error and continues with remaining checks
- A crashed check does not block review — it's logged as a warning
- After 3 consecutive crashes, the check is flagged for human attention

### 5.2 Observability Stack Failures
- If the observability stack fails to start, the agent proceeds without it (degraded mode)
- A warning comment is posted on the issue
- Investigation and fix proceed without telemetry (current behavior)

### 5.3 Auto-Advance Failures
- If an auto-advance in `guided`/`autonomous` mode fails (e.g., status transition error), fall back to supervised behavior
- Post a comment explaining the fallback
- Retry on next scan cycle

### 5.4 Gardener Hat Failures
- If the gardener scan fails, it posts a failure comment and retries on the next cycle
- Does not block any other work
- Maximum 3 retries before flagging for human attention

---

## 6. Impact on Existing System

### 6.1 Modified Components

| Component | Change | Risk |
|-----------|--------|------|
| `ralph.yml` | Add `autonomy` field per project | Low — new field, backward compatible (defaults to `supervised`) |
| `po_reviewer` hat | Check autonomy tier before gating | Medium — core workflow change |
| `dev_code_reviewer` hat | Run executable invariant checks | Low — additive capability |
| `qe_verifier` hat | Run invariant checks as part of verification | Low — additive |
| `arch_planner` hat | Write execution plans to files | Low — additive |
| `arch_monitor` hat | Update plan files, move to completed | Low — additive |
| Board scanner skill | Log transitions to JSONL | Low — additive |

### 6.2 New Components

| Component | Type | Purpose |
|-----------|------|---------|
| `arch_gardener` hat | Hat | Periodic codebase cleanup and quality assessment |
| `architecture-layers.yml` | Config | Machine-readable layer definitions |
| `golden-principles.yml` | Config | Mechanical consistency rules |
| `checks/` scripts | Executable | Invariant check scripts |
| `plans/` directory | Knowledge | Execution plans and tech debt tracker |
| `metrics/` directory | Data | Transition logs and weekly reports |
| `QUALITY_SCORE.md` | Knowledge | Per-domain quality grading |

### 6.3 Backward Compatibility

- All changes are additive or opt-in
- Default `autonomy: supervised` preserves current behavior
- Existing invariant markdown files remain as reference documentation
- No existing hat behavior changes unless the project opts in

---

## 7. Security Considerations

### 7.1 Graduated Autonomy
- `autonomous` mode removes human gates — ensure agent review quality is sufficient before enabling
- Override mechanism (`Rejected:` comment) provides emergency brake
- Audit trail preserved via notification comments on every auto-advance
- Recommendation: start with `guided` on one low-risk project; monitor rejection rates before upgrading

### 7.2 Mechanical Enforcement
- Check scripts execute in CI/agent context — must not introduce command injection
- Scripts should be read-only (analyze, not modify) during the check phase
- Only the gardener hat's cleanup PRs should modify code

### 7.3 Metrics Data
- Transition logs contain issue numbers and status names, not sensitive data
- Stored in `team/` repo, same access control as other team artifacts

### 7.4 Observability Stack
- Per-worktree stacks are ephemeral — torn down after task completes
- No production data enters the local stack — only test/dev data
- Stack runs locally, not exposed to network

---

## 8. Implementation Phases

| Phase | Scope | Stories (est.) | Risk | Impact |
|---|---|---|---|---|
| 1 | Mechanical Enforcement | 3-4 | Low | High |
| 2 | Plans + Knowledge Structure | 2-3 | Low | Medium |
| 3 | Garbage Collection | 3-4 | Low | High |
| 4 | Metrics | 2-3 | Low | Medium |
| 5 | Graduated Autonomy | 2-3 | Medium | High |
| 6 | Application Legibility | 4-6 | Medium-High | High |

**Recommended order:** 1 → 2 → 3 → 4 → 5 → 6 (enforcement first, autonomy after quality infrastructure is in place)

---

## 9. Success Criteria

| Metric | Current | Target |
|---|---|---|
| Human gate wait time | Unknown | < 4 hours median |
| Rejection rate at code review | Unknown | < 15% |
| Invariant violations reaching review | Some (untracked) | Zero |
| Stale knowledge docs | Unknown | Detected within 1 week |
| Autonomy tier | `supervised` only | `guided` on >= 1 project |
| Cycle time trend | Untracked | Measured and improving |
| First-pass success rate | Unknown | > 70% |

---

## 10. References

- [OpenAI Harness Engineering (Feb 2026)](https://openai.com/index/harness-engineering/)
- [Agentic SDLC Blueprint — BayTech Consulting](https://www.baytechconsulting.com/blog/agentic-sdlc-ai-software-blueprint)
- [How Agentic AI Reshapes Engineering Workflows — CIO](https://www.cio.com/article/4134741/how-agentic-ai-will-reshape-engineering-workflows-in-2026.html)
- BotMinter PROCESS.md — current status graph and workflow conventions
- BotMinter team/invariants/ — current invariant definitions
