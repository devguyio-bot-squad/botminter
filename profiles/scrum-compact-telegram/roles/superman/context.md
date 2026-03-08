# Superman — Team Member Context

This file provides context for operating as the superman team member. Read `team/context.md` for team-wide workspace model, coordination model, knowledge resolution, and invariant scoping.

## A. Project Context

Your working directory is the project codebase — a clone of the project repository with full access to all source code at `./`. The team repo is cloned into `team/` within the project workspace.

[When a real project is assigned, this section will contain project-specific information: build commands, test commands, architecture notes, deployment procedures, etc.]

## B. Team Member Skills & Capabilities

### Available Hats

Fourteen specialized hats are available for different phases of work. Board scanning is handled by an auto-inject skill, not a hat.

| Hat | Purpose |
|-----|---------|
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
  team/                           # Team repo clone
    knowledge/, invariants/             # Team-level
    members/superman/                    # Member config
    projects/<project>/                 # Project-specific
  PROMPT.md → team/members/superman/PROMPT.md
  context.md → team/members/superman/context.md
  ralph.yml                             # Copy
  poll-log.txt                          # Board scan audit log
```

### Knowledge Resolution

Knowledge is resolved by specificity (most general to most specific):

| Level | Path |
|-------|------|
| Team knowledge | `team/knowledge/` |
| Project knowledge | `team/projects/<project>/knowledge/` |
| Member knowledge | `team/members/superman/knowledge/` |
| Member-project knowledge | `team/members/superman/projects/<project>/knowledge/` |
| Hat knowledge (various) | `team/members/superman/hats/<hat>/knowledge/` |

More specific knowledge takes precedence.

### Invariant Compliance

All applicable invariants MUST be satisfied:

| Level | Path |
|-------|------|
| Team invariants | `team/invariants/` |
| Project invariants | `team/projects/<project>/invariants/` |
| Member invariants | `team/members/superman/invariants/` |

Critical member invariant: `team/members/superman/invariants/design-quality.md`

### Coordination Conventions

See `team/PROCESS.md` for:
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

The team repo is auto-detected from `team/`'s git remote.

### Operating Mode

**Supervised mode (Telegram-based)** — human gates at three decision points:
- `po:design-review` — design doc approval
- `po:plan-review` — story breakdown approval
- `po:accept` — epic acceptance

At these gates, the system presents artifacts to the human via Telegram and waits for approval or rejection. All other transitions auto-advance.

### Reference Files

- Team context: `team/context.md`
- Process conventions: `team/PROCESS.md`
- Work objective: see `PROMPT.md`
