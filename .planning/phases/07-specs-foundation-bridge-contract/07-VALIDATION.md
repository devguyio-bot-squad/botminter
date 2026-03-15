---
phase: 7
slug: specs-foundation-bridge-contract
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-08
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test framework (cargo test) |
| **Config file** | `crates/bm/Cargo.toml` (existing) |
| **Quick run command** | `cargo test -p bm conformance` |
| **Full suite command** | `cargo test -p bm` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p bm conformance`
- **After every plan wave:** Run `cargo test -p bm`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 07-01-xx | 01 | 1 | SPEC-01 | manual | Verify ADR files exist with MADR 4.0.0 structure | N/A | ⬜ pending |
| 07-01-xx | 01 | 1 | SPEC-02 | manual | Verify bridge abstraction ADR exists | N/A | ⬜ pending |
| 07-01-xx | 01 | 1 | SPEC-05 | manual | Verify directory structure | N/A | ⬜ pending |
| 07-02-xx | 02 | 1 | SPEC-03 | manual | Verify spec uses RFC 2119 language | N/A | ⬜ pending |
| 07-02-xx | 02 | 1 | BRDG-01 | unit | `cargo test -p bm conformance::bridge_commands` | ❌ W0 | ⬜ pending |
| 07-02-xx | 02 | 1 | BRDG-02 | unit | `cargo test -p bm conformance::schema_structure` | ❌ W0 | ⬜ pending |
| 07-02-xx | 02 | 1 | BRDG-03 | unit | `cargo test -p bm conformance::external_bridge` | ❌ W0 | ⬜ pending |
| 07-02-xx | 02 | 1 | BRDG-04 | unit | `cargo test -p bm conformance::identity_commands` | ❌ W0 | ⬜ pending |
| 07-02-xx | 02 | 1 | BRDG-07 | manual | Review bridge-spec.md text for file-based config | N/A | ⬜ pending |
| 07-03-xx | 03 | 2 | SPEC-04 | unit | `cargo test -p bm conformance::bridge_yml` | ❌ W0 | ⬜ pending |
| 07-03-xx | 03 | 2 | SPEC-04 | unit | `cargo test -p bm conformance::schema_json` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/bm/tests/conformance.rs` — new test file for spec conformance tests
- [ ] `.planning/specs/bridge/examples/stub/` — stub bridge fixture for tests
- [ ] No framework install needed — Rust test framework already configured

*Wave 0 stubs created as part of the conformance test plan.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ADR files exist with MADR 4.0.0 structure | SPEC-01 | Document format, not runtime | Verify `.planning/adrs/` has template + 3 ADRs |
| Bridge abstraction ADR content | SPEC-02 | Design content, not structure | Read ADR and verify decision rationale |
| Bridge spec uses RFC 2119 language | SPEC-03 | Prose analysis | Grep for MUST/SHOULD/MAY in spec |
| Legacy specs/ removed | SPEC-05 | Directory structure | Verify `specs/` contents removed |
| File-based config exchange documented | BRDG-07 | Spec prose | Review bridge-spec.md references `$BRIDGE_CONFIG_DIR` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
