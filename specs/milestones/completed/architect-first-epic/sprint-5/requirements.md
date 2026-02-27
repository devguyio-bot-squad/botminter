# Requirements — Sprint 5: Compact Single-Member Profile

> Q&A record from requirements clarification.
> Context: [rough-idea.md](rough-idea.md), M2 [design.md](../design.md)

---

## Standing Assumptions

These are reasonable defaults assumed unless overridden by Q&A:

- **Profile name:** `compact` (not `hypershift-compact` — project-agnostic, reusable)
- **Coexistence:** `compact` sits alongside `rh-scrum` in `skeletons/profiles/`
- **Generator model:** Same `just init --profile=compact` flow
- **Single Telegram bot:** One member = one bot
- **Knowledge/invariant scoping:** Same recursive model, simplified for one member
- **Sprint 5 scope:** Profile skeleton + member skeleton + validation with synthetic tasks. No changes to generator infrastructure (Justfile `init` already supports arbitrary profiles).

---

## Q1: Does the compact profile still use `.github-sim/`?

**Q:** The `rh-scrum` profile's coordination model is built around `.github-sim/` issues with status labels, write-locks, and submodule syncing — all designed for multi-agent coordination. With a single member, this machinery is overhead. Does the compact profile still use `.github-sim/` for tracking work, or does it use a simpler task model (e.g., a scratchpad, a task list file, or just the PROMPT.md objective)?

**A:** Same model — `.github-sim/` with status labels, issues, the full coordination fabric. The compact profile reuses the same infrastructure even though there's only one agent. This keeps the model uniform and means work done in the compact profile is visible/transferable to a full `rh-scrum` team if needed.

---

## Q2: Status label lifecycle — who drives transitions?

**Q:** In `rh-scrum`, status labels encode role ownership (`status/arch:design`, `status/po:review`). The lifecycle works because different agents watch for their role prefix. In the compact profile, the single member owns all roles. Does the agent still use `status/<role>:*` labels and self-transition between them (acting as architect, then as PO, then as dev, etc.)? Or does it use a simplified label scheme since there's no role-based dispatch?

**A:** Same label scheme. The board scanner watches for all `status/*` labels (not just one role's prefix) and triggers the appropriate hat. Same self-transition pattern — the agent acts as architect, transitions to `status/po:design-review`, then its board scanner picks that up and triggers the review_gater hat. Keeps the model uniform with `rh-scrum`.

---

## Q3: Hat decomposition — how do the six roles map to hats?

**Q:** You listed six roles: human-assistant, architect, dev, SRE, QE, content writer. In `rh-scrum` M2, the architect has 5 hats (board_scanner, designer, planner, breakdown_executor, epic_monitor) and the human-assistant has 3 (board_scanner, backlog_manager, review_gater). For the compact profile, these all merge into one member. What's the hat model? Specifically:

- The board_scanner merges into one that watches all `status/*` labels — straightforward.
- Do the role-specific hats (designer, planner, review_gater, etc.) stay as-is, just all owned by one member?
- What new hats do dev, SRE, QE, and content writer need?

**A:** Hat decomposition by role:

| Role | Hats | Responsibility |
|------|------|---------------|
| (unified) | `board_scanner` | Watches all `status/*` labels, dispatches to appropriate hat |
| Human-assistant | `backlog_manager` | Triage, backlog prioritization |
| Human-assistant | `review_gater` | Present artifacts to human via HIL, gate decisions |
| Architect | `designer` | Creates proposal designs |
| Architect | `breakdown` | Story breakdown from approved design |
| Developer | `implementer` | Writes code |
| Developer | `code_reviewer` | Reviews PRs from dev perspective |
| SRE | `infra_setup` | Sets up test infrastructure (clusters for E2E tests) |
| QE | `test_designer` | Designs tests before dev writes them (TDD) |
| QE | `test_debugger` | Debugs tests, reviews PRs from QE perspective |
| Content writer | `writer` | Writes documentation/content |
| Content writer | `content_reviewer` | Reviews content |

That's 12 hats total in one Ralph instance. The board_scanner dispatches to the right hat based on the status label it finds.

---

## Q4: Story lifecycle — what statuses trigger the new hats?

**Q:** The epic lifecycle from M2 is: `po:triage → po:backlog → arch:design → po:design-review → arch:plan → po:plan-review → arch:breakdown → po:ready → arch:in-progress → po:accept → done`. For stories under an epic, what's the lifecycle? Specifically, where do dev, QE, SRE, and content writer fit? A reasonable default based on the M3 master plan sketch:

```
status/qe:test-design → status/dev:implement → status/qe:verify → status/dev:code-review → status/arch:sign-off → status/po:merge → status/done
```

With SRE triggered on-demand (e.g., `status/sre:infra-setup` when E2E tests need a cluster) and content writer triggered for docs stories (`status/cw:write → status/cw:review`). Does this match your thinking, or different?

**A:** Yes. Story lifecycle follows the TDD flow. SRE and content writer are on-demand/docs-specific.

---

## Q5: HIL model — does the human still gate all decisions?

**Q:** In `rh-scrum` M2 training mode, the human gates every transition via Telegram. In the compact profile, the single agent self-transitions between roles rapidly (designer → review_gater → breakdown → etc.). If every transition requires human approval via Telegram, the workflow becomes very chatty — 10+ approval prompts per epic.

Reasonable assumption: the compact profile defaults to **supervised mode** — the human gates major decisions (design approval, plan approval, final acceptance) but intermediate transitions (e.g., `qe:test-design → dev:implement`) auto-advance. The review_gater hat only fires for the major gates. Does this work, or do you want full training mode?

**A:** Yes — supervised mode. Major gates only.

---

## Q6: Codebase access — same fork chain model?

**Q:** The architect in `rh-scrum` clones the agentic-team fork into `workspace/projects/hypershift/` (gitignored). Since the compact member also has dev, QE, and SRE hats that need code access, the same model applies — but now the single member reads AND writes code, runs tests, and sets up infra, all in the same cloned repo. Same agent-cloned fork model?

**A:** Yes. All hats share the same cloned repo in the workspace.

---

## Q7: Sprint 5 scope — what's the deliverable?

**Q:** Sprint 5 needs to produce the `compact` profile skeleton and validate it. Reasonable scope:

1. **Profile skeleton:** `skeletons/profiles/compact/` — PROCESS.md (same status conventions, supervised mode defaults), CLAUDE.md, knowledge/, invariants/, single member skeleton with ralph.yml (12 hats), PROMPT.md, hat instructions
2. **Generator compatibility:** `just init --profile=compact` works out of the box (no generator changes needed — it already supports arbitrary profiles)
3. **Validation:** Synthetic epic exercised end-to-end by the single member — design, breakdown, TDD story flow, completion. Similar to Sprint 2's autonomous validation but with one agent.

Is this the right scope, or do you want to include/exclude anything?

**A:** Yes — profile skeleton, generator compatibility (no changes needed), and synthetic end-to-end validation.

---

## Q8: Write-lock simplification — skip or keep?

**Q:** With a single member, write-locks are unnecessary (no concurrent writers). Two options:
- **(a)** Keep the write-lock protocol in PROCESS.md for uniformity but the agent never actually contends — effectively a no-op.
- **(b)** Skip write-locks entirely in the compact profile's PROCESS.md — simpler, fewer wasted cycles.

Assuming (b) — the compact PROCESS.md documents that write-locks are not needed since there's a single member, but the status label and `.github-sim/` conventions are identical to `rh-scrum`.

**A:** Skip write-locks. Single member, no contention.

---

## Q9: PROCESS.md — shared or independent?

**Q:** The compact profile needs a PROCESS.md covering status labels, issue format, comment format, etc. Most of this is identical to `rh-scrum`'s PROCESS.md. Two options:
- **(a)** Copy and adapt — compact gets its own PROCESS.md, mostly duplicated from `rh-scrum` but with additions (story lifecycle statuses, no write-locks, supervised mode defaults).
- **(b)** Factor out a shared base — extract common conventions into a reusable base, both profiles reference it.

Assuming (a) for Sprint 5 — copy and adapt. Factoring out a shared base is a refactoring concern for later. Keep Sprint 5 focused on the new profile.

**A:** Copy and adapt. Shared base is a future refactoring concern.

---

## Requirements Complete

All major design questions resolved. Summary of the compact profile:

- **Profile:** `compact` in `skeletons/profiles/compact/`
- **Single member:** One Ralph instance, 15 hats covering all roles
- **Same `.github-sim/` model:** Status labels, issue format, comment format — identical to `rh-scrum`
- **Unified board scanner:** Watches all `status/*` labels, dispatches to appropriate hat
- **Self-transitioning:** Agent transitions between role-specific hats via status labels
- **Supervised mode:** Human gates major decisions only (design approval, plan approval, final acceptance); intermediate transitions auto-advance
- **No write-locks:** Single member, no contention
- **Same fork chain:** Agent-cloned repo shared by all hats
- **Story lifecycle:** `qe:test-design → dev:implement → dev:code-review → qe:verify → arch:sign-off → po:merge → done`
- **SRE:** On-demand `sre:infra-setup` for E2E test clusters
- **Content writer:** Docs stories via `cw:write → cw:review`
- **PROCESS.md:** Copied and adapted from `rh-scrum`
- **Scope:** Profile skeleton + generator compatibility + synthetic end-to-end validation

Ready for research / design.
