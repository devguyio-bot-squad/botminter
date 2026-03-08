# Requirements — Minty and Friends [RAPID]

> **Rapid milestone** — safe assumptions confirmed in batches, minimal back-and-forth.
>
> Q&A record from requirements clarification sessions.

---

## Q1: Do you see dependencies or a preferred ordering between the four enhancements?

The milestone has four pieces: (1) Team Manager role, (2) Profile externalization, (3) Minty, (4) Coding-agent-agnostic cleanup. Some of these might need to land before others — for example, profile externalization might need to happen before Minty can modify on-disk profiles, or the coding-agent-agnostic cleanup might be a prerequisite for the skill-based designs. Do you have a preferred build order, or should they be independent and deliverable in any sequence?

**A1:** Ordered sequence:
1. Coding-agent-agnostic cleanup
2. Profile externalization
3. Workspace repository model
4. Team Manager role
5. Minty

---

## Q2: For the coding-agent-agnostic cleanup — what's your mental model for the abstraction boundary?

Right now, BotMinter generates files like `CLAUDE.md`, references "Claude Code" in profiles, and surfaces `.claude/agents/` directories. When you say "abstract this away," are you thinking:

(a) A **config-driven mapping** — e.g., `botminter.yml` declares `coding_agent: claude-code` and the CLI uses that to decide filenames (`CLAUDE.md` vs `AGENTS.md` vs whatever), directory names, etc.
(b) A **generic naming convention** — replace all Claude-specific names with BotMinter's own conventions (e.g., `AGENT.md` instead of `CLAUDE.md`) regardless of which coding agent is used
(c) Something else entirely?

**A2:** Option (a) — config-driven mapping. The profile or team config declares which coding agent is in use, and the CLI uses that to determine filenames, directory names, and other agent-specific conventions.

---

## Q3: Where should the coding agent declaration live?

Since we're going with a config-driven mapping, where should `coding_agent: claude-code` be declared?

(a) **Profile level** — in `botminter.yml` within the profile. All teams generated from that profile inherit the same coding agent.
(b) **Team level** — in the team's config (`~/.botminter/config.yml` or the team repo). Different teams from the same profile could use different coding agents.
(c) **Both** — profile sets a default, team can override.

**A3:** Option (c) — both. Profile sets the default coding agent, team can override. Viability to be confirmed during research/design.

---

## Q4: What should happen at the file/directory level when the coding agent changes?

With `coding_agent: claude-code`, the system currently generates `CLAUDE.md` and `.claude/agents/`. If a team overrides to, say, `coding_agent: gemini-cli`, should:

(a) The CLI generate the **equivalent files for that agent** (e.g., `GEMINI.md`, `.gemini/agents/`) — a 1:1 mapping per agent
(b) The CLI generate **BotMinter-owned generic files** (e.g., `AGENT.md`) and then a thin adapter translates them into agent-specific format at launch time
(c) The **profiles themselves** contain agent-specific file variants, and the CLI picks the right set based on config

**A4:** Option (c) — profiles contain agent-specific file variants. A profile declares which coding agents it supports (one or more). If it supports one agent, it has one set of variants. If it supports two, it has both. The CLI picks the right variant set based on the resolved `coding_agent` config.

---

## Q5: Which coding agents should be supported at launch?

We know Claude Code is the current default. Which other coding agents should the system support from day one of this milestone? Or is the goal just to make the architecture pluggable, with Claude Code as the only concrete implementation for now?

**A5:** Pluggable architecture with Claude Code as the only concrete implementation for now. The abstraction should make adding a second coding agent straightforward in the future.

---

## Q6: For profile externalization — should `bm init` still work on a fresh install with no profiles on disk?

Today `bm init` pulls from embedded profiles. After externalization, the embedded profiles become seed data extracted to disk. Should there be an explicit setup step (e.g., `bm profiles init` or `bm setup`) that the operator runs first, or should `bm init` auto-extract profiles to disk on first use if none are found?

**A6:** Dedicated `bm profiles init` command for extracting profiles to disk. Any command that requires profiles on disk (including `bm init`) should detect missing profiles and prompt the user: "Profiles not initialized. Do you want me to initialize them now?" If yes, run the initialization inline. If no, abort with a message telling the user they can run `bm profiles init` at any time. Follows the same pattern as other subcommands — dedicated command exists, but the init wizard can trigger it interactively when needed.

---

## Q7: Where on disk should externalized profiles be stored?

The current BotMinter config lives at `~/.botminter/config.yml`. Should profiles go under the same root (e.g., `~/.botminter/profiles/`), or somewhere else like `~/.config/botminter/profiles/`?

**A7:** `~/.config/botminter/profiles/`. Two-directory split: `~/.config/botminter/` for user configuration (profiles, preferences), `~/.botminter/` for runtime/operational data (config.yml with credentials and team registry, workspaces). The operator machine may not have agents running locally, so `~/.botminter/` may not exist there. `config.yml` stays in `~/.botminter/` — it contains credentials and operational state, not user preferences.

---

## Q8–Q13: Batched assumptions for rapid progress

**Q8: Should `bm profiles init` support selective extraction?**
**A8:** No — extract all embedded profiles at once. Simple and predictable. User can delete ones they don't want.

**Q9: Profile updates — what happens when the binary ships newer profiles than what's on disk?**
**A9:** `bm profiles init` warns if profiles already exist on disk and offers to overwrite or skip. No automatic updates — the operator is in control. A `--force` flag for scripted use.

**Q10: Team Manager — what's the scope of "process improvements" it can do? And what's its workspace model?**
Assumed (original): The Team Manager operates on files within the team repo. Does NOT touch profiles.

**A10:** The Team Manager's default project is the team repo itself. This introduces two new concepts:

**Default project:** Each role has a default project. When `bm start` launches an agent, it starts in that project's context.

**Workspace repository (new model):** Each agent gets its own dedicated git repository as its workspace. Instead of embedding the team repo in `.botminter/` inside the project (which caused friction — agents confused by nested CLAUDE.md/skills, failing to push because they thought changes were in the same repo), the workspace repo contains:
- The team repo as a git submodule
- Fork(s) of assigned project(s) as git submodules
- `CLAUDE.md`, `ralph.yml`, `PROMPT.md` at the root (where Ralph Orchestrator runs)

This also enables multi-project agents — the same agent can work on multiple projects, routing work based on issue labels. The Team Manager is the natural first example: team repo is its default project, but it could also be assigned actual projects.

Design details (submodule layout, how `bm teams sync` provisions this, etc.) deferred to design phase.

**Q11: Team Manager — should it have its own GitHub issue statuses or reuse existing ones?**
**A11:** Minimal dedicated statuses (e.g., `status/mgr:todo`, `status/mgr:in-progress`, `status/mgr:done`). No complex kanban — simplistic workflow.

**Q12: Role-as-skill — what does "invoke a role from a coding agent session" look like concretely?**
Assumed (original): A command like `bm chat <role>` launches a session with prompt/knowledge injected.

**A12:** Simpler than assumed — `bm chat <member> [-t team]` just launches the coding agent inside that member's workspace repository at the root level. Since the workspace already has `CLAUDE.md`, `ralph.yml`, `PROMPT.md`, and the submodules set up, the coding agent inherits all the context it needs by virtue of being launched in the right directory. No special prompt injection required beyond what's already in the workspace.

**Q13: Minty — should Minty be aware of all teams or scoped to one at a time?**
Assumed: Minty is aware of all registered teams and can operate across them, scoped via `-t`.

**A13:** Correct. Minty sees all registered teams and can be scoped with `-t`. Additionally, Minty should handle the case where `~/.botminter/` doesn't exist on the operator's machine — the operator may be running `bm` CLI and Minty on a different machine from where agents run. Minty reads from `~/.config/botminter/` (user config, profiles) and `~/.botminter/` (runtime, if present) independently.

---

## Q14: Workspace repo — who creates and hosts these git repos?

Should `bm teams sync` automatically create the workspace repos (e.g., as local git repos, or on GitHub under the same org), or is this a manual step the operator does? And should they be hosted on GitHub alongside the team repo, or are they local-only?

**A14:** Workspace repos are GitHub repositories, following the same lifecycle as the team repo:
- **New team/member:** `bm teams sync --push` creates the repo on GitHub and pushes.
- **Existing team/member (fresh setup):** `bm init` on a new machine for an existing team clones existing workspace repos from GitHub.
- Same create-or-clone pattern already used for team repos.

---

## Q15: Workspace repo naming convention on GitHub?

**A15:** `<team-name>-<member-name>` in the same GitHub org as the team repo. E.g., team repo `org/my-team`, member workspace `org/my-team-alice`.

---

## Q16: Breaking changes during Alpha?

Should we worry about migration paths or backwards compatibility with existing teams/workspaces?

**A16:** No. During Alpha, every change is a breaking change. No migration, no backwards compatibility. Existing teams/workspaces are simply invalidated. Operators re-create from scratch. No upgrade paths or compatibility shims during Alpha.

---

## Q17: How does the operator launch Minty?

**A17:** `bm minty` launches a coding agent session in BotMinter's own context (not a team workspace). Minty's config lives in `~/.config/botminter/minty/`.

---

## Q18: Where do Minty's skills and config live?

**A18:** `~/.config/botminter/minty/` — dedicated directory for all Minty-related files (skills, system prompt, config). Keeps things clean alongside `~/.config/botminter/profiles/`.

Directory layout:
```
~/.config/botminter/
  profiles/          # Externalized profiles
  minty/             # All Minty-related files (skills, prompt, config)
```

---

## Q19: Are requirements complete enough to proceed?

We've covered:
- Build order (5 steps)
- Coding-agent-agnostic: config-driven mapping, profile + team override, profile variants, Claude Code only for now
- Profile externalization: `bm profiles init`, `~/.config/botminter/profiles/`, all-at-once extraction, overwrite/skip on re-run
- Workspace repo model: GitHub-hosted, same lifecycle as team repo, `<team>-<member>` naming, submodules for team repo + project forks
- Team Manager: team repo as default project, minimal statuses, independent workflow
- Role-as-skill: `bm chat <member>` launches coding agent in member's workspace
- Minty: `bm minty`, `~/.config/botminter/minty/`, skill-driven architecture
- Alpha policy: breaking changes, no migration

Anything missing, or should we move on to research or design?

**A19:** Requirements confirmed complete. This is a rapid milestone — proceed to research/design with safe assumptions and minimal gates.
