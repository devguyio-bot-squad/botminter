# FAQ

## Does BotMinter only work with Claude Code?

Claude Code is the recommended backend and all shipped profiles are built for it today. Under the hood, BotMinter uses [Ralph orchestrator](https://github.com/mikeyobrien/ralph-orchestrator), which also supports Gemini CLI, Codex, Kiro, Amp, Copilot CLI, and OpenCode. Profiles for these backends are planned.

## Do I need multiple agents running in parallel?

Not at all. It depends on the profile you choose. The `agentic-sdlc-minimal` profile uses three roles (engineer, chief-of-staff, sentinel) with the engineer wearing multiple hats — planner, implementer, reviewer. Other profiles like `scrum` distribute those hats across more specialized agents. You pick the formation that fits your workflow.

## Is this only for Scrum teams?

No. BotMinter focuses on **conventions**, not a specific methodology. It ships with opinionated defaults (like the Scrum profile), but profiles are fully customizable. Think of it like Rails for web or Spring for enterprise — baked-in conventions that you can tweak to fit your process. You can fork an existing profile or create one from scratch.

## What's the difference between BotMinter and just using Claude Code?

Claude Code is a powerful single-agent tool. BotMinter adds structured conventions on top: layered knowledge scoping (team → project → member), process enforcement via status gates, cumulative learning through knowledge files, and a coordination fabric via GitHub issues. The value isn't in the runtime — it's in the conventions that make your investment in one agent carry forward to every other agent you run.

## What is a "profile"?

A profile is a reusable methodology template — it defines roles, pipeline stages, quality gates, and knowledge structure. `bm init` extracts a profile into a new team repo. You can use the built-in profiles as-is, fork them, or create your own.

## What is Minty?

Minty is BotMinter's interactive assistant — a lightweight persona shell that you launch with `bm minty`. Unlike team members, Minty is not orchestrated by Ralph and does not run in a loop. It is simply a coding agent session primed with BotMinter-specific knowledge and skills, designed for ad-hoc operator tasks like browsing profiles, checking team status, or getting guidance on hiring decisions.

Minty works without any teams configured — if `~/.botminter/` doesn't exist, it runs in "profiles-only" mode and can still browse profiles and answer general questions.

## How does Minty differ from team members?

Team members (launched with `bm start` or interacted with via `bm chat`) are full Ralph Orchestrator instances — they have hats, a persistent loop, memories, and coordinate through GitHub issues. Minty has none of that. It is a one-shot coding agent session with a persona prompt and a set of composable skills. Think of it as the difference between a team player running a structured workflow and a knowledgeable helper you can ask quick questions.

| | Team members | Minty |
|---|---|---|
| Orchestration | Ralph loop with hats | None — single session |
| Coordination | GitHub issues + status labels | None |
| Persistence | Memories, scratchpad, tasks | None |
| Launch | `bm start` / `bm chat <member>` | `bm minty` |
| Purpose | Execute structured workflows | Ad-hoc operator assistance |

## What can Minty do?

Minty ships with four composable skills:

- **team-overview** — reads `~/.botminter/config.yml` and reports on teams, members, running state, and workspace status
- **profile-browser** — explores profiles at `~/.config/botminter/profiles/`, showing roles, statuses, coding agents, and conventions
- **hire-guide** — interactive guide for `bm hire` decisions: which role to hire, naming suggestions, and implications
- **workspace-doctor** — diagnoses workspace health: checks for missing files, stale submodules, broken symlinks, and Ralph lock state

These skills are markdown files under `~/.config/botminter/minty/skills/` and are injected into the coding agent session via Minty's persona prompt.

## Why do I need to prefix my comments with `@bot`?

Each agent has its own GitHub App identity and posts as a bot user (e.g., `team-engineer[bot]`). The `@bot` prefix helps the agent reliably identify human input when parsing issue comments. While agent and human comments are now distinguishable by author, the prefix remains a useful convention for clarity.
