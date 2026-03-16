# Getting Started

Running one coding agent is easy. Running a team of them is challenging.

BotMinter is a CLI that brings conventions to running a team of coding agents. Your process, knowledge, and constraints live in a Git repo, and every agent picks them up automatically. Built for Claude Code today, with architecture to support Gemini CLI, Codex, and more.

## The problem BotMinter solves

Most tools in this space focus on how to run agents - spawning them, orchestrating multi-agent pipelines, managing lifecycle. BotMinter solves a different problem: how your agents should work, what they know, and how you stay in the loop.

Because when you run several agents across multiple projects, the gaps show up fast. Same conventions copied everywhere, changes applied one agent at a time, and the only way to see what your agents decided is to read through their terminal sessions.

- **Reuse**: How do you apply the same conventions to all your agents without copying them into every config?
- **Customization**: How do you keep shared defaults but override just one thing for a specific project or agent - without forking the entire config?
- **Propagation**: When you update a convention, how does it reach every agent - without you touching each one?
- **Visibility**: When your agents are working, how do you know what each one decided and why - without reading terminal logs?

BotMinter answers all four with a batteries-included approach: **profiles** - Git-backed convention packages you pick once and customize from there. Push a file to the repo at the right scope, every relevant agent picks it up. Agents coordinate through GitHub issues, so every decision is traceable on a board - not buried in a terminal session.

## Profiles

Like Helm for Kubernetes or Rails for web, a profile ships opinionated defaults for coding agents. It defines:

- **Roles & Process** - who does what, how work flows between them, what quality gates apply
- **Knowledge & Constraints** - four-level scoping system (team, project, member, member+project)
- **Communication** - how agents surface decisions to you for approval
- **Runtime & Workspace** - where agents execute, how directories are laid out

You pick a profile when you run `bm init`. The default bridge (Matrix via Tuwunel) gives your agents presence on a messaging platform out of the box. It stamps out a team repo you own and customize from there.

All profiles share the same knowledge scoping, constraint system, workspace layout, and local sandboxed runtime. They differ in two dimensions:

| | Roles | Communication |
|---|---|---|
| **`scrum-compact`** | Single agent - PO, architect, dev, QE | GitHub Issues + Matrix (default) |

> The `scrum` profile (multi-role teams with separate agents per role) is in development and will ship in a future release.

Everything is customizable after init - add roles, redefine pipeline phases, change gate criteria, or extend the workspace layout.

## Layered knowledge scoping

This is BotMinter's primary differentiator. Knowledge and constraints resolve at four levels - all additive:

```
team-wide              All your agents, all projects
  └─ project-wide      All your agents on this project
      └─ member-wide   This agent, all projects
          └─ member+project   This agent, this project
```

**Example:** You decide all your agents should use `pnpm`, never `npm`. You create `knowledge/use-pnpm.md` at the team level. Every agent on every project sees it on next launch. Later, your backend project has a specific database constraint - you add it at the project level. Only agents working on that project pick it up. No copy-pasting between agents. No repeating yourself.

Knowledge files are information agents should know. Invariants are constraints agents must not violate. Both follow the same scoping rules.

```
my-team/                                    # Team repo
  knowledge/                                # Team-wide - all agents see this
    use-pnpm.md
    no-raw-sql.md
  invariants/                               # Team-wide constraints
    pr-coverage-80.md
  projects/backend/
    knowledge/                              # Project-wide - only backend agents
      db-migration-rules.md
  members/dev-01/
    knowledge/                              # Member-wide - only dev-01
      azure-deploy-notes.md
    projects/backend/
      knowledge/                            # Member+project - dev-01 on backend only
        backend-quirks.md
```

## Next steps

1. **[Prerequisites](prerequisites.md)** - Install the required tools and set up GitHub auth
2. **[Bootstrap Your Team](bootstrap-your-team.md)** - Create your first team with `bm init`
3. **[Your First Journey](first-journey.md)** - End-to-end walkthrough of creating an epic and watching the pipeline
