---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: bm minty CLI Subcommand and Launch

## Description
Implement the `bm minty [-t team]` CLI subcommand that launches an interactive coding agent session with Minty's persona prompt. Minty runs in the current working directory, not a workspace.

## Background
`bm minty` is the operator's entry point to Minty. Unlike `bm chat` (which requires a hired member and workspace), `bm minty` works anywhere — it resolves the coding agent, loads Minty's prompt, and launches an interactive session. The `-t team` flag gives Minty team-specific context.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Minty")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 19)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Add `minty` subcommand to `cli.rs`:
   - Optional: `-t`/`--team`
2. Implement `commands/minty.rs`:
   - Ensure Minty is initialized at `~/.config/botminter/minty/` (auto-prompt if missing)
   - Resolve coding agent:
     - If `-t` specified: from team config
     - If no `-t`: from first available profile's default
   - Launch coding agent in current working directory with `--append-system-prompt-file` pointing to Minty's prompt
3. Handle missing `~/.botminter/`:
   - Log informational note about profiles-only mode
   - Don't fail — Minty works without any teams configured
4. Use `exec()` to replace the bm process with the coding agent process

## Dependencies
- Task 1 of this step (Minty config and prompt exist on disk)

## Implementation Approach
1. Add CLI subcommand definition
2. Implement command handler with Minty config reading
3. Add auto-prompt for Minty initialization (similar to profile auto-prompt)
4. Implement coding agent resolution
5. Implement agent launch with exec
6. Handle edge cases (no teams, no profiles, missing config)
7. Write tests for resolution and launch setup

## Acceptance Criteria

1. **bm minty launches coding agent**
   - Given Minty config on disk
   - When `bm minty` runs
   - Then the coding agent launches with Minty's prompt as system prompt

2. **Works without any teams**
   - Given no `~/.botminter/config.yml` (no teams)
   - When `bm minty` runs
   - Then it launches successfully with profiles-only mode note

3. **-t flag gives team context**
   - Given `bm minty -t my-team`
   - When resolving the coding agent
   - Then it uses the team's configured coding agent

4. **Auto-prompt on missing Minty config**
   - Given no `~/.config/botminter/minty/`
   - When `bm minty` runs
   - Then it prompts to initialize (or auto-initializes)

5. **Runs in current directory**
   - Given `bm minty` launched from any directory
   - When the coding agent starts
   - Then its working directory is the user's current directory (not a workspace)

6. **Coding agent receives system prompt**
   - Given Minty's prompt.md
   - When the coding agent launches
   - Then `--append-system-prompt-file` points to Minty's prompt

## Metadata
- **Complexity**: Medium
- **Labels**: minty, cli, sprint-6
- **Required Skills**: Rust, clap CLI, process exec
