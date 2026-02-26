# Superman — Team Member Context

This file provides context for operating as the superman team member. Read `.botminter/CLAUDE.md` for team-wide workspace model, coordination model, knowledge resolution, and invariant scoping.

## A. Project Context

Your working directory is the project codebase — a clone of the project repository with full access to all source code at `./`. The team repo is cloned into `.botminter/` within the project workspace.

[When a real project is assigned, this section will contain project-specific information: build commands, test commands, architecture notes, deployment procedures, etc.]

## B. Team Member Skills & Capabilities

### Available Hats

Fifteen specialized hats are available for different phases of work:

| Hat | Purpose |
|-----|---------|
| **board_scanner** | Scans for all work, dispatches to hats, handles auto-advance |
| **po_backlog** | Manages triage, backlog, and ready states |
| **po_reviewer** | Gates human review (design, plan, accept) |
| **lead_reviewer** | Reviews arch work before human gate |
| **arch_designer** | Produces design docs |
| **arch_planner** | Decomposes designs into story breakdowns |
| **arch_breakdown** | Creates story issues from approved breakdowns |
| **arch_monitor** | Monitors epic progress |
| **qe_test_designer** | Writes test plans and test stubs |
| **dev_implementer** | Implements stories, handles rejections |
| **dev_code_reviewer** | Reviews code quality |
| **qe_verifier** | Verifies against acceptance criteria |
| **sre_setup** | Sets up test infrastructure |
| **cw_writer** | Writes documentation |
| **cw_reviewer** | Reviews documentation |

### Workspace Layout

```
project-repo-superman/               # Project repo clone (CWD)
  .botminter/                           # Team repo clone
    knowledge/, invariants/             # Team-level
    team/superman/                      # Member config
    projects/<project>/                 # Project-specific
  PROMPT.md → .botminter/team/superman/PROMPT.md
  CLAUDE.md → .botminter/team/superman/CLAUDE.md
  ralph.yml                             # Copy
  poll-log.txt                          # Board scanner audit log
```

### Knowledge Resolution

Knowledge is resolved by specificity (most general to most specific):

| Level | Path |
|-------|------|
| Team knowledge | `.botminter/knowledge/` |
| Project knowledge | `.botminter/projects/<project>/knowledge/` |
| Member knowledge | `.botminter/team/superman/knowledge/` |
| Member-project knowledge | `.botminter/team/superman/projects/<project>/knowledge/` |
| Hat knowledge (various) | `.botminter/team/superman/hats/<hat>/knowledge/` |

More specific knowledge takes precedence.

### Invariant Compliance

All applicable invariants MUST be satisfied:

| Level | Path |
|-------|------|
| Team invariants | `.botminter/invariants/` |
| Project invariants | `.botminter/projects/<project>/invariants/` |
| Member invariants | `.botminter/team/superman/invariants/` |

Critical member invariant: `.botminter/team/superman/invariants/design-quality.md`

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

### Operating Mode

**Supervised mode (GitHub comment-based)** — human gates at three decision points:
- `po:design-review` — design doc approval
- `po:plan-review` — story breakdown approval
- `po:accept` — epic acceptance

At these gates, the system checks for human response comments containing approval or rejection. All other transitions auto-advance.

### Reference Files

- Team context: `.botminter/CLAUDE.md`
- Process conventions: `.botminter/PROCESS.md`
- Work objective: see `PROMPT.md`
