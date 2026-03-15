---
phase: 8
slug: bridge-abstraction-cli
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-08
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test framework (cargo test) |
| **Config file** | `crates/bm/Cargo.toml` |
| **Quick run command** | `cargo test -p bm` |
| **Full suite command** | `cargo test -p bm && cargo clippy -p bm -- -D warnings` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p bm`
- **After every plan wave:** Run `cargo test -p bm && cargo clippy -p bm -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | BRDG-05 | unit | `cargo test -p bm bridge` | Wave 0 | pending |
| 08-01-02 | 01 | 1 | BRDG-06 | unit | `cargo test -p bm bridge` | Wave 0 | pending |
| 08-01-03 | 01 | 1 | BRDG-08 | unit | `cargo test -p bm bridge` | Wave 0 | pending |
| 08-01-04 | 01 | 1 | BRDG-09 | unit | `cargo test -p bm bridge` | Wave 0 | pending |
| 08-02-01 | 02 | 2 | CLI-01 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-02 | 02 | 2 | CLI-02 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-03 | 02 | 2 | CLI-03 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-04 | 02 | 2 | CLI-04 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-05 | 02 | 2 | CLI-05 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-06 | 02 | 2 | CLI-06 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-07 | 02 | 2 | CLI-07 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-08 | 02 | 2 | CLI-10 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-02-09 | 02 | 2 | CLI-11 | integration | `cargo test -p bm --test integration bridge` | Wave 0 | pending |
| 08-03-01 | 03 | 3 | TELE-01 | integration | `cargo test -p bm --test integration telegram` | Wave 0 | pending |
| 08-03-02 | 03 | 3 | TELE-02 | unit | `cargo test -p bm profile` | Wave 0 | pending |
| 08-04-01 | 04 | 3 | CLI-08 | integration | `cargo test -p bm --test integration start` | Wave 0 | pending |
| 08-04-02 | 04 | 3 | CLI-09 | integration | `cargo test -p bm --test integration status` | Wave 0 | pending |
| ALL | all | all | ALL | unit | `cargo test -p bm --test cli_parsing bridge` | Wave 0 | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] `crates/bm/src/bridge.rs` -- BridgeManifest, BridgeState structs with unit tests
- [ ] `crates/bm/tests/integration.rs` -- bridge integration test section using stub fixture
- [ ] `crates/bm/tests/cli_parsing.rs` -- bridge subcommand parsing tests
- [ ] `.planning/specs/bridge/examples/stub/Justfile` -- add room-create and room-list recipes
- [ ] `.planning/specs/bridge/examples/stub/bridge.yml` -- add optional `spec.room` section

*Existing infrastructure (cargo test, clippy, conformance) covers framework needs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Bridge status table formatting | CLI-03 | Visual output layout | Run `bm bridge status` with stub bridge, verify table is readable |
| Identity list table formatting | CLI-06 | Visual output layout | Run `bm bridge identity list`, verify columns align |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
