---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Auto-Prompt Pattern for Profile Initialization

## Description
Implement `ensure_profiles_initialized()` — a check that runs at the top of commands requiring profiles. If profiles aren't on disk yet, it prompts the user to initialize them inline. This creates a smooth first-run experience without a separate manual step.

## Background
With profiles now disk-based, a fresh `bm` install has no profiles on disk. Rather than requiring operators to know about `bm profiles init` first, commands that need profiles should detect the missing state and offer to fix it inline.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 8)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Implement `ensure_profiles_initialized() -> Result<()>`:
   - Check if `profiles_dir()` exists and is non-empty
   - If not: prompt "Profiles not initialized. Initialize now? [Y/n]"
   - If yes: run extraction inline (same logic as `bm profiles init`)
   - If no: print help message ("Run `bm profiles init` to set up profiles") and bail gracefully
2. Add `ensure_profiles_initialized()` call at the top of commands that require profiles:
   - `bm init`, `bm hire`, `bm teams sync`, `bm profiles list`, `bm profiles describe`, `bm roles list`
3. Handle non-interactive environments (no TTY): auto-initialize without prompting or fail with clear message

## Dependencies
- Task 1 of this step (filesystem profile API in place)

## Implementation Approach
1. Implement the check function with TTY detection
2. Add the check to each relevant command handler
3. Test the yes/no/non-interactive paths
4. Verify the inline extraction produces the same result as `bm profiles init`

## Acceptance Criteria

1. **Fresh install triggers prompt on bm init**
   - Given no profiles on disk
   - When `bm init` runs
   - Then it prompts "Profiles not initialized. Initialize now?"

2. **Prompt yes initializes and continues**
   - Given the user answers "yes" to the prompt
   - When initialization completes
   - Then the original command continues normally

3. **Prompt no aborts gracefully**
   - Given the user answers "no" to the prompt
   - When the check returns
   - Then it prints a help message and exits without error

4. **Already initialized skips prompt**
   - Given profiles already on disk
   - When `bm init` runs
   - Then no prompt is shown — the command proceeds directly

5. **All profile-dependent commands have the check**
   - Given each of: `bm init`, `bm hire`, `bm teams sync`, `bm profiles list`, `bm profiles describe`, `bm roles list`
   - When run with no profiles on disk
   - Then each triggers the initialization prompt

6. **Non-interactive environment handled**
   - Given a non-TTY environment (piped input)
   - When a profile-dependent command runs with no profiles
   - Then it either auto-initializes or fails with a clear message (not a hang)

## Metadata
- **Complexity**: Medium
- **Labels**: profile-externalization, ux, sprint-2
- **Required Skills**: Rust, TTY detection, CLI UX patterns
