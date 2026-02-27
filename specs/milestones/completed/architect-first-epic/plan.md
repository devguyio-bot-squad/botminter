# Implementation Plan — Milestone 2: Architect + First Epic

> Five-sprint vertical-slice plan. Each sprint delivers an end-to-end demoable flow
> adding progressively more agent capabilities.
> Design: [design.md](design.md)

## Checklist

- [x] **Sprint 1:** One agent, one hat — architect produces a design
- [x] **Sprint 2:** Two agents, full lifecycle — autonomous coordination
- [x] **Sprint 3:** Telegram, HIL, training mode — human in the loop (Step 5 manual test plan not produced; manual Telegram tests not run)
- [ ] **Sprint 4:** Automated HIL Telegram tests
- [ ] **Sprint 5:** Compact single-member profile

---

## Sprint 1: One Agent, One Hat — Design Production

**Vertical slice:** Workspace infrastructure + architect with board_scanner and designer hats. A single agent scans the board, finds an epic, produces a design doc, transitions status.

**No human-assistant changes.** The synthetic epic is seeded directly at `status/arch:design`, bypassing triage/backlog.

**Demo:** Architect launches, detects epic, produces design doc at `.botminter/projects/hypershift/knowledge/designs/epic-1.md`, transitions to `status/po:design-review`. Design doc contains markers from team, project, and member knowledge scopes. Invariant compliance verified.

**Plan:** [sprint-1/plan.md](sprint-1/plan.md)
**PROMPT:** [sprint-1/PROMPT.md](sprint-1/PROMPT.md)

---

## Sprint 2: Two Agents, Full Lifecycle — Autonomous

**Vertical slice:** Add remaining architect hats + evolve human-assistant to three-hat model. Both agents coordinate through the full epic lifecycle autonomously (review gates auto-advance without HIL).

**Demo:** Epic starts at `status/po:triage`, both agents run concurrently, epic traverses the complete lifecycle to `status/done`. Stories created with proper frontmatter and parent linking. No human interaction required.

**Plan:** [sprint-2/plan.md](sprint-2/plan.md)
**PROMPT:** [sprint-2/PROMPT.md](sprint-2/PROMPT.md)

---

## Sprint 3: Telegram, HIL, Training Mode — Human in the Loop

**Vertical slice:** Add Telegram routing, training mode, HIL review gates, and rejection loops. The human gates all decisions via separate Telegram bots.

**Demo:** Full integration test. Both agents with separate Telegram bots. Human approves triage, reviews designs (rejects one, approves revision), approves plan, activates epic, accepts completion. Complete lifecycle with human in the loop.

**Plan:** [sprint-3/plan.md](sprint-3/plan.md)
**PROMPT:** [sprint-3/PROMPT.md](sprint-3/PROMPT.md)

---

## Sprint 4: Automated HIL Telegram Tests

**Vertical slice:** Automated end-to-end tests replacing Sprint 3's unexecuted manual test plan. Both agents run against a mock Telegram server with a scripted human driving all HIL gates.

**Demo:** `just test-hil` runs all 7 design Section 8 test scenarios unattended — lifecycle traversal, rejection loops, push conflicts, crash recovery, knowledge propagation. No real Telegram. CI-ready.

**Plan:** [sprint-4/plan.md](sprint-4/plan.md)
**PROMPT:** [sprint-4/PROMPT.md](sprint-4/PROMPT.md)

---

## Sprint 5: Compact Single-Member Profile

**Vertical slice:** New `compact` profile with a single `superman` member wearing 15 hats. Same `.github-sim/` model, supervised mode (human gates major decisions only), lead_reviewer reviews all work before human review, story TDD flow with direct chain dispatch (qe_test_designer → dev_implementer → dev_code_reviewer → qe_verifier), auto-advance at sign-off and merge.

**Demo:** One agent, all roles. Epic seeded at `po:triage`, traverses the complete lifecycle with lead_reviewer reviewing every work artifact before the human sees it. Story TDD flow (`qe:test-design → dev:implement → dev:code-review → qe:verify → done`) ensures every piece of work is followed by a review step. Single workspace, no coordination overhead.

**Plan:** [sprint-5/plan.md](sprint-5/plan.md)
**PROMPT:** [sprint-5/PROMPT.md](sprint-5/PROMPT.md)
