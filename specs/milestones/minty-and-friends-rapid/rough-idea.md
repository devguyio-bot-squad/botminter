# Minty and Friends — Rough Idea [RAPID]

> **Rapid milestone** — safe assumptions, fast iteration, minimal gates.
>
> Multiple UX enhancements to improve the operator experience at both the BotMinter and team layers.

## 1. Coding-Agent-Agnostic Cleanup

Audit and abstract away any hard-coded Claude Code-specific assumptions throughout the codebase. Ralph Orchestrator already supports multiple coding agents, so the rest of the BotMinter stack should not assume a specific coding agent product.

- Identify places where "Claude Code", "CLAUDE.md", or Claude Code-specific concepts are hard-coded
- Abstract behind coding-agent-agnostic interfaces where they don't already exist
- Ensure profiles, team repos, member skeletons, and CLI output don't unnecessarily couple to a single coding agent
- The role-as-skill and Minty designs should also be coding-agent-agnostic from the start

## 2. Profile Externalization

Change the profile storage model from compile-time embedded to disk-based:

- **Current:** Profiles are baked into the `bm` binary at compile time via `include_dir`. All operations read from embedded profiles.
- **Proposed:** A `bm` CLI command initializes/extracts the baked-in profiles to a local directory on disk. This extraction is the ONLY use of the embedded profiles. All subsequent operations (team creation, hire, sync, etc.) use the disk-stored profiles — making them editable, customizable, and versionable without rebuilding the binary.

## 3. Workspace Repository Model

Replace the current workspace model (project workspace with team repo embedded in `.botminter/`) with a dedicated workspace repository per agent.

- **Problem:** The current model causes agent confusion — nested `CLAUDE.md` files, skills directories from both layers, agents failing to push because they think changes belong to the same repo.
- **Proposed:** Each agent gets its own git repository as workspace. The team repo and project fork(s) are added as git submodules. `CLAUDE.md`, `ralph.yml`, `PROMPT.md` live at the workspace repo root — giving Ralph Orchestrator a clean, unambiguous context.
- **Default project:** Each role has a default project. The Team Manager's default project is the team repo itself.
- **Multi-project agents:** An agent can be assigned multiple projects as submodules, routing work based on issue labels.

## 4. Team Manager Role

A new role within the team (not BotMinter-level) for process improvement tasks. The Team Manager:

- Operates independently from other roles (dev, QE, architect, etc.)
- Picks up tasks assigned directly to it via GitHub issues
- Focuses on process improvements within the team
- Has a simplistic workflow compared to other roles

Additionally, the Team Manager is the first experiment with the **role-as-skill pattern**: a role that can be invoked both through the typical GitHub issues pull-based workflow AND as a coding agent skill from an interactive session.

## 5. Minty — BotMinter Interactive Assistant

An interactive assistant persona at the BotMinter layer (not team-scoped). Minty is:

- Aware of BotMinter concepts: profiles, teams, config, CLI, conventions
- Spun up as a coding agent session injected with a system prompt
- **Skill-driven architecture:** All knowledge and capabilities are baked into composable skills. Minty itself is a thin persona shell; the skills provide the actual functionality.
- Named "Minty" (derived from BotMinter) — friendly, approachable persona

## Unifying Themes

- **Skills as building blocks:** The role-as-skill pattern (Team Manager) and the skill-driven assistant (Minty) share the same foundation: composable skills as the delivery mechanism for both team-level and BotMinter-level interactions.
- **Coding-agent-agnostic by design:** All new work in this milestone should avoid coding-agent-specific assumptions, and existing hard-coded assumptions should be cleaned up.
