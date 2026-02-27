# Specification — Milestone 1: Structure + human-assistant

> **Summary:** Build the `botminter` generator repo that stamps out GitOps-style agentic team repos via `just init`, and validate the first team member (human-assistant) running with persistent polling and HIL via Telegram.

**Inputs:** [design.md](../master-plan/design.md) (Sections 3–4), [plan.md](../master-plan/plan.md) (Steps 1–9), [requirements.md](../master-plan/requirements.md), ralph-orchestrator at `/opt/workspace/ralph-orchestrator/`.

---

## 1. Deliverables

M1 produces three categories of output:

| # | Deliverable | Location | Description |
|---|-------------|----------|-------------|
| D1 | Persistent polling validation | `/tmp/poll-test/` (throwaway) | Proves Ralph stays alive indefinitely when idle |
| D2 | Generator Justfile | `Justfile` (repo root) | Single `init` recipe |
| D3 | Generator skeleton | `skeletons/team-repo/` | Bare team repo structure + operational Justfile |
| D4 | RH Scrum profile | `skeletons/profiles/rh-scrum/` | PROCESS.md, CLAUDE.md, knowledge/, invariants/, members/human-assistant/ |
| D5 | Team repo Justfile | `skeletons/team-repo/Justfile` | `add-member`, `create-workspace`, `launch` recipes |
| D6 | Generated team repo | `~/workspace/hypershift-team/` (runtime) | Integration test output |
| D7 | human-assistant workspace | `~/workspace/workspace-human-assistant/` (runtime) | Running human-assistant with persistent polling |

---

## 2. Step 1 — Validate Persistent Polling

### Summary

Confirm Ralph's event loop stays alive indefinitely when no work is found, using a minimal throwaway configuration.

### Specification

#### 2.1 Configuration

Create a throwaway test directory at `/tmp/poll-test/` with the following files:

**`/tmp/poll-test/ralph.yml`:**
```yaml
event_loop:
  prompt_file: PROMPT.md
  completion_promise: LOOP_COMPLETE
  max_iterations: 200
  max_runtime_seconds: 600
  cooldown_delay_seconds: 30
  starting_event: board.scan
  persistent: true

cli:
  backend: claude

hats:
  board_scanner:
    name: Board Scanner
    description: Scans for work, logs activity, stays alive when idle.
    triggers:
      - board.scan
      - board.rescan
    publishes:
      - board.idle
      - board.rescan
    default_publishes: board.rescan
    instructions: |
      ## Board Scanner

      You are a board scanner. Your job is to check for work and log your activity.

      ### Every cycle, do exactly this:

      1. Append a START line to `poll-log.txt`:
         ```
         <ISO-8601-UTC> — board.scan — START
         ```
      2. Check if any work exists (there is none — this is a validation test).
      3. Append a result line:
         ```
         <ISO-8601-UTC> — board.scan — no work found
         ```
      4. Append an END line:
         ```
         <ISO-8601-UTC> — board.scan — END (going idle)
         ```
      5. Publish `board.rescan` to continue the loop.

      ### Rules
      - NEVER publish LOOP_COMPLETE.
      - ALWAYS append all three lines to poll-log.txt before publishing.
      - Use the `ralph emit` command to publish events.

tasks:
  enabled: false

memories:
  enabled: false
```

**`/tmp/poll-test/PROMPT.md`:**
```markdown
You are a persistent board scanner. Your only job is to scan for work, log your activity to poll-log.txt, and stay alive.

Follow the hat instructions exactly. Never exit. Never publish LOOP_COMPLETE.
```

#### 2.2 Execution

```bash
cd /tmp/poll-test
ralph run -p PROMPT.md
```

Run as a background process. Store the PID. Let it run for at least 5 minutes.

**CRITICAL CONSTRAINT:** You MUST NOT kill all ralph processes. You MUST store the PID of the process you launched and kill ONLY that PID when done.

#### 2.3 Acceptance Criteria

| # | Given | When | Then |
|---|-------|------|------|
| AC-1.1 | `/tmp/poll-test/` with ralph.yml and PROMPT.md | `ralph run -p PROMPT.md` is executed | Ralph starts and enters the `board_scanner` hat |
| AC-1.2 | Ralph is running | 1 minute elapses | `poll-log.txt` exists and contains at least 1 complete START/result/END triplet |
| AC-1.3 | Ralph is running | 5+ minutes elapse | `poll-log.txt` contains entries spanning 5+ minutes (timestamps of first and last entry differ by ≥5 min) |
| AC-1.4 | Ralph is running | 5+ minutes elapse | Each scan cycle in `poll-log.txt` has exactly 3 lines: START, result, END |
| AC-1.5 | Ralph is running | At any time | The Ralph process is still alive (PID exists, process is running) |
| AC-1.6 | Validation complete | Operator kills the stored PID | Ralph stops cleanly; only the test process is killed |

#### 2.4 Failure Path

If `persistent: true` does not keep Ralph alive (i.e., Ralph exits after the first cycle or after a few cycles):
1. Document the observed behavior (exit code, last event, iteration count)
2. Attempt approach (b) from design Section 4.12 Q4: wrap Ralph in a restart script that relaunches on exit
3. Document which approach works and carry forward to all subsequent ralph.yml configs
4. Create a memory recording the finding

#### 2.5 Edge Cases

- **Ralph exits with LOOP_COMPLETE:** The scanner hat instructions say never to publish it, but Ralph's internal coordination (the "Ralph" fallback hat) might. If this happens, `persistent: true` should catch it and inject `task.resume`. If it doesn't, this is a failure.
- **Ralph hits max_iterations:** Set to 200, with 30s cooldown, gives ~100 minutes. Sufficient for 5-minute validation. For production, increase to 10000+.
- **Ralph hits max_runtime_seconds:** Set to 600 (10 minutes). Sufficient for validation. For production, increase to 86400+ (24h+).
- **No poll-log.txt created:** The scanner hat failed to execute. Check Ralph's output for errors. Likely cause: hat routing misconfiguration.

---

## 3. Step 2 — Generator Skeleton + `just init`

### Summary

Build the generator repo directory structure and the `just init` recipe that layers skeleton + profile into a new team repo instance.

### Specification

#### 3.1 Generator Repo Structure

```
botminter/                          # THIS REPO (generator)
├── Justfile                               # init recipe (D2)
├── skeletons/
│   ├── team-repo/                         # Generic skeleton (D3)
│   │   ├── Justfile                       # Operational recipes (D5, Steps 5-7)
│   │   ├── team/                          # Empty dir (members added later)
│   │   ├── projects/                      # Empty dir (projects added by init)
│   │   └── .github-sim/
│   │       ├── issues/                    # Empty
│   │       ├── milestones/                # Empty
│   │       └── pulls/                     # Empty
│   └── profiles/
│       └── rh-scrum/                      # RH Scrum profile (D4, Steps 3-4)
│           ├── PROCESS.md
│           ├── CLAUDE.md
│           ├── knowledge/
│           │   ├── commit-convention.md
│           │   ├── pr-standards.md
│           │   └── communication-protocols.md
│           ├── invariants/
│           │   ├── code-review-required.md
│           │   └── test-coverage.md
│           └── members/
│               └── po/                    # human-assistant member skeleton (Step 4)
│                   ├── ralph.yml
│                   ├── PROMPT.md
│                   ├── CLAUDE.md
│                   ├── knowledge/         # Empty dir
│                   ├── invariants/
│                   │   └── always-confirm.md
│                   └── projects/          # Empty dir
└── specs/                                 # Design artifacts (already exists)
```

#### 3.2 Generator Justfile (`Justfile` at repo root)

The generator Justfile has exactly one recipe: `init`.

**Signature:**
```
just init --repo=<path> --profile=<name> [project=<name>]
```

**Behavior:**
1. Validate `--repo` path does NOT exist. If it exists, exit with error: `Error: <path> already exists. Refusing to overwrite.`
2. Validate `skeletons/profiles/<profile>/` exists. If not, exit with error: `Error: Profile '<profile>' not found at skeletons/profiles/<profile>/`
3. Copy `skeletons/team-repo/` to `<repo>/` — bare structure + Justfile
4. Overlay `skeletons/profiles/<profile>/` on top of `<repo>/` — copies PROCESS.md, CLAUDE.md, knowledge/, invariants/ to repo root. Does NOT copy `members/` to repo root (members are added via `add-member`).
5. Copy the full `skeletons/` directory (team-repo + profiles) into `<repo>/.team-template/` — enables `add-member` to find member skeletons without referencing the generator repo. Exclude the generator Justfile (only copy skeletons/).
6. If `project=<name>` is provided, create `<repo>/projects/<name>/knowledge/` and `<repo>/projects/<name>/invariants/` directories (empty, with `.gitkeep` files).
7. Create `.gitkeep` files in all empty directories to ensure git tracks them.
8. Initialize `<repo>` as a git repo: `git init && git add -A && git commit -m "Initial team repo from botminter generator (profile: <profile>)"`
9. Print success message: `Team repo created at <repo> (profile: <profile>)`

**Overlay rules (step 4):**
- Files from the profile root (PROCESS.md, CLAUDE.md) go to `<repo>/` root
- `knowledge/` contents merge into `<repo>/knowledge/`
- `invariants/` contents merge into `<repo>/invariants/`
- `members/` is NOT copied to repo root — it stays only in `.team-template/`

#### 3.3 Acceptance Criteria

| # | Given | When | Then |
|---|-------|------|------|
| AC-2.1 | Generator repo with skeletons/ | `just init --repo=/tmp/test-team --profile=rh-scrum` | `/tmp/test-team/` is created |
| AC-2.2 | AC-2.1 | Inspect target | `/tmp/test-team/Justfile` exists (from skeleton) |
| AC-2.3 | AC-2.1 | Inspect target | `/tmp/test-team/PROCESS.md` exists (from profile) |
| AC-2.4 | AC-2.1 | Inspect target | `/tmp/test-team/CLAUDE.md` exists (from profile) |
| AC-2.5 | AC-2.1 | Inspect target | `/tmp/test-team/.github-sim/{issues,milestones,pulls}/` exist (from skeleton) |
| AC-2.6 | AC-2.1 | Inspect target | `/tmp/test-team/knowledge/` contains profile knowledge files |
| AC-2.7 | AC-2.1 | Inspect target | `/tmp/test-team/invariants/` contains profile invariant files |
| AC-2.8 | AC-2.1 | Inspect target | `/tmp/test-team/.team-template/` contains full skeletons/ content |
| AC-2.9 | AC-2.1 | Inspect target | `/tmp/test-team/.team-template/profiles/rh-scrum/members/human-assistant/` exists |
| AC-2.10 | AC-2.1 | `cd /tmp/test-team && git log` | Repo has initial commit |
| AC-2.11 | `/tmp/test-team/` exists | `just init --repo=/tmp/test-team --profile=rh-scrum` | Command fails with "already exists" error |
| AC-2.12 | Generator repo | `just init --repo=/tmp/test-team --profile=nonexistent` | Command fails with "Profile not found" error |
| AC-2.13 | Generator repo | `just init --repo=/tmp/test-team2 --profile=rh-scrum project=hypershift` | `/tmp/test-team2/projects/hypershift/knowledge/` and `projects/hypershift/invariants/` exist |
| AC-2.14 | AC-2.1 | Inspect target | `members/` directory does NOT exist at repo root (only in .team-template/) |
| AC-2.15 | AC-2.1 | Inspect target | Team repo is self-contained — no references back to generator repo |

#### 3.4 Edge Cases

- **Path with spaces:** `just init --repo="/tmp/my team"` should work (quote handling in Justfile)
- **Absolute vs relative path:** Both should work. `--repo=./test-team` and `--repo=/tmp/test-team`
- **Missing `just` binary:** Out of scope — assume `just` is installed
- **Profile with no knowledge/ or invariants/:** Skeleton dirs still created from team-repo skeleton; just empty

---

## 4. Step 3 — RH Scrum Profile Content

### Summary

Create the RH Scrum profile with process documents, team knowledge, and invariants that define how a Red Hat scrum team operates.

### Specification

#### 4.1 PROCESS.md (`skeletons/profiles/rh-scrum/PROCESS.md`)

Must contain:

1. **Issue frontmatter format** — YAML frontmatter fields:
   - `number` (integer)
   - `title` (string)
   - `state` (open | closed)
   - `labels` (array of strings)
   - `assignee` (string | null)
   - `milestone` (string | null)
   - `parent` (integer | null)
   - `created` (ISO 8601 UTC timestamp)

2. **Kind labels** — `kind/epic`, `kind/story`

3. **Status label convention** — `status/<role>:<phase>` naming pattern. Note that specific statuses are defined incrementally (M2 adds epic statuses, M3 adds story statuses). M1 defines only the naming convention.

4. **Comment format** — `### @<role> — <ISO-8601-timestamp>` followed by comment text

5. **Milestone format** — YAML frontmatter with `title`, `state`, `issues` (array of issue numbers)

6. **Pull request format** — YAML frontmatter with `number`, `title`, `state`, `branch`, `base`, `labels`, `author`. Reviews section with `### @<role> — <timestamp>` and `**Status: approved|changes-requested**`

7. **Communication protocols** — how members coordinate via `.github-sim/`:
   - Status transitions: member directly updates label on issue, commits + pushes via submodule
   - Comments: member adds comment on issue to record work output
   - PRs: for team evolution (knowledge, invariants, process changes), NOT for code

8. **Process evolution** — two paths:
   - Formal: PR in `.github-sim/pulls/`
   - Informal: `human.interact`, PO edits directly

#### 4.2 CLAUDE.md (`skeletons/profiles/rh-scrum/CLAUDE.md`)

Must contain:

1. **Team repo description** — this is the control plane; files are the coordination fabric
2. **Workspace model** — each member runs in its own workspace repo with team repo as submodule
3. **Coordination model** — pull-based; members watch for issues with their status labels
4. **File-based workflow** — `.github-sim/` mirrors GitHub's data model
5. **Knowledge resolution order** — team → project → member → member+project → runtime memories
6. **Invariant scoping** — same recursive pattern as knowledge
7. **Reference to PROCESS.md** for format conventions
8. **Team repo access paths** via submodule:
   - `.github-sim/` → `team-repo/.github-sim/`
   - Team knowledge → `team-repo/knowledge/`
   - Team invariants → `team-repo/invariants/`
   - Project knowledge → `team-repo/projects/<project>/knowledge/`
   - PROCESS.md → `team-repo/PROCESS.md`

#### 4.3 Team Knowledge Files (`skeletons/profiles/rh-scrum/knowledge/`)

| File | Content |
|------|---------|
| `commit-convention.md` | RH engineering commit message format. Stub: conventional commits, reference issue number, one logical change per commit. Mark as placeholder. |
| `pr-standards.md` | RH PR conventions. Stub: description template, review expectations, merge criteria. Mark as placeholder. |
| `communication-protocols.md` | How team members coordinate. Stub: status label transitions, issue comments, escalation paths. Mark as placeholder. |

#### 4.4 Team Invariant Files (`skeletons/profiles/rh-scrum/invariants/`)

| File | Content |
|------|---------|
| `code-review-required.md` | Prompt-based rule: all code changes require peer review before merge. Mark as placeholder. |
| `test-coverage.md` | Prompt-based rule: all stories must have test coverage. Mark as placeholder. |

Each invariant file is a short prompt-based rule (not an executable script). Format:

```markdown
# <Invariant Name>

## Rule
<One-sentence rule statement>

## Applies To
<Which roles/actions this invariant applies to>

## Verification
<How to check compliance>

---
*Placeholder — to be populated with detailed content before the team goes live.*
```

#### 4.5 Acceptance Criteria

| # | Given | When | Then |
|---|-------|------|------|
| AC-3.1 | Profile directory | Read PROCESS.md | Contains all format conventions from design Section 4.8 (issue, milestone, PR frontmatter) |
| AC-3.2 | Profile directory | Read PROCESS.md | Contains `kind/*` label definitions |
| AC-3.3 | Profile directory | Read PROCESS.md | Contains `status/<role>:<phase>` naming convention |
| AC-3.4 | Profile directory | Read PROCESS.md | Contains comment format with `@role — timestamp` |
| AC-3.5 | Profile directory | Read PROCESS.md | Contains communication protocols section |
| AC-3.6 | Profile directory | Read PROCESS.md | Contains process evolution section (formal + informal paths) |
| AC-3.7 | Profile directory | Read CLAUDE.md | Provides sufficient orientation for any team member's first read |
| AC-3.8 | Profile directory | Read CLAUDE.md | References PROCESS.md, does not duplicate it |
| AC-3.9 | Profile directory | List knowledge/ | Contains commit-convention.md, pr-standards.md, communication-protocols.md |
| AC-3.10 | Profile directory | List invariants/ | Contains code-review-required.md, test-coverage.md |
| AC-3.11 | All placeholder files | Read any placeholder | Clearly marked as placeholder with "to be populated" language |

---

## 5. Step 4 — human-assistant Member Skeleton

### Summary

Create the human-assistant member skeleton within the RH Scrum profile. The human-assistant (human-assistant) is the first team member.

### Specification

#### 5.1 Directory Structure

```
skeletons/profiles/rh-scrum/members/human-assistant/
├── ralph.yml
├── PROMPT.md
├── CLAUDE.md
├── knowledge/              # Empty dir with .gitkeep
├── invariants/
│   └── always-confirm.md
└── projects/               # Empty dir with .gitkeep
```

#### 5.2 ralph.yml (`skeletons/profiles/rh-scrum/members/human-assistant/ralph.yml`)

```yaml
event_loop:
  prompt_file: PROMPT.md
  completion_promise: LOOP_COMPLETE
  max_iterations: 10000
  max_runtime_seconds: 86400
  cooldown_delay_seconds: 60
  starting_event: board.scan
  persistent: true

cli:
  backend: claude

hats:
  board_scanner:
    name: Board Scanner
    description: Scans .github-sim/issues/ for status changes, builds board state, reports to human.
    triggers:
      - board.scan
      - board.rescan
    publishes:
      - board.report
      - board.act
      - board.idle
      - board.rescan
    default_publishes: board.rescan
    instructions: |
      ## Board Scanner

      You are the human-assistant's board scanner. Your job is to scan the board, report state to the human, and stay alive.

      ### Every cycle, do exactly this:

      1. Append a START line to `poll-log.txt`:
         ```
         <ISO-8601-UTC> — board.scan — START
         ```

      2. Read all issues in `team-repo/.github-sim/issues/` (via submodule path).
         - Parse each issue's YAML frontmatter for state, labels, assignee.
         - Build a board state summary: open issues grouped by status label.

      3. Append a result line to `poll-log.txt`:
         - If issues found: `<ISO-8601-UTC> — board.scan — <N> open issues found`
         - If no issues: `<ISO-8601-UTC> — board.scan — no work found`

      4. Append an END line to `poll-log.txt`:
         ```
         <ISO-8601-UTC> — board.scan — END (going idle)
         ```

      5. If the board has changes since the last scan, publish `board.report`.
         If the board is empty or unchanged, publish `board.rescan` to continue polling.

      ### Training Mode Report Format

      When reporting to the human (board.report → human.interact):

      ```
      Board scan complete.
      [Current board state summary]
      Next: [What you expect will happen next and why]
      Confirm, or provide guidance?
      ```

      ### Rules
      - NEVER publish LOOP_COMPLETE — stay alive indefinitely.
      - ALWAYS append all three log lines (START/result/END) to poll-log.txt before publishing any event.
      - Use `ralph emit` to publish events.
      - Use `ralph tools interact progress` for non-blocking status updates.

tasks:
  enabled: true

memories:
  enabled: true
  inject: auto
  budget: 2000

skills:
  enabled: true

RObot:
  enabled: true
  timeout_seconds: 600
  checkin_interval_seconds: 300
```

**Key configuration choices:**
- `persistent: true` — the core mechanism for staying alive (validated in Step 1)
- `cooldown_delay_seconds: 60` — 1 minute between cycles to avoid burning API tokens
- `max_iterations: 10000` — effectively unlimited for a day's operation
- `max_runtime_seconds: 86400` — 24 hours
- `starting_event: board.scan` — kicks off the first scan cycle
- Single hat: `board_scanner` — M1 only has one hat; additional hats (epic_creator, prioritizer, design_gate) added in M2
- `default_publishes: board.rescan` — if the hat forgets to emit, it loops back
- RObot enabled with 10-minute timeout and 5-minute checkin interval

#### 5.3 PROMPT.md (`skeletons/profiles/rh-scrum/members/human-assistant/PROMPT.md`)

Must contain:

1. **Role identity** — "You are the human-assistant for an agentic scrum team."
2. **Current mode** — "You are in TRAINING MODE. You observe and report. You do not act autonomously."
3. **Board scanner instructions** — reference hat instructions in ralph.yml, but add context:
   - Board location: `team-repo/.github-sim/issues/` (via submodule)
   - Board is expected to be empty in M1
   - Report format: training mode (current state, what happened, what's expected next)
4. **Poll logging** — append timestamped START/result/END triplets to `poll-log.txt`
5. **Human interaction protocol** — training mode:
   - Report every scan cycle result
   - Wait for human confirmation or guidance before acting
   - Use `ralph tools interact progress` for non-blocking updates
   - Use `human.interact` event for blocking questions
6. **Team context references**:
   - Team CLAUDE.md: `team-repo/CLAUDE.md`
   - Process: `team-repo/PROCESS.md`
   - Team knowledge: `team-repo/knowledge/`
   - Project knowledge: `team-repo/projects/hypershift/knowledge/` (if exists)
7. **Constraints**:
   - Never publish LOOP_COMPLETE
   - Never act without human confirmation
   - Always log to poll-log.txt

#### 5.4 CLAUDE.md (`skeletons/profiles/rh-scrum/members/human-assistant/CLAUDE.md`)

Must contain:

1. **Reference to shared context** — "Read `team-repo/CLAUDE.md` for team-wide context."
2. **Role-specific context**:
   - You are the human-assistant, the human's interface to the team
   - Training mode: observe, report, confirm before acting
   - Board scanner: your primary hat in M1
3. **Knowledge resolution** — where to find knowledge:
   - Team: `team-repo/knowledge/`
   - Project: `team-repo/projects/<project>/knowledge/`
   - Member: `./knowledge/` (workspace root)
4. **Invariant compliance** — check `./invariants/always-confirm.md`

#### 5.5 always-confirm.md (`skeletons/profiles/rh-scrum/members/human-assistant/invariants/always-confirm.md`)

```markdown
# Always Confirm

## Rule
The human-assistant MUST confirm with the human before taking any action that modifies team state (issues, labels, assignments, process changes).

## Applies To
All human-assistant actions in training mode. All human-gated actions in supervised/autonomous mode.

## Verification
Every action is preceded by a human.interact confirmation exchange. No state-changing action occurs without explicit human approval.
```

#### 5.6 Acceptance Criteria

| # | Given | When | Then |
|---|-------|------|------|
| AC-4.1 | Profile members/human-assistant/ | Parse ralph.yml | Valid YAML with `persistent: true`, `starting_event: board.scan`, single `board_scanner` hat |
| AC-4.2 | Profile members/human-assistant/ | Parse ralph.yml | `board_scanner` triggers on `[board.scan, board.rescan]` |
| AC-4.3 | Profile members/human-assistant/ | Parse ralph.yml | `board_scanner` publishes include `[board.report, board.act, board.idle, board.rescan]` |
| AC-4.4 | Profile members/human-assistant/ | Parse ralph.yml | RObot section has `enabled: true`, `timeout_seconds`, `checkin_interval_seconds` |
| AC-4.5 | Profile members/human-assistant/ | Read PROMPT.md | Contains training mode protocol, poll-log.txt logging instructions, team context references |
| AC-4.6 | Profile members/human-assistant/ | Read CLAUDE.md | References `team-repo/CLAUDE.md` for shared context |
| AC-4.7 | Profile members/human-assistant/ | Read invariants/always-confirm.md | Contains rule, applies-to, and verification sections |
| AC-4.8 | Profile members/human-assistant/ | Check ralph.yml against ralph-orchestrator | Config is compatible — all fields are recognized by ralph-orchestrator at `/opt/workspace/ralph-orchestrator/` |
| AC-4.9 | After `just init --profile=rh-scrum` | Inspect `.team-template/` | `.team-template/profiles/rh-scrum/members/human-assistant/` contains all human-assistant skeleton files |

---

## 6. Steps 5–7 — Team Repo Justfile Recipes

### Summary

Implement `add-member`, `create-workspace`, and `launch` recipes in the team repo skeleton Justfile. These recipes are baked into every generated team repo.

### Specification

#### 6.1 `just add-member <role>` (Step 5)

**Signature:**
```
just add-member <role>
```

**Behavior:**
1. Check `.team-template/profiles/*/members/<role>/` exists. If not, exit with error: `Error: No member skeleton found for role '<role>'. Available roles: <list>`
2. Check `team/<role>/` does NOT exist. If it does, exit with error: `Error: Member '<role>' already exists at team/<role>/. Remove it first to re-add.`
3. Copy `.team-template/profiles/*/members/<role>/` to `team/<role>/`
4. Commit: `git add team/<role>/ && git commit -m "Add team member: <role>"`
5. Print success: `Added team member '<role>' at team/<role>/`

**Profile discovery:** The recipe finds the profile by globbing `.team-template/profiles/*/members/<role>/`. Since `just init` copies all profiles into `.team-template/`, and each generated repo was created from a single profile, there should be exactly one match. If multiple matches (unlikely), use the first.

#### 6.2 `just create-workspace <member>` (Step 6)

**Signature:**
```
just create-workspace <member>
```

**Behavior:**
1. Check `team/<member>/` exists. If not, exit with error: `Error: Member '<member>' not found. Run 'just add-member <member>' first.`
2. Determine workspace path: `../workspace-<member>/` (sibling of team repo)
3. If workspace does NOT exist (fresh setup):
   a. Create `../workspace-<member>/`
   b. Initialize as git repo: `git init`
   c. Add team repo as submodule: `git submodule add <team-repo-path> team-repo`
   d. Surface files: copy all files from `team-repo/team/<member>/` to workspace root (ralph.yml, PROMPT.md, CLAUDE.md, knowledge/, invariants/, projects/)
   e. Commit: `git add -A && git commit -m "Initial workspace for <member>"`
4. If workspace EXISTS (incremental sync):
   a. Update submodule: `cd team-repo && git pull && cd ..`
   b. Re-surface files: copy all files from `team-repo/team/<member>/` to workspace root, overwriting existing
   c. Commit if changes: `git add -A && git diff --cached --quiet || git commit -m "Sync workspace for <member>"`
5. Print success: `Workspace ready at ../workspace-<member>/`

**File surfacing rules:**
- Copy, not symlink
- `ralph.yml` → workspace root
- `PROMPT.md` → workspace root
- `CLAUDE.md` → workspace root
- `knowledge/` → workspace root (merge)
- `invariants/` → workspace root (merge)
- `projects/` → workspace root (merge, if exists)
- `.ralph/` is NOT copied (runtime-only, workspace-local)

#### 6.3 `just launch <member>` (Step 7)

**Signature:**
```
just launch <member> [--dry-run]
```

**Behavior:**
1. If `../workspace-<member>/` does not exist, run `just create-workspace <member>` first
2. If it exists, sync: run `just create-workspace <member>` (idempotent sync)
3. If `--dry-run` flag is set:
   - Print workspace state (files surfaced, submodule status)
   - Print: `Dry run complete. Would run: cd ../workspace-<member> && ralph run -p PROMPT.md`
   - Exit without starting Ralph
4. If NOT dry-run:
   - `cd ../workspace-<member> && ralph run -p PROMPT.md`
   - Ralph starts in the workspace directory

#### 6.4 Acceptance Criteria

| # | Given | When | Then |
|---|-------|------|------|
| **add-member** | | | |
| AC-5.1 | Generated team repo | `just add-member human-assistant` | `team/human-assistant/` created with ralph.yml, PROMPT.md, CLAUDE.md, knowledge/, invariants/ |
| AC-5.2 | AC-5.1 | `just add-member human-assistant` (again) | Fails with "already exists" error |
| AC-5.3 | Generated team repo | `just add-member nonexistent` | Fails with "No member skeleton found" error, lists available roles |
| AC-5.4 | AC-5.1 | `git log` | Commit exists for adding the member |
| AC-5.5 | AC-5.1 | Compare `team/human-assistant/ralph.yml` with skeleton | Content matches the profile member skeleton |
| **create-workspace** | | | |
| AC-6.1 | Team repo with PO added | `just create-workspace human-assistant` | `../workspace-human-assistant/` created |
| AC-6.2 | AC-6.1 | Inspect workspace | `../workspace-human-assistant/team-repo/` is a submodule pointing to team repo |
| AC-6.3 | AC-6.1 | Inspect workspace | `../workspace-human-assistant/ralph.yml` matches `team/human-assistant/ralph.yml` |
| AC-6.4 | AC-6.1 | Inspect workspace | `../workspace-human-assistant/PROMPT.md` matches `team/human-assistant/PROMPT.md` |
| AC-6.5 | AC-6.1 | Inspect workspace | `../workspace-human-assistant/CLAUDE.md` matches `team/human-assistant/CLAUDE.md` |
| AC-6.6 | AC-6.1 | Inspect workspace | `../workspace-human-assistant/invariants/always-confirm.md` exists |
| AC-6.7 | AC-6.1 | `just create-workspace human-assistant` (again) | Performs sync without error, does not re-clone |
| AC-6.8 | No PO added to team | `just create-workspace human-assistant` | Fails with "Member not found" error |
| **launch** | | | |
| AC-7.1 | Team repo with PO added, no workspace | `just launch human-assistant --dry-run` | Creates workspace, prints state, does NOT start Ralph |
| AC-7.2 | Team repo with PO added | `just launch human-assistant` | Workspace created (if needed) and Ralph starts |
| AC-7.3 | Workspace exists | `just launch human-assistant` | Syncs workspace and Ralph starts |
| AC-7.4 | AC-7.2 | Check Ralph process | Ralph loaded PROMPT.md and entered board_scanner hat |

#### 6.5 Edge Cases

- **Team repo not a git repo:** `create-workspace` requires the team repo to be a git repo (for submodule). `just init` handles this.
- **Workspace path collision:** If `../workspace-human-assistant/` exists but is not a valid workspace (no submodule), behavior is undefined. Document but don't handle in M1.
- **Relative submodule path:** The submodule must use a relative path so workspaces are portable.

---

## 7. Step 8 — Integration Test

### Summary

Run the full pipeline from generator to running human-assistant, adding HyperShift project knowledge.

### Specification

#### 7.1 Full Sequence

```bash
# 1. From generator repo — stamp out team repo
cd /opt/workspace/botminter
just init --repo=/tmp/hypershift-team --profile=rh-scrum project=hypershift

# 2. From team repo — add HyperShift project knowledge (stubs)
cd /tmp/hypershift-team
# Create stub files in projects/hypershift/knowledge/:
#   - hcp-architecture.md (placeholder)
#   - nodepool-patterns.md (placeholder)
#   - upgrade-flow.md (placeholder)
# Create stub files in projects/hypershift/invariants/:
#   - pre-commit.md (placeholder)
#   - upgrade-path-tests.md (placeholder)
git add -A && git commit -m "Add HyperShift project knowledge stubs"

# 3. Add human-assistant and launch
just add-member human-assistant
just launch human-assistant
# In another terminal: tail -f ../workspace-human-assistant/poll-log.txt
```

#### 7.2 HyperShift Project Knowledge (Stubs)

These are placeholder files that will be populated with real content before the team goes live. Each file follows this format:

```markdown
# <Title>

*Placeholder — to be populated with HyperShift-specific content from SME research before the team goes live.*

## Summary
<One-sentence description of what this knowledge covers>

## Key Points
- TBD
```

| File | Summary |
|------|---------|
| `projects/hypershift/knowledge/hcp-architecture.md` | HyperShift Hosted Control Plane architecture overview |
| `projects/hypershift/knowledge/nodepool-patterns.md` | NodePool reconciliation patterns and lifecycle |
| `projects/hypershift/knowledge/upgrade-flow.md` | Control plane and NodePool upgrade flow |
| `projects/hypershift/invariants/pre-commit.md` | Pre-commit checks for HyperShift contributions |
| `projects/hypershift/invariants/upgrade-path-tests.md` | Upgrade path test requirements |

#### 7.3 Acceptance Criteria

| # | Given | When | Then |
|---|-------|------|------|
| AC-8.1 | Clean environment | Full pipeline runs | No manual intervention required (except launching) |
| AC-8.2 | Generated team repo | Inspect structure | Has Justfile (skeleton), PROCESS.md + CLAUDE.md (profile), projects/hypershift/ (instance-specific) |
| AC-8.3 | Generated team repo | Inspect layers | Profile content (PROCESS.md, CLAUDE.md, knowledge/, invariants/) is from rh-scrum profile |
| AC-8.4 | Generated team repo | Inspect layers | Project knowledge (projects/hypershift/) was added directly to instance |
| AC-8.5 | human-assistant workspace | Inspect | Submodule exists, files surfaced, ralph.yml present |
| AC-8.6 | Ralph started | 1+ cycle | `poll-log.txt` has at least one START/result/END triplet |
| AC-8.7 | Ralph started | 5+ minutes | `poll-log.txt` entries span 5+ minutes |
| AC-8.8 | Ralph started | 5+ minutes | Ralph process is still alive (PID check) |
| AC-8.9 | Ralph started | Inspect output | Scanner reports "no work found" / empty board (M1 board is empty) |
| AC-8.10 | Ralph started | Check for errors | No errors in Ralph's output |

---

## 8. Step 9 — HIL Round-Trip via Telegram

### Summary

Configure RObot and validate the full human-in-the-loop path: human-assistant sends training-mode message → human receives on Telegram → human responds → human-assistant processes response.

**Automation boundary:** This step has both automated and manual parts. Ralph (the implementing agent) handles bot onboarding, preflight testing, human-assistant launch, and verification. The human participates by sending the initial message to the bot (during onboarding) and responding to the human-assistant's training-mode message on Telegram.

### Specification

#### 8.1 Prerequisites

- `RALPH_TELEGRAM_BOT_TOKEN` environment variable MUST be set before launching the implementing Ralph session. If not set, Ralph MUST abort Step 9 with: `Error: RALPH_TELEGRAM_BOT_TOKEN not set. Set this env var and re-run.`
- The human has already created a Telegram bot via @BotFather (the token comes from there)
- The human knows their bot's Telegram username (to find it in Telegram during onboarding)

#### 8.2 Phase A — Bot Onboarding (automated + human action)

Ralph performs bot onboarding in the human-assistant's workspace directory:

1. Ensure the integration test workspace from Step 8 exists at `/tmp/hypershift-team/`. If not, re-run the Step 8 pipeline first.
2. Sync the workspace: `just create-workspace human-assistant` (from `/tmp/hypershift-team/`)
3. In the workspace directory (`/tmp/workspace-human-assistant/`), run:
   ```bash
   ralph bot onboard --telegram --token "$RALPH_TELEGRAM_BOT_TOKEN"
   ```
4. Ralph will start listening for an incoming message. **The human must now open Telegram, find their bot, and send it any message** (e.g., "hello"). This establishes the `chat_id`.
5. Ralph detects the message, saves the `chat_id` to `.ralph/telegram-state.json`, and completes onboarding.

**Timeout:** `ralph bot onboard` waits up to 120 seconds for the human's message. If no message arrives, onboarding fails. Ralph should retry once, then abort with a clear message asking the human to send a message to the bot.

#### 8.3 Phase B — Preflight Smoke Test (automated + human confirms)

After onboarding completes:

1. Run `ralph bot test "Hello from human-assistant — preflight test"` from the workspace directory
2. Verify the command exits successfully (exit code 0)
3. **The human confirms they received the message on Telegram.** Ralph cannot verify this automatically — it proceeds on the assumption that a successful `ralph bot test` means delivery worked. If the bot test command succeeds, move to Phase C.

#### 8.4 Phase C — human-assistant Launch with RObot (automated)

1. Ensure RObot is enabled in the workspace's `ralph.yml`. The human-assistant skeleton already has:
   ```yaml
   RObot:
     enabled: true
     timeout_seconds: 600
     checkin_interval_seconds: 300
   ```
   Verify this section exists and `enabled: true`. If RObot was disabled during Step 8 integration testing, re-enable it.
2. Launch the human-assistant in the background:
   ```bash
   cd /tmp/workspace-human-assistant && CLAUDECODE= ralph run -p PROMPT.md &
   ```
   Store the PID. **CRITICAL:** Do NOT kill all ralph processes — kill ONLY this PID.
3. The human-assistant enters `board_scanner`, scans the empty board, and sends a training-mode message to the human via Telegram:
   ```
   Board scan complete. No open issues found in .github-sim/issues/.
   Next: Continue monitoring. Board is empty — no action needed.
   Confirm, or provide guidance?
   ```

#### 8.5 Phase D — Human Responds on Telegram (manual)

4. **The human receives the training-mode message on Telegram and responds** (e.g., "Acknowledged, no work yet."). Ralph cannot perform this step — it waits and then verifies the outcome.

#### 8.6 Phase E — Verification (automated)

5. After allowing sufficient time for the human to respond (wait at least 2 minutes, or check events file for a `human.response` event):
   - Check `poll-log.txt` for continued START/result/END triplets after the HIL interaction
   - Check the events file (`.ralph/events-*.jsonl`) for `human.interact` and `human.response` events
   - Verify the human-assistant process is still alive (PID check)
6. Kill the human-assistant process by PID when verification is complete.

#### 8.7 RObot Configuration

Already specified in human-assistant's ralph.yml (Section 5.2):
```yaml
RObot:
  enabled: true
  timeout_seconds: 600
  checkin_interval_seconds: 300
```

The `RALPH_TELEGRAM_BOT_TOKEN` env var provides the bot token at runtime. The ralph.yml in the skeleton does NOT hardcode the token. The `chat_id` is discovered during onboarding (Phase A) and stored in `.ralph/telegram-state.json`.

#### 8.8 Acceptance Criteria

| # | Given | When | Then |
|---|-------|------|------|
| AC-9.1 | `RALPH_TELEGRAM_BOT_TOKEN` set | `ralph bot onboard --telegram` runs in workspace | Onboarding completes after human sends message to bot; `.ralph/telegram-state.json` contains `chat_id` |
| AC-9.2 | Bot onboarded | `ralph bot test "Hello"` runs | Command exits 0 (message delivered) |
| AC-9.3 | RObot configured, bot onboarded | human-assistant starts via `CLAUDECODE= ralph run -p PROMPT.md` | RObot connects to Telegram successfully |
| AC-9.4 | human-assistant running | human-assistant scans empty board | Training-mode message appears in Telegram chat (verified by human) |
| AC-9.5 | Message received | Human responds via Telegram | `human.response` event appears in events file |
| AC-9.6 | Response received | human-assistant processes response | human-assistant incorporates response into next scan cycle; poll-log.txt shows continued cycles |
| AC-9.7 | Full round-trip | Check poll-log.txt | Cycles continue throughout the HIL interaction |
| AC-9.8 | human-assistant running | Human does NOT respond within timeout_seconds | human-assistant does not hang — continues scanning after timeout |

#### 8.9 Edge Cases

- **No RALPH_TELEGRAM_BOT_TOKEN:** Ralph MUST abort Step 9 immediately with a clear error. Do not attempt onboarding without the token.
- **Human doesn't send message during onboarding:** `ralph bot onboard` times out after 120s. Ralph retries once, then aborts with instructions for the human.
- **Bot test succeeds but human doesn't see message:** Possible if bot is blocked or wrong chat. Ralph cannot detect this — proceeds to Phase C. If human-assistant's training-mode message also doesn't arrive, the human will not respond, triggering the timeout path (AC-9.8).
- **Network failure during Telegram call:** Ralph should log the error and continue scanning (not crash).
- **Human responds after timeout:** Response is ignored for the current interaction; human-assistant has already moved on.

---

## 9. Non-Functional Requirements

### 9.1 Performance

- Poll cycle interval: ≥30 seconds (cooldown_delay_seconds in ralph.yml) to avoid API token waste
- human-assistant must sustain polling for 24+ hours without degradation
- Workspace creation (create-workspace) should complete in under 30 seconds

### 9.2 Reliability

- Ralph must not exit on empty board (persistent polling)
- `just init` is idempotent-safe (fails on existing target, never corrupts)
- `just create-workspace` is idempotent (sync on existing workspace)
- All Justfile recipes fail with clear error messages on invalid input

### 9.3 Security

- Telegram bot token is NEVER stored in files committed to git
- Bot token is provided via environment variable only
- No secrets in ralph.yml, PROMPT.md, or any skeleton file

### 9.4 Portability

- Generated team repos are self-contained — no references back to generator repo
- Workspace submodule uses relative path
- All paths in Justfile recipes are relative (no hardcoded absolute paths except user-provided --repo)

---

## 10. Out of Scope

The following are explicitly NOT part of M1:

| Item | Reason | When |
|------|--------|------|
| Outer loop coordination | Only one team member in M1 | M2 |
| `.github-sim/issues/` populated with real issues | Board is empty in M1 | M2 |
| Specific kanban statuses (beyond naming convention) | Defined incrementally | M2 (epic), M3 (story) |
| Code work on HyperShift project repo | M1 is structure only | M3 |
| Additional human-assistant hats (epic_creator, prioritizer, design_gate) | Only board_scanner in M1 | M2 |
| Architect, dev, QE, reviewer members | Only human-assistant in M1 | M2/M3 |
| Eval/confidence system | Deferred | M4 |
| Real GitHub integration | File-based only | M5 |
| Automated launcher (Go binary) | Manual launch via Justfile | M5 |
| Sprint/retro loop | Out for POC | Deferred |
| Concurrent multi-member operation | Sequential only | M2+ |

---

## 11. Implementation Order

Steps MUST be executed in this order due to dependencies:

```
Step 1: Validate persistent polling
    ↓ (confirms event loop design)
Step 2: Generator skeleton + just init
    ↓ (creates generator structure)
Step 3: RH Scrum profile content
    ↓ (populates profile)
Step 4: human-assistant member skeleton
    ↓ (creates human-assistant config)
Step 5-7: Team repo Justfile recipes (add-member, create-workspace, launch)
    ↓ (enables team operation)
Step 8: Integration test (full pipeline)
    ↓ (validates everything works together)
Step 9: HIL via Telegram (requires running human-assistant)
```

Step 1 is the critical validation. If persistent polling fails, the event loop design for ALL subsequent steps must be revised before proceeding.

---

## 12. Glossary

| Term | Definition |
|------|------------|
| **Generator repo** | `botminter` — stamps out team repos via `just init` |
| **Team repo** | A generated instance — the control plane for a team |
| **Workspace** | A member's runtime environment, sibling to team repo |
| **Profile** | A reusable team methodology (e.g., `rh-scrum`) |
| **Skeleton** | The process-agnostic directory structure |
| **human-assistant** | human-assistant — the first team member |
| **Hat** | A specialized persona within a Ralph instance |
| **HIL** | Human-in-the-loop — human approval/guidance |
| **RObot** | Ralph's Telegram bot integration |
| **Training mode** | human-assistant observes and reports; never acts without human confirmation |
| **Surfacing** | Copying files from team repo member dir to workspace root |
| **`.team-template/`** | Copy of generator skeletons baked into each team repo for self-contained operation |
