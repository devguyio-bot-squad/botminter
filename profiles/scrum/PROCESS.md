# Scrum Process

This document defines the conventions used by the agentic scrum team. All team members follow these formats when creating and updating issues, milestones, PRs, and comments on GitHub. All GitHub operations go through the `github-project` skill.

---

## Issue Format

Issues are GitHub issues on the **team repo** (not the project repo). The `github-project` skill auto-detects the team repo from `team/`'s git remote.

### Fields

| Field | GitHub Mapping | Description |
|-------|---------------|-------------|
| `title` | Issue title | Concise, descriptive issue title |
| `state` | Issue state | `open` or `closed` |
| `type` | Native issue type | Epic, Task (story), Bug |
| `assignee` | Issue assignee | GitHub username or unassigned |
| `milestone` | Issue milestone | Milestone name or none |
| `parent` | Native sub-issue relationship | Links stories to their parent epic |
| `body` | Issue body | Description, acceptance criteria, and context (markdown) |

Issues are created via the `github-project` skill (create-issue operation). See the skill for exact commands.

---

## Issue Types

Issue classification uses GitHub's native issue types:

| Issue Type | Kind | Description |
|------------|------|-------------|
| **Epic** | `epic` | A large body of work spanning multiple stories |
| **Task** | `story` | A single deliverable unit of work (sub-issue of an Epic) |
| **Bug** | `bug` | A bug requiring investigation, planning, and fix |

Stories are linked to epics as native sub-issues.

Every issue MUST have exactly one issue type set.

---

## Project Status Convention

Status is tracked via a single-select "Status" field on the team's GitHub Project board (v2), NOT via labels. Status values follow the naming pattern:

```
<role>:<phase>
```

- `<role>` — the team member role responsible (e.g., `po`, `arch`, `dev`, `qe`)
- `<phase>` — the current phase within that role's workflow

Examples:
- `po:triage` — PO is triaging the issue
- `dev:in-progress` — developer is working on the issue
- `qe:review` — QE is reviewing the issue

Specific statuses are defined incrementally per milestone. M1 defines only the naming convention. M2 adds epic statuses, M3 adds story statuses.

**Transition rule:** Only the role named in the status may transition it. The PO may override any status.

---

## Epic Statuses (M2)

The epic lifecycle statuses, with the role responsible at each stage:

| Status | Role | Description |
|--------|------|-------------|
| `po:triage` | human-assistant | New epic, awaiting evaluation |
| `po:backlog` | human-assistant | Accepted, prioritized, awaiting activation |
| `arch:design` | architect | Architect producing design doc |
| `po:design-review` | human-assistant | Design doc awaiting human review |
| `arch:plan` | architect | Architect proposing story breakdown (plan) |
| `po:plan-review` | human-assistant | Story breakdown plan awaiting human review |
| `arch:breakdown` | architect | Architect creating story issues |
| `po:ready` | human-assistant | Stories created, epic parked in ready backlog. Human decides when to activate. |
| `arch:in-progress` | architect | Architect monitoring story execution (M2: fast-forward to `po:accept`) |
| `po:accept` | human-assistant | Epic awaiting human acceptance |
| `done` | — | Epic complete |

### Rejection Loops

At any review gate, the human can reject and send the epic back:
- `po:design-review` → `arch:design` (with feedback comment)
- `po:plan-review` → `arch:plan` (with feedback comment)
- `po:accept` → `arch:in-progress` (with feedback comment)

The feedback comment uses the standard comment format and includes the human's specific concerns.

### Story Statuses (M2 Placeholder)

| Status | Description |
|--------|-------------|
| `dev:ready` | Deliberate M3 placeholder. Stories sit idle until M3 brings dev/qe agents. |

### Bug Statuses

Bugs use GitHub's native Bug issue type and follow a two-track workflow:

**Simple Track** (QE fixes directly, arch reviews):
`bug:investigate` → `arch:review` → `qe:verify` → `done`

**Complex Track** (QE plans, arch refines, PO approves, subtask breakdown):
`bug:investigate` → `arch:refine` → `po:plan-review` → `bug:breakdown` → `bug:in-progress` → `qe:verify` → `done`

| Status | Owner | Description | Next (happy path) |
|--------|-------|-------------|-------------------|
| `bug:investigate` | QE | Reproduce, root cause, determine simple vs complex | `arch:review` (simple) or `arch:refine` (complex) |
| `arch:review` | Arch | Review simple bug fix, can approve or escalate to complex | `qe:verify` (approve) or `arch:refine` (escalate) |
| `arch:refine` | Arch | Review/amend complex bug plan | `po:plan-review` |
| `po:plan-review` | PO (human) | Approve or reject complex bug plan | `bug:breakdown` (approve) or `arch:refine` (reject) |
| `bug:breakdown` | Arch | Create GitHub native subtask issues | `bug:in-progress` |
| `bug:in-progress` | Monitor | Track subtask completion | `qe:verify` (all done) |
| `qe:verify` | QE | Verify fix (shared with story workflow) | `done` |

Subtasks created during `bug:breakdown` are Task-type sub-issues that flow through the story workflow. When all subtasks reach `done`, the bug monitor advances the parent bug to `qe:verify` for final verification.

### Error Status

| Status | Description |
|--------|-------------|
| `error` | Issue failed processing 3 times. Board scanner skips it. Human investigates and resets the status to retry. |

---

## Comment Format

Comments are GitHub issue comments, added via `gh issue comment`. Each comment uses this format:

```markdown
### <emoji> <role> — <ISO-8601-UTC-timestamp>

Comment text here. May contain markdown formatting, code blocks, etc.
```

The `<emoji>` and `<role>` are read from the member's `.botminter.yml` file at runtime by the `github-project` skill. Since all agents share one `GH_TOKEN` (one GitHub user), the role attribution in the comment body is the primary way to identify which hat/role wrote it.

### Standard Emoji Mapping

| Role | Emoji | Example Header |
|------|-------|----------------|
| po | 📝 | `### 📝 po — 2026-01-15T10:30:00Z` |
| architect | 🏗️ | `### 🏗️ architect — 2026-01-15T10:30:00Z` |
| dev | 💻 | `### 💻 dev — 2026-01-15T10:30:00Z` |
| qe | 🧪 | `### 🧪 qe — 2026-01-15T10:30:00Z` |
| sre | 🛠️ | `### 🛠️ sre — 2026-01-15T10:30:00Z` |
| cw | ✍️ | `### ✍️ cw — 2026-01-15T10:30:00Z` |
| lead | 👑 | `### 👑 lead — 2026-01-15T10:30:00Z` |

Example:

```markdown
### 📝 po — 2026-01-15T10:30:00Z

Triaged. This is a high-priority story for the current milestone. Assigning to dev.
```

Comments are append-only. Never edit or delete existing comments.

---

## Milestone Format

Milestones are GitHub milestones on the team repo, managed via the `github-project` skill.

**Fields:**

| Field | GitHub Mapping | Description |
|-------|---------------|-------------|
| `title` | Milestone title | Milestone name (e.g., `M1: Structure + human-assistant`) |
| `state` | Milestone state | `open` or `closed` |
| `description` | Milestone description | Goals and scope of the milestone |
| `due_on` | Milestone due date | Optional ISO 8601 date |

Issues are assigned to milestones via `gh issue edit --milestone "<title>"`. The `github-project` skill provides commands for creating, listing, and managing milestones.

---

## Pull Request Format

Pull requests are real GitHub PRs on the team repo. PRs are used for team evolution (knowledge, invariants, process changes), NOT for code changes.

**Fields:**

| Field | GitHub Mapping | Description |
|-------|---------------|-------------|
| `title` | PR title | Descriptive title of the change |
| `state` | PR state | `open`, `merged`, or `closed` |
| `base` | Base branch | Target branch (usually `main`) |
| `head` | Head branch | Feature branch |
| `labels` | PR labels | e.g., `kind/process-change` |
| `body` | PR body | Description of the change (markdown) |

### Reviews

Reviews use GitHub's native review system via `gh pr review`:

- `gh pr review <number> --approve` — approve the PR
- `gh pr review <number> --request-changes` — request changes

Review comments follow the standard comment format with an explicit status:

```markdown
### <emoji> <role> — <ISO-8601-UTC-timestamp>

**Status: approved**

Review comments here.
```

Valid review statuses: `approved`, `changes-requested`.

---

## Communication Protocols

Team members coordinate through GitHub issues and PRs on the team repo using the `github-project` skill:

### Status Transitions

A member transitions an issue's status by:
1. Using `gh project item-edit` to update the Status field on the project board
2. Adding an attribution comment documenting the transition

Other members detect the transition on their next board scan (querying the project board via `gh project item-list`).

### Comments

A member records work output by:
1. Adding a GitHub issue comment via `gh issue comment` using the standard comment format

### Pull Requests

PRs on the team repo are for team-level changes:
- Process changes (PROCESS.md updates)
- Knowledge additions or updates
- Invariant modifications

PRs are NOT used for code changes to the project repo. Code changes go through the project's own review process.

### Coordination Model

The team uses a pull-based coordination model:
- Each member scans the project board for issues with status values relevant to their role
- No central dispatcher — members pull work based on their role's status values
- The PO is the only role that can assign work and override statuses

---

## Process Evolution

The team process can evolve through two paths:

### Formal Path

1. Create a PR on the team repo proposing the change
2. Affected team members review and comment
3. PO approves and merges

### Informal Path

1. Human interacts with the PO via `human.interact`
2. PO edits the process file directly
3. Commit the change to the team repo

The informal path is appropriate for urgent corrections or clarifications. The formal path is preferred for significant process changes that affect multiple team members.

### Team Agreements

All significant process changes, role changes, and team decisions MUST be recorded as team agreements before the change is applied. Agreements provide traceability for why changes were made and who participated in the decision.

- **Decisions** go in `agreements/decisions/` — role changes, process changes, tool adoption
- **Retrospective outcomes** go in `agreements/retros/` — summaries from retrospective sessions
- **Working norms** go in `agreements/norms/` — living team agreements (e.g., "we prefer small PRs")

See `knowledge/team-agreements.md` for the full convention including file format and lifecycle.
