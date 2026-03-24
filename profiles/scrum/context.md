# Scrum Team Context

## What This Repo Is

This is a **team repo** — the control plane for an agentic scrum team. Files are the coordination fabric. Every team member reads from and writes to this repo to coordinate work.

The team repo is NOT a code repo. It defines the team's structure, process, knowledge, and work items. Code work happens in separate project repos.

## Workspace Model (M2)

Each team member runs in a **project repo clone** with the team repo cloned into `team/` inside it. The agent's CWD is the project codebase — agents have direct access to source code at `./`.

```
parent-directory/
  my-team/                    # Team repo (human operates here)
  my-project-ha/                      # human-assistant workspace (project repo clone)
    team/                       # Team repo clone
    PROMPT.md → team/members/human-assistant/PROMPT.md
    context.md → team/members/human-assistant/context.md
<!-- +agent:claude-code -->
    .claude/                          # Assembled from coding-agent/ layers
<!-- -agent -->
    ralph.yml                         # Copy
  my-project-arch/                    # architect workspace (project repo clone)
    team/                       # Team repo clone
    PROMPT.md → team/members/architect/PROMPT.md
    context.md → team/members/architect/context.md
<!-- +agent:claude-code -->
    .claude/                          # Assembled from coding-agent/ layers
<!-- -agent -->
    ralph.yml                         # Copy
```

Pulling `team/` updates all team configuration. Copies (ralph.yml, settings.local.json) require `just sync`.

## Coordination Model

The team uses **pull-based coordination**:
- Each member scans the board (GitHub issues on the team repo) for issues with status labels matching their role
- Board scanning and all issue operations use the `github-project` skill (wraps `gh` CLI)
- No central dispatcher — coordination is emergent from shared process conventions
- The human-assistant is the human's interface to the team and the only role that can assign work

## GitHub-Native Workflow

Work items, milestones, and PRs live on the team repo's GitHub:

| Resource | Access Method | Tool |
|----------|--------------|------|
| Issues (epics + stories) | `gh issue list/view/create/edit` | `github-project` skill |
| Milestones | `gh api` (milestones endpoint) | `github-project` skill |
| Pull requests | `gh pr create/view/merge` | `github-project` skill |

See `PROCESS.md` for label conventions, status transitions, and comment format.

## Knowledge Resolution Order

Knowledge is resolved in order of specificity. All levels are additive:

1. **Team knowledge** — `team/knowledge/` (applies to all members)
2. **Project knowledge** — `team/projects/<project>/knowledge/` (project-specific)
3. **Member knowledge** — `team/members/<member>/knowledge/` (role-specific)
4. **Member+project knowledge** — `team/members/<member>/projects/<project>/knowledge/` (role+project-specific)
5. **Hat knowledge** — `team/members/<member>/hats/<hat>/knowledge/` (hat-specific)

## Invariant Scoping

Invariants follow the same recursive pattern as knowledge. All applicable invariants MUST be satisfied — they are additive.

1. **Team invariants** — `team/invariants/` (apply to all members)
2. **Project invariants** — `team/projects/<project>/invariants/` (apply to project work)
3. **Member invariants** — `team/members/<member>/invariants/` (role-specific)

## Agent Capabilities (`coding-agent/` directory)

Skills, sub-agents, and settings are scoped across multiple levels using a `coding-agent/` directory that mirrors the knowledge/invariant scoping model. All layers live inside `team/`.

| Level | Location | Naming Convention |
|-------|----------|-------------------|
| Team | `team/coding-agent/{skills,agents}/` | `{item-name}` (e.g., `gh`) |
| Project | `team/projects/<project>/coding-agent/{skills,agents}/` | `{project}.{item-name}` (e.g., `my-project.codebase-search`) |
| Member | `team/members/<member>/coding-agent/{skills,agents}/` | `{member}.{item-name}` (e.g., `architect.design-template`) |

**Skills** — Ralph reads them directly from source directories via `skills.dirs` in ralph.yml. No merging needed.

<!-- +agent:claude-code -->
**Agents** — symlinked into `.claude/agents/` at workspace creation. All agent files from team, project, and member scopes are merged into one directory via symlinks.

**Settings** — `.claude/settings.json` is copied from the team's `coding-agent/settings.json` (shared hooks like PostToolUse). `.claude/settings.local.json` is copied from the member's `coding-agent/settings.local.json` if it exists.
<!-- -agent -->

## Propagation Model

| What changes | How it reaches agents |
|---|---|
| Knowledge, invariants, PROCESS.md, team context.md | Auto — agents pull `team/` every scan, read directly |
| Member PROMPT.md, context.md | Auto — workspace files are symlinks into `team/` |
<!-- +agent:claude-code -->
| Skills, agents (all levels) | Auto — read via `team/` paths (skills.dirs) or symlinks (.claude/agents/) |
<!-- -agent -->
| ralph.yml | **Manual** — requires `just sync` + agent restart |
| settings.json (team hooks) | Auto — copied from `coding-agent/settings.json` on every sync |
| settings.local.json | **Manual** — requires `just sync` (re-copy) |

## Team Repo Access Paths

From a workspace, access team repo content through `team/` and the `github-project` skill:

| Content | Access Method |
|---------|--------------|
| Board (issues) | `gh issue list --repo "$TEAM_REPO"` (via `github-project` skill) |
| Milestones | `gh api` milestones endpoint (via `github-project` skill) |
| Pull requests | `gh pr list --repo "$TEAM_REPO"` (via `github-project` skill) |
| Team knowledge | `team/knowledge/` |
| Team invariants | `team/invariants/` |
| Project knowledge | `team/projects/<project>/knowledge/` |
| Project invariants | `team/projects/<project>/invariants/` |
| Process conventions | `team/PROCESS.md` |
| Team context | `team/context.md` |

The team repo (`$TEAM_REPO`) is auto-detected from `team/`'s git remote.

## Reference

- Process conventions and label scheme: see `PROCESS.md`
- Member-specific context: see each member's `context.md` in their workspace
