---
status: completed
created: 2026-02-19
started: 2026-02-19
completed: 2026-02-19
---
# Task: Create `gh` CLI Interaction Skill

## Description
Create a single, unified skill that wraps `gh` CLI for all GitHub interaction needed by team agents. This skill replaces the current file-based `board` and `create-epic` skills with real GitHub operations. The user will finalize the choice of an open-source skill async — this task creates a first-pass implementation to establish the interface and validate the approach.

## Background
The current agent skills (`board` and `create-epic`) operate on `.github-sim/issues/` markdown files with YAML frontmatter. Since `.github-sim` was designed as a 1:1 simulation of GitHub, the operations map directly to `gh` CLI equivalents:

- **Board scan** → `gh issue list --label "status/*" --json number,title,state,labels,assignees,milestone`
- **Create epic** → `gh issue create --title ... --label kind/epic --label status/po:triage`
- **Status transition** → `gh issue edit <number> --remove-label ... --add-label ...`
- **Comment** → `gh issue comment <number> --body ...`
- **Milestone** → `gh api` for milestone CRUD
- **PR operations** → `gh pr create`, `gh pr review`, `gh pr comment`

The write-lock protocol is no longer needed — GitHub handles concurrent access natively.

## Technical Requirements
1. Create a single skill at **skeleton level**: `skeletons/team-repo/agent/skills/gh/SKILL.md`. This is shared across all profiles (process-agnostic).
2. The skill MUST cover all operations currently split across `board` and `create-epic`:
   - List/query issues by label (board view)
   - Create issues (epics and stories)
   - Update issue labels (status transitions)
   - Add comments to issues
   - Milestone management
   - PR operations (create, review, comment)
3. The skill MUST use `gh` CLI — not the GitHub REST API directly
4. The skill MUST document that authentication is via `GH_TOKEN` env var (shared team token, set by `just launch`)
5. **Repo targeting:** The skill MUST auto-detect the target repo from `.botminter/`'s git remote (e.g., `cd .botminter && gh repo view --json nameWithOwner -q .nameWithOwner`). All `gh` commands use `--repo <detected-repo>`. No additional configuration needed.
6. **Comment format:** When adding comments, the skill MUST read the member's identity from `.botminter.yml` (see below) and format comments as `### <emoji> <role> — <ISO-timestamp>`. Example: `### 🏗️ architect — 2026-02-19T13:00:00Z`
7. The skill MUST preserve the existing label scheme (`status/<role>:<phase>`, `kind/epic`, `kind/story`) since the coordination model is unchanged
8. Remove the old `board` and `create-epic` skill directories from both profiles
9. No write-lock logic — GitHub handles concurrency

### `.botminter.yml` (per-member identity file)
Each member skeleton MUST include a `.botminter.yml` at `skeletons/profiles/<profile>/members/<member>/.botminter.yml` with per-member settings:

```yaml
role: architect
comment_emoji: "🏗️"
```

The `gh` skill reads this file from `.botminter/team/<member>/.botminter.yml` (resolved via the `.botminter/.member` marker) to determine comment attribution. Standard emoji mapping:
- 📝 po (human-assistant)
- 🏗️ architect
- 💻 dev
- 🧪 qe
- 🛠️ sre
- ✍️ cw (content writer)
- 👑 lead (team lead)
- 🦸 superman (compact — uses role-specific emoji per hat)

## Dependencies
- `gh` CLI must be installed in the workspace
- `GH_TOKEN` environment variable must be set (shared team token — passed via `just launch`)
- The team repo must be hosted on GitHub (issues live on the team repo)
- `.botminter/` must have a GitHub remote (skill auto-detects repo from it)
- `.botminter.yml` must exist in the member's directory (provides role + emoji for comments)

## Implementation Approach
1. Study the existing `board/SKILL.md` and `create-epic/SKILL.md` to extract the full interface
2. Design a single `gh/SKILL.md` that maps each file-based operation to its `gh` CLI equivalent
3. Preserve the output format from the `board` skill (grouped table by status)
4. Simplify `create-epic` by removing all write-lock and race-condition logic
5. Add operations not previously needed (label management, PR operations via `gh`)
6. Remove `board/` and `create-epic/` skill directories from both `rh-scrum` and `compact` profiles
7. Verify the skill works with `gh issue list --json` output parsing

## Files to Modify
- **Create:** `skeletons/team-repo/agent/skills/gh/SKILL.md` (skeleton-level, shared across all profiles)
- **Create:** `skeletons/profiles/rh-scrum/members/human-assistant/.botminter.yml`
- **Create:** `skeletons/profiles/rh-scrum/members/architect/.botminter.yml`
- **Create:** `skeletons/profiles/compact/members/superman/.botminter.yml`
- **Remove:** `skeletons/profiles/rh-scrum/agent/skills/board/SKILL.md`
- **Remove:** `skeletons/profiles/rh-scrum/agent/skills/create-epic/SKILL.md`
- **Remove:** `skeletons/profiles/compact/agent/skills/board/SKILL.md`
- **Remove:** `skeletons/profiles/compact/agent/skills/create-epic/SKILL.md`

## Acceptance Criteria

1. **Board query via `gh` CLI**
   - Given a GitHub repo with issues labeled `status/arch:design` and `kind/epic`
   - When the agent invokes the `gh` skill to view the board
   - Then it runs `gh issue list` with appropriate `--json` fields and displays a grouped board view matching the existing output format

2. **Issue creation via `gh` CLI**
   - Given a GitHub repo with existing issues
   - When the agent invokes the skill to create an epic
   - Then it runs `gh issue create` with correct `--label kind/epic --label status/po:triage` and returns the new issue number

3. **Status transition via `gh` CLI**
   - Given an issue with label `status/po:triage`
   - When the agent transitions it to `status/arch:design`
   - Then it runs `gh issue edit` to remove the old label and add the new one

4. **Comment via `gh` CLI with role attribution**
   - Given an existing issue and a member with `.botminter.yml` containing `role: architect` and `comment_emoji: "🏗️"`
   - When the agent adds a comment
   - Then it runs `gh issue comment` with body starting with `### 🏗️ architect — <ISO-timestamp>`

5. **Repo auto-detection**
   - Given a workspace where `.botminter/` has a GitHub remote `https://github.com/myorg/myteam.git`
   - When the skill determines the target repo
   - Then it resolves to `myorg/myteam` and uses `--repo myorg/myteam` for all `gh` commands

6. **`.botminter.yml` files created**
   - Given the member skeleton directories
   - When listing files
   - Then each member has a `.botminter.yml` with `role` and `comment_emoji` fields

7. **No write-lock logic**
   - Given the new skill
   - When reviewing its contents
   - Then there are zero references to `.lock` files, write-lock acquisition, or stale lock cleanup

8. **Old skills removed**
   - Given the profiles directory
   - When listing skill directories
   - Then `board/` and `create-epic/` no longer exist in either profile

9. **Skill at skeleton level**
   - Given the `skeletons/team-repo/agent/skills/` directory
   - When listing skill directories
   - Then `gh/SKILL.md` exists at the skeleton level (not duplicated per profile)

## Metadata
- **Complexity**: Medium
- **Labels**: skills, gh-cli, migration
- **Required Skills**: gh CLI, SKILL.md format, label scheme
