---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: --force Flag and Overwrite Behavior

## Description
Implement the `--force` flag for `bm profiles init` and the per-profile overwrite/skip interactive prompting when profiles already exist on disk.

## Background
Operators may run `bm profiles init` multiple times — after binary upgrades, to reset customizations, or to add new profiles. Without `--force`, the command should prompt per-profile to overwrite or skip. With `--force`, it overwrites all silently.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 7)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. When target directory exists and `--force` is NOT set:
   - List existing profiles at the target
   - For each embedded profile that already exists on disk: prompt "Overwrite <name>? [y/N]"
   - Overwrite only profiles the user confirms
   - Extract new profiles (not on disk) without prompting
   - Print summary of overwritten, skipped, and new profiles
2. When target directory exists and `--force` IS set:
   - Overwrite all profiles without prompting
   - Print summary
3. Handle edge case: target directory exists but is empty (treat as fresh install)

## Dependencies
- Task 1 of this step (basic profiles init working)

## Implementation Approach
1. Add existence check at the start of the init command
2. Compare embedded profile list against disk profile list
3. Implement interactive prompting (respecting TTY availability)
4. Add --force short-circuit path
5. Write tests with mock stdin or by testing the --force path directly

## Acceptance Criteria

1. **Re-run without --force prompts per profile**
   - Given profiles already exist on disk
   - When `bm profiles init` runs without `--force`
   - Then it prompts for each existing profile (overwrite or skip)

2. **Re-run with --force overwrites silently**
   - Given profiles already exist on disk
   - When `bm profiles init --force` runs
   - Then all profiles are overwritten without prompting

3. **New profiles extracted without prompting**
   - Given a new profile was added to the binary (not on disk yet)
   - When `bm profiles init` runs
   - Then the new profile is extracted without prompting

4. **Skipped profiles preserved**
   - Given a profile exists on disk with custom modifications
   - When the user skips it during prompting
   - Then the on-disk version is preserved unchanged

5. **Summary shows actions taken**
   - Given a mix of overwritten, skipped, and new profiles
   - When the command completes
   - Then the summary lists each profile with its action (overwritten/skipped/new)

6. **Empty target dir treated as fresh**
   - Given `~/.config/botminter/profiles/` exists but is empty
   - When `bm profiles init` runs
   - Then all profiles are extracted without prompting

## Metadata
- **Complexity**: Medium
- **Labels**: profile-externalization, cli, sprint-2
- **Required Skills**: Rust, interactive prompting, filesystem
