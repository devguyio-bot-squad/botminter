# Implementation Plan — HyperShift Agentic Team

> Incremental implementation steps. Each step builds on the previous, results in working demoable functionality, and follows TDD practices.
> Inputs: [design.md](design.md), [requirements.md](requirements.md).
>
> **Key framing:** `botminter` is a **generator repo** — a reusable metaprompting toolkit that stamps out team repos. It is NOT a team repo itself. The deliverables are Justfiles and skeleton directories.
>
> **Milestone vs. operational use:** Each milestone builds and tests the **machinery** (generator skeleton, profiles, prompts, ralph configs, Justfile recipes). Validation is done by running agents with synthetic test tasks, observing behavior, and iterating on prompts. **Operational use** (running the team on real work like OCPSTRAT-1751) happens after the milestone is complete. Detailed per-milestone plans live in `specs/milestone-N-*/plan.md`.

---

## Three-Layer Architecture

Changes happen at three levels. Each level has a distinct scope and audience:

| Layer | Location | What lives here | Who changes it | Example |
|-------|----------|-----------------|----------------|---------|
| **Generator skeleton** | `skeletons/team-repo/` | Bare directory structure + Justfile. Process-agnostic, company-agnostic. | Generator maintainers | Justfile recipes, `.github-sim/` structure |
| **Profile** | `skeletons/profiles/rh-scrum/` | Team process, role definitions, member skeletons, company norms. Reusable across teams within this methodology. | Profile authors | PROCESS.md, CLAUDE.md, `members/human-assistant/`, `members/dev/`, team knowledge/invariants |
| **Team repo instance** | `~/workspace/hypershift-team/` | Project-specific knowledge, actual issues, runtime state. Unique to this team. | Team operators (PO, human) | HyperShift project knowledge, `.github-sim/issues/`, member additions |

**`just init` layers the profile on top of the skeleton** to produce a team repo instance. Changes flow downward (profile → instance) on init, and learnings flow upward (instance → profile) as the team validates with real work.

**Usage model:**
```
$ cd botminter
$ just init --repo=~/workspace/hypershift-team --profile=rh-scrum
$ cd ~/workspace/hypershift-team
$ just add-member human-assistant
$ just launch human-assistant
```

**Two Justfiles:**
- **Generator Justfile** (`botminter/Justfile`): `init` recipe — run from inside the generator, layers skeleton + profile into a team repo at the specified path
- **Team repo Justfile** (baked into skeleton): `add-member`, `create-workspace`, `launch` — operational recipes

**Feedback loop:** As we validate with HyperShift stories, non-project-specific learnings (process improvements, better prompts, new invariants) flow back up to the `rh-scrum` profile. Project-specific learnings stay in the team repo instance.

---

## Checklist

### Milestone 1: Structure + human-assistant
- [ ] Step 1: Validate persistent polling
- [ ] Step 2: Generator skeleton + `just init`
- [ ] Step 3: RH Scrum profile — PROCESS.md, CLAUDE.md, team knowledge/invariants
- [ ] Step 4: RH Scrum profile — human-assistant member skeleton
- [ ] Step 5: Team repo Justfile — `just add-member`
- [ ] Step 6: Team repo Justfile — `just create-workspace`
- [ ] Step 7: Team repo Justfile — `just launch`
- [ ] Step 8: Integration — init → add-member → launch + HyperShift project knowledge
- [ ] Step 9: HIL round-trip via Telegram

### Milestone 1.5: Autonomous Ralph (Spike)
- [ ] *(Detailed plan in `specs/milestone-1.5-autonomous-ralph/plan.md`)*

### Milestone 2: Architect + First Epic
- [ ] *(Detailed plan in `specs/milestone-2-architect-first-epic/plan.md`)*

### Milestone 3: `bm` CLI
- [ ] *(Planning in progress — `specs/milestone-3-bm-cli/`)*

### Milestone 4: Full Team + First Story (deferred)
- [ ] *(Deferred — design.md Section 6)*

### Milestone 5: Eval/Confidence System (deferred)
- [ ] *(Deferred — design.md Section 7)*

### GitHub Integration (complete — pulled forward)
- [x] *(Completed — `specs/github-migration/`)*

---

## Milestone 1 Steps

### Step 1: Validate Persistent Polling

**Objective:** Confirm Ralph's event loop stays alive when no work is found. This is the #1 key assumption (design Section 4.2) — the entire pull-based coordination model depends on team members staying alive indefinitely.

**Implementation guidance:**
- Create a minimal `ralph.yml` with a single hat (`scanner`) that triggers on `board.scan` and publishes `board.idle`
- Write a minimal `PROMPT.md` with scanner instructions: "On each cycle, append three timestamped entries to a log file (`poll-log.txt`): (1) scan start, (2) scan result (no work found), (3) scan end / going idle. Then re-scan."
- The scanner hat must never emit a terminal event — on idle, it sleeps and re-emits `board.scan` to keep the event loop running
- Test both approaches from design Section 4.12 Q4: (a) never emit terminal event (preferred), (b) external restart wrapper
- Use a throwaway directory for this validation — not the generator repo structure

**Test requirements:**
- Ralph starts and enters the scanner hat
- With no work to find, Ralph loops back to `board.scan` after a cooldown
- Each scan cycle appends three timestamped lines to `poll-log.txt`:
  ```
  2026-02-14T12:00:05Z — board.scan — START
  2026-02-14T12:00:06Z — board.scan — no work found
  2026-02-14T12:00:06Z — board.scan — END (going idle)
  ```
- The START/END markers make each cycle traceable — you can verify the scanner completes full cycles, not just that it's alive
- External validation: `tail -f poll-log.txt` shows triplets of entries appearing over time, confirming the loop completes full cycles without needing to inspect Ralph's internal event log
- Ralph stays alive for 5+ minutes — verified by checking that `poll-log.txt` has entries spanning 5+ minutes
- If approach (a) fails, validate approach (b) and document findings

**Integration notes:** This validation determines how all subsequent ralph.yml configs are structured. If persistent polling doesn't work as expected, the event loop design for all team members must be revised before proceeding.

**Demo:** Two terminals side by side: (1) Ralph running, (2) `tail -f poll-log.txt` showing timestamped entries appearing at regular intervals. The log file is the external proof that the event loop is alive. Kill manually to end.

---

### Step 2: Generator Skeleton + `just init`

**Objective:** Build the generator repo structure and the `just init` recipe that layers skeleton + profile into a new team repo.

**Layer:** Generator skeleton

**Implementation guidance:**
- Generator repo structure:
  ```
  botminter/                              # GENERATOR REPO
  ├── Justfile                                   # init recipe
  ├── skeletons/
  │   ├── team-repo/                             # Generic skeleton (process-agnostic)
  │   │   ├── Justfile                           # Baked into generated repos (Steps 5-7)
  │   │   ├── team/
  │   │   ├── projects/
  │   │   └── .github-sim/
  │   │       ├── issues/
  │   │       ├── milestones/
  │   │       └── pulls/
  │   └── profiles/
  │       └── rh-scrum/                          # RH Scrum profile (Steps 3-4)
  │           ├── PROCESS.md
  │           ├── CLAUDE.md
  │           ├── knowledge/
  │           ├── invariants/
  │           └── members/
  │               └── human-assistant/             # human-assistant member skeleton (Step 4)
  └── specs/                                     # Design artifacts (already exists)
  ```
- Implement `just init` in the generator Justfile (run from inside the generator repo):
  - Accepts `--repo=<path>` (target path) and `--profile=<name>` (profile to apply)
  - Step 1: Copies `skeletons/team-repo/` to the target path (bare structure + Justfile)
  - Step 2: Overlays `skeletons/profiles/<profile>/` on top (PROCESS.md, CLAUDE.md, knowledge, invariants)
  - Step 3: Copies the full generator content (skeleton + profile, minus the generator Justfile) into `<target>/.team-template/` so the team repo has access to member skeletons and profile content for `just add-member` and future syncs
  - Step 4: Initializes as a git repo, makes initial commit
  - Fails if the target path already exists (does NOT overwrite)
  - Accepts an optional `project=<name>` argument to create a `projects/<name>/` subtree with knowledge/ and invariants/
- The generated team repo is self-contained — it has its own Justfile, profile content, and member skeletons

**Test requirements:**
- `just init --repo=/tmp/test-team --profile=rh-scrum` creates `/tmp/test-team/`
- The target is a valid git repo with an initial commit
- Target has `Justfile` (from skeleton), `PROCESS.md`, `CLAUDE.md` (from profile), `.github-sim/{issues,milestones,pulls}/`
- Target has `.team-template/` containing the full generator content (skeleton + profile, minus Justfile)
- `.team-template/profiles/rh-scrum/members/human-assistant/` exists (member skeletons accessible for `just add-member`)
- Running `just init` twice for the same path fails
- `just init --repo=/tmp/test-team --profile=rh-scrum project=hypershift` also creates `projects/hypershift/knowledge/` and `projects/hypershift/invariants/`

**Integration notes:** This is the only recipe in the generator Justfile. Everything else (`add-member`, `create-workspace`, `launch`) lives in the generated team repo's Justfile. The generator's job is done after `init`.

**Demo:**
```
$ cd botminter
$ just init --repo=~/workspace/demo-team --profile=rh-scrum
$ tree ~/workspace/demo-team/
```

---

### Step 3: RH Scrum Profile — PROCESS.md, CLAUDE.md, Knowledge, Invariants

**Objective:** Create the RH Scrum profile with process documents, team knowledge, and invariants that define how a Red Hat scrum team operates (design Sections 3.4, 3.6, 3.8, 3.9).

**Layer:** Profile (`skeletons/profiles/rh-scrum/`)

**Implementation guidance:**
- **PROCESS.md** (`skeletons/profiles/rh-scrum/PROCESS.md`):
  - Issue frontmatter format (number, title, state, labels, assignee, milestone, parent, created) — design Section 4.8
  - `kind/*` labels: `kind/epic`, `kind/story`
  - `status/<role>:<phase>` naming convention (no specific statuses yet — M2/M3)
  - Comment format: `### @role — timestamp`
  - Communication protocols: how members coordinate via `.github-sim/`
  - Milestone and PR formats (design Section 4.8)
  - Process evolution paths: formal (PR-based) and informal (direct) — design Section 4.6
  - Concise — this is a reference for agents, not a human guide
- **CLAUDE.md** (`skeletons/profiles/rh-scrum/CLAUDE.md`):
  - Describes the team repo as control plane, references PROCESS.md
  - Describes workspace model, coordination model, file-based pull-based status labels
  - Oriented toward what any team member needs to know on first read
  - Does NOT duplicate PROCESS.md
- **Team knowledge** (`skeletons/profiles/rh-scrum/knowledge/`):
  - `commit-convention.md`, `pr-standards.md`, `communication-protocols.md` — stubs reflecting RH engineering norms
- **Team invariants** (`skeletons/profiles/rh-scrum/invariants/`):
  - `code-review-required.md`, `test-coverage.md` — prompt-based rules reflecting RH quality standards
- Mark placeholder files clearly — "to be populated with real content before the team goes live."

**Test requirements:**
- PROCESS.md contains all format conventions from design Section 4.8
- CLAUDE.md provides sufficient context for team member orientation
- Team knowledge/invariant files exist with meaningful stub content
- After `just init --profile=rh-scrum`, the generated team repo contains all profile content
- Profile content is distinct from skeleton content — skeleton provides structure, profile provides process

**Integration notes:** This profile is reusable. A different RH team (e.g., CAPI, controller-runtime) would use the same `rh-scrum` profile with a different `project=` argument. Non-RH teams could create their own profiles (e.g., `skeletons/profiles/startup-kanban/`).

**Demo:** `just init --repo=~/workspace/demo-team --profile=rh-scrum && cat ~/workspace/demo-team/PROCESS.md` showing a complete, self-contained process reference.

---

### Step 4: RH Scrum Profile — human-assistant Member Skeleton

**Objective:** Create the human-assistant member skeleton within the RH Scrum profile (design Section 4.7).

**Layer:** Profile (`skeletons/profiles/rh-scrum/members/human-assistant/`)

**Implementation guidance:**
- Build `skeletons/profiles/rh-scrum/members/human-assistant/`:
  ```
  skeletons/profiles/rh-scrum/members/human-assistant/
  ├── ralph.yml          # Event loop: persistent, board.scan starting event, board_scanner hat
  ├── PROMPT.md          # Scanner instructions, training mode protocol
  ├── CLAUDE.md          # Role-specific context, references team-repo CLAUDE.md
  ├── knowledge/         # Empty initially
  ├── invariants/
  │   └── always-confirm.md
  └── projects/          # Empty initially
  ```
- **ralph.yml:** `persistent: true`, `starting_event: board.scan`. Single hat: `board_scanner` triggers on `[board.scan, board.rescan]`, publishes `[board.report, board.act, board.idle]`. Use the validated pattern from Step 1.
- **PROMPT.md:** Training mode instructions. Scanner reads `.github-sim/issues/` via submodule path, builds board state, reports to human via `human.interact`. Training mode protocol: (1) current board state, (2) what just happened, (3) what it expects next and why. Includes `poll-log.txt` timestamped logging (START/result/END per cycle).
- **CLAUDE.md:** References `team-repo/CLAUDE.md` for shared context. Human-assistant-specific: training mode, always confirm, observation protocol.
- **invariants/always-confirm.md:** The human-assistant always confirms with the human before acting.
- This skeleton is copied into generated team repos by `just init` (into `.team-template/`) and used by `just add-member` (Step 5).

**Test requirements:**
- `skeletons/profiles/rh-scrum/members/human-assistant/ralph.yml` is valid YAML matching design Section 4.7 event flow
- `skeletons/profiles/rh-scrum/members/human-assistant/PROMPT.md` contains scanner instructions with training mode protocol and poll-log.txt logging
- `skeletons/profiles/rh-scrum/members/human-assistant/CLAUDE.md` references `team-repo/CLAUDE.md`
- ralph.yml is compatible with ralph-orchestrator at `/opt/workspace/ralph-orchestrator/`
- After `just init --profile=rh-scrum`, the member skeleton is available inside the generated team repo at `.team-template/profiles/rh-scrum/members/human-assistant/`

**Integration notes:** In M2+, additional member skeletons are created in the profile (`members/architect/`, `members/dev/`, etc.). The `add-member` recipe (Step 5) is role-agnostic — it copies whatever skeleton exists for the given role. Roles are profile-specific: `rh-scrum` has human-assistant/architect/dev/qe/reviewer; a different profile might have entirely different roles.

**Demo:** `cat skeletons/profiles/rh-scrum/members/human-assistant/ralph.yml` showing a clean, valid config with persistent polling.

---

### Step 5: Team Repo Justfile — `just add-member`

**Objective:** Implement the `add-member` recipe in the generic team repo skeleton's Justfile. This recipe copies a member skeleton into the team repo's `team/` directory.

**Layer:** Generator skeleton (`skeletons/team-repo/Justfile`)

**Implementation guidance:**
- Implement in `skeletons/team-repo/Justfile`:
  ```
  just add-member <role>
  ```
- Recipe:
  1. Checks `.team-template/profiles/*/members/<role>/` exists — fails with error if not
  2. Checks `team/<role>/` does NOT exist — fails if member already added
  3. Copies the member skeleton from `.team-template/` to `team/<role>/`
  4. Commits the addition to the team repo
- This recipe is generic — it doesn't know about specific roles. It copies whatever skeleton the profile provided. Since this recipe is in the generic skeleton, every generated team repo inherits it regardless of profile.

**Test requirements:**
- From a generated team repo: `just add-member human-assistant` creates `team/human-assistant/` with all human-assistant files
- `team/human-assistant/ralph.yml`, `team/human-assistant/PROMPT.md`, `team/human-assistant/CLAUDE.md` match the skeleton
- Running `just add-member human-assistant` twice fails (does NOT overwrite)
- `just add-member nonexistent` fails with a clear error
- The addition is committed to the team repo

**Integration notes:** This is the first recipe the user runs inside a generated team repo. In M2+, `just add-member architect`, `just add-member dev`, etc. work identically. The available roles depend on which profile was used during `init`.

**Demo:**
```
$ cd botminter
$ just init --repo=~/workspace/demo-team --profile=rh-scrum
$ cd ~/workspace/demo-team
$ just add-member human-assistant
$ tree team/human-assistant/
```

---

### Step 6: Team Repo Justfile — `just create-workspace`

**Objective:** Implement the `create-workspace` recipe that stamps out a member workspace with the team repo as a submodule (design Section 4.5).

**Layer:** Generator skeleton (`skeletons/team-repo/Justfile`)

**Implementation guidance:**
- Implement in `skeletons/team-repo/Justfile`:
  ```
  just create-workspace <member>
  ```
- Recipe:
  1. Checks `team/<member>/` exists — fails if member not added yet
  2. Creates `workspace-<member>/` as a sibling of the team repo
  3. Initializes it as a git repo
  4. Adds the team repo as a git submodule at `team-repo/`
  5. Copies (surfaces) all files from `team-repo/team/<member>/` to workspace root
  6. Commits
- Handle version checking: compare `ralph.yml` version in `team-repo/team/<member>/ralph.yml` vs workspace root to decide full setup vs incremental sync
- Running on an existing workspace performs incremental sync (re-surface files)

**Test requirements:**
- `just create-workspace human-assistant` produces `../workspace-human-assistant/` (sibling to team repo)
- `workspace-human-assistant/team-repo/` is a working submodule pointing to the team repo
- `workspace-human-assistant/ralph.yml` matches `team/human-assistant/ralph.yml`
- `workspace-human-assistant/PROMPT.md` matches `team/human-assistant/PROMPT.md`
- `workspace-human-assistant/CLAUDE.md` matches `team/human-assistant/CLAUDE.md`
- Workspace root is otherwise clean (no stray files)
- Re-running on existing workspace performs sync without re-cloning

**Integration notes:** The workspace is where Ralph actually runs. The team repo (via submodule) provides `.github-sim/`. The surfaced files give Ralph a clean top-level view. Workspace is a sibling directory to the team repo, not inside it.

**Demo:**
```
$ cd ~/workspace/demo-team
$ just create-workspace human-assistant
$ tree ../workspace-human-assistant/
```

---

### Step 7: Team Repo Justfile — `just launch`

**Objective:** Implement the `launch` recipe that creates/syncs the workspace and starts Ralph (design Section 4.5 launch sequence).

**Layer:** Generator skeleton (`skeletons/team-repo/Justfile`)

**Implementation guidance:**
- Implement in `skeletons/team-repo/Justfile`:
  ```
  just launch <member>
  ```
- Recipe:
  1. If `../workspace-<member>/` doesn't exist, run `just create-workspace <member>` first
  2. If it exists, sync (re-surface files if version changed)
  3. `cd ../workspace-<member> && ralph run -p PROMPT.md`
- Add `just launch <member> --dry-run` mode that does everything except `ralph run` (for testing setup)
- Log each step for debuggability
- Do NOT start Ralph in a mode that requires interactive input

**Test requirements:**
- `just launch human-assistant --dry-run` creates/syncs the workspace without starting Ralph
- `just launch human-assistant` starts Ralph in the workspace
- Ralph loads `PROMPT.md` and enters the scanner hat
- If workspace already exists and is current, launch skips setup and goes straight to `ralph run`

**Integration notes:** This is the primary human interface. In M1, the human runs `just launch human-assistant`. In M2+, `just launch architect`, `just launch dev`, etc. The recipe composes `create-workspace` + `ralph run`.

**Demo:** `just launch human-assistant --dry-run` showing workspace setup output, confirming files surfaced correctly.

---

### Step 8: Integration — init → add-member → launch + HyperShift Project Knowledge

**Objective:** Run the full pipeline from generator to running human-assistant. Add HyperShift-specific project knowledge directly to the team repo instance (not to the profile).

**Layer:** All three — generator (init), profile (rh-scrum), team repo instance (HyperShift knowledge)

**Implementation guidance:**
- Full sequence:
  ```
  # Generator: stamp out team repo with RH Scrum profile
  cd botminter
  just init --repo=~/workspace/hypershift-team --profile=rh-scrum project=hypershift

  # Team repo instance: add HyperShift-specific project knowledge
  cd ~/workspace/hypershift-team
  # Populate projects/hypershift/knowledge/ with content from SME research
  # (hcp-architecture.md, nodepool-patterns.md, upgrade-flow.md)
  # Populate projects/hypershift/invariants/ (pre-commit.md, upgrade-path-tests.md)
  # This is project-specific — stays in the instance, not the profile

  # Team repo: add human-assistant and launch
  just add-member human-assistant
  just launch human-assistant
  ```
- HyperShift project knowledge is populated from SME research in `specs/master-plan/research/sme-*.md` — either manually or via a helper recipe
- Ralph should start, load `PROMPT.md`, enter the `board_scanner` hat
- Scanner reads `team-repo/.github-sim/issues/` via submodule — finds no issues
- Scanner reports empty board state in training-mode format
- Each cycle appends START/result/END triplet to `poll-log.txt`
- Verify Ralph does NOT exit on empty board

**Test requirements:**
- Full pipeline runs without manual intervention
- Generated team repo has: Justfile (skeleton), PROCESS.md + CLAUDE.md + team knowledge/invariants (profile), HyperShift project knowledge (instance-specific)
- The three layers are clearly separated — profile content came from `rh-scrum`, project knowledge was added directly
- human-assistant is configured in the team repo (from profile's member skeleton)
- Workspace is created as sibling with surfaced files and submodule
- Ralph starts and scans successfully
- `tail -f ../workspace-human-assistant/poll-log.txt` shows timestamped START/result/END triplets
- Event loop continues cycling for 5+ minutes
- No errors in Ralph's output

**Integration notes:** This step exercises all three layers for the first time. The HyperShift project knowledge is the first instance-specific content. As the team validates with real HyperShift stories (M2+), learnings that are process-related (not project-specific) should flow back to the `rh-scrum` profile.

**Demo:**
```
$ cd botminter && just init --repo=~/workspace/hypershift-team --profile=rh-scrum project=hypershift
$ cd ~/workspace/hypershift-team && just add-member human-assistant && just launch human-assistant
# second terminal:
$ tail -f ../workspace-human-assistant/poll-log.txt
```
"The team is alive — a team of one, generated from skeleton + profile."

---

### Step 9: HIL Round-Trip via Telegram

**Objective:** Configure RObot and validate the full human-in-the-loop path: human-assistant sends message via Telegram, human receives and responds, human-assistant processes the response (design Section 4.10).

**Layer:** Runtime (workspace configuration, bot onboarding)

**Prerequisite:** `RALPH_TELEGRAM_BOT_TOKEN` environment variable MUST be set before this step runs. If not set, abort with a clear error message. The human has already created a bot via @BotFather.

**Automation boundary:** This step has both automated and manual parts. Ralph automates onboarding, preflight, launch, and verification. The human participates by (1) sending a message to the bot during onboarding and (2) responding to the human-assistant's training-mode message on Telegram.

**Implementation guidance:**

Phase A — Bot Onboarding:
- Use the Step 8 integration test workspace (`/tmp/hypershift-team/`). Re-run `just create-workspace human-assistant` to sync.
- In the workspace (`/tmp/workspace-human-assistant/`), run `ralph bot onboard --telegram --token "$RALPH_TELEGRAM_BOT_TOKEN"`
- The human must send a message to the bot on Telegram to establish the `chat_id`. Ralph waits up to 120s for this.
- Onboarding saves `chat_id` to `.ralph/telegram-state.json`

Phase B — Preflight Smoke Test:
- Run `ralph bot test "Hello from human-assistant — preflight test"` from the workspace
- Verify exit code 0. This confirms the token and chat_id are valid and messages can be delivered.

Phase C — human-assistant Launch:
- Ensure RObot is enabled in the workspace's `ralph.yml` (`enabled: true`). If it was disabled during Step 8, re-enable it.
- Launch human-assistant: `cd /tmp/workspace-human-assistant && CLAUDECODE= ralph run -p PROMPT.md &` (store PID)
- human-assistant enters `board_scanner`, scans empty board, sends training-mode message via Telegram

Phase D — Human Responds (manual):
- The human sees the training-mode message on Telegram and responds (e.g., "Acknowledged, no work yet.")
- Ralph cannot perform this step — it waits and then verifies.

Phase E — Verification:
- Wait at least 2 minutes for human response, or poll the events file for `human.response`
- Verify poll-log.txt shows continued cycles after the interaction
- Verify events file contains `human.interact` and `human.response` events
- Verify human-assistant process is still alive (PID check)
- Kill human-assistant by PID when done

**Test requirements:**
- Bot onboarding completes and `.ralph/telegram-state.json` has `chat_id`
- `ralph bot test` exits 0 (preflight passes)
- human-assistant starts with RObot connected
- Training-mode message appears in Telegram (human confirms by responding)
- `human.response` event appears in events file after human responds
- `poll-log.txt` continues showing cycles throughout the HIL interaction
- Timeout handling works (human-assistant doesn't hang if human doesn't respond within `timeout_seconds`)

**Integration notes:** This is the final M1 deliverable. The human-assistant skeleton already has RObot config (from Step 4). The token is provided via env var — never stored in committed files. The chat_id is runtime state discovered during onboarding.

**Demo:** Split screen: (1) `tail -f workspace-human-assistant/poll-log.txt` showing cycles, (2) Telegram showing the human-assistant message and human response. **M1 complete.**

---

## Milestone 1.5 Steps — Autonomous Ralph (Spike)

> Detailed plan in `specs/milestone-1.5-autonomous-ralph/plan.md`.
>
> **Prerequisite:** M1 Step 1 complete (persistent polling validated). M1.5 extends that validation to multi-work-item autonomous operation.
>
> **What:** Minimal prototype proving Ralph can run autonomously in persistent mode — picking up work items from a directory, processing them one at a time, self-clearing scratchpad/tasks between items, and idling when no work remains. Validates the `persistent: true` + `task.resume` → board scanner pattern that M2 adopts.

- [ ] Set up prototype workspace with ralph.yml, PROMPT.md, and 3 work items
- [ ] Run prototype with `RALPH_DIAGNOSTICS=1` and verify all 7 success criteria
- [ ] Capture artifacts to `specs/milestone-1.5-autonomous-ralph/artifacts/`
- [ ] Write findings.md summarizing validated patterns and observed tradeoffs

---

## Milestone 2 Steps — Architect + First Epic

> Detailed plan in `specs/milestone-2-architect-first-epic/plan.md`.
>
> **Prerequisite:** M1 complete (human-assistant running, HIL validated). M1.5 complete (autonomous loop pattern validated).

---

## Milestone 3 Steps — `bm` CLI

> Planning in progress — `specs/milestone-3-bm-cli/`.
>
> **Prerequisite:** M2 complete (architect member skeleton built and tested, two-member coordination validated).

---

## Milestone 4 Steps — Full Team + First Story (Deferred)

> *Deferred. To be planned when M3 is complete.*
>
> Originally Milestone 3 in the master plan. Deferred in favor of the `bm` CLI + daemon milestone.
>
> **Prerequisite:** M3 complete (`bm` CLI operational).

---

## Milestone 5 Steps — Eval/Confidence System (Deferred)

> *Deferred. To be planned when M4 is complete.*
>
> Originally Milestone 4 in the master plan. Scope from design Section 7.
>
> **Prerequisite:** M4 complete (full team operational, first story executed).
