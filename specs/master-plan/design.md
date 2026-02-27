%%  %%# Design — HyperShift Agentic Team

> Detailed design for the HyperShift Agentic Team POC.
> Inputs: [rough-idea.md](rough-idea.md), [requirements.md](requirements.md), [ralph-capabilities-audit.md](research/ralph-capabilities-audit.md).
>
> **Document structure:** Section 3 describes the generator architecture (three-layer model, Justfiles, skeletons, profiles). Sections 4–8 contain per-milestone summaries with pointers to detailed designs in `specs/milestone-N-*/design.md`.

---

## 1. Overview

**What:** Build a GitOps-style agentic team system where independent Ralph instances (team members) coordinate through a shared directory structure. The team repo is the control plane; files are the coordination fabric.

**Scope:** 2-day POC. The proof point is OCPSTRAT-1751 (Streamline and decouple Control-plane and NodePool Upgrades). The goal is not to finish the JIRA — it's to prove the team concept works.

**Key Architectural Insight ([Q15-Q17](requirements.md#q15)):** The system has two layers:
- **Inner loop** — each team member is a full Ralph instance with its own hats, memories, and internal workflow. Ralph's baked-in coordinator prompt is appropriate here.
- **Outer loop** — the team repo (`botminter`) is the control plane. It defines team members, process, and conventions. Team members coordinate via shared files (issues, reviews, handoffs). No central orchestrator — coordination is emergent from shared process and file conventions.

**POC Simplification ([Q20](requirements.md#q20)):** No real GitHub for the POC. GitHub's data model (issues, milestones, PRs, reviews) is simulated as files in `.github-sim/`. PRs are real git branches — only the metadata is file-based. PO runs multiple ralphs locally. Migration to real GitHub is a future milestone where file ops become `gh` CLI calls.

**Constraints:**
- Use ralph-orchestrator as-is where possible; fork/modify only where necessary
- Ralph-Wiggum style (sequential) within each team member
- Research docs are inspiration, not prescriptive
- No sprint/retro loop (out for POC)
- No GitHub — file-based coordination only

---

## 2. Milestones

> **Framing:** Each milestone builds and tests the **machinery** — generator skeleton, profiles, prompts, ralph configs, Justfile recipes. Validation is done by running agents with synthetic test tasks, observing behavior, and iterating on prompts until the agents behave correctly. **Operational use** (running the team on real work like OCPSTRAT-1751) happens **after** the milestone is complete and the human spins up the team. Detailed per-milestone designs live in `specs/milestone-N-*/design.md`.

### Milestone 1: Structure + human-assistant

The minimum to prove the inner loop and workspace model work: repo skeleton, one team member (human-assistant), and a validated HIL round-trip via Telegram.

1. Create repo directory structure (knowledge/, invariants/, projects/, team/, .github-sim/)
2. Write `PROCESS.md` — issue format conventions, label naming, communication protocols
3. Set up knowledge layers (team knowledge + project knowledge for hypershift)
4. Set up invariant layers (team + project)
5. Create workspace template repo (scaffolding + submodule pointer to botminter)
6. Create `team/human-assistant/` in team repo — fully configured (ralph.yml with version, PROMPT.md, CLAUDE.md, knowledge, invariants)
7. Write launcher (clone workspace from template, surface member files, `ralph run -p PROMPT.md`)
8. Launch human-assistant via launcher, verify Ralph starts correctly
9. Configure RObot for PO gating (human interacts with human-assistant via Telegram)

**Proves:** Inner loop works (ralph + hats for human-assistant). Workspace model works (template clone → surface → run). Launcher works (launches a team of one). HIL works (human ↔ human-assistant via Telegram, training mode). The team repo is a functioning control plane.

**End state:** human-assistant running, connected to Telegram, scanning an empty board. Ready for a second team member.

### Milestone 2: Architect + First Epic

Add the architect as a second team member. Build and test the two-member coordination machinery through the outer loop.

- Architect member skeleton in `rh-scrum` profile (ralph.yml, PROMPT.md, CLAUDE.md, hats)
- human-assistant evolution — new hats for epic creation, design gating, prioritization
- Epic lifecycle statuses added to PROCESS.md
- `.github-sim/` write-lock mechanism for concurrent access
- Two-member outer loop coordination validated with synthetic test tasks

**Proves:** Outer loop works (.github-sim issues, `status/*` labels, knowledge resolution). Pull model works (architect picks up work via `status/*` watch). Two-member coordination works (PO creates → architect designs → PO gates).

**Does NOT exercise:** Story kanban flow, code work, TDD.

→ `specs/milestone-2-architect-first-epic/` for detailed design

### Milestone 3: `bm` CLI
- Replaces Justfile-based tooling with a Rust CLI binary (`bm`)
- Single operator interface for managing agentic teams
- Detailed design in `specs/milestone-3-bm-cli/`

### Milestone 4: Full Team + First Story (deferred)
- Detailed design of dev, qe, reviewer team members (ralph.yml, PROMPT.md, hats)
- TDD flow: QE writes tests → dev implements → QE verifies → reviewer reviews → architect signs off
- One story executed end-to-end through the full kanban
- Proves the pull-based coordination works across all team members

### Milestone 5: Eval/Confidence System (deferred)
- Distributed eval framework across recursive scopes (team, project, member, member+project)
- To be designed when we get here ([Q11](requirements.md#q11), [Q19](requirements.md#q19))

### GitHub Integration (complete — pulled forward)
- Replaced file-based coordination with GitHub issues, PRs, reviews via `gh` CLI
- Detailed artifacts in `specs/github-migration/`

###### Requirement → Milestone Mapping (from Q4)

| Requirement | Milestone |
|-------------|-----------|
| Multiple personas (dev, QE, architect, reviewer) | M2 (architect), M4 (full team) |
| Team fork | GitHub migration (complete) |
| Feature breakdown (cooperative sprint planning) | M2 |
| PO/Tech Lead gating (HIL) | M1 |
| Knowledge/memory accumulation | M1 (structure), M4 (first real accumulation) |
| CI on fork | GitHub migration (complete) |
| Eval/confidence system | M5 |

> Requirements capture the full target. Milestones deliver them incrementally. All requirements are fulfilled when all 5 milestones are complete.

---

## 3. Generator Architecture

`botminter` is a **generator repo** — a reusable metaprompting toolkit that stamps out team repos. It is NOT a team repo itself. The deliverables are Justfiles and skeleton directories that automate team creation.

### 3.1 Three-Layer Model

Changes happen at three levels. Each level has a distinct scope and audience:

| Layer | Location | What lives here | Who changes it |
|-------|----------|-----------------|----------------|
| **Generator skeleton** | `skeletons/team-repo/` | Bare directory structure + Justfile. Process-agnostic, company-agnostic. | Generator maintainers |
| **Profile** | `skeletons/profiles/<name>/` | Team process, role definitions, member skeletons, company norms. Reusable across teams within a methodology. | Profile authors |
| **Team repo instance** | e.g., `~/workspace/hypershift-team/` | Project-specific knowledge, actual issues, runtime state. Unique to this team. | Team operators (PO, human) |

`just init` layers the profile on top of the skeleton to produce a team repo instance. Changes flow downward (profile → instance) on init, and learnings flow upward (instance → profile) as the team validates with real work.

###### Layer examples

| Content | Layer | Rationale |
|---------|-------|-----------|
| Justfile recipes (`add-member`, `launch`) | Generator skeleton | Mechanical operations, process-agnostic |
| `.github-sim/` directory structure | Generator skeleton | Coordination infrastructure, not process-specific |
| PROCESS.md (issue format, kanban, labels) | Profile | Defines how this type of team works |
| CLAUDE.md (team context) | Profile | Methodology-specific orientation |
| Member skeletons (human-assistant, dev, qe, architect) | Profile | Role definitions are methodology-specific |
| Team knowledge (commit conventions, PR standards) | Profile | Company/methodology norms |
| Team invariants (code review required, test coverage) | Profile | Company/methodology quality standards |
| HyperShift project knowledge (hcp-architecture, upgrade-flow) | Team repo instance | Project-specific, not reusable across teams |
| `.github-sim/issues/` content | Team repo instance | Runtime state, unique to this team |

### 3.2 Generator Repo Structure

```
botminter/                                    # GENERATOR REPO
├── Justfile                                         # init recipe (the only generator recipe)
├── skeletons/
│   ├── team-repo/                                   # Generic skeleton (process-agnostic)
│   │   ├── Justfile                                 # Baked into every generated repo
│   │   ├── team/
│   │   ├── projects/
│   │   └── .github-sim/
│   │       ├── issues/
│   │       ├── milestones/
│   │       └── pulls/
│   └── profiles/
│       └── rh-scrum/                                # Red Hat scrum team profile
│           ├── PROCESS.md                           # Issue format, labels, kanban, communication
│           ├── CLAUDE.md                            # Team-wide context for agents
│           ├── knowledge/                           # Team-level knowledge (RH norms)
│           ├── invariants/                          # Team-level invariants (RH quality)
│           └── members/                             # Member skeletons by role
│               ├── human-assistant/                   # Human-Assistant (M1)
│               ├── architect/                       # Architect (M2)
│               ├── dev/                             # Developer (M3)
│               ├── qe/                              # QE Engineer (M3)
│               └── reviewer/                        # Dev Reviewer (M3)
└── specs/                                           # Design artifacts
```

### 3.3 Usage Model

```bash
# From inside the generator repo — stamp out a team repo
$ cd botminter
$ just init --repo=~/workspace/hypershift-team --profile=rh-scrum project=hypershift

# From inside the generated team repo — operate the team
$ cd ~/workspace/hypershift-team
$ just add-member human-assistant
$ just launch human-assistant
```

**Two Justfiles:**
- **Generator Justfile** (`botminter/Justfile`): `init` recipe — layers skeleton + profile into a team repo at the specified path. This is the only generator recipe.
- **Team repo Justfile** (baked into `skeletons/team-repo/Justfile`): `add-member`, `create-workspace`, `launch` — operational recipes inherited by every generated team repo.

### 3.4 `just init` Behavior

1. Copies `skeletons/team-repo/` to the target path (bare structure + Justfile)
2. Overlays `skeletons/profiles/<profile>/` on top (PROCESS.md, CLAUDE.md, knowledge, invariants)
3. Copies the full generator content (skeleton + profile, minus the generator Justfile) into `<target>/.team-template/` so the team repo has access to member skeletons and profile content for `just add-member` and future syncs
4. If `project=<name>` is provided, creates `projects/<name>/knowledge/` and `projects/<name>/invariants/`
5. Initializes as a git repo, makes initial commit
6. Fails if the target path already exists (does NOT overwrite)

The generated team repo is self-contained — it has its own Justfile, profile content, and member skeletons. No runtime dependency back to the generator.

### 3.5 Feedback Loop

As the team validates with real work, learnings flow in two directions:

```
Generator skeleton ← Profile ← Team repo instance
     (rare)          (frequent)    (continuous)
```

- **Instance → Profile:** Non-project-specific learnings (process improvements, better prompts, refined invariants) flow back to the profile. E.g., if the HyperShift team discovers a better way to structure design reviews, that improvement goes into `skeletons/profiles/rh-scrum/PROCESS.md`.
- **Instance → Skeleton:** Rarely, learnings affect the generator itself. E.g., if a new Justfile recipe is needed, it goes into `skeletons/team-repo/Justfile`.
- **Profile stays:** Project-specific learnings (HyperShift architecture patterns, codebase pitfalls) stay in the team repo instance. They don't pollute the reusable profile.

### 3.6 Profile: `rh-scrum`

The first profile built for this POC. Defines a Red Hat scrum team with:

- **Roles:** human-assistant, Architect, Developer, QE Engineer, Dev Reviewer
- **Process:** GitOps-style pull-based kanban with `.github-sim/`, status labels, issue-driven coordination
- **HIL:** Training mode (observe & report every transition), graduation path to supervised/autonomous
- **Knowledge norms:** RH engineering conventions (commit standards, PR conventions, communication protocols)
- **Quality invariants:** Code review required, test coverage, prompt-based rules

Other teams could create different profiles (e.g., `startup-kanban`, `oss-maintainer`) with entirely different roles, processes, and norms — all using the same generator skeleton.

---

## 4. Milestone 1 Design — Structure + human-assistant

> Detailed design in `specs/milestone-1-structure-poa/design.md`.

**What M1 builds:**
- Generator skeleton — bare team repo structure + Justfile (`add-member`, `create-workspace`, `launch`)
- `rh-scrum` profile — PROCESS.md, CLAUDE.md, team knowledge/invariants, human-assistant member skeleton
- `just init` recipe — layers skeleton + profile into a self-contained team repo instance
- human-assistant — the first team member (defined in the profile, instantiated via `just add-member human-assistant`)
- Workspace model — `just create-workspace` + `just launch` automate workspace setup and Ralph startup
- HIL round-trip via Telegram

**Proves:** Inner loop works (ralph + hats for human-assistant). Workspace model works (template clone → surface → run). Launcher works (launches a team of one). HIL works (human ↔ human-assistant via Telegram, training mode). The team repo is a functioning control plane.

**End state:** human-assistant running, connected to Telegram, scanning an empty board. Ready for a second team member.

→ `specs/milestone-1-structure-poa/` for detailed design, spec, and plan

---

## 5. Milestone 2 Design — Architect + First Epic

> Detailed design in `specs/milestone-2-architect-first-epic/`. This section is a rough epic only.

**What M2 builds:**
- Architect member skeleton in `rh-scrum` profile (ralph.yml, PROMPT.md, CLAUDE.md, hats)
- human-assistant evolution — new hats for epic creation, design gating, story prioritization
- Epic lifecycle statuses added to PROCESS.md
- `.github-sim/` write-lock mechanism for concurrent access
- Two-member outer loop coordination

**What M2 proves:**
- Outer loop works (.github-sim issues, `status/*` labels, knowledge resolution)
- Pull model works (architect picks up work via `status/*` watch)
- Two-member coordination works (PO creates → architect designs → PO gates)

**What M2 does NOT exercise:**
- Story kanban flow (stories created but not executed)
- Code work on any project repo
- TDD (no dev or QE yet)

**Open questions — all resolved in M2 requirements:**
1. ~~**Telegram routing:**~~ Separate bots per member (Q11)
2. ~~**Concurrent file access:**~~ Per-issue write-locks with stale detection (Q9)
3. ~~**Submodule sync:**~~ Pull at loop start + optional mid-loop pull (Q10)
4. ~~**Project repo access:**~~ Agent-cloned fork chain (Q4)
5. ~~**Human command routing:**~~ Board scanner dispatches to appropriate hat based on board state (Q12, Q13)
6. ~~**human-assistant guardrails evolution:**~~ All HIL in training mode; review_gater handles all review gates (Q7, Q13)

---

## 6. Milestone 4 Design — Full Team + First Story (Deferred)

> Detailed design in `specs/deferred/full-team-first-story.md`. This section is a rough epic only.

**What M4 builds:**
- Dev, QE, and Reviewer member skeletons in `rh-scrum` profile
- Full story kanban statuses added to PROCESS.md
- TDD flow baked into QE and dev prompts
- human-assistant evolution — merge gate hat, knowledge placement
- Codebase access model (project fork, agent-cloned into gitignored `projects/` dir)

**What M4 proves:**
- Pull-based coordination works across all five team members
- TDD flow works (QE writes tests → dev implements → QE verifies → reviewer reviews → architect signs off → PO merges)
- Knowledge accumulates from real work and flows to the right scope
- The full story kanban completes end-to-end

**What M4 does NOT exercise:**
- Eval/confidence scoring (M5)

**Open questions (to resolve during M4 design):**
1. **HyperShift build environment:** Go toolchain, dependencies, and potentially a running cluster for E2E tests. What's available in the agent's execution environment?

---

## 7. Milestone 5 Design — Eval/Confidence System (Deferred)

> **Implementation note:** M5 likely spans all three layers (Section 3). Eval framework infrastructure goes into the generator skeleton. Eval definitions and HIL graduation criteria go into the profile. Project-specific eval rules and accumulated confidence data stay in the team repo instance.

### 7.1 Scope

Formalize the eval/confidence system. Currently, invariants are prompt-based rules and confidence is implicit (built from the evidence chain of QE tests + reviewer approval + architect sign-off). M4 makes this explicit and scored.

### 7.2 Distributed Eval Model

Confidence in a change = aggregate of independent assessments from multiple team members. Evals follow the same recursive scoping as knowledge ([Q10](requirements.md#q10), [Q19](requirements.md#q19)).

###### Eval scopes

| Scope | Examples | Type |
|-------|----------|------|
| Team invariants (`invariants/`) | "Every story must have QE validation" | Hard gate (PO enforces) |
| Project invariants (`projects/hypershift/invariants/`) | "HyperShift changes must not break NodePool reconciliation" | Hard gate |
| Member invariants (`team/{member}/invariants/`) | Dev: build passes, tests pass, lint clean | Inner backpressure |
| Member+project invariants (`team/{member}/projects/hypershift/invariants/`) | Dev: lint-clean for hypershift | Inner backpressure |

Invariants are hard-gate evals (must pass). Other confidence dimensions are advisory evals (scored). Single unified framework where each eval has a prompt, a type (hard-gate or advisory), and a score.

M5 also formalizes the HIL graduation path defined in M1 (Section 4.7): training → supervised → autonomous. The eval/confidence scores from M5 inform when a team member or the human-assistant can graduate to a less supervised HIL mode.

### 7.3 Improvement Ideas

- **human-assistant escalation_handler hat:** Add a hat to the human-assistant that receives and triages escalations from team members, rather than members escalating directly to the human. Could enable the human-assistant to resolve simple escalations autonomously before involving the human.

### 7.4 Acceptance Criteria

- [ ] Formal eval framework defined across recursive scopes
- [ ] Evals scored and transparent
- [ ] Hard-gate vs advisory distinction operational
- [ ] Evidence chain verification automated

---

## 8. GitHub Integration (Complete — Pulled Forward)

> Originally Milestone 5, pulled forward and completed before M3. Detailed artifacts in `specs/github-migration/`.

### 8.1 Scope

Replaced file-based coordination (`.github-sim/`) with real GitHub via the `gh` CLI. The `.github-sim/` format was designed to mirror GitHub's data model, so migration was 1:1: each file operation mapped to a `gh` CLI call.

### 8.2 Key Changes

- `.github-sim/issues/` → `gh issue create/edit/comment`
- `.github-sim/milestones/` → `gh api` for milestones
- `.github-sim/pulls/` → `gh pr create/review`
- Fork chain: openshift/hypershift → personal fork → team fork
- CI on team fork (Prow, GitHub Actions, or alternative)
- Automated launcher (Go binary with health checks, restart, monitoring) (bundled into M5 for convenience — not GitHub-specific)

### 8.3 Acceptance Criteria

- [ ] File-based coordination replaced with real GitHub
- [ ] Fork chain established
- [ ] CI running on team fork
- [ ] Automated launcher operational

### 8.4 Open Questions

1. **GitHub-sim to real GitHub migration:** The `.github-sim/` format mirrors GitHub's data model intentionally. When ready to migrate, each file operation (create issue, add comment, open PR, add review) maps 1:1 to a `gh` CLI call. The branch-as-PR model is already real git — only the metadata files need replacing.

---

## 9. Future Ideas

1. **Extract human-assistant from profiles.** The human-assistant is infrastructure, not a team role. It should live outside any profile (in the generator skeleton or as a separate concern) — it's the human's proxy, not a scrum-specific role.

2. **Rename `rh-scrum` profile to `scrum-team`.** Drop the company prefix — the profile defines a scrum methodology, not a Red Hat–specific one. Makes it more reusable.

3. **Access control per scope + SLSA-style attestation.** Enforce who can modify what based on the recursive knowledge hierarchy — e.g., only managers/PO can change team-level knowledge and invariants, project leads can change project-level, individual members can only change their own scope. Additionally, PRs submitted upstream must carry attestation proving they were produced through the team's defined workflow (design review, QE validation, code review, architect sign-off). Similar to [SLSA](https://slsa.dev/) provenance — the artifact (PR) carries a verifiable chain of evidence that the process was followed, not just that tests passed.

4. **ralph.yml hot-reload.** Currently ralph.yml is the only file copied (not symlinked) into workspaces, and changes require manual `just sync` + agent restart. Investigate whether Ralph can detect and reload ralph.yml changes without a restart — if so, ralph.yml could also be a symlink, making workspace propagation fully automatic.

5. **Symlink compatibility verification.** M2 switches from copy-based to symlink-based workspace surfacing. Verify that Ralph and Claude Code handle symlinked PROMPT.md and CLAUDE.md correctly across all platforms (they should — symlinks resolve at the OS level — but edge cases around relative paths and git submodules should be tested).

6. **Knowledge observation mechanism + knowledge search and management tool.** Add a dedicated mechanism (possibly a hat) that observes agent activity and captures reusable learnings into the knowledge hierarchy. Pair with a search/management tool that lets agents (and humans) query, prune, and organize accumulated knowledge across all scoping levels (team, project, member). Currently agents can read knowledge but have no structured way to contribute back to it or search it efficiently at scale.
7. Extract variable process parts out of hat instructions as much as possible
