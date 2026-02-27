# Requirements — Milestone 2: Architect + First Epic

> Q&A record from requirements clarification.
> Master plan: [design.md](../master-plan/design.md) (Section 5), [plan.md](../master-plan/plan.md).

---

## Q1: What does the architect produce?

**Q:** What does the architect actually produce when it picks up an epic? A design doc, a story breakdown, or both?

**A:** Both. Design first, then story breakdown after PO approves the design.

---

## Q2: Where does the architect's design live?

**Q:** The architect produces a design doc before breaking into stories. Where does that design go — inline in the epic issue, or as a separate file?

**A:** Separate file, linked from the epic.

---

## Q3: Does the architect need codebase access?

**Q:** Does the architect need access to the actual project codebase (e.g., HyperShift Go code) to produce its design, or is the project knowledge in `projects/hypershift/knowledge/` sufficient for M2?

**A:** Yes, the architect needs codebase access.

---

## Q4: How does the architect access the codebase?

**Q:** How does the architect access the project codebase? Submodule, agent-cloned, or config path?

**A:** Agent-cloned into a gitignored `projects/` directory in the workspace at runtime. The fork chain is:

```
openshift/hypershift (upstream)
  └── devguyio/hypershift (human's fork)
        └── devguyio-minions/hypershift (agentic-team fork)
```

Team members (agents) work on the agentic-team fork via branches. The architect clones the agentic-team fork into `workspace-architect/projects/hypershift/` (gitignored).

---

## Q5: What is the epic lifecycle?

**Q:** The architect picks up epics via status labels and the human-assistant gates transitions. What are the specific statuses an epic moves through, and who is responsible for each transition? For example: `status/po:triage` → `status/architect:design` → `status/po:design-review` → ... What does the full epic lifecycle look like?

**A:** The lifecycle is NOT a rigid top-down pipeline. It's **bottom-up and emergent**: each role defines what statuses it watches for and what transitions it makes. The end-to-end lifecycle falls out of combining all roles' perspectives. Statuses can move freely between states (not like Jira where transitions are locked).

The known statuses for epics (happy path):

`status/po:triage`, `status/po:backlog`, `status/arch:design`, `status/po:design-review`, `status/arch:plan`, `status/po:plan-review`, `status/arch:breakdown`, `status/po:ready`, `status/arch:in-progress`, `status/po:accept`, `status/done`

But the design approach is: define each role's view of the lifecycle separately (what it watches, what it does, what it transitions to), then combine to get the end-to-end flow and find gaps. Flexibility is intentional — any status can move to any other status when needed.

---

## Q6: What are the architect's status transitions?

**Q:** Defining the architect's view of the epic lifecycle: what `status/*` labels does the architect watch for, what does it do when it sees each one, and what status does it transition to when done?

**A:** The architect watches for `status/arch:*` labels and ignores `status/po:*`:

| Watches for               | Action                                                                                                                                                                                                                     | Transitions to            |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| `status/arch:design`      | Reads epic + project codebase (agent-cloned fork). Produces design doc as separate file, links from epic.                                                                                                                  | `status/po:design-review` |
| `status/arch:plan`        | Takes PO-approved design, proposes story breakdown (stories with descriptions + acceptance criteria).                                                                                                                      | `status/po:plan-review`   |
| `status/arch:breakdown`   | Creates story issues in `.github-sim/issues/` with `kind/story` + `parent` linking to epic.                                                                                                                                | `status/po:ready`         |
| `status/arch:in-progress` | **M2:** No-op (no dev/qe yet). **Future:** Watches stories under the epic as they complete — reviews, sends back if gaps found, creates new stories to fill gaps. When satisfied the epic is fully addressed, transitions. | `status/po:accept`        |

The architect is the technical quality gate before the PO's acceptance (`status/po:accept` → `status/done`).

---

## Q7: What are the human-assistant's M2 status transitions?

**Q:** The human-assistant evolves in M2 with new hats for epic creation, design gating, and prioritization. What `status/po:*` labels does the human-assistant watch for, and what does it do at each stage? For example: at `status/po:triage` does the human-assistant create the epic from a human request and assign it? At `status/po:design-review` does it present the architect's design to the human for approval? What's the human-assistant's per-status behavior?

**A:** Keep `status/po:*` for all PO-side labels (human-assistant is the PO's proxy — other members don't need to distinguish). Added `status/po:backlog` as a new status between triage and design.

Updated epic lifecycle:
```
status/po:triage → status/po:backlog → status/arch:design → status/po:design-review → status/arch:plan → status/po:plan-review → status/arch:breakdown → status/po:ready → status/arch:in-progress → status/po:accept → status/done
```

Human-assistant per-status behavior:

| Watches for | Action | Transitions to |
|---|---|---|
| `status/po:triage` | Incoming queue. Epics land here from any source (human, agents, human-assistant). Presents new epics to human, helps evaluate. | `status/po:backlog` (accepted) |
| `status/po:backlog` | Prioritized backlog. Helps human keep it prioritized, surfaces what's next. When human says "start this one," advances. | `status/arch:design` |
| `status/po:design-review` | Presents architect's design doc to human via HIL. Approved → advance. Rejected → back with feedback. | `status/arch:plan` or back to `status/arch:design` |
| `status/po:plan-review` | Presents architect's story breakdown to human via HIL. Approved → advance. Rejected → back with feedback. | `status/arch:breakdown` or back to `status/arch:plan` |
| `status/po:ready` | Confirms stories created, epic queued for work. M2 end state (no dev/qe). Future: kicks off execution. | `status/arch:in-progress` |
| `status/po:accept` | Presents completed epic to human for acceptance. | `status/done` |

Review gates are all HIL — human-assistant presents to human and waits. In training mode everything goes through the human.

---

## Q8: Where does the architect's design doc live?

**Q:** Q2 established the design is a separate file linked from the epic. What's the file location? Options: (a) alongside the epic in `.github-sim/` (e.g., `.github-sim/designs/epic-{number}.md`), (b) in the project knowledge directory (e.g., `projects/hypershift/knowledge/designs/`), or (c) somewhere else?

**A:** Option (b) — project knowledge directory. `.github-sim/` is strictly a simulation of GitHub's data model (issues, PRs, milestones) and nothing else. Design docs are project knowledge, so they live in `projects/hypershift/knowledge/` (e.g., `projects/hypershift/knowledge/designs/epic-{number}.md`). The epic issue links to the design doc by path.

---

## Q9: How do the two members avoid stepping on each other in `.github-sim/`?

**Q:** With both human-assistant and architect reading/writing `.github-sim/` issues via their submodules, how should concurrent access be handled? The master plan mentions a "write-lock mechanism" as an option. Given the POC, is sequential operation (one member acts at a time) sufficient for M2, or do we need a locking mechanism?

**A:** A locking mechanism is needed. Design:

- **Scope:** Per-issue and per-pull-request (not a global lock on all issues/PRs)
- **Lock file per type:** e.g., `.github-sim/issues/{number}.lock`, `.github-sim/pulls/{number}.lock`
- **Write-only:** Locks are required for writes, not reads. Any member can read at any time.
- **Acquire:** Agent writes its ID to the lock file, commits, and pushes. If the lock file already exists, the lock cannot be acquired.
- **Lock ID format:** `{role}:{ralph_loop_id}` — role prefix (e.g., `architect`, `human-assistant`) + the current loop ID from Ralph Orchestrator (`current_loop_id`)
- **Release:** Agent deletes the lock file, commits, and pushes.
- **Lock file contents:** `{role}:{ralph_loop_id}` + timestamp (ISO 8601). Timestamp is needed for stale lock detection.

**Stale lock handling:**

- **Human-assistant hat:** New board-scanning responsibility — scan for stale locks (acquired > 5 minutes). Locks should only be held during issue/PR updates, so 5 minutes is generous. Stale locks are assumed to belong to crashed agents and are deleted/cleaned up by the human-assistant.
- **Agent self-defense:** If an agent finds its lock was deleted (by the human-assistant or otherwise), it must re-acquire the lock before pushing. Agents should verify their lock still exists right before push.
- **Agent startup:** Each agent checks for stale locks with its own role on startup. Since the assumption is one agent per role, any existing lock for that role at startup is stale (previous crash). The agent cleans it up.

---

## Q10: How do members sync `.github-sim/` changes?

**Q:** The architect pushes a status transition via its submodule, but the human-assistant needs to detect it on its next board scan. How do members see each other's changes?

**A:** The team repo submodule is pulled at the beginning of each loop iteration (board scan cycle). Additionally, agents may decide to pull mid-loop when they need fresh state (e.g., before acting on an issue they're about to update). The pull is on the submodule pointing to the team repo — that's where `.github-sim/` lives.

---

## Q11: Telegram routing — separate bots or shared bot?

**Q:** In M1, only the human-assistant uses Telegram (via RObot). With the architect added in M2, does the architect also communicate with the human via Telegram? If so, is it a separate bot per member, a shared bot with routing, or does only the human-assistant talk to the human (and the architect communicates indirectly through `.github-sim/`)?

**A:** Separate bot per member. Each team member gets its own Telegram bot token and 1:1 chat with the human. This works with Ralph's current implementation (single `chat_id` per instance, no group/topic support). No Ralph code changes needed.

---

## Q12: What hats does the architect have?

**Q:** The human-assistant has a `board_scanner` hat from M1. What hats does the architect need? Based on the status transitions (Q6), the architect's work spans design, breakdown proposal, and breakdown execution — are these separate hats, or one hat that handles all `status/arch:*` statuses?

**A:** Separate hats per status. The architect needs:

1. `board_scanner` — scans for `status/arch:*` issues, dispatches to the appropriate hat
2. `designer` — handles `status/arch:design` (produces design doc)
3. `planner` — handles `status/arch:plan` (proposes story breakdown)
4. `breakdown_executor` — handles `status/arch:breakdown` (creates story issues)
5. `epic_monitor` — handles `status/arch:in-progress` (M2: no-op. Future: watches stories, reviews, finds gaps)

---

## Q13: What new hats does the human-assistant get in M2?

**Q:** The master plan says the human-assistant evolves in M2 with "new hats for epic creation, design gating, story prioritization." Based on the human-assistant's status transitions (Q7), the new behaviors are: triage, backlog management, design review, breakdown review, ready confirmation, and final review. Plus the stale lock cleanup from Q9. Are these all separate hats, and do they replace or extend the existing `board_scanner` hat from M1?

**A:** Three hats total (1 extended, 2 new). Group by behavior pattern — don't over-split:

1. **`board_scanner` (extended from M1)** — Scans board for `status/po:*` changes, detects and cleans stale locks, dispatches to other hats. Runs every loop iteration.
2. **`backlog_manager` (new)** — Handles `status/po:triage` and `status/po:backlog`. Helps human evaluate new epics, prioritize the backlog, decide what to activate.
3. **`review_gater` (new)** — Handles `status/po:design-review`, `status/po:plan-review`, `status/po:ready`, and `status/po:accept`. Same pattern for all: present artifact to human via HIL, get approval/rejection, transition accordingly.

Grouping principle: if the behavior pattern is the same (present → wait → transition), it's one hat. If fundamentally different (managing a queue vs. gating a review), separate hat.

---

## Q14: Should M2 validate knowledge and invariant propagation?

**Q:** The master plan defers evals to M4 and real knowledge accumulation to M3. But with the architect as the first role that consumes project knowledge to produce designs, should M2 validate that the multi-level knowledge and invariant model actually works?

**A:** Yes. M2 must validate that the architect respects knowledge and invariants at all levels. The validation should test:

1. **Knowledge propagation:** The architect's design output reflects knowledge from all applicable levels — team knowledge, project knowledge (`projects/hypershift/knowledge/`), and member-specific knowledge. Test that knowledge at each scope is actually consumed and influences the architect's work.
2. **Invariant propagation:** The architect respects invariants at all applicable levels — team invariants, project invariants, and member-specific invariants. Test that invariants from each scope are enforced.

M2 validation uses **synthetic test fixtures** — not real HyperShift content. Each milestone has two phases:
1. **Implementation phase:** Build and validate the machinery using synthetic data, test fixtures, and dummy content. Knowledge and invariant files are populated with test content designed to verify propagation (e.g., a team-level invariant that the design must address, a project-level knowledge file that should influence the design).
2. **Operational phase:** The human uses the validated machinery to populate real project content (HyperShift knowledge, real invariants) and starts using the team for actual work.

The synthetic test data should be designed so violations are detectable — proving the architect actually reads and respects each scope.

---

## Requirements Complete

All requirements for M2 have been clarified. The master plan's open questions (Section 5) are resolved:

1. ~~Telegram routing~~ → Q11 (separate bots per member)
2. ~~Concurrent file access~~ → Q9 (per-issue write-locks with stale detection)
3. ~~Submodule sync~~ → Q10 (pull at loop start + optional mid-loop)
4. ~~Project repo access~~ → Q4 (agent-cloned fork chain)
5. ~~Human command routing~~ → Q12, Q13 (board scanner dispatches based on board state)
6. ~~Human-assistant guardrails~~ → Q7, Q13 (all HIL in training mode; review_gater handles gates)

Ready for design.
