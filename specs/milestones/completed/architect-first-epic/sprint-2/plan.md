# Sprint 2 Plan: Two Agents, Full Lifecycle — Autonomous Coordination

> Vertical slice: add remaining architect hats + evolve human-assistant to three-hat
> model. Both agents coordinate through the full epic lifecycle autonomously (review
> gates auto-advance without HIL).
>
> Prerequisite: Sprint 1 complete. Architect produces designs from `status/arch:design`.
> Design reference: [../design.md](../design.md)

## Sprint 2 Deviations from Design

The following are **intentional Sprint 2 scope decisions** that deviate from the design.
They are resolved in Sprint 3.

| Deviation | Rationale | Resolved in |
|-----------|-----------|-------------|
| Training mode: DISABLED (design says ENABLED) | No Telegram/RObot in Sprint 2, so no HIL channel for confirmations. Both agents act autonomously. | Sprint 3 |
| RObot: disabled (design says enabled) | No Telegram bots in Sprint 2. | Sprint 3 |
| `just launch` has no `--telegram-bot-token` (design says mandatory) | Telegram deferred. Design AC 7.9 deferred. | Sprint 3 |
| Review gates auto-advance (design requires HIL approval) | No Telegram means no HIL channel. All `po:*` review and backlog gates auto-advance with comments noting auto-advance. | Sprint 3 |
| Project name `hypershift` hardcoded in `skills.dirs` and hat instructions | Design review Finding 47 — no `project` field in epic frontmatter. Single-project assumption for M2. | Post-M2 |

## Checklist

- [x] Step 1: Remaining architect hats
- [x] Step 2: Human-assistant evolution
- [x] Step 3: Team-level skills
- [x] Step 4: Documentation — `docs/`
- [x] Step 5: Two-agent integration validation

---

## Step 1: Remaining Architect Hats

**Objective:** Add planner, breakdown_executor, and epic_monitor hats to the architect.
Update board_scanner to dispatch all four `arch.*` events.

**Implementation:**

Update `skeletons/profiles/rh-scrum/members/architect/ralph.yml`:

### Board scanner updates

**Event context propagation:** The board scanner passes the issue number to downstream
hats via the scratchpad. Before publishing an event, the board scanner writes the target
issue number and current status to `.ralph/agent/scratchpad.md`. Downstream hats read
the scratchpad to determine which issue to process. This is the same mechanism used by
the Sprint 1 designer hat (board scanner writes, designer reads).

- Add new publishes: `arch.plan`, `arch.breakdown`, `arch.in_progress`
  (Sprint 1 only published `arch.design`).
  Note: Ralph event names use underscores (`arch.in_progress`), status labels use
  hyphens (`status/arch:in-progress`). Keep these distinct.
- Add priority order: `arch:breakdown` > `arch:plan` > `arch:design` > `arch:in-progress`.

### Planner hat

Per design.md Section 4.1.1 (planner). Triggers on `arch.plan`, publishes `board.rescan`.

Workflow: read epic + linked design doc (follow the design doc link in the epic's
comments — the designer hat appends a comment with the path, typically
`.botminter/projects/<project>/knowledge/designs/epic-<number>.md`), consult knowledge
(team, project, member, member-project, hat/planner), acquire lock, **verify issue is
still at `status/arch:plan`** (if not, release lock and publish `board.rescan`),
decompose design into stories (title,
description, acceptance criteria, dependencies), append breakdown as comment, transition
`status/arch:plan` to `status/po:plan-review`, release lock.

Backpressure gates: each story independently deployable/testable, each has Given-When-Then
criteria, dependencies explicit, core e2e in early stories.

### Breakdown executor hat

Per design.md Section 4.1.1 (breakdown_executor). Triggers on `arch.breakdown`,
publishes `board.rescan`.

Workflow: read epic + approved breakdown comment, acquire lock on the epic, **verify
issue is still at `status/arch:breakdown`** (if not, release lock and publish
`board.rescan`), create story
issues in `.botminter/.github-sim/issues/` with next available numbers (scan for highest + 1),
each with `kind/story`, `status/dev:ready`, `parent` linking to epic, milestone from epic.
Append comment to epic listing created story numbers. Transition `status/arch:breakdown`
to `status/po:ready`. Commit all stories + epic transition in one commit, push, release lock.

**Known M2 limitation:** Story issues are created under the epic lock, not individual
per-issue locks. This means a concurrent `create-epic` invocation could race on issue
numbers. The single atomic commit minimizes this window. With only two agents in M2 and
`create-epic` being human-initiated, the probability is negligible. A centralized issue
counter (design review Finding 44) would eliminate this for M3+.

Backpressure gates: each story has Given-When-Then criteria, proper labels, epic comment
lists all story numbers.

### Epic monitor hat

Per design.md Section 4.1.1 (epic_monitor). Triggers on `arch.in_progress`,
publishes `board.rescan`.

M2 behavior: fast-forward only. Acquire lock, **verify issue is still at
`status/arch:in-progress`** (if not, release lock and publish `board.rescan`),
append comment "Epic monitor: no stories
in progress (M2 — dev/qe not yet available). Fast-forwarding to acceptance." Transition
`status/arch:in-progress` to `status/po:accept`. Release lock.

### Architect PROMPT.md + CLAUDE.md updates

Sprint 1 created the architect's PROMPT.md and CLAUDE.md with only board_scanner + designer
documented. Update both to reference all five hats and the updated board_scanner dispatch
model. The architect PROMPT.md should list the four `arch.*` events and their hat mappings.
The architect CLAUDE.md should document the full hat set and updated knowledge paths
(including `hats/planner/knowledge/`).

### Architect `core.guardrails` — no changes needed

The existing `core.guardrails` section (invariant compliance paths, lock-late principle)
applies equally to all hats. The planner and breakdown_executor produce artifacts
(story breakdowns, story issues) that should comply with project invariants — the existing
guardrail paths already cover this. No updates required.

### `cooldown_delay_seconds` — removal

Design review Finding 69 resolved that `cooldown_delay_seconds` should be removed per
design-principles.md Principle 5 ("Agent processing time provides natural throttling").
Sprint 1 included `cooldown_delay_seconds: 60` in the architect ralph.yml. Remove it
from the architect ralph.yml when adding the new hats. Do NOT include it in the HA
ralph.yml either.

**Test:**
- Seed epic at `status/arch:plan` with a design doc already linked.
- Launch architect. Verify planner fires, story breakdown comment appended, status
  transitions to `status/po:plan-review`.
- Manually advance to `status/arch:breakdown`. Verify story issues created with
  correct frontmatter, parent links, and Given-When-Then acceptance criteria.
- Manually advance to `status/arch:in-progress`. Verify fast-forward to `status/po:accept`.

**Integration:** Architect handles all four status phases. Ready for two-agent coordination.

---

## Step 2: Human-Assistant Evolution

**Objective:** Evolve the human-assistant from M1's single-hat observer to a three-hat
model. Sprint 2: review gates and backlog management auto-advance (no HIL).

**Implementation:**

### `ralph.yml`

Rewrite `skeletons/profiles/rh-scrum/members/human-assistant/ralph.yml`:

- **M1 to M2 event model transition:** Remove M1 events (`board.report`, `board.act`,
  `board.idle`). Replace with M2 events (`po.backlog`, `po.review`, `board.rescan`).
- `RObot: enabled: false` (Sprint 2 — no Telegram, see deviations table).
- **No `core.guardrails` section** — deliberate omission. The HA does not produce
  artifacts requiring invariant compliance. Lock discipline is encoded in each hat's
  instructions directly. Carry over `tasks`, `memories`, and `skills` sections from
  design.md Section 4.2.1 unchanged.
- Three hats per design.md Section 4.2.1:

**board_scanner** — triggers on `board.scan`/`board.rescan`, publishes `po.backlog`,
`po.review`, `LOOP_COMPLETE`.
- Self-clear, sync (`.botminter/`), agent startup self-cleanup (scan for own
  `human-assistant:*` locks on first cycle and delete them), stale lock cleanup
  (all stale locks regardless of role prefix, threshold read from
  `.botminter/PROCESS.md` Write-Lock Settings section, default 5 minutes),
  scan issues for `status/po:*`, poll-log, dispatch by status, idempotency check,
  priority order
  (`po:triage` > `po:design-review` > `po:plan-review` > `po:accept` > `po:backlog` > `po:ready`).
  Note: design review Finding 39 suggested swapping to prioritize unblocking in-progress
  work over triaging new work. Deferred — acceptable for M2 with one epic at a time.
- Failed processing escalation (3-strike `status/error`). Implemented in board_scanner
  instructions but tested only via the integration test (Step 5) — not unit-tested in
  Step 2. The design review dismissed dedicated escalation testing as non-critical for
  POC (Finding 21).

**backlog_manager** — triggers on `po.backlog`, publishes `board.rescan`.
- Handles `status/po:triage`: Sprint 2 treats this as a single atomic hop — acquire
  lock, transition directly from `po:triage` to `status/arch:design` (skipping intermediate
  `po:backlog` commit), append comment noting auto-advance through triage+backlog, release
  lock, push. The `po:backlog` state is only meaningful when HIL is active (Sprint 3).
- Handles `status/po:backlog`: auto-advance to `status/arch:design`.
- Handles `status/po:ready`: auto-advance to `status/arch:in-progress`.
- Sprint 2 instruction: "HIL is not available this sprint. Auto-advance all backlog gates.
  Acquire lock, transition status, append comment noting auto-advance, release lock, push."

**review_gater** — triggers on `po.review`, publishes `board.rescan`.
- Handles `status/po:design-review`: auto-advance to `status/arch:plan`.
- Handles `status/po:plan-review`: auto-advance to `status/arch:breakdown`.
- Handles `status/po:accept`: auto-advance to `status/done`, close the issue.
- Sprint 2 instruction: "HIL is not available this sprint. Auto-approve all reviews.
  Acquire lock, transition status, append approval comment noting auto-advance,
  release lock, push."

### `PROMPT.md`

Rewrite for three-hat model. Training mode: DISABLED. Reference `.botminter/` paths.
Document all three hats and their dispatch model. Note Sprint 2 auto-advance behavior.

### `CLAUDE.md`

Update to reflect three-hat model, `.botminter/` workspace model, updated knowledge
and invariant paths.

### PROCESS.md — Communication Protocols update

Sprint 1 deferred this (design review Finding 49): the "Communication Protocols >
Status Transitions" section in `skeletons/profiles/rh-scrum/PROCESS.md` still references
the M1 submodule model ("pushing via the submodule"). Update to reference the `.botminter/`
model: agents push from `.botminter/` (the team repo clone inside the project repo), not
via a submodule.

### `invariants/always-confirm.md`

Sprint 2: update to note that this invariant is suspended while training mode is disabled
and HIL is not available. It will be re-enabled in Sprint 3. Note: this is a
**team-level** invariant at `skeletons/profiles/rh-scrum/invariants/always-confirm.md`,
not a member-specific file.

Updated content should read:

```markdown
# Always Confirm (SUSPENDED — Sprint 2)

> **Status: SUSPENDED** — Training mode is DISABLED in Sprint 2. No HIL channel
> (Telegram/RObot) is available. All gates auto-advance without human confirmation.
> This invariant will be re-enabled in Sprint 3 when training mode and Telegram are added.

## Rule (when active)
Always confirm state-modifying actions with the human before executing them.
```

**Test:**
- Generate team repo, add human-assistant, create workspace.
- Seed epic at `status/po:triage`.
- Launch HA alone. Verify: backlog_manager auto-advances triage to backlog to arch:design.
- Seed epic at `status/po:design-review`. Verify: review_gater auto-advances to arch:plan.
- Seed epic at `status/po:plan-review`. Verify: review_gater auto-advances to arch:breakdown.
- Seed epic at `status/po:accept`. Verify: review_gater auto-advances to done, issue closed.
- Seed epic at `status/po:ready`. Verify: backlog_manager auto-advances to arch:in-progress.
- **Board scanner periodic stale lock cleanup (any role):** Create stale lock with
  `architect:` prefix (timestamp > 5 min ago). Verify: HA board_scanner cleans it up
  (HA cleans all stale locks regardless of role prefix on every scan cycle).
- **Agent startup self-cleanup (own role only):** Create stale lock with
  `human-assistant:` prefix (timestamp > 5 min ago). Start HA. Verify: HA cleans its
  own stale lock on startup before first scan (design Section 4.4.2).
- Verify PROCESS.md no longer references "submodule" in Communication Protocols section.

**Integration:** HA can auto-gate all lifecycle stages. Ready for two-agent coordination.

---

## Step 3: Team-Level Skills

**Objective:** Create `create-epic` and `board` skills available to all members.

**Implementation:**

### `create-epic` skill

Create `skeletons/profiles/rh-scrum/agent/skills/create-epic/SKILL.md`:

- **Parameters:** title (required), description (required), project (optional).
- **Behavior:** Scan `.botminter/.github-sim/issues/` for highest issue number.
  Create next-numbered issue file with YAML frontmatter: `kind/epic`,
  `status/po:triage`, `state: open`, ISO 8601 created timestamp. Body contains
  description. Append creation comment using standard PROCESS.md comment format:
  `### @<role> — <ISO-timestamp>` followed by "Created epic: <title>".
- **Write-lock:** Follow design Section 4.4.2 "Acquire for new issue/PR creation":
  scan for highest number + 1, acquire lock for that number using the invoking
  member's role as the lock ID prefix (e.g., `human-assistant:<loop_id>`), verify
  no issue file with that number was created between scan and lock acquisition.
  If the issue file already exists after lock acquisition (race), release lock,
  re-scan for new highest number, and retry. Commit and push atomically (lock +
  issue file in one commit to minimize race window — inspired by design review
  Finding 1's analysis, though that finding was dismissed at design scope).
- **Output:** Confirm issue number and path.

### `board` skill

Create `skeletons/profiles/rh-scrum/agent/skills/board/SKILL.md`:

- **Parameters:** none.
- **Behavior:** Run `just -f .botminter/Justfile sync` to ensure fresh state, then
  read all issues in `.botminter/.github-sim/issues/`. Parse YAML frontmatter.
  Present issues grouped by status label. Show epic-to-story relationships
  (via `parent` field). Include issue number, title, status, assignee.
  Read-only — no write-lock required.
- **Output:** Formatted board view.

**Test:**
- Verify skills are present in generated team repo's `agent/skills/`.
- Verify skills are discoverable via Ralph's `skills.dirs` in both agents' ralph.yml
  (the `.botminter/agent/skills/` path is in `skills.dirs`).
- Invoke `create-epic` from both member roles — verify issue file created with
  correct frontmatter and lock IDs use respective role prefixes
  (`human-assistant:*` vs `architect:*`).
- Verify no lock file remains after `create-epic` completes.
- Verify the commit log shows lock + issue creation in a single commit.
- Invoke `board` — verify formatted output with issue grouping.

**Integration:** Skills available to all members through workspace assembly. `create-epic`
provides a structured way to create epics (alternative to manual file creation).

---

## Step 4: Documentation — `docs/`

**Objective:** Update operator docs to cover two-agent coordination, the human-assistant
role, team-level skills, and the full epic lifecycle.

**Implementation:**

### Update `docs/getting-started.md`

- Add section: "Adding a second member" — `just add-member architect` after
  `just add-member human-assistant`
- Add section: "Running two agents" — create workspaces for both, launch both,
  observe lifecycle traversal
- Update launch instructions with Sprint 2 behavior (autonomous, no Telegram yet)

### Update `docs/epic-lifecycle.md`

- Add full lifecycle walkthrough: triage → done with both agents
- Document which agent handles which status transitions
- Add two-agent coordination diagram (HA auto-advances gates, architect produces artifacts)
- Document stale lock cleanup (HA board scanner responsibility)
- Document failed processing escalation (3-strike → `status/error`)

### New: `docs/member-roles.md` — Member Roles

- Architect: role description, hats (board_scanner, designer, planner,
  breakdown_executor, epic_monitor), what each hat produces
- Human-assistant: role description, hats (board_scanner, backlog_manager,
  review_gater), gate responsibilities
- How to create a new member role (skeleton structure, ralph.yml, PROMPT.md, CLAUDE.md)
- Knowledge and invariant scoping per member

### New: `docs/skills.md` — Team-Level Skills

- What skills are and how they work (Ralph `skills.dirs`, SKILL.md format)
- `create-epic` — usage, parameters, what it creates
- `board` — usage, output format
- Scoping: team, project, member-level skills
- How to add a new skill

**Test:** All new and updated pages render as valid markdown. Cross-reference with
actual skeleton files for accuracy.

**Integration:** Operators can now understand multi-agent coordination and available skills.

---

## Step 5: Two-Agent Integration Validation

**Objective:** Both agents running concurrently, epic traverses full lifecycle
from triage to done without human interaction.

**Implementation:**

### Integration test sequence

Adapted from design.md Section 8.2 for Sprint 2 (no HIL — all gates auto-advance):

1. Generate team repo: `just init --repo=/tmp/m2-s2-test --profile=rh-scrum project=hypershift`
2. Deploy synthetic fixtures (from Sprint 1 fixtures, updated to `status/po:triage`
   instead of Sprint 1's `status/arch:design`)
3. Add both members: `just add-member human-assistant && just add-member architect`
4. Create synthetic project repo (reuse Sprint 1's `/tmp/synth-hypershift` or create
   fresh). Create workspaces for both:
   `just create-workspace human-assistant /tmp/synth-hypershift`
   `just create-workspace architect /tmp/synth-hypershift`
   Note: `create-workspace` clones the team repo into `.botminter/` inside each workspace.
   Both workspaces' `.botminter/` origins point at the local team repo path. Pushes to
   non-bare local repos require `git config receive.denyCurrentBranch updateInstead` on
   the team repo — verify Sprint 1's `create-workspace` recipe handles this.
5. Verify both workspaces: symlinks, ralph.yml copies, `.claude/` assembly,
   `.botminter/.member` markers correct
6. Seed synthetic epic at `status/po:triage` in `.botminter/.github-sim/issues/1.md`
7. Launch both agents concurrently
8. Verify lifecycle traversal:
   - `po:triage` → `po:backlog` → `arch:design` (HA auto-advances)
   - `arch:design` → `po:design-review` (architect produces design)
   - `po:design-review` → `arch:plan` (HA auto-approves)
   - `arch:plan` → `po:plan-review` (architect proposes breakdown)
   - `po:plan-review` → `arch:breakdown` (HA auto-approves)
   - `arch:breakdown` → `po:ready` (architect creates stories)
   - `po:ready` → `arch:in-progress` (HA auto-advances)
   - `arch:in-progress` → `po:accept` (architect fast-forwards)
   - `po:accept` → `done` (HA auto-approves, closes issue)
9. Verify: story issues created with `kind/story`, `parent: 1`, `status/dev:ready`,
   and Given-When-Then acceptance criteria
10. Verify: design doc at `.botminter/projects/hypershift/knowledge/designs/epic-1.md`
    with knowledge markers from all scopes
11. Verify: epic issue `state: closed` in frontmatter (review_gater closes on `done`)
12. Verify: no lock collisions, no stale locks remaining
13. Verify: both agents' `poll-log.txt` files show clean scan cycles
14. Verify: no duplicate processing — check poll-log.txt for absence of duplicate
    dispatch entries for the same issue in the same scan cycle (idempotent dispatch)

### Additional tests

**Lock contention** (design.md Section 8.3):
- Manually create lock file for an issue. Start architect. Verify it skips the issue.
  Delete lock. Verify it picks up on next scan.

**Stale lock cleanup** (design.md Section 8.4):
- Create lock with old timestamp. Start HA. Verify cleanup.

**Push-conflict handling** (design.md Section 8.8):
- Create two epics at different statuses. Launch both agents. Verify both pushes
  succeed (one directly, one via pull-rebase-retry). Verify no data loss.

**Test:** All verification steps pass. Full lifecycle traversal completes without
manual intervention.

**Demo:** Two agents running simultaneously. Epic traverses triage to done. Stories
created. Design doc produced. All automated, zero human interaction.
