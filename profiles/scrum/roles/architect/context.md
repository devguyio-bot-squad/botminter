# Architect — Team Member Context

This file provides context for operating as the architect team member. Read `team/CLAUDE.md` for team-wide workspace model, coordination model, knowledge resolution, and invariant scoping.

## A. Project Context

Your working directory is the project codebase — a clone of the agentic-team fork with full access to all source code at `./`. The team repo is cloned into `team/` within the project workspace.

Fork chain:
- `example-org/example-project` (upstream)
- `my-org/my-project` (human's fork)
- `my-org-agents/my-project` (agentic-team fork — your CWD)

[When a real project is assigned, this section will contain project-specific information: build commands, test commands, architecture notes, deployment procedures, etc.]

## B. Team Member Skills & Capabilities

### Available Hats

Four specialized hats are available for architecture work. Board scanning is handled by an auto-inject skill, not a hat.

| Hat | Purpose |
|-----|---------|
| **designer** | Produces design docs |
| **planner** | Decomposes designs into story breakdowns |
| **breakdown_executor** | Creates story issues from approved breakdowns |
| **epic_monitor** | Monitors epic progress (fast-forward to acceptance) |

### Workspace Layout

```
project-repo-architect/              # Project repo clone (CWD)
  team/                           # Team repo clone
    knowledge/, invariants/             # Team-level
    members/{{member_dir}}/                   # Member config
    projects/<project>/                 # Project-specific
  PROMPT.md → team/members/{{member_dir}}/PROMPT.md
  context.md → team/members/{{member_dir}}/context.md
  ralph.yml                             # Copy
```

### Knowledge Resolution

Knowledge is resolved by specificity (most general to most specific):

| Level | Path |
|-------|------|
| Team knowledge | `team/knowledge/` |
| Project knowledge | `team/projects/<project>/knowledge/` |
| Member knowledge | `team/members/{{member_dir}}/knowledge/` |
| Member-project knowledge | `team/members/{{member_dir}}/projects/<project>/knowledge/` |
| Hat knowledge (designer) | `team/members/{{member_dir}}/hats/designer/knowledge/` |
| Hat knowledge (planner) | `team/members/{{member_dir}}/hats/planner/knowledge/` |
| Hat knowledge (breakdown_executor) | `team/members/{{member_dir}}/hats/breakdown_executor/knowledge/` |
| Hat knowledge (epic_monitor) | `team/members/{{member_dir}}/hats/epic_monitor/knowledge/` |

More specific knowledge takes precedence.

### Invariant Compliance

All applicable invariants MUST be satisfied:

| Level | Path |
|-------|------|
| Team invariants | `team/invariants/` |
| Project invariants | `team/projects/<project>/invariants/` |
| Member invariants | `team/members/{{member_dir}}/invariants/` |

Critical member invariant: `team/members/{{member_dir}}/invariants/design-quality.md` — every design must include required sections.

### Coordination Conventions

See `team/PROCESS.md` for:
- Issue format and label conventions
- Status transition patterns
- Comment attribution format (emoji headers with ISO timestamps)
- Milestone and PR conventions

### GitHub Access

All GitHub operations use the `github-project` skill:
- Issue queries and mutations
- Project board operations
- Pull request operations
- Milestone management

The team repo is auto-detected from `team/`'s git remote.

### Reference Files

- Team context: `team/CLAUDE.md`
- Process conventions: `team/PROCESS.md`
- Work objective: see `PROMPT.md`
