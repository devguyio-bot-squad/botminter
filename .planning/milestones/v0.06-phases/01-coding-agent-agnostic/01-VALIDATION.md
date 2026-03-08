---
phase: 01
slug: coding-agent-agnostic
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-05
---

# Phase 01 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `libtest-mimic 0.8` custom harness (E2E) |
| **Config file** | `crates/bm/Cargo.toml` (E2E gated behind `[features] e2e = []`) |
| **Quick run command** | `just test` (cargo test -p bm) |
| **Full suite command** | `just test && just e2e` |
| **Estimated runtime** | ~15s unit/integration, ~60s E2E |

---

## Sampling Rate

- **After every task commit:** Run `just test`
- **After every plan wave:** Run `just test && just e2e`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds (unit/integration)

---

## Per-Task Verification Map

### Plan 01-01: Agent Tag + CodingAgentDef + Profile Restructuring

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-01-T1 | Agent tag filter library | unit | `cargo test --lib -- agent_tags::tests` | COVERED (36 tests) |
| 01-01-T2 | CodingAgentDef data model | unit | `cargo test --lib -- profile::tests` | COVERED (6 tests) |
| 01-01-T3 | Profile restructuring (agent/ -> coding-agent/) | unit | `cargo test --lib -- profile::tests` | COVERED (4 tests) |
| 01-01-T4 | Unified-to-agent-specific extraction | unit | `cargo test --lib -- profile::tests` | COVERED (8 tests) |
| 01-01-T5 | Workspace parameterization | unit | `cargo test --lib -- workspace::tests` | COVERED (4 tests) |
| 01-01-T6 | scan_agent_tags / --show-tags | unit | `cargo test --lib -- profile::tests` | COVERED (5 tests) |

### Plan 01-02: Test Path Isolation + Show-Tags Label

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-02-T1 | Fix cli_parsing.rs path isolation | integration | `cargo test --test cli_parsing` | COVERED (49 tests) |
| 01-02-T2 | Rename show-tags label | integration | `cargo test -- profiles_describe` | COVERED (2 tests) |

### Plan 01-03: Profile Staleness Detection + Non-Interactive Init

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-03-T1 | Version marker staleness detection | unit | `cargo test -- ensure_profiles_initialized` | COVERED (3 tests) |
| 01-03-T2 | --non-interactive flag + E2E smoke | integration | `cargo test -- init_smoke` | COVERED (4 tests) |

### Plan 01-04: Version-Field Staleness Comparison

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-04-T1 | Replace marker with version-field comparison | unit | `cargo test -- ensure_profiles_initialized` | COVERED (6 tests) |
| 01-04-T2 | No marker file references remain | codebase | `grep -r "version_marker"` | COVERED (verified clean) |
| 01-04-T3 | Documentation updated | docs | `grep` verification | COVERED |

### Plan 01-05: Real E2E Init Test (GitHub API)

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-05-T1 | e2e_init_non_interactive_full_github | E2E | `cargo test --test e2e -- e2e_init_non_interactive_full_github` | COVERED (1 Trial) |

### Plan 01-06: E2E Profile Path Isolation

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-06-T1 | list_embedded_roles + bootstrap helper | unit+E2E | `cargo test -- embedded` | COVERED |
| 01-06-T2 | All E2E tests use embedded data | E2E | `cargo test --test e2e` | COVERED (all modules converted) |

### Plan 01-07: Session Binary Check Injection

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-07-T1 | Injectable binary check in session.rs | unit | `cargo test --lib -- session::tests` | COVERED (2 tests) |

### Plan 01-08: Unconditional Test Assertions

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-08-T1 | state.json assertions unconditional | E2E | `cargo test --test e2e -- start_to_stop` | COVERED (3 sites) |
| 01-08-T2 | Daemon test namesake claims | E2E | `cargo test --test e2e -- daemon` | COVERED (resolved by Plan 09) |

### Plan 01-09: Custom E2E Test Harness

| Task ID | Requirement | Test Type | Automated Command | Status |
|---------|-------------|-----------|-------------------|--------|
| 01-09-T1 | libtest-mimic dependency, harness=false | build | `cargo check --features e2e` | COVERED |
| 01-09-T2 | Custom harness main.rs | E2E | `cargo check --features e2e` | COVERED |
| 01-09-T3 | All modules converted to Trial format | E2E | `cargo check --features e2e` | COVERED |
| 01-09-T4 | Daemon tests with real GitHub repos | E2E | `just e2e` | COVERED (8 Trials) |
| 01-09-T5 | Justfile updated with passthrough | build | `just test` | COVERED |

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have automated verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none needed)
- [x] No watch-mode flags
- [x] Feedback latency < 15s (unit/integration)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-05

---

## Validation Audit 2026-03-05

| Metric | Count |
|--------|-------|
| Total tasks audited | 28 |
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
| Coverage | 28/28 (100%) |
