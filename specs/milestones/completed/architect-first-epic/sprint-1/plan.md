# Sprint 1 Plan: One Agent, One Hat — Architect Produces a Design

> Vertical slice: workspace infrastructure + architect (board_scanner + designer).
> A single agent scans the board, finds an epic at `status/arch:design`, produces a
> design doc with knowledge propagation and invariant compliance, transitions to
> `status/po:design-review`.
>
> Design reference: [../design.md](../design.md)

## Checklist

- [x] Step 1: PROCESS.md — epic lifecycle statuses
- [x] Step 2: Team CLAUDE.md — `.botminter/` workspace model
- [x] Step 3: Profile agent/ directory + generator init
- [x] Step 4: Team repo Justfile — create-workspace, sync, launch
- [x] Step 5: Architect member skeleton
- [x] Step 6: Documentation — `docs/`
- [x] Step 7: Synthetic fixtures + end-to-end validation

## Sprint 1 Deviations from Design

The following are **intentional Sprint 1 scope decisions** that deviate from the design.
They are resolved in later sprints.

| Deviation | Rationale | Resolved in |
|-----------|-----------|-------------|
| Training mode: DISABLED (design says ENABLED) | No Telegram/RObot in Sprint 1, so no HIL channel for confirmations. Agent acts autonomously. | Sprint 3 |
| RObot: disabled (design says enabled) | No Telegram bots in Sprint 1. | Sprint 3 |
| `just launch` has no `--telegram-bot-token` (design says mandatory) | Telegram deferred. Design AC 7.9 deferred. | Sprint 3 |
| Project name `hypershift` hardcoded in `skills.dirs` and designer hat | Design review Finding 47 — no `project` field in epic frontmatter. Single-project assumption for M2. | Post-M2 |
| Human-assistant skeleton unchanged | HA still references M1 `team-repo/` paths. Inconsistent within the generated repo until Sprint 2 updates the HA. | Sprint 2 |
| PROCESS.md "Communication Protocols" still references "submodule" | Design review Finding 49 — deferred update. | Sprint 2 |

## Justfile Invocation Context

Recipes are invoked from different contexts depending on whether the workspace exists:

| Recipe | Invoked from | Justfile location | `justfile_directory()` | Notes |
|--------|-------------|-------------------|----------------------|-------|
| `create-workspace` | Team repo directory | `<team-repo>/Justfile` | Team repo root | Workspace doesn't exist yet. Uses `team_root := justfile_directory()`. |
| `sync` | Project repo (agent CWD) | `.botminter/Justfile` | `.botminter/` | Called as `just -f .botminter/Justfile sync`. Uses `project_root := parent_directory(justfile_directory())` for project-level ops. |
| `launch` | Team repo directory | `<team-repo>/Justfile` | Team repo root | Requires workspace to already exist (created by `create-workspace`). |

The `sync` recipe lives in the **same Justfile** as `create-workspace`, but is invoked
differently. When agents call `just -f .botminter/Justfile sync`, the Justfile is inside
`.botminter/` so `justfile_directory()` resolves to `.botminter/`. Recipes that need the
project repo root must use `project_root := parent_directory(justfile_directory())`.

---

## Step 1: PROCESS.md — Epic Lifecycle Statuses

**Objective:** Define the epic statuses, write-lock settings, and error status so all
subsequent work has a shared vocabulary.

**Implementation:**

Update `skeletons/profiles/rh-scrum/PROCESS.md` — append after the existing
"Status Label Convention" section:

- **Epic Statuses (M2)** table — all 11 statuses with role and description
  (design.md Section 4.3):

  | Status | Role | Description |
  |--------|------|-------------|
  | `status/po:triage` | human-assistant | New epic, awaiting evaluation |
  | `status/po:backlog` | human-assistant | Accepted, prioritized, awaiting activation |
  | `status/arch:design` | architect | Architect producing design doc |
  | `status/po:design-review` | human-assistant | Design doc awaiting human review |
  | `status/arch:plan` | architect | Architect proposing story breakdown |
  | `status/po:plan-review` | human-assistant | Story breakdown awaiting human review |
  | `status/arch:breakdown` | architect | Architect creating story issues |
  | `status/po:ready` | human-assistant | Stories created, epic in ready backlog |
  | `status/arch:in-progress` | architect | Architect monitoring story execution |
  | `status/po:accept` | human-assistant | Epic awaiting human acceptance |
  | `status/done` | — | Epic complete |

- **Rejection Loops** — which review gates can reject and where they send the epic back.
- **Story Statuses (M2 Placeholder)** — `status/dev:ready` as a deliberate M3 placeholder.
- **Error Status** — `status/error` for issues that failed processing 3 times.
- **Write-Lock Settings** — `stale_lock_threshold_minutes` default 5.

Note: The existing "Communication Protocols > Status Transitions" section still
references the M1 submodule model ("pushing via the submodule"). This is a known
inconsistency (design review Finding 49) deferred to Sprint 2.

**Test:** Generate a team repo with `just init`, verify PROCESS.md contains all new sections.

**Integration:** Foundation vocabulary for all subsequent steps. The `status/po:*` statuses
are defined now as vocabulary but only operationally exercised by the HA in Sprint 2.

---

## Step 2: Team CLAUDE.md — `.botminter/` Workspace Model

**Objective:** Replace the M1 submodule workspace model documentation with the
`.botminter/` model so agents read the correct paths.

**Implementation:**

Rewrite `skeletons/profiles/rh-scrum/CLAUDE.md` (design.md Section 4.11 for the workspace
model section; Section 4.6.2 for propagation; Section 4.7 for agent capabilities):

- **What This Repo Is** — keep existing content (team repo = control plane, not code repo).
- **Workspace Model (M2)** — replace submodule model. Agent CWD is the project repo,
  team repo cloned into `.botminter/`. Show layout with two members per design.md Section 4.11.
- **Coordination Model** — update to reference `.botminter/.github-sim/issues/` instead
  of `team-repo/.github-sim/issues/`.
- **File-Based Workflow** — update table paths from `team-repo/` to `.botminter/`.
- **Knowledge Resolution** — update all paths from `team-repo/` prefix to `.botminter/`
  prefix. Add member+project scope. Add hat-level scope. Five levels total:
  1. Team: `.botminter/knowledge/`
  2. Project: `.botminter/projects/<project>/knowledge/`
  3. Member: `.botminter/team/<member>/knowledge/`
  4. Member+project: `.botminter/team/<member>/projects/<project>/knowledge/`
  5. Hat: `.botminter/team/<member>/hats/<hat>/knowledge/`
- **Invariant Scoping** — update paths from `team-repo/` to `.botminter/`.
- **Agent Capabilities** — new section per design.md Section 4.7. Document the `agent/`
  directory scoping (team, project, member). Naming convention: dot-delimited scope
  prefixes (e.g., `hypershift.codebase-search`, `architect.design-template`). Skills
  read via `skills.dirs` in ralph.yml; agents symlinked into `.claude/agents/`;
  settings copied to `.claude/settings.local.json`.
- **Propagation Model** — per design.md Section 4.6.2. What auto-updates on `git pull`
  (knowledge, invariants, skills, PROMPT.md/CLAUDE.md via symlinks) vs what requires
  `just sync` (ralph.yml, settings.local.json).
- **Team Repo Access Paths** — update table to `.botminter/` paths.
- **Write-Lock Protocol** — summary of per-issue locking. Lock files at
  `.botminter/.github-sim/issues/<number>.lock`. Reference PROCESS.md for settings.

**Test:** Generate a team repo, verify CLAUDE.md describes `.botminter/` paths throughout.
No `team-repo/` references remain.

**Integration:** All agents (current and future) will read this CLAUDE.md for workspace context.

---

## Step 3: Profile agent/ Directory + Generator Init

**Objective:** Add the agent capabilities directory structure to the profile and teach
the generator to overlay it.

**Implementation:**

1. Create profile-level agent directories:
   - `skeletons/profiles/rh-scrum/agent/skills/.gitkeep`
   - `skeletons/profiles/rh-scrum/agent/agents/.gitkeep`

   Note: Team-level skills (`create-epic`, `board` per design Section 3.2) are
   deferred to Sprint 2 when the HA is updated. Sprint 1 creates empty directories
   as scaffolding.

2. Update root `Justfile` `init` recipe:
   - After overlaying `knowledge/` and `invariants/`, also overlay `agent/` from profile:
     ```bash
     if [ -d "$PROFILE_DIR/agent" ]; then
         cp -r "$PROFILE_DIR/agent" "$REPO/agent"
     fi
     ```
   - When `project=` is specified, also create project-level agent directories:
     ```bash
     mkdir -p "$REPO/projects/$PROJECT/agent/skills"
     mkdir -p "$REPO/projects/$PROJECT/agent/agents"
     ```
   - Note: `.team-template/` already copies the full `skeletons/profiles/` directory tree
     (existing line 94), so the new `agent/` directories are automatically included.
     Verify this during testing.

**Test:**
- `just init --repo=/tmp/test --profile=rh-scrum project=hypershift`
- Verify `/tmp/test/agent/skills/` and `/tmp/test/agent/agents/` exist
- Verify `/tmp/test/projects/hypershift/agent/skills/` and `agents/` exist
- Verify `/tmp/test/.team-template/profiles/rh-scrum/agent/` exists (propagated for
  `add-member` to work)

**Integration:** Generated repos now have the agent capabilities directory structure.
Workspace assembly (Step 4) will link from these directories.

---

## Step 4: Team Repo Justfile — create-workspace, sync, launch

**Objective:** Rewrite workspace management recipes for the `.botminter/` model.

**Implementation:**

Rewrite `skeletons/team-repo/Justfile` with three updated recipes. Add a shared
variable at the top of the Justfile:

```just
# Team repo root (where this Justfile lives).
# When invoked directly: this is the team repo.
# When invoked via `just -f .botminter/Justfile`: this is .botminter/ inside the project repo.
team_root := justfile_directory()

# Project repo root (parent of team_root).
# Only meaningful when invoked from a workspace via `just -f .botminter/Justfile`.
project_root := parent_directory(justfile_directory())
```

### `create-workspace <member> <project-repo-url>`

Per design.md Section 4.8. Replaces M1's submodule model entirely.

Invoked from the team repo directory: `just create-workspace architect <url>`.

1. Validate member exists in `team/<member>/`.
2. Clone `<project-repo-url>` to sibling directory. Workspace name: basename of the
   project repo URL with the member name appended (e.g., `hypershift-architect/`).
3. Clone team repo into `.botminter/` inside the cloned project repo. **Clone source:**
   use the local filesystem path `$TEAM_ROOT` (= `justfile_directory()`). This works
   for both local-only repos (from `just init`) and repos with remotes. The `.botminter/`
   clone's `origin` will point at the local team repo path.
4. Create symlinks at project root: `PROMPT.md` and `CLAUDE.md` pointing into
   `.botminter/team/<member>/`.
5. Copy `ralph.yml` from `.botminter/team/<member>/ralph.yml`.
6. Assemble `.claude/` directory:
   - `.claude/agents/` — symlinks from `agent/agents/` at team, project, and member
     levels. Use a loop over the three `agent/agents/` directories (all under `.botminter/`):
     ```bash
     mkdir -p .claude/agents
     for dir in .botminter/agent/agents \
                .botminter/projects/*/agent/agents \
                .botminter/team/$MEMBER/agent/agents; do
         for f in "$dir"/*.md 2>/dev/null; do
             [ -f "$f" ] && ln -sf "$(realpath "$f")" .claude/agents/
         done
     done
     ```
     Note: In Sprint 1, all `agent/agents/` directories contain only `.gitkeep`, so
     no symlinks will be created. The loop is tested structurally but not functionally
     until agent files are added (Sprint 2+).
   - `.claude/settings.local.json` — copy from member's `agent/settings.local.json` if exists.
7. Write `.botminter/.member` marker file containing the member role name.
8. Write `.git/info/exclude` with agent artifact patterns (`.botminter/`, `PROMPT.md`,
   `CLAUDE.md`, `.claude`, `.ralph/`, `ralph.yml`, `poll-log.txt`, `.gitignore`).
9. Write belt-and-suspenders `.gitignore` with same patterns.

### `sync`

Per design.md Section 4.6.3. Called by agents at the start of every scan cycle.

Invoked from the project repo: `just -f .botminter/Justfile sync`.

All project-level paths use `$project_root` (= `parent_directory(justfile_directory())`).
All team repo paths use `$team_root` (= `justfile_directory()` = `.botminter/`).

1. Read member name from `$team_root/.member` (= `.botminter/.member`).
2. Pull team repo: `cd $team_root && git pull origin main`.
   - Note: For Sprint 1 local-path clones, `origin` points at the local team repo
     directory. This works for `git pull` as long as the team repo has commits.
3. Pull project repo: `cd $project_root && git pull` (if project repo has a remote;
   skip gracefully if no remote is configured).
4. If `$team_root/team/<member>/ralph.yml` is newer than `$project_root/ralph.yml`,
   re-copy and warn restart needed.
5. If `$team_root/team/<member>/agent/settings.local.json` is newer than
   `$project_root/.claude/settings.local.json`, re-copy.
6. Re-assemble `$project_root/.claude/agents/` symlinks (same loop as create-workspace,
   idempotent — remove existing symlinks first, then recreate).
7. Verify PROMPT.md and CLAUDE.md symlinks intact (recreate if broken).
8. Verify `.git/info/exclude` has all required patterns.

### `launch <member> [--dry-run]`

Updated for `.botminter/` model. Sprint 1 version: no `--telegram-bot-token`
(added in Sprint 3, satisfying design AC 7.9).

Invoked from the team repo directory: `just launch architect`.

1. Determine workspace path. The recipe must know where the workspace is. Convention:
   the workspace was created by `create-workspace` as a sibling of the team repo.
   `launch` scans for a sibling directory containing `.botminter/.member` with the
   matching member name. If not found, abort with error:
   `"Error: No workspace found for '<member>'. Run 'just create-workspace <member> <url>' first."`
   Note: Unlike M1, `launch` does NOT auto-create workspaces — the project repo URL
   is unknown. The user must run `create-workspace` first.
2. Sync workspace: `cd <workspace> && just -f .botminter/Justfile sync`.
3. Launch Ralph: `cd <workspace> && ralph run -p PROMPT.md`.
4. `--dry-run` prints workspace state without launching.

**Test:**
- Generate team repo, add architect.
- Create a synthetic project repo: `git init /tmp/synth-project && cd /tmp/synth-project && echo "# Synthetic" > README.md && git add -A && git commit -m "init"`.
- `just create-workspace architect /tmp/synth-project`.
- Verify layout: PROMPT.md is symlink, CLAUDE.md is symlink, ralph.yml is copy,
  `.botminter/.member` = "architect", `.git/info/exclude` has patterns,
  `.gitignore` exists with patterns, `.botminter/` is a clone of the team repo.
- Run `just -f <workspace>/.botminter/Justfile sync`, verify no errors.
- Run `just launch architect --dry-run`, verify workspace discovery works.

**Integration:** Workspace infrastructure ready. Any member can be deployed with
create-workspace + launch.

**Demo:** Created workspace has correct `.botminter/` layout (manually inspectable).

---

## Step 5: Architect Member Skeleton

**Objective:** Create the architect role with board_scanner + designer hats. Only two
hats in Sprint 1 — remaining hats added in Sprint 2.

**Implementation:**

Create `skeletons/profiles/rh-scrum/members/architect/` with:

### `ralph.yml`

Per design.md Section 4.1.1, but with only `board_scanner` and `designer` hats.
Strip `planner`, `breakdown_executor`, `epic_monitor` (Sprint 2).

- `event_loop`: persistent, board.scan starting event, 10000 max iterations,
  86400 max runtime, 60s cooldown_delay_seconds.
- `core.guardrails`: invariant compliance paths, lock-late principle.
- `board_scanner` hat:
  - triggers: `board.scan`, `board.rescan`
  - publishes: `arch.design`, `LOOP_COMPLETE` (Sprint 2 adds `arch.plan`,
    `arch.breakdown`, `arch.in_progress`)
  - default_publishes: `LOOP_COMPLETE`
  - Instructions include: self-clear (overwrite scratchpad, delete tasks.jsonl),
    sync (`just -f .botminter/Justfile sync`), scan `.botminter/.github-sim/issues/`
    for `status/arch:*`, poll-log, idempotency check, failed processing escalation
    (3-strike → `status/error`), agent startup self-cleanup (scan for own stale locks
    on first cycle).
  - Training mode conditional: "If TRAINING MODE is ENABLED: report board state to
    human before dispatching." Inert in Sprint 1 (training mode disabled).
- `designer` hat:
  - triggers: `arch.design`
  - publishes: `board.rescan`
  - default_publishes: `board.rescan`
  - Full workflow from design.md — read epic, read codebase from `./` (project repo),
    consult knowledge at all applicable scopes (team, project, member, hat), acquire lock,
    produce design doc at `.botminter/projects/hypershift/knowledge/designs/epic-<number>.md`
    (project name hardcoded as `hypershift` for M2), link from epic via comment, transition
    status `arch:design` → `po:design-review`, release lock.
  - Knowledge paths listed explicitly in hat instructions per design.md Section 4.1.1.
  - Backpressure gates (all must be satisfied before transitioning):
    - Design doc has a Security Considerations section
    - Design doc has acceptance criteria (Given-When-Then)
    - Design doc references applicable project knowledge
    - Design doc addresses all applicable invariants
  - On failure: append comment `Processing failed: <reason>. Attempt N/3.`,
    publish `board.rescan`.
  - Training mode conditional: "If TRAINING MODE is ENABLED: report intent to human
    via `human.interact` and wait for confirmation." Inert in Sprint 1.
- `skills.dirs`: team, project, member agent/skills paths through `.botminter/`
  (hardcodes `hypershift` for project path — known limitation, design review Finding 35).
- `RObot: enabled: false` — Sprint 1 deviation from design (see deviations table above).
  Sprint 3 enables RObot.
- `tasks: enabled: true`
- `memories: enabled: true, inject: auto, budget: 2000`
- `skills: enabled: true`

### `PROMPT.md`

Per design.md Section 4.1.2 with Sprint 1 modifications.

- Role identity: architect, technical authority, pull-based agent.
- **TRAINING MODE: DISABLED** — Sprint 1 deviation. Agent acts autonomously without
  human confirmation. Sprint 3 re-enables training mode with Telegram.
- Codebase access: CWD is project repo, fork chain.
- Team configuration: `.botminter/`.
- Write-lock protocol: full acquire/verify/release sequence.
- Workspace sync: `just -f .botminter/Justfile sync`.
- Team context: paths to `.botminter/CLAUDE.md`, `.botminter/PROCESS.md`,
  `.botminter/knowledge/`, `.botminter/projects/<project>/knowledge/`.
- Constraints: never modify without lock, always pull/push, follow knowledge scoping.

### `CLAUDE.md`

Per design.md Section 4.1.3.

- Role: architect, technical authority.
- Workspace model: CWD is project repo, `.botminter/` is team repo clone.
- Codebase access: fork chain.
- Knowledge resolution paths (all `.botminter/` prefixed).
- Invariant compliance paths.
- Write-lock protocol.
- Push-conflict protocol (pull --rebase, verify lock, retry).

### Other files

- `invariants/design-quality.md` — per design.md Section 4.1.4. Required sections:
  overview, architecture (with diagram), components and interfaces, acceptance criteria
  (Given-When-Then), impact on existing system.
- `knowledge/.gitkeep`
- `agent/skills/.gitkeep`
- `agent/agents/.gitkeep`
- `hats/designer/knowledge/.gitkeep`
- `hats/planner/knowledge/.gitkeep` (scaffolding — planner hat added in Sprint 2)
- `projects/.gitkeep`

**Test:**
- `just add-member architect` in a generated team repo — verify `team/architect/` has
  all expected files. Note: requires re-running `just init` after creating the architect
  skeleton so `.team-template/` includes it.
- `just create-workspace architect <url>` — verify symlinks, ralph.yml copy, `.claude/`
  assembly.
- Inspect ralph.yml — only board_scanner + designer hats present, `publishes` lists
  match, `default_publishes` set on both hats.

**Integration:** Architect member fully deployable. Combined with workspace infrastructure,
ready for end-to-end validation.

---

## Step 6: Documentation — `docs/`

**Objective:** Create operator-facing documentation for the workspace model, commands,
and architecture introduced in Sprint 1.

**Implementation:**

Create `docs/` at the generator repo root with the following pages:

### `docs/index.md` — Overview

- What botminter is (generator for agentic team repos)
- Link to architecture, getting started, and reference pages
- Current milestone status

### `docs/getting-started.md` — Getting Started

- Prerequisites (just, ralph, git)
- Generate a team repo: `just init --repo=<path> --profile=rh-scrum project=<name>`
- Add a member: `just add-member <role>`
- Create a workspace: `just create-workspace <member> <project-repo-url>`
- Launch a member: `just launch <member>`
- What happens on first launch (board scanner scans, poll-log.txt created)

### `docs/architecture.md` — Architecture

- Three-layer generator model (skeleton → profile → instance)
- Two-layer runtime model (inner loop per member, outer loop via team repo)
- `.botminter/` workspace model with diagram:
  ```
  project-repo/                    # Agent CWD (project codebase)
    .botminter/                    # Team repo clone
      team/<member>/               # Member config
      .github-sim/issues/          # Coordination fabric
      knowledge/                   # Team knowledge
      projects/<project>/          # Project-scoped knowledge
    PROMPT.md → .botminter/...     # Symlink
    CLAUDE.md → .botminter/...     # Symlink
    ralph.yml                      # Copy
    .claude/agents/                # Assembled symlinks
  ```
- Knowledge and invariant scoping (5 levels)
- Agent capabilities scoping (`agent/` directory)
- Propagation model (what auto-updates vs what needs `just sync`)

### `docs/workspace-commands.md` — Workspace Commands Reference

- `just create-workspace <member> <project-repo-url>` — what it does step by step,
  what files are symlinked vs copied, invocation context
- `just sync` — what it pulls, what it re-copies, how agents invoke it
  (`just -f .botminter/Justfile sync`)
- `just launch <member> [--dry-run]` — workspace discovery, sync, Ralph invocation.
  Note: Sprint 1 has no `--telegram-bot-token` (Sprint 3)
- `just add-member <role>` — where skeletons come from, what gets committed

### `docs/epic-lifecycle.md` — Epic Lifecycle

- Status table (all 11 statuses with role and description)
- Status flow diagram (triage → ... → done)
- Rejection loops (design review, plan review)
- Write-lock protocol (acquire late, release after push)
- Error status and 3-strike escalation
- Note: Sprint 1 covers architect side only; HA covered in Sprint 2

**Test:** All pages render as valid markdown. No broken links between pages.
Cross-reference accuracy with Justfile recipes and PROCESS.md.

**Integration:** Documentation is the entry point for operators and contributors.
Updated incrementally in Sprint 2 and Sprint 3.

---

## Step 7: Synthetic Fixtures + End-to-End Validation

**Objective:** Validate the full vertical slice — architect scans board, produces design
from synthetic epic, with knowledge propagation and invariant compliance.

**Implementation:**

### Synthetic Project Repo

Create a minimal synthetic project repo for testing. The architect's CWD must be a
git repo (it reads the codebase from `./`). For Sprint 1, this is a stub:

```bash
git init /tmp/synth-hypershift
cd /tmp/synth-hypershift
cat > README.md << 'EOF'
# Synthetic HCP Project (M2 Sprint 1)

Minimal project repo for Sprint 1 validation.
The architect reads this codebase when producing designs.

## Architecture

HCP uses a reconciler pattern with controller-runtime.
The main reconciler is in `pkg/controllers/hcp/`.
EOF
mkdir -p pkg/controllers/hcp
cat > pkg/controllers/hcp/reconciler.go << 'EOF'
package hcp

// Reconciler reconciles HCP objects.
// This is a synthetic stub for M2 Sprint 1 validation.
type Reconciler struct {
    // Composition-based: embeds client, not inherits
    Client client.Client
}
EOF
git add -A && git commit -m "Synthetic HCP project for M2 validation"
```

This gives the architect something to read when producing the design.
The README and stub code intentionally contain the knowledge propagation
markers (reconciler pattern, composition) so the architect has codebase
context that aligns with the synthetic knowledge files.

### Synthetic Fixtures

Create `specs/milestone-2-architect-first-epic/fixtures/`:

**Knowledge files** (per design.md Section 8.1):
- `knowledge/commit-convention.md` — "All commits must reference an issue number."
  Detection marker: grep for `issue number` or `Ref: #` in design doc.
- `projects/hypershift/knowledge/hcp-architecture.md` — "HCP uses a reconciler pattern
  with controller-runtime." Detection marker: grep for `reconciler` in design doc.
- `team/architect/knowledge/design-patterns.md` — "Prefer composition over inheritance
  in Go designs." Detection marker: grep for `composition` in design doc.

**Invariant files** (per design.md Section 8.1):
- `invariants/code-review-required.md` — "All changes require code review."
  Detection: design includes a review step.
- `projects/hypershift/invariants/upgrade-path-tests.md` — "Upgrade paths must have
  integration tests." Detection: design includes upgrade test plan.
- `team/architect/invariants/design-quality.md` — "Designs must include acceptance
  criteria." Detection: design has Given-When-Then criteria. (Also in architect skeleton —
  fixtures version augments/replaces to ensure detection marker.)

**Synthetic epic** (per design.md Section 8.1):
- `fixtures/synthetic-epic-1.md` — `[SYNTHETIC] Add health check endpoint to HCP controller`
  with `status/arch:design` (NOT `status/po:triage` as in design.md's fixture template —
  Sprint 1 bypasses triage/backlog since HA is not updated).

**Fixture deployment script:**
- `fixtures/deploy.sh` — takes team repo path as argument. Copies fixtures to correct
  locations in the team repo. **Must `git add -A && git commit`** after copying so the
  fixtures are included when the team repo is cloned into `.botminter/`. Idempotent
  (re-running produces no changes if fixtures already deployed). Also copies the synthetic
  epic to `.github-sim/issues/1.md`.

### End-to-End Validation Sequence

Per design.md Section 8.2 (steps 1-12, adapted for Sprint 1 — no HA involvement):

1. Generate team repo:
   `just init --repo=/tmp/m2-s1-test --profile=rh-scrum project=hypershift`
2. Add architect: `cd /tmp/m2-s1-test && just add-member architect`
3. Deploy synthetic fixtures:
   `bash specs/milestone-2-architect-first-epic/fixtures/deploy.sh /tmp/m2-s1-test`
   (This commits the fixtures to the team repo.)
4. Create synthetic project repo (see above) at `/tmp/synth-hypershift`
5. Create workspace:
   `cd /tmp/m2-s1-test && just create-workspace architect /tmp/synth-hypershift`
   (The team repo is cloned from `/tmp/m2-s1-test` into `.botminter/` inside the
   project repo clone. The epic and fixtures are included because deploy.sh committed them.)
6. Verify workspace layout:
   - PROMPT.md is symlink → `.botminter/team/architect/PROMPT.md`
   - CLAUDE.md is symlink → `.botminter/team/architect/CLAUDE.md`
   - ralph.yml is a copy
   - `.botminter/.member` contains `architect`
   - `.git/info/exclude` has agent patterns
   - `.gitignore` exists
   - `.botminter/.github-sim/issues/1.md` exists with `status/arch:design`
   - Synthetic knowledge files exist at `.botminter/knowledge/`, `.botminter/projects/hypershift/knowledge/`, `.botminter/team/architect/knowledge/`
7. Launch architect: `cd /tmp/m2-s1-test && just launch architect`
8. Verify: design doc exists at `.botminter/projects/hypershift/knowledge/designs/epic-1.md`
9. Verify: epic status transitioned to `status/po:design-review`
10. Verify knowledge propagation (grep-able markers in design doc):
    - `issue number` or `Ref: #` present (team knowledge: commit convention)
    - `reconciler` present (project knowledge: HCP architecture)
    - `composition` present (member knowledge: design patterns)
11. Verify design doc sections (invariant + backpressure compliance):
    - Overview section exists with substantive content
    - Architecture section exists (with diagram if applicable)
    - Components and interfaces section exists
    - Data models section exists (if applicable)
    - Error handling section exists
    - Acceptance criteria in Given-When-Then format
    - Impact on existing system section exists
    - Security Considerations section exists (designer backpressure gate)
12. Verify invariant compliance:
    - Design includes review step (team invariant: code-review-required)
    - Design includes upgrade test plan (project invariant: upgrade-path-tests)
13. Verify: write-lock was acquired and released (no `.lock` file remains)
14. Verify: `poll-log.txt` shows clean scan cycle (START/result/END triplet)

**Test:** All 14 verification steps pass.

**Demo:** Design doc produced by the architect from a synthetic epic. Knowledge
propagation markers detectable via grep. Invariant compliance verified by section
inspection. Write-lock lifecycle clean.
