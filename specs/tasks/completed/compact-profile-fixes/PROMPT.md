# Profile Fixes & Tooling

## Objective

Fix bugs discovered during profile testing (both compact and rh-scrum), create tooling to prevent this class of bugs in future profiles, and migrate the status tracking mechanism from labels to GitHub Projects v2.

## Spec Directory

All task files are in `specs/compact-profile-fixes/tasks/`. Read each task's `.code-task.md` file before starting it — they contain the full context, technical requirements, acceptance criteria, and implementation approach.

## Execution Order

Work through tasks in this exact sequence. Each task builds on the previous ones.

### Phase 1 — CLI Bugfixes

1. **task-01-fix-team-repo-branch-name.code-task.md**
   Fix `bm init` to use `git init -b main` instead of bare `git init`.

2. **task-02-fix-botminter-remote-url.code-task.md**
   Fix `bm teams sync` so `.botminter/` remote points to GitHub, not a local file path.

### Phase 2 — Profile Bugfixes (both profiles)

3. **task-03-fix-board-scanner-sync-and-detection.code-task.md**
   Fix board scanner broken `Justfile` references, add team repo detection fallback, fix rh-scrum's glob-in-label bug, and replace hardcoded `hypershift` project name. Applies to both compact and rh-scrum profiles.

4. **task-04-remove-loop-complete.code-task.md**
   Remove all `LOOP_COMPLETE` usage from both profiles — no `completion_promise`, no `default_publishes`, no instruction references. Let the persistent loop handle restarts.

### Phase 3 — GitHub Projects Migration

5. **task-07-migrate-labels-to-github-project-statuses.code-task.md**
   Replace label-based `status/*` tracking with GitHub Projects v2 status fields across both profiles. Rewrites the `gh` skill, board scanners, all hat instructions, and `bm init` bootstrapping. Do this before creating the skills (tasks 5-6) so they encode the final patterns.

### Phase 4 — Preventive Tooling

6. **task-05-create-profile-hat-generator-skill.code-task.md**
   Create a botminter-specific skill for generating profile hats that encodes all architectural decisions (board scanner pattern, status conventions, knowledge scoping, no LOOP_COMPLETE, etc.).

7. **task-06-create-profile-hat-reviewer-skill.code-task.md**
   Create a reviewer skill that validates profile hats against botminter conventions. Catches the class of bugs found during testing before they ship.

## Acceptance Criteria

- [ ] `bm init` creates team repo on `main` branch
- [ ] `.botminter/` remote in workspaces points to GitHub URL
- [ ] Board scanners sync via `git pull`, not `Justfile` (both profiles)
- [ ] Zero occurrences of `LOOP_COMPLETE` across all profiles
- [ ] No glob patterns in `--label` flags (rh-scrum)
- [ ] No hardcoded project names — all use `<project>` placeholder (rh-scrum)
- [ ] Status transitions use GitHub Projects v2, not labels (both profiles)
- [ ] Profile hat generator skill exists and encodes all conventions
- [ ] Profile hat reviewer skill exists and catches all known bug patterns
- [ ] All existing tests pass (`just test`)
- [ ] Use the `documentation-expert` skill to validate all created/modified documentation

## Constraints

- One task per commit
- Run `cargo check && cargo test && cargo clippy -- -D warnings` after CLI changes (tasks 1-2)
- Validate YAML parses correctly after profile changes (tasks 3-4, 7)
- Use the `documentation-expert` skill before committing any documentation artifacts
- Both profiles must be updated in the same task — do not leave one profile fixed and the other broken
