# Design: Transition BotMinter to Fully Agentic SDLC

**Epic:** #106
**Author:** bob (superman)
**Date:** 2026-04-04
**Status:** Draft

---

## 1. Overview

BotMinter's scrum-compact profile ships a complete agentic SDLC: 18 specialized hats, board-driven dispatch across a 34-status workflow graph, a four-level knowledge hierarchy, and rejection loops at every review gate. The machinery exists. The problem is that it runs on trust.

Invariants are prose markdown that instruct agents what to do — but nothing mechanically prevents violations. ADR-0007 establishes domain-command layering: domain modules must not use `println!`, `eprintln!`, or reference CLI libraries. Today, `println!` calls and `clap` imports are confined to the command layer (`main.rs`, `cli.rs`, `agent_main.rs`, `agent_cli.rs`, `commands/`) — but `eprintln!` appears in 5 domain modules:

| Module | Lines |
|---|---|
| `profile/agent.rs` | 151, 159 |
| `bridge/provisioning.rs` | 81, 183 |
| `formation/start_members.rs` | 167 |
| `formation/local/linux/mod.rs` | 209 |
| `git/manifest_flow.rs` | 244, 249, 252 |

These are existing ADR-0007 violations that no tooling catches. The `dev_code_reviewer` hat reads the prose invariant, applies judgment, and hopes for the best.

The same gap applies to other mechanically-checkable rules: `test-path-isolation` (tests must use temp directories for `$HOME` — a grep for `home_dir()` outside test setup would catch violations), `no-hardcoded-profiles` (no hardcoded profile/role names in code — also greppable), and `file-size-limit` per ADR-0006. None are automated.

The project has 8 test-related invariants (`e2e-scenario-coverage`, `exploratory-test-scope`, `exploratory-test-single-journey-smell`, `exploratory-test-user-journey`, `flaky-tests`, `gh-api-e2e`, `test-path-isolation`, `zero-test-failures`) and 11 ADRs — all enforced by prose and trust.

The enforcement gap compounds across the pipeline. Design reviews that should interrogate content instead check a process list. Code reviews that should validate architectural rules instead trust the implementer. Verification that should confirm structured test output instead parses raw console text.

OpenAI's Harness Engineering team built a production product — ~1M lines of code, five months, three engineers, zero manually-written code. The core lesson:

> *"When something failed, the fix was almost never 'try harder.' Human engineers always asked: 'what capability is missing, and how do we make it both legible and enforceable for the agent?'"*

Their engineers stopped writing code and started building environments where agents produce correct code by construction. Custom linters catch architecture violations with error messages written *for agents*. A structured knowledge base replaces the monolithic instruction file. Golden principles are encoded once and enforced automatically. Quality is maintained by a garbage-collection process that detects and fixes drift daily.

This design proposes six features that close the gap between BotMinter's existing process structure and mechanical enforcement. Each feature is a scrum-compact profile enhancement. No changes to the Ralph Orchestrator product (ralph.yml schema, event loop, hat dispatch engine). Three features modify profile-level orchestration artifacts — board-scanner skill instructions, hat definitions, and the `botminter.yml` manifest schema. Section 2.4 maps each change to its product layer.

### Scope

All changes target the scrum-compact profile and the BotMinter project. This is pre-alpha software — changes are delivered directly, not behind compatibility shims or feature flags.

Three features modify profile-level orchestration artifacts:
- **Feature 3 (Garbage Collection)** adds a `gardener.scan` event and an `arch_gardener` hat to the board-scanner dispatch table and role definition.
- **Feature 5 (Graduated Autonomy)** changes the `po_reviewer` hat's behavior based on a runtime config value read from `botminter.yml`.
- **Feature 6 (Metrics)** adds a JSONL write step to the board-scanner skill's transition logging.

These are profile-level changes (hat definitions, skill instructions, manifest schema), not changes to Ralph's core engine.

### Out of Scope

- Ralph Orchestrator product changes
- New profiles (features enhance scrum-compact)
- New GitHub Projects statuses
- Formation system or bridge system changes
- ADR-0011 (per-member GitHub App identity) implementation — see Section 6.5 for interaction analysis

---

## 2. Architecture

### 2.1 BotMinter Architecture

BotMinter has four runtime components:

| Component | Module | Description |
|---|---|---|
| **CLI** (`bm`) | `commands/`, `cli.rs`, `main.rs` | Primary interface. Profile init, team management, member lifecycle. |
| **Agent CLI** (`bm-agent`) | `commands/`, `agent_cli.rs`, `agent_main.rs` | Agent-facing interface. Inbox, loop start, Claude hooks. |
| **HTTP Daemon** | `daemon/` (9 files) | axum server. Webhook endpoints, polling mode, REST API for loop/member management. Started via `bm daemon start`. |
| **Web Console** | `web/` (9 files) | Embedded axum Router. REST endpoints (`/api/teams/`, file endpoints), web UI via `rust-embed`. Accessible at `http://localhost:{port}`. |

The codebase has two binaries (`bm` and `bm-agent`) built from `crates/bm/`. Source is organized into a command layer and 15 domain modules:

**Command layer** (thin wrappers per ADR-0007): `commands/`, `main.rs`, `cli.rs`, `agent_main.rs`, `agent_cli.rs`

**Domain modules** (all directories under `crates/bm/src/` except `commands/`):

| Module | Purpose |
|---|---|
| `acp/` | Agent Control Protocol client and types |
| `agent_tags/` | Agent tag management |
| `brain/` | Brain adapter, event watcher, heartbeat, inbox, queue |
| `bridge/` | Credential, identity, lifecycle, provisioning, room management |
| `chat/` | Chat config and skills |
| `config/` | Configuration management |
| `daemon/` | HTTP daemon — axum server, webhook handling, daemon lifecycle |
| `formation/` | Member start/stop, local/Linux/macOS topology |
| `git/` | GitHub App auth, manifest flow, project management |
| `profile/` | Profile extraction, manifest parsing, team repo operations |
| `session/` | Session management |
| `state/` | Application state, dashboard data |
| `topology/` | Topology definitions |
| `web/` | Web console — axum Router, REST API, embedded assets |
| `workspace/` | Workspace repo, sync, team sync |

Key dependencies: `axum` (0.8), `tower-http`, `tokio`, `rust-embed` (optional, `console` feature), `reqwest`, `clap` (4, derive).

This architecture matters for this design because:
- Feature 1 (checks) must scan all 15 domain modules, not a subset
- Feature 2 (legibility) must account for daemon startup and API validation, not just CLI output
- The daemon and web modules are domain modules — ADR-0007's layering rules apply to them

### 2.2 Mapping Harness Techniques to BotMinter

Harness's techniques evolved over five months. Some patterns match BotMinter's structure. Others expose genuine gaps.

| Harness Technique | What BotMinter Has | What's Missing |
|---|---|---|
| Agent specialization | 18 hats with distinct instructions and backpressure rules | Structural match. |
| Agent-to-agent review chain | `lead_reviewer` → `dev_code_reviewer` → `qe_verifier` pipeline | Reviews apply process checklists, not adversarial technical interrogation. |
| AGENTS.md as table of contents | CLAUDE.md → `knowledge/` hierarchy → hat knowledge | Match in structure. |
| Structured `docs/` as system of record | Knowledge hierarchy (team, project, member, hat). Design docs in project knowledge. | Planning artifacts fragmented across four methodology phases. No execution plans. |
| Custom linters with agent-readable error messages | 11 prose invariants in `projects/botminter/invariants/`. 11 ADRs. 2 profile-generic invariants in `team/invariants/`. | **Zero mechanical enforcement.** Existing ADR-0007 violations (`eprintln!` in 5 domain modules) go undetected. |
| Per-worktree app boot + Chrome DevTools for UI | Formation abstraction (ADR-0008). Local and K8s formations. HTTP daemon + embedded web console (`rust-embed`). | No per-worktree boot for agent use. No structured test output. Daemon and API are not part of the agent's test loop. Console UI exists but agents don't test it. |
| Golden principles + doc-gardening | Nothing. | No quality scoring, no stale-doc detection, no automated cleanup. |
| Plans as first-class artifacts | Design docs exist. Story breakdowns in issue comments. | No execution plans. No living progress documents. |
| Graduated autonomy | Three fixed human gates: design-review, plan-review, accept. | No progression path. |
| Transition logging and metrics | `poll-log.txt` audit log. | No structured metrics. Cannot answer "what's the rejection rate?" |

### 2.3 The Enforcement Gap

BotMinter has 11 project-specific invariants (in `projects/botminter/invariants/`) and 2 profile-generic invariants (in `team/invariants/`). Three project invariants illustrate the problem:

**`test-path-isolation`** requires tests set `$HOME` to a temporary directory. A check script could grep for `dirs::home_dir()` or `env::home_dir()` outside test setup. No such script exists.

**`no-hardcoded-profiles`** requires no hardcoded profile/role/status names in code. A check script could grep for known string literals. No such script exists.

**ADR-0007 (domain-command layering)** requires domain modules to not import CLI libraries or format terminal output. The command layer is clean — `clap` imports and `println!` are confined there. But 5 domain modules contain `eprintln!` calls violating ADR-0007's rule "domain modules do NOT format output for the terminal":

| Module | Lines | What It Does |
|---|---|---|
| `profile/agent.rs` | 151, 159 | Status messages during Minty config init |
| `bridge/provisioning.rs` | 81, 183 | Progress and warning output during provisioning |
| `formation/start_members.rs` | 167 | Multi-line status during member start |
| `formation/local/linux/mod.rs` | 209 | Status message during daemon start |
| `git/manifest_flow.rs` | 244, 249, 252 | Progress messages during App installation check |

A check script scanning all 15 domain module directories (all directories under `crates/bm/src/` except `commands/`) for `println!`, `eprintln!`, and `clap` imports would flag these immediately. No such script exists.

The project also has 8 test-related invariants — all prose-only. The e2e test harness (`tests/e2e/` with `libtest-mimic`, `--features e2e`) validates runtime behavior, but check scripts could enforce static properties of tests (path isolation, scenario coverage patterns).

Harness built custom linters because error messages could be written *for agents*:

> *"Because the lints are custom, we write the error messages to inject remediation instructions into agent context."*

### 2.4 Where Changes Land

Every change maps to a specific BotMinter product layer:

| Change | Product Layer | Location (from workspace root) |
|---|---|---|
| Profile-generic check scripts | Profile: invariant checks | `team/invariants/checks/` |
| Project-specific check scripts | Project repo: invariant checks | `projects/<project>/invariants/checks/` |
| Check runner script | Profile: skill | `team/coding-agent/skills/check-runner/run-checks.sh` |
| Check script contract doc | Profile: knowledge | `team/knowledge/check-script-contract.md` |
| Gardener hat definition | Profile: role definition | `ralph.yml` hats entry + board-scanner dispatch table |
| Reviewer hat updates | Profile: hat instructions | `ralph.yml` hat instructions for `dev_code_reviewer`, `qe_verifier` |
| Golden principles config | Project repo: invariants | `projects/<project>/invariants/golden-principles.yml` |
| Plan directory template | Team repo: project plans | `team/projects/<project>/plans/` |
| Structured test output convention | Profile: knowledge | `team/knowledge/structured-test-output.md` |
| Dev-boot configuration | Team repo: project knowledge | `team/projects/<project>/knowledge/dev-boot.yml` |
| Autonomy manifest field | Manifest schema | `team/botminter.yml` (`autonomy` field) |
| Autonomy CLI parsing | CLI source | `crates/bm/src/profile/manifest.rs` |
| Transition JSONL logging | Profile: skill instructions | Board-scanner skill instructions |
| Metrics JSONL file | Workspace artifact | `metrics/transitions.jsonl` |
| CLAUDE.md updates | Project repo | `projects/<project>/CLAUDE.md` |

---

## 3. Components and Interfaces

### 3.1 Feature 1: Executable Invariant Checks

#### Problem

The `dev_code_reviewer` hat is instructed to "verify compliance with project invariants." It reads the prose files and attempts to check them. Without tooling, the hat applies inconsistent judgment. ADR-0007 violations (`eprintln!` in 5 domain modules) have persisted through multiple code review cycles because no tool flags them.

Harness's insight: encode rules as executable checks with structured, agent-readable output.

> *"Because the lints are custom, we write the error messages to inject remediation instructions into agent context."*

#### Design

**Check script contract.** Each check is a shell script:

- **Working directory:** The check runner sets `cwd = projects/<project>/` (the project repo root). All relative paths resolve against the project repo. A script grepping `crates/bm/src/` works because `cwd` is `projects/botminter/`.
- **Exit codes:** Exit 0 = pass. Exit 1 with VIOLATION output on stdout = violation. Exit 1 without VIOLATION output, or exit > 1 = script crash (logged as warning, does not block review).
- **Output format on violation (exit 1):**

```
VIOLATION: Domain module crates/bm/src/bridge/provisioning.rs uses eprintln! (lines 81, 183)
RULE: ADR-0007 domain-command layering — domain modules must not format output for the terminal
REMEDIATION: Return structured Result types from domain functions. Use tracing::warn! or tracing::info! for diagnostic output. Let the command layer decide how to display.
REFERENCE: .planning/adrs/0007-domain-command-layering.md
```

The `REMEDIATION` line gives the agent its next action. The `REFERENCE` line points to the governing rule. This follows ADR-0002's design principle: structured output from shell scripts.

**Two check script scopes.** Check scripts live in two locations:

| Scope | Location (from workspace root) | Contains | Example |
|---|---|---|---|
| Profile-generic | `team/invariants/checks/` | Checks for any project using scrum-compact | `file-size-limit.sh` (ADR-0006), `test-path-isolation.sh` |
| Project-specific | `projects/<project>/invariants/checks/` | Checks for this project's ADRs and invariants | `domain-layer-imports.sh` (ADR-0007), `no-hardcoded-profiles.sh` |

Profile-generic checks are extracted from the profile into the team repo during `bm init`. Project-specific checks are authored in the project repo.

**Check runner.** A runner script at `team/coding-agent/skills/check-runner/run-checks.sh`:

1. Accepts a project name argument (e.g., `botminter`)
2. Discovers all `.sh` files in `team/invariants/checks/` and `projects/<project>/invariants/checks/`
3. Executes each with `cwd = projects/<project>/`
4. Distinguishes violations (exit 1 + VIOLATION output) from crashes (exit 1 without VIOLATION, or exit > 1)
5. Aggregates results: all violations reported, crashes logged as warnings
6. Exits 0 if all checks pass, exits 1 if any violations found

**Baseline check scripts.**

Profile-generic (extracted to `team/invariants/checks/`):

| Script | Invariant | Pattern | False Positives |
|---|---|---|---|
| `file-size-limit.sh` | ADR-0006 | `wc -l` on `.rs` source files, fail if >300 non-test lines | Generated files (none currently). Exclude `target/`. |
| `test-path-isolation.sh` | `test-path-isolation` | Grep for `dirs::home_dir()` and `std::env::home_dir()` in test files outside `setup` functions | Legitimate `home_dir` references in production code (exclude non-test files). |

Project-specific (in `projects/botminter/invariants/checks/`):

| Script | Invariant | Pattern | False Positives |
|---|---|---|---|
| `domain-layer-imports.sh` | ADR-0007 | Scan all directories under `crates/bm/src/` except `commands/`, `main.rs`, `cli.rs`, `agent_main.rs`, `agent_cli.rs`. Grep for `use clap`, `println!`, and `eprintln!`. Uses directory exclusion — new domain modules are automatically scanned. | None expected. All matches are real violations per ADR-0007. The 5 existing `eprintln!` calls are genuine violations. |
| `no-hardcoded-profiles.sh` | `no-hardcoded-profiles` | Grep for string literals matching known profile names (`scrum-compact`, `scrum`), role names, and status values in `.rs` files. Exclude `tests/`, `fixtures/`, and `profiles/` directories. | Test fixtures that intentionally reference profile names (excluded by directory). |

**First run impact.** The `domain-layer-imports.sh` script will immediately flag the 5 existing `eprintln!` violations. These are real violations — the fix is to replace `eprintln!` with `tracing::warn!`/`tracing::info!` or return structured errors to the command layer. Resolving these existing violations is the first concrete deliverable of this feature.

**CI integration.** Check scripts are shell scripts — they run anywhere `bash` runs:
- CI invokes the same runner hats use: `bash team/coding-agent/skills/check-runner/run-checks.sh botminter`
- CI runs checks on every PR targeting the project repo
- The e2e test harness (existing `libtest-mimic` in `tests/e2e/`) is separate — check scripts validate static properties, e2e tests validate runtime behavior
- Exploratory tests (per invariants `exploratory-test-scope`, `exploratory-test-user-journey`) remain agent-driven. Check scripts complement them by catching static violations before the agent starts testing.

**Hat integration.** Two hats gain check-running steps:
- `dev_code_reviewer`: runs the check runner before reviewing. Violations reject to `dev:implement` with VIOLATION/REMEDIATION output as feedback.
- `qe_verifier`: runs the check runner as part of verification. Violations block verification.

Hat instructions specify: `bash team/coding-agent/skills/check-runner/run-checks.sh botminter`. The runner handles cwd, discovery, and error classification.

**Prose invariants remain.** Not every rule can be mechanically checked. `cli-idempotency` requires behavioral testing. `exploratory-test-user-journey` requires judgment about test scope. Prose invariants stay for rules that need judgment. Check scripts handle what can be automated.

#### Acceptance Criteria

- **Given** a code change introduces `println!` in a domain module, **when** `dev_code_reviewer` runs invariant checks, **then** the check fails with a structured VIOLATION/REMEDIATION message and the story returns to `dev:implement`.
- **Given** all checks pass, **when** `dev_code_reviewer` runs checks, **then** review proceeds normally.
- **Given** the existing 5 `eprintln!` violations in domain modules, **when** the check runner executes `domain-layer-imports.sh`, **then** each violation is reported with module path, line numbers, and remediation instructions.
- **Given** a new `.sh` script is added to either checks directory, **when** subsequent reviews run the check runner, **then** the new check is discovered and executed.
- **Given** a check script has a syntax error, **when** the runner executes it, **then** the crash is logged as a warning but does not block review.
- **Given** the check runner is invoked in CI, **when** a PR introduces a domain-layer violation, **then** CI fails with the same VIOLATION/REMEDIATION output agents see.

---

### 3.2 Feature 2: Application Legibility

#### Problem

When `qe_investigator` investigates a bug, it reads source code. It has no running application to probe, no structured test output to parse. Harness addressed this by making the application itself legible to agents:

> *"We made the app bootable per git worktree, so Codex could launch and drive one instance per change."*

BotMinter has four runtime components — CLI, agent CLI, HTTP daemon, and embedded web console. Agents currently interact only with the CLI. The daemon's REST API (`/api/teams/`, member management, webhook handling), polling mode, and web console are not part of the agent's test loop. The e2e test harness exists (`tests/e2e/` with `libtest-mimic`, gated behind `--features e2e`) but produces unstructured text output.

#### Design

**Phase A: Structured test output.**

`cargo test` produces raw console text. Agents pattern-match against this to find failures — fragile, loses detail. The profile adds a knowledge convention defining structured JSON output:

```json
{"test": "formation::local::test_start_members", "status": "FAIL", "duration_ms": 1204, "error": "assertion failed: member.is_healthy()", "file": "crates/bm/src/formation/local/mod.rs", "line": 245}
```

Implementation: a wrapper script (shipped with the profile, extracted to `team/coding-agent/skills/`) runs `cargo test` and post-processes output into JSON. The wrapper supports both unit tests (`cargo test`) and e2e tests (`cargo test --features e2e`). Hats (`qe_investigator`, `dev_implementer`, `qe_verifier`) use this wrapper to navigate directly to failures.

Enforcement: a check script (Feature 1) verifies that test commands in hat instructions use the structured output wrapper.

**Phase B: Dev-environment bootstrapping.**

Projects with a runnable application define boot/teardown configuration in `team/projects/<project>/knowledge/dev-boot.yml`:

```yaml
dev_boot:
  steps:
    - name: "Build"
      command: "cargo build --features console"
    - name: "Start daemon"
      command: "cargo run --features console -- daemon start --team test-team"
      health_check: "curl -sf http://localhost:8080/health"
      teardown: "cargo run --features console -- daemon stop --team test-team"
    - name: "Verify API"
      command: "curl -sf http://localhost:8080/api/teams/"
  isolation: worktree
```

This file lives in the team repo's project knowledge directory — project-specific and consumed by hats via knowledge resolution. It is not in `botminter.yml` because dev-boot is a project concern, not a team-level manifest field.

The `dev_implementer` and `qe_verifier` hats check for `dev-boot.yml`. If present, they boot the application with `cwd = projects/<project>/`, validate behavior at runtime, and tear down when done.

For BotMinter specifically, dev-boot covers:
1. **Build** with the `console` feature (enables web UI via `rust-embed`)
2. **Daemon startup** via `bm daemon start` — starts the axum HTTP server with webhook endpoints and polling mode
3. **API validation** — verify REST endpoints respond (`/api/teams/`, member endpoints)
4. **Web console** — verify the embedded UI serves at the console URL (`http://localhost:{port}`)

This aligns with ADR-0008's formation abstraction. Each worktree gets an isolated daemon instance.

**Phase C: Daemon and API testing integration.**

The daemon (`crates/bm/src/daemon/`) exposes REST endpoints for loop management (`StartLoopRequest`/`StartLoopResponse`), member management (`StartMembersRequest`, `StopMembersRequest`), and webhook handling (`GitHubEvent` with signature validation). The web module (`crates/bm/src/web/`) serves team overview, member status, file, and sync endpoints.

- The structured test output wrapper (Phase A) covers integration tests exercising the daemon API
- The e2e test harness can boot a daemon instance and validate API behavior
- The `qe_verifier` hat validates API endpoints when `dev-boot.yml` specifies daemon steps

**Phase D deferred.** Browser automation for the web console is deferred until Phases A-C prove useful.

#### Acceptance Criteria

- **Given** tests run during implementation, **when** they complete, **then** structured JSON output is available with test name, status, duration, error, file, and line.
- **Given** a project with `dev-boot.yml`, **when** `dev_implementer` works on a story, **then** the daemon boots, API endpoints respond, and the agent validates behavior at runtime.
- **Given** an e2e test exercises the daemon API, **when** it completes, **then** structured output includes the endpoint, response status, and failure detail.

---

### 3.3 Feature 3: Garbage Collection

#### Problem

Harness found that agents replicate whatever patterns exist in the repo:

> *"Our team used to spend every Friday (20% of the week) cleaning up 'AI slop.' Unsurprisingly, that didn't scale. Instead, we started encoding what we call 'golden principles' directly into the repository and built a recurring cleanup process."*

BotMinter's codebase has evolved through four methodology phases. Planning artifacts exist across `.planning/`, `specs/`, `docs/`, and knowledge directories. No process detects stale documentation, duplicated utility code, or quality drift.

#### Design

**Gardener hat.** A new hat (`arch_gardener`) added to the superman role definition in `ralph.yml`. Triggered by `gardener.scan` events.

**Scheduling and coexistence with normal dispatch.** The board-scanner skill gains a `gardener.scan` dispatch entry. This is a profile-level orchestration change — it modifies the scanner skill's instructions and adds a new event type.

Scheduling mechanism:
1. The scanner tracks a cycle counter in its scratchpad
2. A knowledge file (`team/knowledge/gardener-config.md`) specifies the interval (e.g., "run gardener every 10 scan cycles")
3. When the counter reaches the interval AND no higher-priority items are on the board, the scanner emits `gardener.scan`
4. If any issue is at a dispatch-ready status, the issue takes precedence — the gardener waits

**How the gardener coexists with normal work:** The gardener is the lowest-priority dispatch item. It only runs when the board has no actionable issues. This means:
- Active stories, bugs, and epics always take precedence over gardening
- The gardener fills idle time between work items
- If a new issue enters the board during a gardener cycle, the scanner dispatches the issue on the next cycle (the gardener completes its current atomic pass, then yields)
- The gardener never blocks issue processing — its output IS issue processing (it creates issues for violations, which flow through the normal `dev:implement` → `dev:code-review` → `qe:verify` pipeline)

The gardener hat:
1. Runs the check runner (Feature 1) and logs aggregate results
2. Scans code against golden principles (below) — detects pattern violations
3. Checks documentation freshness: do referenced functions, files, and paths still exist?
4. Produces a quality score per domain area (coverage, invariant compliance, doc freshness)
5. Opens targeted issues for violations — specific files, specific fixes, specific rationale
6. If more than 7 days have elapsed since the last metrics report (checked via file timestamp of the latest report in `team/projects/<project>/knowledge/reports/`), creates a `cw:write` issue for the weekly summary (see Feature 6)

**Golden principles.** A YAML file in `projects/<project>/invariants/golden-principles.yml`. Split into two enforcement categories:

**Mechanically checkable** — become check scripts (Feature 1). Each principle gets a corresponding script in `projects/<project>/invariants/checks/`:

```yaml
principles:
  mechanically_checkable:
    - name: consistent-error-handling
      description: "Use anyhow for application errors, thiserror for library errors"
      check_script: "consistent-error-handling.sh"
      remediation: "Standardize on the crate's chosen error strategy"
```

**Judgment-based** — the gardener applies LLM judgment. This is honest: these principles are trust-based, same as prose invariants, but centralized and applied on a recurring schedule by a dedicated hat:

```yaml
principles:
  judgment_based:
    - name: shared-utilities-over-duplicated
      description: "Prefer shared utility modules over copy-pasted helpers"
      detection_hint: "Look for functions with similar names and signatures across different modules"
      remediation: "Extract to shared module, update call sites"

    - name: typed-boundaries
      description: "Parse and validate at system boundaries, not inline"
      detection_hint: "Look for raw string parsing or deserialization outside config/input modules"
      remediation: "Move parsing to boundary layer, pass typed data internally"
```

The `detection_hint` tells the gardener what to look for, not how to find it mechanically. The distinction from scattered prose invariants: golden principles are centralized, versioned, and applied on schedule.

#### Acceptance Criteria

- **Given** the board has no actionable issues and the gardener interval is reached, **when** the scanner dispatches, **then** `gardener.scan` fires.
- **Given** the gardener scans and finds duplicated utility code, **then** it opens an issue naming specific files and a remediation plan.
- **Given** a knowledge file references a renamed function, **when** the freshness check runs, **then** a fix-up issue is opened.
- **Given** an active story enters the board during a gardener cycle, **when** the scanner dispatches next, **then** the story takes priority.
- **Given** more than 7 days since the last metrics report, **when** the gardener scans, **then** it creates a `cw:write` issue for the weekly summary.

---

### 3.4 Feature 4: Plans as First-Class Artifacts

#### Problem

Harness treats plans as versioned, in-repo artifacts:

> *"Active plans, completed plans, and known technical debt are all versioned and co-located, allowing agents to operate without relying on external context."*

BotMinter's planning artifacts are scattered. The project repo contains: 11 ADRs in `.planning/adrs/`, feature specs in `specs/`, documentation in `docs/`, knowledge in `knowledge/`, design docs in the team repo. Story breakdowns exist only in GitHub issue comments.

#### Design

**Execution plans.** When `arch_planner` produces a story breakdown, it creates an execution plan — a living document:

```markdown
# Execution Plan: Epic #106 — Transition BotMinter to Fully Agentic SDLC

## Status: In Progress

## Stories
| # | Title | Status | Completed |
|---|-------|--------|-----------|

## Key Decisions
| Date | Decision | Rationale |
|------|----------|-----------|

## Progress Notes
- 2026-04-04: Stories created, implementation starting
```

Plans live at `team/projects/<project>/plans/`.

**Hat integration:**
- `arch_planner`: creates the plan when a breakdown is approved
- `arch_monitor`: updates the plan as stories complete, logs key decisions
- On epic completion: plan status -> Completed

**Artifact home convention:**

| Artifact | Home | Rationale |
|----------|------|-----------|
| ADRs | Project repo (`.planning/adrs/`) | Codebase decisions. Live with the code per ADR-0001. |
| Design docs | `team/projects/<project>/knowledge/designs/` | Design context consumed by hats. |
| Execution plans | `team/projects/<project>/plans/` | Living documents tracking execution. |
| Knowledge | Team repo knowledge hierarchy | Advisory context loaded on-demand. |
| Invariants | Profile (`team/invariants/`) or project repo (`projects/<project>/invariants/`) | Constraints enforced by hats and check scripts. |

#### Acceptance Criteria

- **Given** `arch_planner` produces a breakdown, **when** approved, **then** an execution plan exists at `team/projects/<project>/plans/`.
- **Given** a story completes, **when** `arch_monitor` scans, **then** the plan is updated.
- **Given** all stories complete, **when** the epic is accepted, **then** the plan is marked completed.

---

### 3.5 Feature 5: Graduated Autonomy

#### Problem

The scrum-compact profile hard-codes three human gates: `po:design-review`, `po:plan-review`, `po:accept`. Every epic hits all three regardless of agent output quality.

Harness progressively reduced human involvement as enforcement matured:

> *"Humans may review pull requests, but aren't required to. Over time, we've pushed almost all review effort towards being handled agent-to-agent."*

Critical sequencing: Harness built mechanical enforcement *first*, then graduated to reduced oversight.

#### Design

**Manifest extension.** The `team/botminter.yml` manifest gains an `autonomy` field:

```yaml
autonomy:
  tier: supervised    # supervised | guided | autonomous
```

| Tier | Human Gates | Prerequisite |
|------|-------------|--------------|
| `supervised` (default) | design-review, plan-review, accept | None — current behavior |
| `guided` | accept only | Executable checks passing consistently. Rejection rate < 15% at lead review (measured by Feature 6). |
| `autonomous` | none — async notification only | Quality metrics sustained at `guided` for at least one full epic cycle. |

**CLI implementation.** The `ProfileManifest` struct in `crates/bm/src/profile/manifest.rs` gains an `autonomy` field. The field is written to `team/botminter.yml` during extraction and persists as the runtime source of truth.

**Manifest-to-runtime flow.** `team/botminter.yml` IS the runtime config — the codebase reads it via `read_team_repo_manifest()` (in `profile/team_repo.rs`). Hats access the team repo at `team/`. The flow:

1. **Extraction:** `bm init` writes the `autonomy` field to `team/botminter.yml` (default: `supervised`)
2. **Change:** Operator runs `bm teams sync` to update the tier. Agents cannot change this field.
3. **Runtime:** The `po_reviewer` hat reads `team/botminter.yml` and checks `autonomy.tier`.

**Hat behavior.** The `po_reviewer` hat checks the tier:
- `supervised`: current behavior — post review request, wait for human comment
- `guided`: auto-advance `po:design-review` and `po:plan-review` after lead approval. Wait for human only at `po:accept`. Post notification comment on each auto-advance.
- `autonomous`: auto-advance all gates. Post notification comments. Human retains override.

**Override mechanism.** At any tier:
- `Rejected: <feedback>` on any issue reverts an auto-advance
- `Hold` pauses auto-advance for that specific issue
- Tier changes require `bm teams sync` — agents cannot escalate their own autonomy

**Dependency on Features 1 and 6.** Autonomy is only safe when enforcement is mechanical and quality is measurable. Implementation order reflects this.

#### Acceptance Criteria

- **Given** `autonomy: guided` in `team/botminter.yml`, **when** lead review approves a design, **then** `po:design-review` auto-advances with a notification comment.
- **Given** `autonomy: guided` and an epic at `po:accept`, **when** the scanner dispatches, **then** the agent waits for human comment.
- **Given** a human comments `Rejected: <feedback>` on an auto-advanced issue, **when** the agent scans, **then** the status reverts and feedback is processed.

---

### 3.6 Feature 6: Metrics and Feedback Loops

#### Problem

BotMinter's `poll-log.txt` provides an audit trail but no analytics. Nobody can answer: what's the rejection rate at code review? How long do issues wait for human approval? Is cycle time improving?

Without metrics, the progression from `supervised` to `guided` autonomy is a guess. With metrics, it's evidence.

#### Design

**Transition logging.** The board-scanner skill gains a step: after each status transition, append a JSONL entry to `metrics/transitions.jsonl`:

```json
{"issue": 106, "type": "Epic", "from": "arch:design", "to": "lead:design-review", "ts": "2026-04-04T01:10:00Z", "hat": "arch_designer"}
```

This is a profile-level orchestration change — one line added to the scanner skill's instructions.

**Derived metrics:**

| Metric | Measures | Use |
|--------|----------|-----|
| Design cycle time | `arch:design` -> `po:ready` | Identify bottlenecks |
| Implementation cycle time | `dev:implement` -> `qe:verify` | Track velocity |
| Human gate wait time | Duration in `po:*-review` statuses | Justify `guided` autonomy |
| Rejection rate per gate | Rejections / total at each status | Quality signal. < 15% qualifies for `guided`. |
| First-pass rate | Stories reaching `done` without rejection | Quality indicator |

**Reporting.** The gardener (Feature 3) checks whether a report is due. If >7 days since the last report (file timestamp in `team/projects/<project>/knowledge/reports/`), the gardener creates a `cw:write` issue. This flows through normal dispatch to `cw_writer`, which reads the JSONL and generates a summary. The `retrospective` skill receives metrics for data-driven retros.

#### Acceptance Criteria

- **Given** the scanner transitions an issue, **when** the transition completes, **then** a JSONL entry is appended to `metrics/transitions.jsonl`.
- **Given** >7 days since the last report and the gardener runs, **when** it creates a `cw:write` issue, **then** `cw_writer` generates a summary with cycle times, rejection rates, and throughput.

---

## 4. Data Models

### Check Script Output (stdout on failure)
```
VIOLATION: <what was detected — file, line, specific code>
RULE: <which invariant or ADR>
REMEDIATION: <what the agent should do — specific action, not generic advice>
REFERENCE: <path to governing document>
```

### Golden Principles (YAML — in project repo)
```yaml
principles:
  mechanically_checkable:
    - name: string
      description: string
      check_script: string    # filename in invariants/checks/
      remediation: string
  judgment_based:
    - name: string
      description: string
      detection_hint: string  # what the gardener looks for
      remediation: string
```

### Dev-Boot Configuration (YAML — in team project knowledge)
```yaml
# team/projects/<project>/knowledge/dev-boot.yml
dev_boot:
  steps:
    - name: string
      command: string          # run with cwd = projects/<project>/
      health_check: string     # optional — verify step succeeded
      teardown: string         # optional — cleanup
  isolation: worktree          # each worktree gets its own instance
```

### Autonomy Configuration (in `team/botminter.yml`)
```yaml
autonomy:
  tier: supervised | guided | autonomous
```

### Transition Log Entry (JSONL — in workspace root)
```json
{"issue": 0, "type": "Epic|Task|Bug", "from": "status", "to": "status", "ts": "ISO 8601", "hat": "hat_name"}
```

### Execution Plan (Markdown — in team project plans)
```markdown
# Execution Plan: Epic #<n> — <title>
## Status: In Progress | Completed
## Stories
| # | Title | Status | Completed |
## Key Decisions
| Date | Decision | Rationale |
## Progress Notes
- <date>: <event>
```

---

## 5. Error Handling

**Check script crashes vs. violations.** A script that errors (syntax error, runtime crash) produces exit 1 without VIOLATION output, or exit > 1. The check runner logs this as a warning. Only explicit violations (exit 1 with VIOLATION output) block review. Three consecutive crashes from the same script flag it for human attention via an issue. The `dev_code_reviewer`, `qe_verifier`, and gardener hats all use the check runner.

**Dev-boot failures.** Boot failure or health-check timeout: agent proceeds without the running application (degraded mode). Warning comment posted on the issue. The agent can still run unit tests and static checks.

**Auto-advance failures.** Status transition error during auto-advance: fall back to supervised behavior for that gate. Retry on next scan cycle.

**Gardener failures.** Scan failure: retry on next cycle, maximum 3 retries before flagging for human attention. Gardener failures never block other work — gardening is background maintenance that yields to all higher-priority dispatch items.

**Metrics write failures.** JSONL write failure is logged but does not block the status transition. Metrics are observational, not transactional.

---

## 6. Impact on Existing System

### 6.1 Profile-Level Changes

| Component | Change |
|---|---|
| `team/invariants/checks/` | New directory + 2 profile-generic check scripts |
| `ralph.yml` hats | New `arch_gardener` hat definition |
| `ralph.yml` hats | Updated `dev_code_reviewer` and `qe_verifier` instructions (add check-running step) |
| Board-scanner skill | Add `gardener.scan` dispatch entry + cycle counter |
| Board-scanner skill | Add JSONL transition logging instruction |
| `team/knowledge/` | Two new docs: structured test output convention, check script contract |
| `team/coding-agent/skills/check-runner/` | New check runner script |
| `team/projects/<project>/plans/` | New directory for execution plans |

### 6.2 Project Repo Changes

| Component | Change |
|---|---|
| `projects/<project>/invariants/checks/` | New directory + 2 project-specific check scripts |
| `projects/<project>/invariants/golden-principles.yml` | New config file |

### 6.3 Manifest / CLI Changes

| Component | Change |
|---|---|
| `team/botminter.yml` | `autonomy` field (defaults to `supervised`) |
| `crates/bm/src/profile/manifest.rs` | Parse `autonomy` field in `ProfileManifest` |
| Extraction logic | Extract `invariants/checks/` directory |

### 6.4 CLAUDE.md and Hat Instruction Updates

Features 1-6 introduce new knowledge, scripts, and conventions. Configuration update path:

| What | Where | Update |
|---|---|---|
| Check runner invocation | `dev_code_reviewer` hat instructions (`ralph.yml`) | Add: "Run `bash team/coding-agent/skills/check-runner/run-checks.sh <project>` before reviewing. Reject if violations found." |
| Check runner invocation | `qe_verifier` hat instructions (`ralph.yml`) | Add: "Run check runner as part of verification. Violations block verification." |
| Structured test output | `qe_investigator`, `dev_implementer`, `qe_verifier` hat instructions | Add: "Use structured test output wrapper. Parse JSON results." |
| Dev-boot | `dev_implementer`, `qe_verifier` hat instructions | Add: "Check for `team/projects/<project>/knowledge/dev-boot.yml`. If present, boot application and validate at runtime." |
| Gardener | Board-scanner skill instructions | Add: `gardener.scan` dispatch entry, cycle counter, scheduling config reference. |
| Autonomy | `po_reviewer` hat instructions | Add: "Read `team/botminter.yml` `autonomy.tier`. Behavior varies by tier." |
| CLAUDE.md | `projects/botminter/CLAUDE.md` | Add: reference to check runner, structured test output convention, dev-boot config. Update key directories table to include `invariants/checks/`. |
| Metrics | Board-scanner skill instructions | Add: "After each status transition, append JSONL to `metrics/transitions.jsonl`." |

These are profile-level hat instruction changes (`ralph.yml`) and project-level CLAUDE.md edits. They do not change Ralph's engine — they change what hats are told to do.

### 6.5 Auth and Identity

ADR-0011 proposes per-member GitHub App identity (replacing the shared PAT). This design's features interact with ADR-0011 as follows:

| Feature | Auth Interaction |
|---|---|
| F1 (Checks) | None. Check scripts are read-only file analysis — no GitHub API calls. |
| F2 (Legibility) | None. Dev-boot runs local processes. |
| F3 (Gardener) | Creates GitHub issues. Works with either PAT or GitHub App — the `github-project` skill abstracts the auth method. |
| F4 (Plans) | Updates files in the team repo (git operations). No GitHub API auth implications. |
| F5 (Autonomy) | Auto-advances status transitions via GitHub Projects API. Same auth path as current board scanner. |
| F6 (Metrics) | Writes local JSONL. No auth needed. |

Features 1-6 do not require ADR-0011 implementation. They work with the current auth mechanism. When ADR-0011 is implemented, Features 3 and 5 automatically benefit (the `github-project` skill abstracts the auth layer). ADR-0011 is out of scope for this epic.

### 6.6 What Does NOT Change

- Ralph Orchestrator product (ralph.yml schema definition, event loop engine, hat dispatch engine)
- Formation system and credential management (ADR-0002, ADR-0008)
- Bridge system
- GitHub Projects integration (same statuses, same board structure)
- Existing invariant and ADR markdown files (check scripts are additive)
- The `bm init`, `bm hire`, `bm start` command interfaces

---

## 7. Security Considerations

**Check scripts execute in the agent's context.** Scripts are read-only analyzers — they scan code, they do not modify it. They are version-controlled in the team repo or project repo. Following ADR-0002: declarative scripts with structured output. A check script must not write files, make network requests, or modify git state.

**Autonomy escalation prevention.** The `autonomous` tier removes all human gates. The setting lives in `team/botminter.yml` and requires `bm teams sync` to change — agents cannot modify this file through normal workflow. The `Rejected:` comment provides an emergency brake at any tier.

**Per-member identity (ADR-0011).** When implemented, per-member GitHub App identity ensures the gardener's issue creation and autonomy's status transitions are attributed to the correct bot identity. Until then, the shared PAT works. No auth escalation path exists — agents cannot create or modify GitHub App registrations through the normal workflow.

**Gardener changes go through normal review.** The gardener opens issues for cleanup work. These flow through `dev:implement` → `dev:code-review` → `qe:verify`. The gardener does not merge its own changes.

**Metrics contain no sensitive data.** Transition logs record issue numbers, status names, timestamps, and hat names. No PII, credentials, or application data.

---

## 8. Implementation Order

| Phase | Feature | Depends On | Rationale |
|---|---|---|---|
| 1 | Executable Invariant Checks | — | Foundation. Enforcement must exist first. Immediately flags 5 existing ADR-0007 violations. |
| 2 | Plans as First-Class Artifacts | — | Low-risk. Resolves artifact fragmentation. |
| 3 | Garbage Collection | Phase 1 | Gardener runs checks from Phase 1 + golden principles. |
| 4 | Metrics and Feedback Loops | — | Produces data to justify Phase 5. |
| 5 | Graduated Autonomy | Phases 1, 4 | Requires proven enforcement + quality evidence. |
| 6 | Application Legibility | — | Most complex. Structured test output is standalone; dev-boot requires daemon integration. |

Enforcement first. Autonomy only after quality infrastructure is proven. Legibility last because it's most complex and least coupled.

---

## 9. References

- OpenAI Harness Engineering (Feb 2026) — enforcement-first agentic development
- ADR-0001: ADR Process — governs where ADRs live
- ADR-0002: Shell Script Bridge — design pattern for check script contract
- ADR-0004: Scenario-Based E2E Tests — e2e test framework
- ADR-0005: E2E Test Environment and Isolation — test isolation patterns
- ADR-0006: Directory Modules — convention that `file-size-limit` check enforces
- ADR-0007: Domain-Command Layering — primary enforcement target; 5 existing violations in domain modules
- ADR-0008: Formation as Deployment Strategy — alignment for dev-boot
- ADR-0009: Manual Integration Tests — integration test conventions
- ADR-0011: GitHub App Per-Member Identity — auth model (out of scope, interaction analyzed in Section 6.5)
- `team/botminter.yml` — manifest being extended with `autonomy` field
- `crates/bm/src/profile/manifest.rs` — `ProfileManifest` struct gaining `autonomy` field
- `profile/team_repo.rs` (`read_team_repo_manifest()`) — runtime manifest read path
- BotMinter project invariants (11 files in `projects/botminter/invariants/`) — existing prose invariants
