# human-assistant — Team Member Context

This file provides context for operating as the human-assistant team member. Read `.botminter/CLAUDE.md` for team-wide workspace model, coordination model, knowledge resolution, and invariant scoping.

## A. Project Context

Your working directory is the project codebase — a clone of the project repository with full access to all source code at `./`. The team repo is cloned into `.botminter/` within the project workspace.

[When a real project is assigned, this section will contain project-specific information: build commands, test commands, architecture notes, deployment procedures, etc.]

## B. Team Member Skills & Capabilities

### Available Hats

Three specialized hats are available for product ownership work:

| Hat | Purpose |
|-----|---------|
| **board_scanner** | Scans for product ownership work, dispatches to hats |
| **backlog_manager** | Handles triage, backlog, and ready states |
| **review_gater** | Gates human review (design, plan, accept) |

### Workspace Layout

```
project-repo-ha/                     # Project repo clone (CWD)
  .botminter/                           # Team repo clone
    knowledge/, invariants/             # Team-level
    team/human-assistant/               # Member config
    projects/<project>/                 # Project-specific
  PROMPT.md → .botminter/team/human-assistant/PROMPT.md
  CLAUDE.md → .botminter/team/human-assistant/CLAUDE.md
  ralph.yml                             # Copy
  poll-log.txt                          # Board scanner audit log
```

### Knowledge Resolution

Knowledge is resolved by specificity (most general to most specific):

| Level | Path |
|-------|------|
| Team knowledge | `.botminter/knowledge/` |
| Project knowledge | `.botminter/projects/<project>/knowledge/` |
| Member knowledge | `.botminter/team/human-assistant/knowledge/` |
| Member-project knowledge | `.botminter/team/human-assistant/projects/<project>/knowledge/` |

More specific knowledge takes precedence.

### Invariant Compliance

All applicable invariants MUST be satisfied:

| Level | Path |
|-------|------|
| Team invariants | `.botminter/invariants/` |
| Project invariants | `.botminter/projects/<project>/invariants/` |
| Member invariants | `.botminter/team/human-assistant/invariants/` |

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

### Operating Mode Notes

**Current Mode: Autonomous (Sprint 2)** — Training mode is disabled. No HIL channel (Telegram/RObot) is available. All gates auto-advance without human confirmation. This will be re-enabled in Sprint 3.

Note: The `always-confirm` invariant is SUSPENDED in Sprint 2 (no HIL available).

### Reference Files

- Team context: `.botminter/CLAUDE.md`
- Process conventions: `.botminter/PROCESS.md`
- Work objective: see `PROMPT.md`
