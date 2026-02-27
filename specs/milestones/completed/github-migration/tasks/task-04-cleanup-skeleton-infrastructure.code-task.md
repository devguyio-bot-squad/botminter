---
status: completed
created: 2026-02-19
started: 2026-02-19
completed: 2026-02-19
---
# Task: Clean Up Skeleton and Infrastructure

## Description
Remove the `.github-sim/` directory structure from the team repo skeleton, drop the `create-issue` Justfile recipe (now replaced by `gh issue create`), clean up fixture deployment scripts, and add any GitHub bootstrap infrastructure needed (label scheme setup). This is the final cleanup task that removes all vestiges of the file-based simulation.

## Background
The team repo skeleton at `skeletons/team-repo/` currently includes:
- `.github-sim/issues/` (with `.gitkeep`)
- `.github-sim/milestones/` (with `.gitkeep`)
- `.github-sim/pulls/` (with `.gitkeep`)

The team repo Justfile includes a `create-issue` recipe (lines 73-153) that creates issue files in `.github-sim/issues/`. This was a convenience method for the human operator and is now replaced by `gh issue create` (or the `gh` skill).

Fixture deployment scripts in `specs/milestone-2-architect-first-epic/fixtures/deploy.sh` and `sprint-5/fixtures/deploy.sh` deploy synthetic issues to `.github-sim/issues/1.md`. These need to be updated to use `gh issue create` or removed.

Additionally, the root `CLAUDE.md` and `README.md` for the agentminter project itself reference `.github-sim/` and need updating.

## Technical Requirements

### Skeleton Cleanup
1. Remove `.github-sim/` directory and all `.gitkeep` files from `skeletons/team-repo/`
2. Remove the `create-issue` recipe from `skeletons/team-repo/Justfile` (lines 73-153)
3. Keep all other Justfile recipes (`add-member`, `create-workspace`, `sync`, `launch`)
4. Update the `launch` recipe to accept `--gh-token <TOKEN>` and export it as `GH_TOKEN` env var (follows the same pattern as `--telegram-bot-token` → `RALPH_TELEGRAM_BOT_TOKEN`). This is a shared team token — all agents use the same one.
5. Add a `bootstrap-labels` recipe to the Justfile that creates the standard label scheme on the GitHub repo via `gh label create`

### Fixture Scripts
5. Update or remove fixture deployment scripts:
   - `specs/milestone-2-architect-first-epic/fixtures/deploy.sh`
   - `specs/milestone-2-architect-first-epic/sprint-5/fixtures/deploy.sh`
   - These should either use `gh issue create` for deploying synthetic fixtures, or be marked as deprecated/archived

### Project-Level Documentation
6. Update `CLAUDE.md` (root) — remove `.github-sim/` references in the project overview
7. Update `README.md` (root) — update coordination description
8. Update relevant docs in `docs/` directory:
   - `docs/architecture.md`
   - `docs/epic-lifecycle.md`
   - `docs/getting-started.md`
   - `docs/member-roles.md`
   - `docs/operations.md`
   - `docs/skills.md`
   - `docs/index.md`

## Dependencies
- Tasks 01-03 should be complete — this task is the final cleanup

## Files to Modify
- **Remove:** `skeletons/team-repo/.github-sim/` (entire directory)
- **Verify:** `skeletons/team-repo/agent/skills/gh/SKILL.md` exists (created in task 01)
- **Edit:** `skeletons/team-repo/Justfile` (remove `create-issue` recipe, add `--gh-token` to `launch`, add `bootstrap-labels`)
- **Edit or remove:** `specs/milestone-2-architect-first-epic/fixtures/deploy.sh`
- **Edit or remove:** `specs/milestone-2-architect-first-epic/sprint-5/fixtures/deploy.sh`
- **Edit:** `CLAUDE.md` (root)
- **Edit:** `README.md` (root)
- **Edit:** `docs/architecture.md`
- **Edit:** `docs/epic-lifecycle.md`
- **Edit:** `docs/getting-started.md`
- **Edit:** `docs/member-roles.md`
- **Edit:** `docs/operations.md`
- **Edit:** `docs/skills.md`
- **Edit:** `docs/index.md`

## Implementation Approach
1. Remove `.github-sim/` directory from skeleton
2. Remove `create-issue` recipe from Justfile
3. Add `bootstrap-labels` recipe that runs `gh label create` for each standard label:
   - `kind/epic`, `kind/story`
   - All `status/*` labels from the epic and story lifecycle
   - `status/error`
4. Update fixture deploy scripts to use `gh issue create` (or add deprecation notice)
5. Update root CLAUDE.md and README.md
6. Update all docs/*.md files
7. Final verification: grep entire repo for `.github-sim` — should only appear in historical spec documents (requirements, design docs, meeting notes)

## Acceptance Criteria

1. **Skeleton has no `.github-sim/`**
   - Given the `skeletons/team-repo/` directory
   - When listing its contents
   - Then there is no `.github-sim/` directory

2. **`create-issue` recipe removed**
   - Given the `skeletons/team-repo/Justfile`
   - When searching for `create-issue`
   - Then it is not present as a recipe

3. **`launch` recipe accepts `--gh-token`**
   - Given the `skeletons/team-repo/Justfile`
   - When running `just launch <member> --telegram-bot-token <T> --gh-token <G>`
   - Then `GH_TOKEN` is exported as an environment variable for the Ralph process

4. **`bootstrap-labels` recipe exists**
   - Given the `skeletons/team-repo/Justfile`
   - When running `just bootstrap-labels` (with `GH_TOKEN` set)
   - Then it creates all standard labels (`kind/epic`, `kind/story`, `status/po:triage`, etc.) on the GitHub repo

4. **Root documentation updated**
   - Given `CLAUDE.md` and `README.md` at the project root
   - When searching for `.github-sim`
   - Then zero matches are found (outside of historical context in specs/)

5. **Docs directory updated**
   - Given the `docs/` directory
   - When searching for `.github-sim`
   - Then zero matches are found

6. **No orphaned references**
   - Given the entire repository
   - When running `grep -r ".github-sim" --include="*.md" --include="*.yml" --include="Justfile"`
   - Then matches only appear in historical spec documents (specs/master-plan/, specs/milestone-*/) which serve as design history

## Metadata
- **Complexity**: Medium
- **Labels**: skeleton, infrastructure, cleanup, migration
- **Required Skills**: Justfile, gh CLI, documentation
