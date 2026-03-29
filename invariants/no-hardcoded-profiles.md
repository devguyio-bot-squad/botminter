# No Hardcoded Profile Data

Code and tests MUST NOT depend on specific profile contents — profiles are dynamic data, not compile-time constants.

## Rule

Code and tests **MUST NOT** hardcode profile names, role names, status values, label names, view names, display names, or any other profile-derived data. If a profile were deleted or renamed, no code or test should break.

Tests that need profile data **MUST** use fixtures generated from the current profiles. These fixtures should be updated when profiles change, not maintained by hand.

When testing profile-related functionality:
1. **Read profile data dynamically** — use the profile API (`list_profiles`, `read_manifest`, `list_roles`, etc.) to discover available profiles and their contents at test time.
2. **Use generated fixtures** when dynamic reads are impractical — fixtures that are derived from profiles and updated alongside them, not hand-written string literals.
3. **Test behavior, not content** — assert that "all profiles have at least one role" rather than "scrum has architect, human-assistant, and chief-of-staff."

## Applies To

- All code in `crates/bm/src/` and `crates/bm/tests/`.
- Unit tests, integration tests, and E2E tests.
- Any assertion, match arm, or conditional that references profile-specific values.

Does **NOT** apply to:
- Profile definition files themselves (`profiles/*/botminter.yml`, `profiles/*/PROCESS.md`, etc.) — those _are_ the source of truth.
- Test helpers that accept a profile name as a parameter (e.g., `setup_team("scrum")` is fine — the caller picks the profile, the helper doesn't assume one).
- Documentation and specs.

## Examples

**Compliant:**

```rust
#[test]
fn all_profiles_have_at_least_one_role() {
    for name in profile::list_profiles() {
        let manifest = profile::read_manifest(&name).unwrap();
        assert!(!manifest.roles.is_empty(), "{name} has no roles");
    }
}
```

Tests behavior across all profiles — survives profile additions and deletions.

**Violating:**

```rust
#[test]
fn test_profile_roles() {
    let profiles = profile::list_profiles();
    assert!(profiles.contains(&"scrum".to_string()));

    let manifest = profile::read_manifest("scrum").unwrap();
    assert_eq!(manifest.roles.len(), 3);
    assert_eq!(manifest.roles[0].name, "architect");
}
```

Breaks if the scrum profile is renamed, a role is added, or roles are reordered.

## Rationale

Profiles are user-facing, editor-maintained data — they change often. Hardcoding their contents in tests creates a hidden coupling that turns every profile edit into a test maintenance chore. Worse, it creates the illusion that profile contents are guaranteed by the codebase, when they are actually governed by profile authors. Tests should verify the machinery that reads and applies profiles, not the profiles themselves.
