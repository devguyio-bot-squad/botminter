---
status: completed
created: 2026-02-19
started: 2026-02-19
completed: 2026-02-19
---
# Task: Update Agent Context Files (CLAUDE.md, PROMPT.md, Knowledge, ralph.yml)

## Description
Update all agent context files across both profiles to replace `.github-sim/` references with real GitHub. This includes CLAUDE.md files (team and member level), PROMPT.md files (all members), knowledge files, and ralph.yml board scanner configurations. The goal is that every file an agent reads at runtime reflects the GitHub-native coordination model.

## Background
Agents read context from multiple layered sources at runtime:
- **CLAUDE.md** (team-level and member-level) — describes the coordination model, workspace layout, access paths
- **PROMPT.md** (per member) — runtime instructions including board scanning, status transitions, write-lock protocol
- **Knowledge files** — commit conventions, PR standards, communication protocols
- **ralph.yml** — board scanner configuration (currently scans `.botminter/.github-sim/issues/`)

All of these currently reference `.github-sim/` paths, file-based operations, and the write-lock protocol. They need to reference GitHub issues/PRs/milestones accessed via the `gh` skill.

## Technical Requirements

### CLAUDE.md Files
1. **Team CLAUDE.md** (`skeletons/profiles/rh-scrum/CLAUDE.md`, `skeletons/profiles/compact/members/superman/CLAUDE.md`):
   - Update "Coordination Model" section — board is GitHub issues, not `.botminter/.github-sim/issues/`
   - Update "File-Based Workflow" table — replace with GitHub-native equivalents
   - Update "Team Repo Access Paths" table — remove `.github-sim/` rows, add GitHub references (issues/milestones/PRs live on the team repo's GitHub, accessed via `gh` skill)
   - Remove "Write-Lock Protocol" section entirely
   - Update workspace model diagram if it references `.github-sim/`

### PROMPT.md Files
2. **All member PROMPT.md files** (human-assistant, architect, superman):
   - Update board location references from `.botminter/.github-sim/issues/` to GitHub
   - Remove write-lock protocol instructions
   - Remove stale lock cleanup instructions
   - Update status transition instructions to use `gh` skill
   - Update comment instructions to use `gh` skill

### Knowledge Files
3. **commit-convention.md** (both profiles):
   - Update issue number references — still `#<number>` but now referencing GitHub issues
   - Remove any `.github-sim/` path references

4. **pr-standards.md** (both profiles):
   - Update PR references from `.github-sim/pulls/` to real GitHub PRs
   - PRs are now `gh pr create`, not file-based

5. **communication-protocols.md** (both profiles):
   - Update status transition mechanism — `gh issue edit` instead of file editing
   - Remove file-based operation descriptions

### ralph.yml Board Scanner
6. **All ralph.yml files** (human-assistant, architect, superman):
   - Update `board_scanner` instructions to use `gh issue list --repo <auto-detected> --label "status/*" --json ...` instead of reading `.botminter/.github-sim/issues/` files
   - The repo is auto-detected from `.botminter/`'s git remote (the team repo is on GitHub)
   - Remove stale lock cleanup from board scanner (no more `.lock` files)
   - Remove all git commit/push steps for status transitions — `gh issue edit` is atomic
   - Update hat instructions that reference `.github-sim/` paths
   - Update `skills.dirs` to include `agent/skills/` (skeleton-level `gh` skill) and remove `board`/`create-epic` references
   - Update comment formatting in hat instructions to use `### <emoji> <role> — <timestamp>` format (read from `.botminter.yml`)

## Dependencies
- Task 01 (gh CLI skill) — so we can reference the skill name correctly
- Task 02 (PROCESS.md update) — so context files can reference the updated process docs

## Files to Modify
- `skeletons/profiles/rh-scrum/CLAUDE.md`
- `skeletons/profiles/compact/members/superman/CLAUDE.md`
- `skeletons/profiles/rh-scrum/members/human-assistant/CLAUDE.md`
- `skeletons/profiles/rh-scrum/members/architect/CLAUDE.md`
- `skeletons/profiles/rh-scrum/members/human-assistant/PROMPT.md`
- `skeletons/profiles/rh-scrum/members/architect/PROMPT.md`
- `skeletons/profiles/compact/members/superman/PROMPT.md`
- `skeletons/profiles/rh-scrum/knowledge/commit-convention.md`
- `skeletons/profiles/rh-scrum/knowledge/pr-standards.md`
- `skeletons/profiles/rh-scrum/knowledge/communication-protocols.md`
- `skeletons/profiles/compact/knowledge/commit-convention.md`
- `skeletons/profiles/compact/knowledge/pr-standards.md`
- `skeletons/profiles/compact/knowledge/communication-protocols.md`
- `skeletons/profiles/rh-scrum/members/human-assistant/ralph.yml`
- `skeletons/profiles/rh-scrum/members/architect/ralph.yml`
- `skeletons/profiles/compact/members/superman/ralph.yml`

## Implementation Approach
1. Start with team-level CLAUDE.md files — these define the conceptual model that all other files reference
2. Update member PROMPT.md files — remove write-lock protocol, update board scanning, update skill references
3. Update member CLAUDE.md files — align with team CLAUDE.md changes
4. Update knowledge files — mostly search-and-replace with context-aware adjustments
5. Update ralph.yml files — change board_scanner config, update hat instructions, update skills.dirs
6. Cross-check: grep for any remaining `.github-sim` references across all modified files

## Acceptance Criteria

1. **No `.github-sim/` references in any context file**
   - Given all CLAUDE.md, PROMPT.md, and knowledge files
   - When searching for `.github-sim`
   - Then zero matches are found

2. **No write-lock references in any context file**
   - Given all CLAUDE.md, PROMPT.md, and ralph.yml files
   - When searching for "write-lock", "lock file", `.lock`, or "stale_lock"
   - Then zero matches are found

3. **Board scanner uses `gh` CLI**
   - Given the ralph.yml files for all members
   - When reading the board_scanner configuration
   - Then it references `gh issue list` or the `gh` skill for issue discovery

4. **Skills directories updated**
   - Given the ralph.yml files
   - When reading `skills.dirs`
   - Then the `gh` skill directory is included and `board`/`create-epic` references are removed

5. **Workspace model still coherent**
   - Given the updated team CLAUDE.md
   - When reading the workspace model and access paths
   - Then the documentation accurately describes how agents access GitHub (via `gh` CLI in the project workspace)

6. **Knowledge files consistent**
   - Given the updated knowledge files
   - When reading commit-convention.md
   - Then issue references use `#<number>` format referencing real GitHub issues

## Metadata
- **Complexity**: High
- **Labels**: agent-context, claude-md, prompt-md, ralph-yml, knowledge, migration
- **Required Skills**: Ralph configuration, CLAUDE.md/PROMPT.md authoring, label scheme
