# Requirements — HyperShift Agentic Team

> Q&A record from requirements clarification sessions.
> Each question and answer is appended in real time.

---

## Q1
**Q:** Your research spans from a tight 4-week wedge (one change type, prove the thesis) to a full multi-year vision (The Bridge, confidence scoring, planning cascade, etc.). For this design — what's the scope of the first deliverable? Are we designing the full RH orchestrator system (the thing that does `rh intake` through `rh approve` through CI triage), or are we designing the wedge first (e.g., one story type end-to-end with evidence chain)?

**A:** Neither. The scope is a 2-day POC. Use ralph-orchestrator (possibly forked/modified) to implement the vision from rough-idea.md. The research docs are inspiration only — not prescriptive. Concrete execution, not grand design.

## Q2
**Q:** What's the real HyperShift task you want to run through this POC? The rough idea mentions OCPSTRAT-1751 as an example — is there an actual JIRA story or concrete code change you want the agentic team to execute end-to-end as the proof point?

**A:** Yes — OCPSTRAT-1751: "Streamline and decouple Control-plane and NodePool Upgrades Management/Reporting." Goal: enable independent HCP upgrade status reporting outside CVO, allow CP upgrades without NodePools, isolate CP upgrade status from unhealthy customer operators. Acceptance criteria: independent status reporting, E2E coverage of decoupled upgrades (in-z and cross-y), upgradeability checks regardless of NodePool presence.

## Q3
**Q:** What's your current familiarity with ralph-orchestrator's internals? Have you used it enough to know its hat/event YAML configuration, or do we need to research how Ralph works and what needs to be modified/forked?

**A:** Pretty familiar — took a few days deep dive, explored the codebase with Claude Code, used it to build a small project. Knows architecture and rough workings, not line-by-line.

## Q4
**Q:** The POC is about standing up the agentic team — ralph-orchestrator configured with agent personas (devs, QE, etc.), operating on a team fork with branches/issues/PRs, with you as PO+Tech Lead gating output. OCPSTRAT-1751 is the task the team works on to prove it. For the 2-day scope: which parts of the team setup are must-haves? E.g., must we have multiple distinct personas, the fork with its own CI, knowledge accumulation, sprint/retro process — or is there a smaller subset that proves the team concept?

**A:** Must-haves: 1 (multiple personas), 2 (team fork), 3 (feature breakdown — cooperative, like a sprint planning meeting, not fully autonomous), 5 (PO/Tech Lead gating), 6 (knowledge/memory), 8 (CI on fork). Out for POC: 7 (sprint/retro loop). Note: the kanban status chain is pull-based (not a rigid enforced phase chain) — see design Section 3 for details.

## Q5
**Q:** For the team fork (#2) — do you already have a fork of openshift/hypershift, or does it need to be created? For CI (#8), are you thinking actual OpenShift CI (Prow) or something lighter like GitHub Actions for the POC?

**A:** Fork chain: openshift/hypershift → my-fork → agentic-team-fork. Personal fork exists, agentic team fork needs to be created. CI: needs research, could be solved together with the team once it's alive. Key realization: get the team alive first, then tackle problems like CI one by one. Reframing: what's the bare minimum to have the team alive? That's milestone 1.

## Q6
**Q:** Checking understanding of "the team is alive" (milestone 1): (1) Ralph configured with agent personas (dev, QE, more?), (2) agentic-team-fork exists on GitHub, (3) cooperative sprint planning to break down OCPSTRAT-1751, (4) team can pick up a story and start working, (5) you gate/review output. Is that what "alive" means, or even simpler?

**A:** It depends — the process matters. Need an evidence-based trust/confidence model. Ralph has "backpressure" but it needs to be defined concretely. Open question: do we define the trust/confidence model as part of this milestone (before the team is alive), or bootstrap it WITH the team in a team bootstrap session?

## Q7
**Q:** Proposed split for milestone 1: **Phase A (skeleton)** — you and me define the fork, ralph config with personas, minimal trust/confidence model (concrete gates: tests pass, diff coherence, self-review, etc.), backpressure rules mapped to ralph config. **Phase B (first breath)** — team bootstrap session: cooperative sprint planning on OCPSTRAT-1751, team refines its own process/trust model as its first act. Define just enough process so the team can function, then the team improves its own process. Does that split work?

**A:** Yes, that split works. Phase A (skeleton) before the team, Phase B (first breath) with the team.

## Q8
**Q:** For Phase A personas — the rough idea lists devs, QE, UX, content writers, SREs, field architects. For a HyperShift POC on OCPSTRAT-1751, minimum viable team instinct: dev (writes code), QE (writes/runs tests), maybe architect (designs, reviews). Others feel out of scope. What's your thinking?

**A:** Dev, QE, architect, and dev-reviewer. Four personas for the POC team.

## Q9
**Q (revised after research review):** Research describes 3 overlapping trust models across the vision docs. For Phase A, proposed actionable dimensions (no history needed): (1) tests pass, (2) self-review by dev-reviewer persona, (3) design adherence, (4) diff coherence, (5) knowledge alignment (checked pitfalls), (6) prior art consistency, (7) invariant compliance. Which are hard gates vs advisory? Review triage levels (scan/focused/thorough/pair) in or out for POC?

**A:** Invariants for the POC are mini prompts for the agent — not executable check scripts, not pure knowledge reasoning. Prompt-based rules the agent follows and checks against. Decision on hard gate vs advisory and review triage levels still open.

## Q10
**Q:** Should we use evals instead of invariants as a separate concept? Evals are scored quality assessments, invariants are binary pass/fail rules. Invariants are a special case of evals (100% threshold). Could unify into one eval system where each eval has a prompt, a type (hard-gate or advisory), and a score.

**A:** Yes — unify into one eval system. Invariants are hard-gate evals (must pass). Other confidence dimensions are advisory evals (scored). Single framework for the trust/confidence model.

## Q11
**Q:** Continue refining eval specifics now or leave for later?

**A:** Neither — stop going deeper on the confidence/eval system. Focus on building an MVP with milestones. Confidence system + evals will be one milestone, discussed when we get there.

## Q12
**Q:** Do you have ralph-orchestrator cloned locally, or do we need to pull it? What's the repo URL?

**A:** Already cloned at /opt/workspace/ralph-orchestrator/

## Q13
**Q:** Ready to move from requirements to research on ralph-orchestrator's current capabilities (personas, phases, backpressure, knowledge) so we can propose concrete milestones grounded in what exists vs what needs building?

**A:** Yes.

## Q14
**Q:** (Clarification during research planning) Task tracking has two distinct levels?

**A:** Yes — don't conflate them. (1) MY tasks: tracked in JIRA (RFE → Feature → Epic → Story/Task/Subtask). (2) The team's tasks: two sub-levels — external-facing with me via GitHub issues, and internal agentic tracking (beads, claude code tasks, ralph task subcommand).

## Q15
**Q:** Research into Ralph's baked-in prompts revealed that the coordinator prompt (~1500 lines of hardcoded Rust format strings) governs how a single agent manages its own workflow through hats. This is appropriate for one agent's inner loop (analyze → plan → implement → verify). But the current design puts all 4 personas (architect, dev, QE, dev-reviewer) as hats in one Ralph loop — making them one agent wearing different masks, sharing one scratchpad, one memory, one context. That's not a team. Should the architecture be two layers: (1) **inner loop** — each team member is a full Ralph instance with its own hats, memories, and workflow; (2) **outer loop** — a team coordination layer that manages interactions between team members (issue assignment, PR handoffs, review cycles)?

**A:** Yes — two layers. (1) Inner loop: each team member is a full Ralph instance with its own hats, memories, and workflow. (2) Outer loop: a team coordination layer that manages interactions between team members (issue assignment, PR handoffs, review cycles). Ralph's baked-in coordinator prompt is appropriate for the inner loop. The outer loop is what needs designing/building.

## Q16
**Q:** For the outer loop (team coordination layer), three options: (1) another Ralph instance as meta-orchestrator that spawns child Ralphs, (2) a lightweight custom script/state machine watching GitHub events and spawning the right Ralph, (3) GitHub-native (Actions/webhooks triggering Ralph runs). Ralph already has parallel loops via git worktrees, but those are designed for parallelizing work with the same config, not for different personas with different configs interacting. What should the outer loop be?

**A:** The POC repo itself IS the outer loop. It deploys different "ralph profiles" — each team member gets its own ralph config. The outer loop is the process, the profiles, and the conventions. Each ralph knows which team member it is, how it should get its work, where it should put its output, and is empowered to do its job. No central orchestrator — the coordination is emergent from shared process and infrastructure (GitHub issues, PRs, branches). Building that = building the outer loop.

## Q17
**Q:** This sounds like a GitOps model: a central repo as source of truth, worker processes (ralph instances) that reconcile desired state. A change to the repo (new issue, PR opened, review requested) dispatches work to the right worker. Is that the mental model — a GitOps-style system where the team fork is the control plane and ralph instances are the workers?

**A:** Yes, GitOps-style. But the central repo is the **team repo** (botminter), NOT the project repo (hypershift, capi, controller-runtime, etc.). The team repo is the control plane — it defines profiles, process, knowledge, and dispatches work. The project repos are where the actual code changes happen, but they're targets, not the control plane.

## Q18
**Q:** Since the team repo is project-agnostic (could target hypershift, capi, controller-runtime, etc.), knowledge should be split into two layers?

**A:** Yes — team knowledge (how the team works, cross-project patterns, process conventions) lives in the team repo. Project knowledge (codebase patterns, pitfalls, troubleshooting for a specific project like hypershift) lives with or near the project. Two distinct layers.

## Q19
**Q:** The two-layer architecture (inner loop per team member, outer loop as team repo) changes the eval/confidence system. Instead of one flat eval system, it splits into: (1) inner evals — each team member's ralph validates its own work internally (backpressure in hat instructions), (2) outer evals — team-level quality checks between team members via GitHub (PR reviews, QE sign-off, architecture review). Invariants also split: team invariants (PROCESS.md), project invariants (project knowledge), profile invariants (hat instructions). Confidence becomes distributed — the aggregate of independent assessments from multiple team members. Does this match your thinking?

**A:** Yes — evals and invariants split across three layers matching the architecture: team (process), project (domain), and profile (individual). Confidence is distributed across independent team member assessments, not centralized in one agent checking its own work.

## Q20
**Q:** For the POC, should we use actual GitHub (fork, issues, PRs) or simulate the coordination layer locally?

**A:** Simulate it. Everything is files. No GitHub for the POC — the "issues", "PRs", and coordination are all file-based in the central directory. PO runs multiple ralphs locally that watch the central dir. GitHub integration is a future milestone.

## Q21
**Q:** The current design has team members running directly from `team/<member>/` within the team repo. You want to introduce a "team member workspace" model instead. Can you describe the full concept?

**A:** Key changes:
1. There's a **template** team-member workspace repo — same structure starting at `team/<member-repo>/`.
2. When adding a new team member, **clone** that template to a member-specific workspace repo.
3. The workspace tracks the team repo via **git submodule**.
4. Every agent starts by: (a) cloning its workspace repo recursively, (b) syncing the submodule dir — surfacing the latest version of its files from the team repo to the top of the workspace repo (gets the PROMPT, the files for the team member, updates tracking if needed), (c) `ralph run -p prompt`.
5. There's a **version number** of the workspace in the team repo that determines if the workspace needs initial setup or just incremental sync.

## Q22
**Q:** Who does the clone + file surfacing — the agent itself on startup, or an external launcher?

**A:** The **launcher** (already mentioned in the design doc) handles the full sequence: cloning the workspace repo recursively, surfacing the team member's files to the workspace root, and running `ralph run -p prompt`. For now, the PO does this manually. The agent doesn't manage its own workspace setup or launch — the launcher does everything.

## Q23
**Q:** What does the workspace template repo contain at its root level? Scaffolding only (submodule pointer + empty dirs, all content from surfacing), or its own structure (ralph.yml, CLAUDE.md, etc.)?

**A:** Scaffolding only for now. All content comes from surfacing the team repo files into the workspace root.

## Q24
**Q:** When the launcher surfaces files from the submodule to the workspace root — is that a copy, symlink, or something else?

**A:** Copies. The team repo doesn't have runtime files — those can only be committed to workspace repos. Note that `ralph.yml` is NOT a runtime file (it's a config that comes from the team repo). Runtime files (memories, scratchpad, etc.) are workspace-only and never flow back to the team repo.

## Q25
**Q:** Version tracking mechanism — how does the launcher know whether to do a full setup vs incremental sync?

**A:** If there's a version in `ralph.yml`, use it. The launcher checks the version field in the team repo's `ralph.yml` against what's in the workspace to decide if re-sync is needed.

## Q26
**Q:** Should every team member have a direct HIL escape hatch to the human, or should all escalations go through the PO-assistant?

**A:** Every team member gets a direct `escalate` hat — a single HIL escape hatch to report/escalate directly to the human when stuck. This bypasses the PO-assistant. The normal flow still goes through status changes and the PO-assistant, but when a member is stuck it should be able to reach the human directly.
