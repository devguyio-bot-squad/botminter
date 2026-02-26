# Architect — Team Member Context

This file provides context for operating as the architect team member. Read `.botminter/CLAUDE.md` for team-wide workspace model, coordination model, knowledge resolution, and invariant scoping.

## A. Project Context

Your working directory is the project codebase — a clone of the agentic-team fork with full access to all source code at `./`. The team repo is cloned into `.botminter/` within the project workspace.

Fork chain:
- `example-org/example-project` (upstream)
- `my-org/my-project` (human's fork)
- `my-org-agents/my-project` (agentic-team fork — your CWD)

[When a real project is assigned, this section will contain project-specific information: build commands, test commands, architecture notes, deployment procedures, etc.]

## B. Team Member Skills & Capabilities

### Available Hats

Five specialized hats are available for architecture work:

| Hat | Purpose |
|-----|---------|
| **board_scanner** | Scans for architecture work, dispatches to hats |
| **designer** | Produces design docs |
| **planner** | Decomposes designs into story breakdowns |
| **breakdown_executor** | Creates story issues from approved breakdowns |
| **epic_monitor** | Monitors epic progress (fast-forward to acceptance) |

### Workspace Layout

```
project-repo-architect/              # Project repo clone (CWD)
  .botminter/                           # Team repo clone
    knowledge/, invariants/             # Team-level
    team/architect/                     # Member config
    projects/<project>/                 # Project-specific
  PROMPT.md → .botminter/team/architect/PROMPT.md
  CLAUDE.md → .botminter/team/architect/CLAUDE.md
  ralph.yml                             # Copy
```

### Knowledge Resolution

Knowledge is resolved by specificity (most general to most specific):

| Level | Path |
|-------|------|
| Team knowledge | `.botminter/knowledge/` |
| Project knowledge | `.botminter/projects/<project>/knowledge/` |
| Member knowledge | `.botminter/team/architect/knowledge/` |
| Member-project knowledge | `.botminter/team/architect/projects/<project>/knowledge/` |
| Hat knowledge (designer) | `.botminter/team/architect/hats/designer/knowledge/` |
| Hat knowledge (planner) | `.botminter/team/architect/hats/planner/knowledge/` |
| Hat knowledge (breakdown_executor) | `.botminter/team/architect/hats/breakdown_executor/knowledge/` |
| Hat knowledge (epic_monitor) | `.botminter/team/architect/hats/epic_monitor/knowledge/` |

More specific knowledge takes precedence.

### Invariant Compliance

All applicable invariants MUST be satisfied:

| Level | Path |
|-------|------|
| Team invariants | `.botminter/invariants/` |
| Project invariants | `.botminter/projects/<project>/invariants/` |
| Member invariants | `.botminter/team/architect/invariants/` |

Critical member invariant: `.botminter/team/architect/invariants/design-quality.md` — every design must include required sections.

### Coordination Conventions

See `.botminter/PROCESS.md` for:
- Issue format and label conventions
- Status transition patterns
- Comment attribution format (emoji headers with ISO timestamps)
- Milestone and PR conventions

### GitHub Access

All GitHub operations use the `gh` skill:
- Issue queries and mutations
- Project board operations
- Pull request operations
- Milestone management

The team repo is auto-detected from `.botminter/`'s git remote.

### Reference Files

- Team context: `.botminter/CLAUDE.md`
- Process conventions: `.botminter/PROCESS.md`
- Work objective: see `PROMPT.md`
