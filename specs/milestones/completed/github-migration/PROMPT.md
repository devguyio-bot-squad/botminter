# GitHub Migration: Replace .github-sim with Real GitHub

## Objective

Replace the file-based `.github-sim/` coordination system with real GitHub, using a
`gh` CLI skill as the single interaction point. The coordination model (labels, statuses,
roles, pull-based discovery) is unchanged — only the backing store moves from markdown
files to the GitHub API.

## Prerequisites

The `.github-sim/` system is fully implemented across both profiles (`rh-scrum`,
`compact`) with board scanning, issue creation, write-locks, and status transitions.
The `gh` CLI is available in the environment.

## Design Decisions

### Issues live on the team repo
GitHub issues, milestones, and PRs live on the **team repo** (not the project repo).
The `gh` skill auto-detects the target repo by reading `.botminter/`'s git remote
(e.g., `cd .botminter && gh repo view --json nameWithOwner -q .nameWithOwner`). All
`gh` commands use `--repo <auto-detected>`.

### Auth: shared team token
One `GH_TOKEN` env var for all agents in a team. Passed via
`just launch --gh-token <TOKEN>`, exported as `GH_TOKEN`. The `gh` CLI natively
respects this env var. All API calls appear as one GitHub user; agents self-attribute
via role emojis in comments.

### Comment format: `### <emoji> <role> — <timestamp>`
Each role has a standard emoji for visual scanning:
- 📝 po, 🏗️ architect, 💻 dev, 🧪 qe, 🛠️ sre, ✍️ cw, 👑 lead, 🦸 superman

The emoji and role name are read from the member's `.botminter.yml` file at runtime.

### Per-member identity file: `.botminter.yml`
Each member skeleton includes `skeletons/profiles/<profile>/members/<member>/.botminter.yml`:
```yaml
role: architect
comment_emoji: "🏗️"
```
The `gh` skill reads this from `.botminter/team/<member>/.botminter.yml` (resolved
via `.botminter/.member` marker) to format comments with correct attribution.

### Skill at skeleton level
The `gh` skill lives at `skeletons/team-repo/agent/skills/gh/SKILL.md` — shared
across all profiles (process-agnostic). Profile-level `board/` and `create-epic/`
skills are removed.

## Key References

- Task files: `specs/github-migration/tasks/` (execute in order 01→04)
- Current PROCESS.md: `skeletons/profiles/rh-scrum/PROCESS.md`
- Current skills: `skeletons/profiles/rh-scrum/agent/skills/{board,create-epic}/`
- Master plan M5 notes: `specs/master-plan/design.md` (Section 8)
- Design principles: `specs/design-principles.md`
- Existing profiles: `skeletons/profiles/rh-scrum/`, `skeletons/profiles/compact/`

## Execution Order

Process tasks sequentially using `/code-assist` for each:

1. `task-01-gh-cli-skill.code-task.md` — Create unified `gh` CLI skill, remove `board`
   and `create-epic` skills
2. `task-02-update-process-docs.code-task.md` — Rewrite PROCESS.md files for both
   profiles
3. `task-03-update-agent-context.code-task.md` — Update CLAUDE.md, PROMPT.md, knowledge
   files, and ralph.yml across all members
4. `task-04-cleanup-skeleton-infrastructure.code-task.md` — Remove `.github-sim/`
   skeleton, drop `create-issue` recipe, add `bootstrap-labels`, update docs

## Constraints

- The label scheme (`status/<role>:<phase>`, `kind/epic`, `kind/story`) MUST NOT change
- The pull-based coordination model MUST NOT change
- All `gh` interaction MUST go through the single `gh` skill — no scattered `gh` calls
  in PROMPT.md, CLAUDE.md, or other skills
- Write-lock protocol MUST be removed entirely (GitHub handles concurrency)
- Auth uses a shared `GH_TOKEN` env var — no per-member tokens. `just launch` passes
  it via `--gh-token` flag, same pattern as `--telegram-bot-token`
- Historical spec documents (specs/master-plan/, specs/milestone-*/) should NOT be
  modified — they are design history

## Acceptance Criteria

- Given a grep for `.github-sim` across non-historical files (skeletons/, docs/,
  CLAUDE.md, README.md), then zero matches are found

- Given `skeletons/team-repo/agent/skills/gh/SKILL.md`, then the `gh` skill exists at
  skeleton level. Profile-level `board/` and `create-epic/` are removed

- Given the updated PROCESS.md files, then the label scheme tables are identical to the
  originals, but file-based format definitions and write-lock protocol are removed

- Given the updated ralph.yml files, then board scanners reference `gh` CLI for issue
  discovery and skills.dirs includes the `gh` skill

- Given `skeletons/team-repo/`, then `.github-sim/` directory does not exist and the
  Justfile contains `bootstrap-labels` and `--gh-token` in `launch`, but not
  `create-issue`

- Given each member skeleton, then a `.botminter.yml` file exists with `role` and
  `comment_emoji` fields
