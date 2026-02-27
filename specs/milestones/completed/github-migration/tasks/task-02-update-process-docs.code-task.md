---
status: completed
created: 2026-02-19
started: 2026-02-19
completed: 2026-02-19
---
# Task: Update PROCESS.md Files for Real GitHub

## Description
Rewrite the PROCESS.md files in both `rh-scrum` and `compact` profiles to reference real GitHub as the backing store instead of `.github-sim/` markdown files. The coordination model (labels, statuses, roles, transitions) stays identical — only the storage and access mechanism changes.

## Background
PROCESS.md is the canonical reference for how the team coordinates. Currently it defines:
- **Issue format** — YAML frontmatter in `.github-sim/issues/<number>.md`
- **Label conventions** — `status/<role>:<phase>`, `kind/epic`, `kind/story`
- **Comment format** — Appended markdown blocks with `### @<role> — <timestamp>` (changing to `### <emoji> <role> — <timestamp>`)
- **Milestone format** — YAML frontmatter in `.github-sim/milestones/<title>.md`
- **PR format** — YAML frontmatter in `.github-sim/pulls/<number>.md`
- **Write-lock protocol** — `.lock` files for concurrent access
- **Communication protocols** — File editing + git commit/push

Since `.github-sim` was always a simulation of GitHub, the label scheme and status flow are unchanged. What changes:
- File paths → GitHub issue/PR/milestone references
- YAML frontmatter format → GitHub issue metadata (via `gh` CLI)
- Appended markdown comments → GitHub issue comments
- Write-lock protocol → removed (GitHub handles concurrency)
- Git commit/push for transitions → `gh` CLI calls

## Technical Requirements
1. Preserve all label conventions exactly (`status/<role>:<phase>`, `kind/*`)
2. Preserve all status flow tables (epic statuses, story statuses, rejection loops)
3. Replace file path references (`.github-sim/issues/<number>.md`) with GitHub references (`issue #<number>`)
4. Replace YAML frontmatter format documentation with `gh` CLI field mapping
5. Replace appended comment format with GitHub issue comment format: `### <emoji> <role> — <ISO-timestamp>`. Each role has a standard emoji (📝 po, 🏗️ architect, 💻 dev, 🧪 qe, 🛠️ sre, ✍️ cw, 👑 lead). The emoji and role are read from the member's `.botminter.yml` file at runtime by the `gh` skill. Since all agents share one `GH_TOKEN` (one GitHub user), the role attribution in the comment body is the primary way to identify which hat/role wrote it
6. Remove the write-lock protocol section entirely
7. Remove `stale_lock_threshold_minutes` setting
8. Update communication protocols section — status transitions via `gh issue edit`, comments via `gh issue comment`
9. Update milestone format — milestones are GitHub milestones, managed via `gh api`
10. Update PR format — PRs are real GitHub PRs, managed via `gh pr`

## Dependencies
- Task 01 (gh CLI skill) should be complete or in progress so the process docs can reference the skill

## Files to Modify
- `skeletons/profiles/rh-scrum/PROCESS.md`
- `skeletons/profiles/compact/PROCESS.md`

## Implementation Approach
1. Read both PROCESS.md files thoroughly
2. Identify every section that references file-based operations
3. Rewrite the "Issue Format" section to describe GitHub issues (fields mapped to GitHub issue metadata)
4. Rewrite the "Comment Format" section — comments are GitHub issue comments with format `### <emoji> <role> — <ISO-timestamp>`. Define the standard emoji mapping. Note that the `gh` skill reads emoji/role from `.botminter.yml` — PROCESS.md defines the convention, the skill implements it
5. Rewrite the "Milestone Format" section — milestones are GitHub milestones
6. Rewrite the "Pull Request Format" section — PRs are real GitHub PRs
7. Remove the "Write-Lock Settings" section
8. Rewrite the "Communication Protocols" section — transitions via `gh` CLI, no git commit/push needed for coordination
9. Keep all label tables, status flow tables, and role definitions identical

## Acceptance Criteria

1. **No `.github-sim/` references**
   - Given the updated PROCESS.md files
   - When searching for `.github-sim`
   - Then zero matches are found

2. **Label scheme preserved**
   - Given the updated PROCESS.md files
   - When comparing the status label tables to the originals
   - Then all labels (`status/po:triage`, `status/arch:design`, etc.) are identical

3. **Epic status flow preserved**
   - Given the updated PROCESS.md
   - When reading the epic statuses table
   - Then all 11 statuses and their role assignments match the original

4. **Write-lock protocol removed**
   - Given the updated PROCESS.md files
   - When searching for "write-lock", "lock file", "stale_lock", or `.lock`
   - Then zero matches are found

5. **GitHub-native operations described**
   - Given the communication protocols section
   - When reading how status transitions work
   - Then the process describes using `gh issue edit` to change labels (not editing YAML frontmatter)

6. **Comment convention updated**
   - Given the comment format section
   - When reading the convention
   - Then comments are described as GitHub issue comments with format `### <emoji> <role> — <ISO-timestamp>` and the standard emoji mapping is documented

7. **Issues live on the team repo**
   - Given the PROCESS.md
   - When reading where issues/milestones/PRs are stored
   - Then it states they are GitHub issues/milestones/PRs on the team repo (not the project repo)

## Metadata
- **Complexity**: Medium
- **Labels**: process, documentation, migration
- **Required Skills**: Process documentation, GitHub conventions, label scheme
