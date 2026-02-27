# Design Review — Milestone 2: Architect + First Epic

> Consolidated findings from three parallel review agents. Each finding is numbered
> for individual triage. Cross-referenced against `specs/design-principles.md`.
>
> **Reviewers:**
> - **R1:** Completeness & Consistency
> - **R2:** Architecture & Technical Soundness
> - **R3:** Acceptance Criteria & Testing Strategy

---

## Triage Decisions

| Disposition | Findings | Notes |
|---|---|---|
| **Dismissed** | 1, 2, 7, 12, 14, 15, 17, 19, 20, 21 | See rationale below |
| **Accepted — design change applied** | 8, 9, 10, 11, 13, 16, 18, 23, 26 | Changes applied to design.md |
| **Accepted — test/AC added** | 3, 4, 5, 6, 22 | Tests and/or ACs added to design.md |
| **Deferred — future improvement** | 24, 25, 27, 28, 29, 30, 31–68 | Documented in Appendix D |
| **Resolved** | 69 | By design-principles.md |

### Dismissal Rationale

- **#1 (issue number race):** Git push will conflict; Claude will detect and recover. Not a design gap.
- **#2 (lock acquisition atomicity):** Same — Claude will detect push failures and recover without explicit instructions.
- **#7 (TOCTOU):** The write lock is write-only. Status prefix partitioning (`arch:*` vs `po:*`) means two agents never modify the same issue simultaneously. The TOCTOU scenario requires both agents to write to the same issue, which can't happen.
- **#12 (`hats:` frontmatter):** Research confirmed Ralph has the infrastructure (`hats:` frontmatter, `SkillRegistry::is_visible()`, unit tests) but the EventLoop doesn't pass the active hat ID at runtime — it calls `build_index(None)`. Hat-level skills deferred to post-POC; hat-level knowledge/invariants remain (instruction-driven, no runtime filtering needed).
- **#14 (migration checklist):** Still a POC. No formal migration needed.
- **#15 (single-branch serialization):** POC with 2 agents. Acceptable.
- **#17 (compound AC 7.6):** Not a problem for a POC.
- **#19 (training mode AC):** Not important for POC scope.
- **#20 (push-conflict AC):** Not important for POC scope.
- **#21 (failed-processing test):** Not important for POC scope.

---

## Findings Table

| # | Severity | Theme | Finding | Reviewers | Disposition |
|---|----------|-------|---------|-----------|-------------|
| 1 | CRITICAL | Lock | Race condition in issue number allocation | R2 | DISMISSED — Git push conflicts; Claude recovers |
| 2 | CRITICAL | Lock | Lock acquisition not atomic; push-failure recovery missing from hat instructions | R1, R2 | DISMISSED — Claude recovers from push failures |
| 3 | CRITICAL | Testing | No concurrent operations test | R3 | **ACCEPTED** — test added (8.7) |
| 4 | CRITICAL | AC | AC 7.5 "demonstrably reflects" is subjective and not machine-verifiable | R3 | **ACCEPTED** — AC rewritten with grep-able markers; test added (8.10) |
| 5 | CRITICAL | Testing | No push-conflict resolution test | R3 | **ACCEPTED** — test added (8.8) |
| 6 | CRITICAL | Testing | No test for agent crash during lock-held state | R3 | **ACCEPTED** — test added (8.9) |
| 7 | MAJOR | Lock | TOCTOU: work hats read stale issue state before acquiring lock | R2 | DISMISSED — write lock + status prefix partitioning prevents this |
| 8 | MAJOR | Workspace | `.git/info/exclude` is local-only; `git add -A` could stage `.botminter/` | R2 | **ACCEPTED** — belt-and-suspenders `.gitignore` added |
| 9 | MAJOR | Workspace | Push-conflict handling missing from individual work hat instructions | R2 | **ACCEPTED** — push-conflict protocol added to CLAUDE.md |
| 10 | MAJOR | Workspace | `just sync` has no mechanism to know which member it serves | R1 | **ACCEPTED** — `.botminter/.member` marker file added |
| 11 | MAJOR | Workspace | Human-assistant workspace layout unclear (clones project repo it never reads?) | R1 | **ACCEPTED** — explicit statement: HA follows same model for consistency |
| 12 | MAJOR | Capabilities | `hats:` frontmatter skill filtering assumed but never validated | R1 | **ACCEPTED** — confirmed Ralph has infrastructure but runtime doesn't wire it up. Hat-level skills deferred to post-POC |
| 13 | MAJOR | Requirements | Workspace model diverges from Q4 requirement without explicit annotation | R1 | **ACCEPTED** — departure noted in Section 4.6 |
| 14 | MAJOR | Migration | Destroy-and-recreate lacks a migration checklist | R2 | DISMISSED — POC scope |
| 15 | MAJOR | Scalability | Single-branch team repo serializes all pushes; quadratic conflict rate at 5 agents | R2 | DISMISSED — POC scope |
| 16 | MAJOR | AC | AC 7.3 "through to `status/po:ready` (M2 end state)" contradicts 7.3b (full lifecycle to done) | R3 | **ACCEPTED** — split into 7.3 (triage→ready) and 7.3b (ready→done) |
| 17 | MAJOR | AC | AC 7.6 is compound — crams 6 assertions into one criterion | R3 | DISMISSED — acceptable for POC |
| 18 | MAJOR | AC | No acceptance criteria for `just init` overlay of new profile content | R3 | **ACCEPTED** — AC 7.6b added |
| 19 | MAJOR | AC | No acceptance criteria for training mode behavior | R3 | DISMISSED — not important for POC |
| 20 | MAJOR | AC | No acceptance criteria for push-conflict handling (Section 6.2) | R3 | DISMISSED — not important for POC |
| 21 | MAJOR | Testing | No test for failed-processing escalation (3-failure → `status/error`) | R3 | DISMISSED — not important for POC |
| 22 | MAJOR | Testing | No test for plan-review rejection loop | R3 | **ACCEPTED** — test added (8.6) |
| 23 | MAJOR | Testing | Integration test sequence mixes automated and manual steps without delineation | R3 | **ACCEPTED** — steps annotated (A)/(M); future idea for Telegram simulation |
| 24 | MAJOR | Testing | No test for `status/error` label removal and retry resumption | R3 | DEFERRED — Appendix D |
| 25 | MAJOR | Testing | No test for Telegram token omission error | R3 | DEFERRED — Appendix D |
| 26 | MAJOR | Testing | Day-2 test (step 19) is underspecified | R3 | **ACCEPTED** — expanded into sub-steps 19a–19e |
| 27 | MAJOR | Testing | No test for invariant updates propagating to running agents mid-operation | R3 | DEFERRED — Appendix D |
| 28 | MAJOR | Testing | No test for training mode vs autonomous mode | R3 | DEFERRED — Appendix D |
| 29 | MAJOR | Testing | No test for multi-bot Telegram setup | R3 | DEFERRED — Appendix D |
| 30 | MAJOR | Testing | No test for `create-epic` and `board` skills | R3 | DEFERRED — Appendix D |
| 31 | MINOR | Lock | 5-minute stale lock threshold could be exceeded by breakdown_executor | R2 | DEFERRED — Appendix D |
| 32 | MINOR | Lock | No AC for lock-verify-before-push behavior | R3 | DEFERRED — Appendix D |
| 33 | MINOR | Lock | No AC for new issue number allocation uniqueness | R3 | DEFERRED — Appendix D |
| 34 | MINOR | Lock | Lock contention test only covers simple case (no true simultaneous contention) | R3 | DEFERRED — Appendix D |
| 35 | MINOR | Workspace | Skills dirs hardcode "hypershift" project name | R2 | DEFERRED — Appendix D |
| 36 | MINOR | Workspace | `just sync` doesn't handle pull conflicts gracefully | R2 | DEFERRED — Appendix D |
| 37 | MINOR | Workspace | No test for `.claude/agents/` symlink assembly correctness | R3 | DEFERRED — Appendix D |
| 38 | MINOR | Workspace | No test for `just sync` re-assembly behavior | R3 | DEFERRED — Appendix D |
| 39 | MINOR | Dispatch | HA priority order may delay architect (triages before reviewing) | R2 | DEFERRED — Appendix D |
| 40 | MINOR | Dispatch | No test for board scanner priority ordering | R3 | DEFERRED — Appendix D |
| 41 | MINOR | Dispatch | No test for idempotent dispatch | R3 | DEFERRED — Appendix D |
| 42 | MINOR | Migration | M1 Justfile recipes must be fully replaced, not augmented — not stated explicitly | R2 | DEFERRED — Appendix D |
| 43 | MINOR | Scalability | Stale lock cleanup centralized in human-assistant only | R2 | DEFERRED — Appendix D |
| 44 | MINOR | Scalability | Issue numbering race worsens with more agents (M3 concern) | R2 | DEFERRED — Appendix D |
| 45 | MINOR | Requirements | "No-op" (requirements) vs "fast-forward" (design) terminology for `arch:in-progress` | R1 | DEFERRED — Appendix D |
| 46 | MINOR | Requirements | Skills dirs list inconsistent with skeleton directory tree | R1 | DEFERRED — Appendix D |
| 47 | MINOR | Requirements | No `project` field in epic frontmatter for multi-project teams | R1, R3 | DEFERRED — Appendix D |
| 48 | MINOR | Requirements | `just launch` recipe implementation not shown (less detail than `create-workspace`) | R1 | DEFERRED — Appendix D |
| 49 | MINOR | Requirements | PROCESS.md sync protocol still references "submodule" | R1 | DEFERRED — Appendix D |
| 50 | MINOR | Requirements | No timeout handling specified for review_gater `human.interact` calls | R1 | DEFERRED — Appendix D |
| 51 | MINOR | AC | "discoverable via skills.dirs" in AC 7.8 — unclear how to verify | R3 | DEFERRED — Appendix D |
| 52 | MINOR | AC | "extended period" in AC 7.3b is vague | R3 | DEFERRED — Appendix D |
| 53 | MINOR | AC | AC 7.2 combines too many conditions (4 behaviors in one) | R3 | DEFERRED — Appendix D |
| 54 | MINOR | AC | No AC for agent startup self-cleanup — wording could be tighter | R3 | DEFERRED — Appendix D |
| 55 | MINOR | Testing | No test for epic rejection at `po:accept` stage | R3 | DEFERRED — Appendix D |
| 56 | MINOR | Testing | No test for PROMPT.md symlink propagation | R3 | DEFERRED — Appendix D |
| 57 | MINOR | Testing | No test for ralph.yml change detection and warning | R3 | DEFERRED — Appendix D |
| 58 | NOTE | Lock | Lock files in git create commit noise (M3 concern) | R2 | DEFERRED — Appendix D |
| 59 | NOTE | Dispatch | Training mode creates double-confirmation per action (board scanner + work hat) | R2 | DEFERRED — Appendix D |
| 60 | NOTE | Dispatch | `po:ready` routing to backlog_manager (not review_gater) is correct | R2 | NOTED |
| 61 | NOTE | Dispatch | `board.rescan` vs `board.scan` dual-path idle detection is well-designed | R1 | NOTED |
| 62 | NOTE | Scalability | Separate Telegram bots per agent is operationally noisy at scale | R2 | DEFERRED — Appendix D |
| 63 | NOTE | Requirements | Training mode scope extension to all members is a good enhancement | R1 | NOTED |
| 64 | NOTE | Requirements | No cross-role stale lock cleanup (if HA crashes, its lock waits for HA restart) | R1 | DEFERRED — Appendix D |
| 65 | NOTE | Requirements | No rejection cycle limit — deliberate, human controls | R1 | NOTED |
| 66 | NOTE | Requirements | Open questions (Section 4.10) appropriately deferred | R3 | NOTED |
| 67 | NOTE | Research | Review presets (Gastown) not considered — future idea | R1 | DEFERRED — Appendix D |
| 68 | NOTE | Testing | No test for `human.guidance` (deferred feature) | R3 | NOTED |
| 69 | RESOLVED | — | `cooldown_delay_seconds` omitted — resolved by design-principles.md Principle 5 | R1 | RESOLVED |

---

## Detailed Findings

### CRITICAL

#### Finding 1: Race condition in issue number allocation

**Reviewers:** R2 (A-1)
**Theme:** Lock Protocol
**Sections:** 4.4.2 (Acquire for new issue/PR creation)

The "scan for highest existing number + 1" approach has a race window:

1. Agent A scans, sees highest issue is #5, decides next is #6
2. Agent B scans, sees highest issue is #5, decides next is #6
3. Both try to acquire the lock for #6

The push step provides some protection (second push fails on divergent histories), but the design does not specify recovery for this case. Step 4 of the protocol says "verify no issue file with that number was created between scan and lock" — but this check is local. If Agent A acquired the lock but hasn't pushed the issue file yet (only the lock), Agent B's verification passes.

**Recommendation:** Combine lock acquisition + issue creation into a single atomic commit-push. If push fails, pull, re-scan for new highest number, retry. The current two-step approach introduces a window where both agents hold locks for the same number on different local copies.

---

#### Finding 2: Lock acquisition not atomic; push-failure recovery missing from hat instructions

**Reviewers:** R1 (C1), R2 (A-2)
**Theme:** Lock Protocol
**Sections:** 4.4.2, 6.2, all hat instructions in 4.1.1 and 4.2.1

The lock acquire protocol is: check locally if lock exists → write lock → commit → push. Between the local check and the push, another agent could push the same lock. The push fails, but the hat instructions don't include recovery.

Section 6.2 describes the mitigation: "Pull with rebase, re-check lock ownership, abort if lock was taken." But hat instructions (e.g., designer hat step 6) just say: "Commit and push. If the lock file already exists, wait and retry on next scan." The "already exists" check happens locally before writing — it doesn't cover the push-failure case.

This matters because LLM agents follow their instructions literally. If the recovery path isn't in the instructions, the agent won't execute it.

**Recommendation:** The full lock protocol (acquire → verify → recover on failure) should be in PROMPT.md as a cross-hat concern (per design-principles.md Principle 1). Each hat's instructions should reference it rather than inlining a partial version.

**Cross-reference with design-principles.md:** Principle 1 says "write-lock protocol" belongs in PROMPT.md as a cross-hat behavioral rule. The design's PROMPT.md template (Section 4.1.2) includes the protocol but omits push-failure recovery.

---

#### Finding 3: No concurrent operations test

**Reviewers:** R3 (F1)
**Theme:** Testing Strategy
**Sections:** 8.2

M2's core thesis is "two independent Ralph instances coordinating through `.github-sim/` without direct communication." The integration test sequence (8.2) runs agents sequentially: step 8 launches the human-assistant, step 10 launches the architect. AC 7.3 says "both agents run simultaneously" but the test steps are sequential.

**Recommendation:** Add a dedicated concurrency test: launch both agents, create an epic, verify the full lifecycle completes without lock collisions, lost updates, or duplicate processing.

---

#### Finding 4: AC 7.5 "demonstrably reflects" is subjective

**Reviewers:** R3 (B1)
**Theme:** Acceptance Criteria Quality
**Sections:** 7.5, 8.1

"The design demonstrably reflects knowledge from ALL three scopes" is not machine-verifiable. Two reviewers could disagree on whether a design "reflects" knowledge. The synthetic fixtures (Section 8.1) define detection criteria ("Design doc mentions issue-referencing commits"), but these are in the testing section, not in the acceptance criteria themselves.

**Recommendation:** Rewrite AC 7.5 to reference specific, grep-able markers. Example: "the design doc contains the phrase 'reconciler pattern' (from project-level knowledge) AND 'composition over inheritance' (from member-level knowledge) AND references issue numbers in commit examples (from team-level knowledge)."

---

#### Finding 5: No push-conflict resolution test

**Reviewers:** R3 (C1)
**Theme:** Testing Strategy
**Sections:** 6.2, 8

Section 6.2 describes a specific push-conflict handling flow: pull-rebase, check lock ownership, log errors, retry push. This is a plausible real-world scenario (architect pushes a status transition while human-assistant pushes a lock cleanup simultaneously). No test in Section 8 covers this.

**Recommendation:** Add a test that simulates concurrent pushes and verifies the pull-rebase-retry recovery.

---

#### Finding 6: No test for agent crash during lock-held state

**Reviewers:** R3 (D1)
**Theme:** Testing Strategy
**Sections:** 6.4, 8

Section 6.4 describes crash recovery: stale lock left behind, human-assistant cleans it up, agent scans for own stale locks on restart. The stale lock test (8.4) partially covers this but doesn't include the "restart and self-cleanup" part.

**Recommendation:** Add a test that: (1) simulates an agent crash while holding a lock, (2) verifies the human-assistant cleans the stale lock, (3) restarts the agent and verifies it cleans its own stale lock on startup.

---

### MAJOR

#### Finding 7: TOCTOU — work hats read stale issue state before acquiring lock

**Reviewers:** R2 (D-1)
**Theme:** Sync & Propagation
**Sections:** 4.1.1 (designer workflow steps 1-6)

The designer hat reads the epic issue (step 1) before acquiring the lock (step 6). Between the read and the lock acquisition, the issue could have been updated by the other agent (e.g., human-assistant pushed rejection feedback). The designer produces a design without incorporating the latest state.

**Recommendation:** Add a "pull before read" step to each work hat's workflow after acquiring the lock. The sequence should be: acquire lock → pull `.botminter/` → read issue → produce work → commit → verify lock → push → release.

**Cross-reference with design-principles.md:** Principle 6 says "sync-before-scan/push-after-write." This should be extended to include "pull-after-lock-acquire."

---

#### Finding 8: `.git/info/exclude` is local-only

**Reviewers:** R2 (B-1)
**Theme:** Workspace Model
**Sections:** 4.6.1

`.git/info/exclude` is not shared between clones. If an agent's workspace is recreated or the file is corrupted, the exclusion rules are lost. Unlike `.gitignore`, this does not propagate through git. An accidental `git add -A` could stage `.botminter/`.

**Recommendation:** Add `.botminter/` to a project-level `.gitignore` in addition to `.git/info/exclude` as belt-and-suspenders. Add a verification step to `just sync` that checks `.git/info/exclude` is populated and repairs if necessary.

---

#### Finding 9: Push-conflict handling missing from individual work hat instructions

**Reviewers:** R2 (B-3)
**Theme:** Workspace Model
**Sections:** 4.1.1, 4.2.1

Hat instructions say "commit, push, release the lock" without specifying what happens if the push fails. Section 6.2 describes the recovery but it's not in the instructions the LLM agent follows. If push fails and the agent doesn't handle it, the lock will be orphaned.

**Recommendation:** Add push-failure handling to each work hat or (better) put the full push protocol in PROMPT.md as a cross-hat concern. Per Finding 2, this overlaps with lock protocol — the entire "commit → verify lock → push → handle failure → release lock" sequence should be a single reusable protocol in PROMPT.md.

---

#### Finding 10: `just sync` has no mechanism to know which member it serves

**Reviewers:** R1 (C3)
**Theme:** Workspace Model
**Sections:** 4.6.3

The `sync` recipe needs to check `team/<member>/ralph.yml` freshness and re-copy member-specific files. But `just -f .botminter/Justfile sync` has no member parameter. The board scanner just calls `just sync` with no argument.

**Recommendation:** Either (a) persist a marker file in the workspace during `create-workspace` (e.g., `.botminter/.member` containing the role name) that `sync` reads, or (b) add a `<member>` argument to `just sync` and update all board scanner instructions.

---

#### Finding 11: Human-assistant workspace layout unclear

**Reviewers:** R1 (C5)
**Theme:** Workspace Model
**Sections:** 4.6, 4.11

The design describes the architect's workspace in detail (CWD = project repo clone with `.botminter/` inside). But the human-assistant does not need codebase access — it is a PO proxy. Does it also clone the entire project repo as its CWD?

Section 4.11 shows `hypershift-ha/` as a "project repo clone," suggesting yes. This clones an entire codebase the HA never reads.

**Recommendation:** Explicitly state whether the HA follows the same `.botminter/` model for consistency or uses a simpler workspace model. If the same model, document the rationale (consistency > efficiency for 2 agents).

---

#### Finding 12: `hats:` frontmatter skill filtering assumed but never validated

**Reviewers:** R1 (C9)
**Theme:** Agent Capabilities
**Sections:** 4.7.3

Section 4.7.3 states: "Hat-level skills use Ralph's `hats:` frontmatter field for visibility filtering." No research document, M1 implementation, or M1.5 spike validates that Ralph supports this feature. If Ralph does not support `hats:` frontmatter filtering, all hat-level skills would be visible to all hats at all times.

**Recommendation:** Validate the `hats:` frontmatter feature with a quick spike before relying on it. If unsupported, design an alternative (e.g., skill naming conventions that hats check, or simply accept all skills visible to all hats for M2).

---

#### Finding 13: Workspace model diverges from Q4 without explicit annotation

**Reviewers:** R1 (A/Q4, B1)
**Theme:** Requirements Coverage
**Sections:** Requirements Q4, Design 4.6

Requirements Q4 says "agent-cloned into a gitignored `projects/` directory in the workspace." The design inverts this: the agent's CWD IS the project repo, with the team repo cloned into `.botminter/`. The design is a better approach but the requirements document tells a contradictory story.

**Recommendation:** Annotate Q4 in requirements.md to note the design departure and rationale.

---

#### Finding 14: Destroy-and-recreate lacks a migration checklist

**Reviewers:** R2 (E-1)
**Theme:** Migration
**Sections:** 4.6

Section 4.6 says "Destroy and recreate workspaces. M1 workspaces are not compatible." But doesn't specify:

- What happens to Ralph memories (`.ralph/`)?
- That `team/human-assistant/ralph.yml` must be updated (new events, new hats)?
- That `team/human-assistant/PROMPT.md` and `CLAUDE.md` must be updated?
- That team-level `CLAUDE.md` must be updated with `.botminter/` model?

**Recommendation:** Add a migration checklist to the design.

---

#### Finding 15: Single-branch team repo serializes all pushes

**Reviewers:** R2 (F-1)
**Theme:** Scalability
**Sections:** 6.2

All agents push to `main` on the team repo. With 2 agents in M2, push conflicts are manageable. With 5 agents in M3, each cycle involves ~2 pushes per agent (lock acquire + work), totaling ~10 pushes per scan cycle. Push-pull conflicts will be frequent.

**Recommendation:** Acceptable for M2. For M3 design, consider per-agent branches, lock-free coordination, or tuned retry backoff.

---

#### Finding 16: AC 7.3 contradicts 7.3b

**Reviewers:** R3 (B2)
**Theme:** Acceptance Criteria Quality
**Sections:** 7.3, 7.3b

Section 7.3 says the epic traverses "through to `status/po:ready` (M2 end state)." Section 7.3b explicitly tests `po:ready` → `arch:in-progress` → `po:accept` → `done`. The parenthetical "(M2 end state)" is misleading.

**Recommendation:** Remove the parenthetical from 7.3 or change it to "through to `status/done`" and let 7.3b elaborate the individual steps.

---

#### Finding 17: AC 7.6 is compound

**Reviewers:** R3 (B3)
**Theme:** Acceptance Criteria Quality
**Sections:** 7.6

AC 7.6 first bullet crams six assertions into one criterion: (1) PROMPT.md is a symlink, (2) CLAUDE.md is a symlink, (3) ralph.yml is a copy, (4) settings.local.json is a copy, (5) `.claude/agents/` contains symlinks, (6) `.git/info/exclude` contains patterns.

**Recommendation:** Split into separate testable assertions.

---

#### Finding 18: No AC for `just init` overlay of new profile content

**Reviewers:** R3 (A1)
**Theme:** Acceptance Criteria
**Sections:** 3.2, 4.7, 7

No criterion verifies that `just init` correctly overlays the new M2 profile artifacts (architect member skeleton, `agent/` directory hierarchy, team-level skills) into a generated team repo.

**Recommendation:** Add an AC verifying that `just init` with `rh-scrum` profile produces a repo containing the `agent/` directory at the correct scoping levels.

---

#### Finding 19: No AC for training mode behavior

**Reviewers:** R3 (A2)
**Theme:** Acceptance Criteria
**Sections:** 2.4, 7

Training mode is defined across all hats. No Given-When-Then covers an agent in training mode reporting its intended action and waiting for confirmation, or what happens on timeout.

**Recommendation:** Add ACs for training mode: agent reports action, waits, proceeds on confirmation; retries on timeout.

---

#### Finding 20: No AC for push-conflict handling

**Reviewers:** R3 (A3)
**Theme:** Acceptance Criteria
**Sections:** 6.2, 7

Section 6.2 describes a specific error handling flow. No AC covers it.

**Recommendation:** Add an AC: "Given two agents push to `.botminter/` near-simultaneously, when one push fails, then the agent pulls with rebase, re-checks lock ownership, and retries."

---

#### Finding 21: No test for failed-processing escalation

**Reviewers:** R3 (C2)
**Theme:** Testing Strategy
**Sections:** 6.5, 7.10, 8

Section 6.5 describes three-stage escalation (retry 1-3, error label, HIL notification). AC 7.10 covers it. But Section 8 has no test step that intentionally causes failures or verifies the 3-failure threshold.

**Recommendation:** Add a test that intentionally triggers processing failures and verifies the escalation to `status/error`.

---

#### Finding 22: No test for plan-review rejection loop

**Reviewers:** R3 (C3)
**Theme:** Testing Strategy
**Sections:** 3.1 Scenario C, 8.5

Section 8.5 (Rejection Loop Test) only covers design rejection. Plan rejection (Scenario C) has different behavior: revised breakdown as a "new comment" vs revised design that "replaces the previous design doc."

**Recommendation:** Add a plan-rejection test step to Section 8.5.

---

#### Finding 23: Integration test mixes automated and manual steps

**Reviewers:** R3 (C4)
**Theme:** Testing Strategy
**Sections:** 8.2

Steps 8-18 require running live Ralph instances and interacting via Telegram. No discussion of how to script the Telegram interactions or whether they require live interaction.

**Recommendation:** Explicitly state which test steps are manual (require live Telegram) and which can be automated. Consider whether Telegram interactions can be simulated for automated testing.

---

#### Finding 24: No test for `status/error` label removal and retry

**Reviewers:** R3 (D2)
**Theme:** Testing Strategy
**Sections:** 7.10, 8

AC 7.10 bullet 3 says removing `status/error` re-enables dispatch. No test step covers this.

**Recommendation:** Add a test step that removes the error label and verifies the board scanner resumes dispatch.

---

#### Finding 25: No test for Telegram token omission error

**Reviewers:** R3 (D3)
**Theme:** Testing Strategy
**Sections:** 7.9, 8

AC 7.9 bullet 2 says launching without `--telegram-bot-token` aborts with a clear error. No test step covers this.

**Recommendation:** Add a test step for the negative case.

---

#### Finding 26: Day-2 test underspecified

**Reviewers:** R3 (E1)
**Theme:** Testing Strategy
**Sections:** 8.2 step 19

Step 19 is a single sentence: "Edit a member knowledge file in the team repo, commit, push. Verify the agent picks up the change on next just sync without workspace recreation." Does not specify which knowledge scope, how to verify pickup, or whether symlink propagation is tested.

**Recommendation:** Expand step 19 into sub-steps covering (a) team-level knowledge edit, (b) project-level knowledge edit, (c) member-level knowledge edit, (d) PROMPT.md symlink verification, (e) verification method (agent output references updated content).

---

#### Finding 27: No test for invariant updates propagating mid-operation

**Reviewers:** R3 (E2)
**Theme:** Testing Strategy
**Sections:** 4.6.2, 8

The synthetic invariant setup is done before first run. No test verifies that a NEW invariant added mid-operation (day-2 scenario) is enforced by the architect on its next design.

**Recommendation:** Add a day-2 invariant propagation test.

---

#### Finding 28: No test for training mode vs autonomous mode

**Reviewers:** R3 (F2)
**Theme:** Testing Strategy
**Sections:** 2.4, 8

Training mode is central to M2 operation. No test verifies agents wait for confirmation, nor what happens when training mode is disabled.

**Recommendation:** Add training mode tests (confirmation flow, timeout behavior).

---

#### Finding 29: No test for multi-bot Telegram setup

**Reviewers:** R3 (F3)
**Theme:** Testing Strategy
**Sections:** 2.7, 7.9, 8

No test verifies that two agents launched with different Telegram tokens communicate independently with the human.

**Recommendation:** Add a test that launches both agents with different tokens and verifies independent communication.

---

#### Finding 30: No test for `create-epic` and `board` skills

**Reviewers:** R3 (F4)
**Theme:** Testing Strategy
**Sections:** 7.8, 8

AC 7.8 bullets 3-4 define acceptance criteria for `create-epic` and `board` skills. No test step exercises them.

**Recommendation:** Add test steps for skill invocation.

---

### MINOR

#### Finding 31: Stale lock threshold could be exceeded by breakdown_executor

**Reviewers:** R2 (A-3)
**Sections:** 2.5, 4.1.1

The 5-minute threshold is "generous" for simple status transitions, but the breakdown_executor creates multiple story issues while holding a single lock on the epic. With 5-8 stories, sequential git operations could approach the threshold.

**Recommendation:** Either raise the threshold to 10-15 minutes or ensure the breakdown_executor acquires the lock only for the final commit (not during individual story creation).

---

#### Finding 32: No AC for lock-verify-before-push behavior

**Reviewers:** R3 (A5)
**Sections:** 4.4.2, 7

The verify step (pull, check lock still yours, abort if stolen) has no AC.

**Recommendation:** Add an AC for the lock-defense scenario.

---

#### Finding 33: No AC for new issue number allocation uniqueness

**Reviewers:** R3 (A6)
**Sections:** 4.4.2, 7

No AC verifies that concurrently created issues receive unique, sequential numbers.

**Recommendation:** Add an AC, especially if Finding 1 is addressed.

---

#### Finding 34: Lock contention test only covers simple case

**Reviewers:** R3 (C5)
**Sections:** 8.3

Section 8.3 tests: create a lock, start architect, verify skip. Does not test true simultaneous contention or verify-before-push failure.

**Recommendation:** Extend the lock contention test with a concurrent scenario.

---

#### Finding 35: Skills dirs hardcode "hypershift" project name

**Reviewers:** R2 (B-4)
**Sections:** 4.1.1, 4.2.1

Both ralph.yml configs hardcode `.botminter/projects/hypershift/agent/skills` in `skills.dirs`. Not project-agnostic.

**Recommendation:** Document as a known limitation. For M3+, consider templating the project name during workspace creation.

---

#### Finding 36: `just sync` doesn't handle pull conflicts gracefully

**Reviewers:** R2 (D-3)
**Sections:** 4.6.3

If either pull (team repo or project repo) results in a merge conflict, `just sync` fails. No error handling specified.

**Recommendation:** Add error handling: if pull fails, log error and continue with stale data rather than crashing.

---

#### Finding 37: No test for `.claude/agents/` symlink assembly

**Reviewers:** R3 (F6)
**Sections:** 7.8, 8

No test verifies the symlinks in `.claude/agents/` are correct, resolve properly, and come from all expected layers.

**Recommendation:** Add a workspace verification test step.

---

#### Finding 38: No test for `just sync` re-assembly behavior

**Reviewers:** R3 (F7)
**Sections:** 7.7, 8

No test verifies that adding a new agent file to one layer and running `just sync` makes it appear in `.claude/agents/`.

**Recommendation:** Add a sync re-assembly test step.

---

#### Finding 39: HA priority order may delay architect

**Reviewers:** R2 (C-2)
**Sections:** 4.2.1

The human-assistant prioritizes `po:triage` over `po:design-review`. If a new epic arrives in triage while a design review is pending, the HA triages the new epic before presenting the review. This delays the architect who is waiting.

**Recommendation:** Consider swapping HA priority: `po:design-review > po:plan-review > po:accept > po:triage > po:backlog > po:ready`. Principle: unblock in-progress work before accepting new work. Minor for M2 (one epic at a time).

---

#### Finding 40: No test for board scanner priority ordering

**Reviewers:** R3 (F5)
**Sections:** 4.1.1, 4.2.1, 8

Both board scanners define priority ordering. No test verifies that with multiple issues at different statuses, the highest-priority one is dispatched first.

**Recommendation:** Add a multi-issue priority test.

---

#### Finding 41: No test for idempotent dispatch

**Reviewers:** R3 (C6)
**Sections:** 4.1.1, 4.2.1, 8

Board scanners describe idempotent dispatch (skip issues already at target status). This was a lesson from M1.5. No test verifies it.

**Recommendation:** Add an idempotent dispatch test.

---

#### Finding 42: M1 Justfile recipes must be fully replaced

**Reviewers:** R2 (E-2)
**Sections:** 4.8

The design specifies what the new recipes do but doesn't explicitly state the old `create-workspace` and `launch` recipes are replaced entirely (not extended).

**Recommendation:** Add an explicit note that M1 recipes are replaced.

---

#### Finding 43: Stale lock cleanup centralized in human-assistant only

**Reviewers:** R2 (F-2)
**Sections:** 2.5

Only the human-assistant cleans stale locks. If the HA crashes, no one cleans other agents' stale locks.

**Recommendation:** For M3, consider having every agent's board scanner check for stale locks, not just the HA's.

---

#### Finding 44: Issue numbering race worsens with more agents

**Reviewers:** R2 (F-3)
**Sections:** 4.4.2

The "scan for highest + 1" works for M2 (only architect creates issues). With 5 agents in M3, the race condition in Finding 1 becomes increasingly likely.

**Recommendation:** For M3, consider a centralized counter file (`.github-sim/NEXT_NUMBER`).

---

#### Finding 45: "No-op" vs "fast-forward" terminology

**Reviewers:** R1 (B7)
**Sections:** Requirements Q6, Design 2.3

Requirements say "No-op" for `arch:in-progress` in M2; design says "fast-forward." Same meaning, different terms.

**Recommendation:** Align terminology. "Fast-forward" is more accurate since the hat does transition status.

---

#### Finding 46: Skills dirs list inconsistent with skeleton directory tree

**Reviewers:** R1 (B11)
**Sections:** 4.1.1

`skills.dirs` includes `breakdown_executor` hat-level skills, but the skeleton tree only shows hat directories for `designer/` and `planner/`. `epic_monitor` hat-level skills are in neither.

**Recommendation:** Align the skeleton tree and skills.dirs list.

---

#### Finding 47: No `project` field in epic frontmatter

**Reviewers:** R1 (C4), R3
**Sections:** 5.1

The designer hat says "read project knowledge at `.botminter/projects/<project>/knowledge/`" but the epic frontmatter has no `project` field. For M2 with one project, the architect assumes `hypershift`. Multi-project support would need this.

**Recommendation:** Either add a `project` field to the epic frontmatter or document the single-project assumption in the hat instructions.

---

#### Finding 48: `just launch` recipe implementation not shown

**Reviewers:** R1 (C6)
**Sections:** 4.8

`create-workspace` has a full bash script. `launch` does not. The recipe needs to validate the token argument and set `RALPH_TELEGRAM_BOT_TOKEN`.

**Recommendation:** Add the recipe implementation for completeness.

---

#### Finding 49: PROCESS.md sync protocol still references "submodule"

**Reviewers:** R1 (C12)
**Sections:** 4.3, current PROCESS.md

The design adds epic lifecycle statuses to PROCESS.md but doesn't update the communication protocols section to replace "submodule" with the `.botminter/` model.

**Recommendation:** Include PROCESS.md sync-protocol update in the M2 implementation.

---

#### Finding 50: No timeout handling for review_gater `human.interact` calls

**Reviewers:** R1 (C13)
**Sections:** 4.2.1

The backlog_manager explicitly handles timeout: "On timeout: No action. Epic stays in `po:triage`." The review_gater does not mention timeout behavior.

**Cross-reference with design-principles.md:** Principle 3 says "Just fire `human.interact` — Ralph handles blocking and response delivery transparently." This implies timeout is handled by Ralph, but the hat should still specify what happens (no action, re-present next cycle).

**Recommendation:** Add timeout handling to review_gater for explicitness.

---

#### Finding 51: "discoverable via skills.dirs" in AC 7.8 unclear

**Reviewers:** R3 (B4)
**Sections:** 7.8

How to verify "discoverable" and "listed in the skill index"? No command or file to check.

**Recommendation:** Tighten the AC to reference a concrete verification method.

---

#### Finding 52: "extended period" in AC 7.3b is vague

**Reviewers:** R3 (B5)
**Sections:** 7.3b

"Given an epic has been in status/po:ready for an extended period" — what constitutes "extended"? Not machine-verifiable.

**Recommendation:** Commit to a specific threshold (e.g., "more than 7 days") or reference a configurable setting.

---

#### Finding 53: AC 7.2 combines too many conditions

**Reviewers:** R3 (B6)
**Sections:** 7.2

Four separate behaviors in one AC: board scanner fires, pulls `.botminter/`, scans for `status/po:*`, scans for stale locks.

**Recommendation:** Split into individual testable assertions.

---

#### Finding 54: AC for agent startup self-cleanup could be tighter

**Reviewers:** R3 (A4)
**Sections:** 7.4

AC 7.4 bullet 4 says "cleans up any stale locks with its own role prefix" — could be read as cleaning all stale locks. The "own role prefix" qualifier is present but could be more explicit.

**Recommendation:** Reword to emphasize "only its own role's locks."

---

#### Finding 55: No test for epic rejection at `po:accept` stage

**Reviewers:** R3 (D5)
**Sections:** 4.2.1, 8

The review_gater supports rejection at `po:accept` (back to `arch:in-progress`). Not tested. Only design and plan rejection are tested.

**Recommendation:** Add a test for acceptance rejection.

---

#### Finding 56: No test for PROMPT.md symlink propagation

**Reviewers:** R3 (E3)
**Sections:** 7.6, 8

AC 7.6 bullet 2 tests PROMPT.md propagation. No test step in Section 8 covers it. Step 19 says "knowledge file" but PROMPT.md is not a knowledge file.

**Recommendation:** Add PROMPT.md propagation to the day-2 test steps.

---

#### Finding 57: No test for ralph.yml change detection and warning

**Reviewers:** R3 (E4)
**Sections:** 4.6.3, 8

`just sync` should copy ralph.yml if newer and warn about restart. Not tested.

**Recommendation:** Add a test for the ralph.yml change-detection flow.

---

### NOTE

#### Finding 58: Lock files in git create commit noise

**Reviewers:** R2 (A-4)

Each lock acquire/release is two commits. A single epic lifecycle generates ~20+ lock-related commits. Acceptable for M2 but becomes noise at M3 scale.

---

#### Finding 59: Training mode creates double-confirmation per action

**Reviewers:** R2 (C-3)

Human confirms twice: once at board scanner dispatch, once at work hat execution. With two agents, the human fields confirmations from two Telegram bots. Acceptable for training mode — document the expected volume.

---

#### Finding 60: `po:ready` routing to backlog_manager is correct

**Reviewers:** R2 (C-4)

`po:ready` is a parking state, not a review gate. Routing to backlog_manager (not review_gater) is architecturally correct.

---

#### Finding 61: `board.rescan` vs `board.scan` dual-path idle detection is well-designed

**Reviewers:** R1 (C10)

Work hats keep the loop active via `board.rescan`; only when the board scanner finds nothing does `LOOP_COMPLETE` fire, causing an idle pause before `task.resume` → `board.scan`. Consistent with M1.5 patterns.

---

#### Finding 62: Separate Telegram bots per agent noisy at scale

**Reviewers:** R2 (F-4)

With 5 agents in M3, the human manages 5 separate Telegram chats. Workable but operationally noisy. The design considered and rejected a shared bot (Section C, Appendix).

---

#### Finding 63: Training mode scope extension is a good enhancement

**Reviewers:** R1 (B8)

Design extends training mode to all members, beyond what requirements state. Good enhancement, well-implemented.

---

#### Finding 64: No cross-role stale lock cleanup

**Reviewers:** R1 (C7)

If the HA crashes and leaves a stale lock, the architect cannot clean it. Only the HA's startup self-cleanup would resolve it. Acceptable for M2.

---

#### Finding 65: No rejection cycle limit — deliberate

**Reviewers:** R1 (C8)

The human controls when to approve. No limit on rejection cycles. Documented as a deliberate design choice.

---

#### Finding 66: Open questions appropriately deferred

**Reviewers:** R3 (A8)

Section 4.10 open questions (hot-reload, symlink compat, shared hats, hat composition, human.guidance) are correctly scoped as beyond M2 POC.

---

#### Finding 67: Review presets not considered — future idea

**Reviewers:** R1 (D7)

Gastown's review presets (gate/full/custom) could inform future review_gater enhancements. Not needed for M2.

---

#### Finding 68: No test for `human.guidance`

**Reviewers:** R3 (F8)

Proactive human guidance is a deferred feature. Not testing it is appropriate.

---

#### Finding 69: RESOLVED — `cooldown_delay_seconds` omitted

**Reviewers:** R1 (C2)

Resolved by `specs/design-principles.md` Principle 5: "No `cooldown_delay_seconds` — removed from ralph.yml configs. Agent processing time provides natural throttling."

---

## Summary by Severity

| Severity | Count | Key Themes |
|----------|-------|------------|
| CRITICAL | 6 | Lock protocol race conditions and recovery; concurrent testing; AC verifiability |
| MAJOR | 24 | Hat instruction completeness; workspace model gaps; AC and testing coverage |
| MINOR | 27 | Edge cases; scalability prep; terminology; additional test coverage |
| NOTE | 11 | Observations, deliberate trade-offs, future considerations |
| RESOLVED | 1 | Addressed by design-principles.md |
| **Total** | **69** | |

## Summary by Disposition

| Disposition | Count | Findings |
|-------------|-------|----------|
| **Accepted — design change applied** | 15 | 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 16, 18, 22, 23, 26 |
| **Dismissed** | 9 | 1, 2, 7, 14, 15, 17, 19, 20, 21 |
| **Deferred — future improvement** | 38 | 24, 25, 27–30, 31–59, 62, 64, 67 |
| **Noted (no action)** | 6 | 60, 61, 63, 65, 66, 68 |
| **Resolved** | 1 | 69 |
| **Total** | **69** | |
