---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: profiles init Subcommand and Extraction Logic

## Description
Implement the `bm profiles init` command that extracts all embedded profiles from the `include_dir!` static to `~/.config/botminter/profiles/`. This bridges compile-time embedded profiles to the disk-based model. Fresh installs get all profiles written to disk.

## Background
Currently profiles are embedded in the binary at compile time via `include_dir!`. The externalization plan moves the active profile source to disk, enabling operator customization. This command performs the initial extraction. Agent tag filtering is NOT applied here — profiles are stored as-is with tags intact.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Profile Externalization")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 7)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Add `profiles init` subcommand to `cli.rs` with optional `--force` flag
2. Implement `commands/profiles_init.rs`:
   - Target: `dirs::config_dir().join("botminter").join("profiles")`
   - If target doesn't exist: create it, extract all embedded profiles
   - Print summary: number of profiles extracted, target path
3. Extraction logic:
   - Iterate embedded profiles from `include_dir!` static
   - Write all files/directories preserving structure (including `.schema/`)
   - Do NOT apply agent tag filtering (profiles stored with tags intact)
   - Create parent directories recursively as needed
4. Wire up the command in the CLI router

## Dependencies
- Steps 1-6 complete (Sprint 1 finished, profiles have new structure)

## Implementation Approach
1. Study existing `include_dir!` usage in `profile.rs`
2. Add new CLI subcommand definition
3. Implement extraction by iterating the embedded directory tree
4. Write files with correct permissions
5. Add unit tests using temp directories

## Acceptance Criteria

1. **Fresh install extracts all profiles**
   - Given no `~/.config/botminter/profiles/` directory
   - When `bm profiles init` runs
   - Then all embedded profiles are written to `~/.config/botminter/profiles/`

2. **Extracted profiles have correct structure**
   - Given a freshly extracted profile
   - When listing its contents
   - Then it has `botminter.yml`, `context.md`, `coding-agent/`, `members/`, etc.

3. **Extracted content matches embedded content**
   - Given an embedded profile file
   - When compared to its extracted counterpart
   - Then content is byte-identical

4. **Target directory created recursively**
   - Given no `~/.config/botminter/` directory at all
   - When `bm profiles init` runs
   - Then all parent directories are created

5. **Summary printed on success**
   - Given successful extraction
   - When the command completes
   - Then it prints the number of profiles extracted and the target path

6. **Unit tests use temp directory**
   - Given tests for the extraction logic
   - When tests run
   - Then they extract to a temp directory (not the real config dir)

## Metadata
- **Complexity**: Medium
- **Labels**: profile-externalization, cli, sprint-2
- **Required Skills**: Rust, include_dir, filesystem operations
