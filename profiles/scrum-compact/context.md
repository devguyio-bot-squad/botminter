# Compact Team Context

## What This Repo Is

This is a **team repo** — the control plane for a compact single-member agentic team. Files are the coordination fabric. The single agent ("superman") reads from and writes to this repo to track work.

The team repo is NOT a code repo. It defines the team's structure, process, knowledge, and work items. Code work happens in separate project repos.

## Single-Member Model

The compact profile has one member — `superman` — who wears all hats (PO, team lead, architect, dev, QE, SRE, content writer). The agent self-transitions through the entire issue lifecycle by switching hats.

## Workspace Model

The member runs in a **project repo clone** with the team repo cloned into `team/` inside it. The agent's CWD is the project codebase — direct access to source code at `./`.

```
project-repo-superman/               # Project repo clone (agent CWD)
  team/                           # Team repo clone
    knowledge/, invariants/             # Team-level
    members/superman/                    # Member config
    projects/<project>/                 # Project-specific
  PROMPT.md → team/members/superman/PROMPT.md
  context.md → team/members/superman/context.md
  ralph.yml                             # Copy
  poll-log.txt                          # Board scan audit log
```

Pulling `team/` updates all team configuration. Copies (ralph.yml, settings.local.json) require `just sync`.

## Coordination Model

The compact profile uses **self-transition coordination**:
- The single member scans the project board for all status values
- Board scanning and all issue operations use the `gh` skill (wraps `gh` CLI)
- The board-scanner skill (auto-injected into the coordinator) dispatches to the appropriate hat based on priority
- No concurrent agents, no coordination overhead

## GitHub-Native Workflow

Work items, milestones, and PRs live on the team repo's GitHub:

| Resource | Access Method | Tool |
|----------|--------------|------|
| Issues (epics + stories) | `gh issue list/view/create/edit` | `gh` skill |
| Milestones | `gh api` (milestones endpoint) | `gh` skill |
| Pull requests | `gh pr create/view/merge` | `gh` skill |

See `PROCESS.md` for label conventions, status transitions, and comment format.

## Knowledge Resolution Order

Knowledge is resolved in order of specificity. All levels are additive:

1. **Team knowledge** — `team/knowledge/` (applies to all hats)
2. **Project knowledge** — `team/projects/<project>/knowledge/` (project-specific)
3. **Member knowledge** — `team/members/superman/knowledge/` (member-specific)
4. **Member+project knowledge** — `team/members/superman/projects/<project>/knowledge/` (member+project-specific)
5. **Hat knowledge** — `team/members/superman/hats/<hat>/knowledge/` (hat-specific)

## Invariant Scoping

Invariants follow the same recursive pattern as knowledge. All applicable invariants MUST be satisfied — they are additive.

1. **Team invariants** — `team/invariants/` (apply to all hats)
2. **Project invariants** — `team/projects/<project>/invariants/` (apply to project work)
3. **Member invariants** — `team/members/superman/invariants/` (member-specific)

## Agent Capabilities (`coding-agent/` directory)

Skills, sub-agents, and settings are scoped across multiple levels using a `coding-agent/` directory that mirrors the knowledge/invariant scoping model. All layers live inside `team/`.

| Level | Location | Naming Convention |
|-------|----------|-------------------|
| Team | `team/coding-agent/{skills,agents}/` | `{item-name}` (e.g., `gh`) |
| Project | `team/projects/<project>/coding-agent/{skills,agents}/` | `{project}.{item-name}` |
| Member | `team/members/superman/coding-agent/{skills,agents}/` | `superman.{item-name}` |

**Skills** — Ralph reads them directly from source directories via `skills.dirs` in ralph.yml. No merging needed.

<!-- +agent:claude-code -->
**Agents** — symlinked into `.claude/agents/` at workspace creation. All agent files from team, project, and member scopes are merged into one directory via symlinks.

**Settings** — `.claude/settings.local.json` is copied from the member's `coding-agent/settings.local.json` if it exists.
<!-- -agent -->

## Propagation Model

| What changes | How it reaches the agent |
|---|---|
| Knowledge, invariants, PROCESS.md, team context.md | Auto — agent pulls `team/` every scan, reads directly |
| Member PROMPT.md, context.md | Auto — workspace files are symlinks into `team/` |
<!-- +agent:claude-code -->
| Skills, agents (all levels) | Auto — read via `team/` paths (skills.dirs) or symlinks (.claude/agents/) |
<!-- -agent -->
| ralph.yml | **Manual** — requires `just sync` + agent restart |
| settings.local.json | **Manual** — requires `just sync` (re-copy) |

## Team Repo Access Paths

From a workspace, access team repo content through `team/` and the `gh` skill:

| Content | Access Method |
|---------|--------------|
| Board (issues) | `gh issue list --repo "$TEAM_REPO"` (via `gh` skill) |
| Milestones | `gh api` milestones endpoint (via `gh` skill) |
| Pull requests | `gh pr list --repo "$TEAM_REPO"` (via `gh` skill) |
| Team knowledge | `team/knowledge/` |
| Team invariants | `team/invariants/` |
| Project knowledge | `team/projects/<project>/knowledge/` |
| Project invariants | `team/projects/<project>/invariants/` |
| Process conventions | `team/PROCESS.md` |
| Team context | `team/context.md` |

The team repo (`$TEAM_REPO`) is auto-detected from `team/`'s git remote.

## Brain Feedback

You may receive messages marked "Brain Feedback" injected after tool calls.
These come from your team member's brain — the consciousness that monitors
your work, receives human messages, and manages the board.

When you receive brain feedback:
1. It takes priority over your current subtask
2. Acknowledge by adjusting your approach
3. If feedback conflicts with your current task, comply with the feedback

## Reference

- Process conventions and label scheme: see `PROCESS.md`
- Member-specific context: see `team/members/superman/context.md`
