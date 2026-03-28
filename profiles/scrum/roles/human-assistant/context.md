# human-assistant — Team Member Context

This file provides context for operating as the human-assistant team member. Read `team/CLAUDE.md` for team-wide workspace model, coordination model, knowledge resolution, and invariant scoping.

## A. Project Context

Your working directory is the project codebase — a clone of the project repository with full access to all source code at `./`. The team repo is cloned into `team/` within the project workspace.

[When a real project is assigned, this section will contain project-specific information: build commands, test commands, architecture notes, deployment procedures, etc.]

## B. Team Member Skills & Capabilities

### Available Hats

Two specialized hats are available for product ownership work. Board scanning is handled by an auto-inject skill, not a hat.

| Hat | Purpose |
|-----|---------|
| **backlog_manager** | Handles triage, backlog, and ready states |
| **review_gater** | Gates human review (design, plan, accept) |

### Workspace Layout

```
project-repo-ha/                     # Project repo clone (CWD)
  team/                           # Team repo clone
    knowledge/, invariants/             # Team-level
    members/{{member_dir}}/             # Member config
    projects/<project>/                 # Project-specific
  PROMPT.md → team/members/{{member_dir}}/PROMPT.md
  context.md → team/members/{{member_dir}}/context.md
  ralph.yml                             # Copy
  poll-log.txt                          # Board scan audit log
```

### Knowledge Resolution

Knowledge is resolved by specificity (most general to most specific):

| Level | Path |
|-------|------|
| Team knowledge | `team/knowledge/` |
| Project knowledge | `team/projects/<project>/knowledge/` |
| Member knowledge | `team/members/{{member_dir}}/knowledge/` |
| Member-project knowledge | `team/members/{{member_dir}}/projects/<project>/knowledge/` |

More specific knowledge takes precedence.

### Invariant Compliance

All applicable invariants MUST be satisfied:

| Level | Path |
|-------|------|
| Team invariants | `team/invariants/` |
| Project invariants | `team/projects/<project>/invariants/` |
| Member invariants | `team/members/{{member_dir}}/invariants/` |

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

### Operating Mode Notes

**Current Mode: Autonomous (Sprint 2)** — Training mode is disabled. No HIL channel (Telegram/RObot) is available. All gates auto-advance without human confirmation. This will be re-enabled in Sprint 3.

Note: The `always-confirm` invariant is SUSPENDED in Sprint 2 (no HIL available).

### Reference Files

- Team context: `team/CLAUDE.md`
- Process conventions: `team/PROCESS.md`
- Work objective: see `PROMPT.md`
