---
phase: 9
slug: profile-integration-cleanup
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-08
---

# Phase 9 — Validation Strategy

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
| 09-01-01 | 01 | 1 | PROF-01 | unit | `cargo test -p bm profile::tests::manifest_with_bridges` | Wave 0 | pending |
| 09-01-02 | 01 | 1 | PROF-01 | unit | `cargo test -p bm profile::tests::manifest_no_bridges` | Wave 0 | pending |
| 09-01-03 | 01 | 1 | PROF-02 | unit | `cargo test -p bm profile::tests::bridge_discovery` | Wave 0 | pending |
| 09-02-01 | 02 | 2 | PROF-05 | integration | `cargo test -p bm --test integration init_with_bridge` | Wave 0 | pending |
| 09-02-02 | 02 | 2 | PROF-05 | integration | `cargo test -p bm --test integration init_no_bridge` | Wave 0 | pending |
| 09-02-03 | 02 | 2 | PROF-03 | integration | `cargo test -p bm --test integration sync_bridge_provision` | Wave 0 | pending |
| 09-02-04 | 02 | 2 | PROF-03 | integration | `cargo test -p bm --test integration sync_bridge_skip_no_creds` | Wave 0 | pending |
| 09-02-05 | 02 | 2 | PROF-03 | integration | `cargo test -p bm --test integration sync_robot_section` | Wave 0 | pending |
| 09-03-01 | 03 | 2 | PROF-06 | unit | `cargo test -p bm profile::tests::no_telegram_profile` | Wave 0 | pending |
| 09-03-02 | 03 | 2 | ALL | unit | `cargo test -p bm --test cli_parsing sync_flags` | Wave 0 | pending |
| 09-03-03 | 03 | 2 | ALL | unit | `cargo test -p bm --test cli_parsing init_bridge` | Wave 0 | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] `ProfileManifest.bridges` field with unit tests for parsing with/without bridges
- [ ] CLI parsing tests for new sync flags (`--repos`, `--bridge`, `--all`)
- [ ] CLI parsing test for init `--bridge` flag
- [ ] Integration tests for sync bridge provisioning (using stub bridge fixture)
- [ ] No framework install needed -- Rust test framework already configured

*Existing infrastructure covers framework requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Init wizard bridge selection UX | PROF-05 | Interactive cliclack prompt cannot be automated | Run `bm init` interactively, verify bridge list appears after profile selection |
| Hire bridge token prompt UX | PROF-01 | Interactive token input prompt | Run `bm hire superman`, verify optional token prompt appears when bridge configured |
| MkDocs documentation renders correctly | PROF-04 | Visual/navigation verification | Run `mkdocs serve`, browse bridge docs pages |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
