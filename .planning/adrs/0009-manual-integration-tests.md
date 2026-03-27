---
status: accepted
date: 2026-03-20
decision-makers: operator (ahmed), claude
---

# Exploratory Integration Tests for Infrastructure-Touching Behavior

## Problem

BotMinter's CLI orchestrates real infrastructure: podman containers (Tuwunel Matrix server), Linux keyring credential storage (D-Bus Secret Service), GitHub repos/projects/labels, Lima VMs, and filesystem workspaces. Automated E2E tests (ADR-0004, ADR-0005) use stubs, isolated D-Bus sessions, and ephemeral GitHub repos to stay hermetic — but this hermiticity means they structurally cannot verify recovery from real infrastructure failures (container force-removed, keyring locked, volume deleted, VM boot script drift). How should we systematically verify these real-infrastructure integration scenarios?

## Constraints

* Tests must exercise real infrastructure — real podman containers, real keyring, real Matrix API, real Lima VMs — not stubs or mocks
* Tests must be repeatable and produce a structured pass/fail report, not ad-hoc sessions
* Tests must not run in CI — they require a workstation with podman, keyring, `gh` auth, and sometimes Lima
* Tests must cover idempotency (running the same command N times) and recovery (infrastructure destroyed mid-lifecycle)
* Tests must stay current with the feature set — stale exploratory tests are as bad as no tests
* An AI coding agent must be able to run the full suite autonomously, interpret results, and act on failures — this is the agentic replacement of a QE/QA engineer's verification workflow

## Decision

Maintain a scripted exploratory test suite at `crates/bm/tests/manual/` with three artifacts:

1. **PLAN.md** — the test plan: phases, numbered scenarios, expected outcomes. This is the source of truth for what's tested.
2. **Justfile** — automated execution recipes that run each phase, collect pass/fail results, and generate a report. Developers or agents run `just exploratory-test` (or `just exploratory-test-full` for Lima) from the project root.
3. **REPORT.md** — generated output from the last run, with per-scenario pass/fail/note verdicts.

The suite is organized into sequential phases:

| Phase | What it tests |
|-------|--------------|
| A | Lima VM boot script idempotency (optional, slow) |
| B | Team init + hire lifecycle |
| C | Bridge provisioning, idempotency, and recovery (stopped/removed/volume-deleted container, pre-existing Matrix users) |
| D | Workspace sync idempotency and recovery (stale workspaces, missing files, junk directories) |
| E | Full sync with bridge flag — all subsystems together |
| F | Error handling and CLI display commands |
| G | Cleanup — remove all test artifacts |

These are exploratory tests in the QE sense: they systematically explore real system behavior under normal, degraded, and failure conditions. They are fully scripted and produce machine-readable results, so an AI agent can run the suite, read the report, diagnose failures, and fix code — fulfilling the role traditionally held by a QE/QA engineer.

Exploratory tests are mandatory after code changes that touch bridge, workspace, sync, or Lima provisioning. This is enforced by convention in CLAUDE.md, not by CI gates.

## Rejected Alternatives

### Automate everything in CI with real containers

Rejected because: CI environments lack podman (rootless), keyring daemons, Lima, and sufficient privileges. Attempting to set these up in CI creates a fragile, slow, expensive pipeline that breaks on infrastructure changes unrelated to the code.

* Docker-in-Docker or podman-in-CI is possible but adds 5-10 minutes of setup and is notoriously flaky
* Keyring requires a running D-Bus session with Secret Service — possible in CI but brittle
* Lima requires nested virtualization, unavailable on most CI runners

### Trust the stubs — skip real infrastructure testing

Rejected because: stub-based E2E tests missed real integration bugs that only surfaced during exploratory testing. Examples: container recovery after `podman rm -f`, password reset for pre-existing Matrix users, `--overwrite` flag needed for idempotent `dnf addrepo` in Lima boot scripts.

* The stub ralph binary validates command-line arguments but not actual Matrix API responses
* Keyring isolation (private D-Bus session) doesn't test real Secret Service behavior
* These gaps caused production failures during early operator onboarding

### Unscripted exploratory testing

Rejected because: unscripted testing is unrepeatable, produces no audit trail, and drifts as features change. Different developers test different things, and nobody knows what was actually verified. Critically, an AI agent cannot autonomously run unscripted tests — it needs a defined plan and structured output to close the loop.

## Consequences

* The exploratory test suite is the agentic equivalent of a QE/QA engineer: scripted, repeatable, interpretable by agents
* Developers or agents must have a workstation with podman, keyring, `gh`, and optionally Lima to run the full suite
* Exploratory tests add ~5 minutes (without Lima) or ~15 minutes (with Lima) to the verification cycle
* The test plan must be updated when features change — this is a maintenance burden, enforced by CLAUDE.md guardrails
* The REPORT.md artifact provides an audit trail of the last verification run
* There is no CI enforcement — compliance depends on agent discipline, developer discipline, and code review

## Anti-patterns

* **Do NOT** attempt to run exploratory tests in CI — they require real infrastructure that CI cannot reliably provide. If a scenario can be reliably automated in a hermetic environment, move it to the E2E suite instead.
* **Do NOT** treat exploratory tests as optional when changing bridge, workspace, sync, or Lima code — these tests exist precisely because automated tests cannot catch the bugs they find.
* **Do NOT** add exploratory test scenarios without updating both PLAN.md and the Justfile — a test that exists only in the plan but not in the Justfile will never be run, and vice versa.
* **Do NOT** leave the REPORT.md from a failing run committed — either fix the failures or document known issues. A green report is the quality gate.
