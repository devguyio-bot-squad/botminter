---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Filesystem Profile API

## Description
Switch all public profile access functions in `profile.rs` from `include_dir!` reads to filesystem reads from `~/.config/botminter/profiles/`. The `include_dir!` static remains in the binary but is only accessed by `bm profiles init`. All other commands read from disk.

## Background
This is the critical transition — the data source for profiles shifts from compile-time to runtime. Disk-based profiles enable operator customization: editing descriptions, adding knowledge, tweaking member skeletons. The embedded static becomes a seed/reset mechanism only.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Profile Externalization")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 8)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `profile.rs` — all public functions switch to filesystem reads:
   - `profiles_dir() -> PathBuf`: returns `dirs::config_dir().join("botminter").join("profiles")`
   - `list_profiles() -> Result<Vec<String>>`: reads from disk directory
   - `read_manifest(name: &str) -> Result<ProfileManifest>`: reads from disk YAML
   - All file/directory access for extraction reads from disk paths
2. Keep `include_dir!` static but restrict access:
   - Move to an `embedded` submodule or gate behind a function only called by `profiles init`
3. Update extraction functions (`extract_profile_to`, `extract_member_to`) to read from disk
4. Update test helpers that currently rely on embedded profiles to extract to tempdir first

## Dependencies
- Step 7 complete (profiles init can extract to disk)

## Implementation Approach
1. Create `profiles_dir()` helper function
2. Rewrite `list_profiles()` to use `fs::read_dir()`
3. Rewrite `read_manifest()` to use `fs::read_to_string()`
4. Move `include_dir!` static to restricted scope
5. Update extraction to iterate disk directories instead of embedded
6. Create test helper that extracts embedded profiles to a tempdir for test setup
7. Verify all commands work with disk-based profiles

## Acceptance Criteria

1. **list_profiles reads from disk**
   - Given profiles extracted to `~/.config/botminter/profiles/`
   - When `list_profiles()` is called
   - Then it returns profile names from the disk directory

2. **read_manifest reads from disk**
   - Given a profile on disk with a modified `botminter.yml`
   - When `read_manifest()` is called
   - Then it returns the disk version (reflecting modifications)

3. **Disk modifications reflected in CLI**
   - Given a profile modified on disk (e.g., description changed)
   - When `bm profiles describe <name>` runs
   - Then the output reflects the disk modification

4. **include_dir only accessed by profiles init**
   - Given the codebase
   - When searching for `include_dir!` or embedded static access
   - Then it only appears in the profiles init code path

5. **Extraction reads from disk**
   - Given `bm init` creating a team
   - When extracting a profile
   - Then files are read from `~/.config/botminter/profiles/<name>/`

6. **Test helper extracts to tempdir**
   - Given integration tests that need profiles
   - When test setup runs
   - Then profiles are extracted to a temporary directory (not the real config dir)

## Metadata
- **Complexity**: High
- **Labels**: profile-externalization, core, sprint-2
- **Required Skills**: Rust, filesystem, include_dir, test architecture
