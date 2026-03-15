---
phase: 10
slug: rocket-chat-bridge
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-10
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | libtest-mimic (custom E2E harness) + cargo test (unit/integration) |
| **Config file** | `crates/bm/tests/e2e/main.rs` |
| **Quick run command** | `just unit` |
| **Full suite command** | `just test` |
| **Estimated runtime** | ~120 seconds |

---

## Sampling Rate

- **After every task commit:** Run `just unit`
- **After every plan wave:** Run `just test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 10-01-01 | 01 | 1 | RC-01 | conformance | `cargo test -p bm conformance` | ❌ W0 | ⬜ pending |
| 10-01-02 | 01 | 1 | RC-02 | e2e | `just e2e` (RC scenario) | ❌ W0 | ⬜ pending |
| 10-01-03 | 01 | 1 | RC-03 | e2e | `just e2e` (RC scenario, start case) | ❌ W0 | ⬜ pending |
| 10-01-04 | 01 | 1 | RC-04 | e2e | `just e2e` (RC scenario, identity case) | ❌ W0 | ⬜ pending |
| 10-01-05 | 01 | 1 | RC-05 | e2e | `just e2e` (RC scenario, sync case) | ❌ W0 | ⬜ pending |
| 10-01-06 | 01 | 1 | RC-06 | manual/spike | Spike script validates bidirectional messaging | ❌ W0 | ⬜ pending |
| 10-01-07 | 01 | 1 | RC-07 | conformance + unit | `cargo test -p bm` (schema validation) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `profiles/scrum-compact/bridges/rocketchat/` — bridge.yml + schema.json + Justfile
- [ ] `profiles/scrum/bridges/rocketchat/` — same bridge files for scrum profile
- [ ] E2E scenario for RC operator journey (replaces Telegram as primary)
- [ ] Conformance test entry for rocketchat bridge
- [ ] Spike script proving Ralph + RC Podman Pod bidirectional messaging
- [ ] Bridge-type-aware `launch_ralph()` in start.rs and daemon.rs
- [ ] Extended `inject_robot_config()` or updated `inject_robot_enabled()` in workspace.rs

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Bot commands work via RC | RC-06 | Requires live RC instance with Ralph connected | Run spike script, send `/status` in RC channel, verify response |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
