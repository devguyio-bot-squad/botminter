# E2E Testing Patterns & Helpers

This document describes the helpers, patterns, and constraints for writing E2E tests in `crates/bm/tests/e2e/`.

## Overview

E2E tests hit real GitHub APIs to verify that commands produce the expected side effects. They are feature-gated behind `--features e2e` and must be run serially with `--test-threads=1`.

## Persistent Test Repository

All E2E tests share a single persistent repository: **`devguyio-bot-squad/test-team-repo`**

### Why a persistent repo?

Using a persistent repo instead of creating/deleting temp repos for each test:
- ✅ **Faster** — no repo creation/deletion API calls (saves ~5-10 seconds per test)
- ✅ **More reliable** — avoids GitHub API rate limits
- ✅ **Simpler** — no RAII cleanup guards needed
- ✅ **More realistic** — tests run against a stable repo URL (same as production)

### Cleanup requirement

**Every E2E test MUST call `super::github::clean_persistent_repo()` at the start.** This ensures the repo is in a pristine state before the test runs.

The cleanup function:
1. Deletes all custom labels (keeps GitHub defaults: "bug", "enhancement", "documentation", etc.)
2. Closes and deletes all issues
3. Deletes all GitHub Projects

## Available Helpers

### Repository Management

```rust
// Constant for the persistent test repo
super::github::PERSISTENT_REPO  // "devguyio-bot-squad/test-team-repo"

// Ensure repo exists, create if needed (called automatically by clean)
super::github::ensure_persistent_repo()

// Clean repo to pristine state (REQUIRED at start of each test)
super::github::clean_persistent_repo()
```

### Label Operations

```rust
// List label names
let labels = super::github::list_labels(repo);  // Vec<String>

// List labels with colors
let labels = super::github::list_labels_json(repo);  // Vec<(String, String)>
```

### Issue Operations

```rust
// List issue titles
let issues = super::github::list_issues(repo);  // Vec<String>
```

### Project Operations

```rust
// Create a temporary project (auto-deleted on drop)
let project = super::github::TempProject::new("devguyio-bot-squad", "My Board")?;

// List project status options
let options = super::github::list_project_status_options(
    "devguyio-bot-squad",
    project.number
);  // Vec<String>
```

### Authentication Check

```rust
// Check if gh CLI is authenticated
if !super::github::gh_auth_ok() {
    eprintln!("SKIP: gh not authenticated");
    return;
}

// Or use the macro (preferred)
require_gh_auth!();
```

## Test Pattern Template

```rust
#[test]
fn e2e_my_feature() {
    require_gh_auth!();

    // 1. Clean the persistent repo
    super::github::clean_persistent_repo();

    // 2. Create temp dir for local state
    let tmp = tempfile::tempdir().unwrap();

    // 3. Set up local team config
    setup_team_with_github(
        tmp.path(),
        "test-team",
        "scrum",
        super::github::PERSISTENT_REPO,
    );

    // 4. Run the CLI operation
    let mut cmd = bm_cmd();
    cmd.args(["my-command", "--arg", "value"])
        .env("HOME", tmp.path());
    let output = assert_cmd_success(&mut cmd);

    // 5. Verify remote state (don't just check stdout)
    let labels = super::github::list_labels(super::github::PERSISTENT_REPO);
    assert!(labels.contains(&"expected-label".to_string()));

    // 6. Test idempotency (re-run should succeed)
    let mut cmd = bm_cmd();
    cmd.args(["my-command", "--arg", "value"])
        .env("HOME", tmp.path());
    assert_cmd_success(&mut cmd);
}
```

## Helper Functions in init_to_sync.rs

### `find_profile_with_roles(min_roles: usize) -> (String, Vec<String>)`

Finds a profile with at least `min_roles` roles. Returns `(profile_name, roles)`.

```rust
let (profile_name, roles) = find_profile_with_roles(2);
let role_1 = &roles[0];
let role_2 = &roles[1];
```

### `setup_team_with_github(tmp: &Path, name: &str, profile: &str, repo: &str) -> PathBuf`

Sets up a local team directory with git remote pointing to the GitHub repo.

```rust
setup_team_with_github(
    tmp.path(),
    "my-team",
    "scrum",
    super::github::PERSISTENT_REPO,
);
```

### `bootstrap_labels(repo: &str, profile: &str)`

Creates labels from the profile manifest on the GitHub repo.

```rust
bootstrap_labels(super::github::PERSISTENT_REPO, "scrum");
```

### `create_fake_fork(tmp: &Path, name: &str) -> PathBuf`

Creates a local git repo to use as a fake project fork.

```rust
let fork = create_fake_fork(tmp.path(), "my-project");
let fork_url = fork.to_string_lossy().to_string();
```

### `bm_cmd() -> Command`

Returns a `Command` configured to run the `bm` binary.

```rust
let mut cmd = bm_cmd();
cmd.args(["hire", "architect", "--name", "alice"])
    .env("HOME", tmp.path());
```

### `assert_cmd_success(cmd: &mut Command) -> String`

Runs the command, asserts success, returns stdout.

```rust
let output = assert_cmd_success(&mut cmd);
assert!(output.contains("Hired architect as alice"));
```

## Constraints & Best Practices

### DO

✅ **Always clean the repo** at the start of each test
✅ **Use temp dirs** for HOME and workzone (never touch `~/.botminter`)
✅ **Verify remote state** after mutations (call `list_labels`, `list_issues`, etc.)
✅ **Test idempotency** by re-running the command
✅ **Use descriptive team names** (e.g., `"e2e-lifecycle"`, `"e2e-labels"`)
✅ **Check for specific values** in assertions (not just "is non-empty")

### DON'T

❌ **Don't create TempRepo** — use `PERSISTENT_REPO` instead
❌ **Don't skip cleanup** — every test must call `clean_persistent_repo()`
❌ **Don't rely only on stdout** — verify the actual GitHub state changed
❌ **Don't hardcode repo URLs** — use `PERSISTENT_REPO` constant
❌ **Don't run tests in parallel** — always use `--test-threads=1`
❌ **Don't leave state behind** — the cleanup handles it, but be aware

## Running E2E Tests

```bash
# All E2E tests
cargo test -p bm --features e2e --test e2e -- --test-threads=1

# Specific test
cargo test -p bm --features e2e --test e2e e2e_init_hire_sync -- --exact --nocapture

# Skip auth check (will skip tests requiring gh)
cargo test -p bm --features e2e --test e2e -- --test-threads=1
```

## Debugging E2E Tests

### View repo state

```bash
# List labels
gh label list -R devguyio-bot-squad/test-team-repo

# List issues
gh issue list -R devguyio-bot-squad/test-team-repo --state all

# List projects
gh project list --owner devguyio-bot-squad
```

### Manual cleanup

```bash
# Delete all custom labels
for label in $(gh label list -R devguyio-bot-squad/test-team-repo --json name -q '.[].name'); do
  gh label delete "$label" -R devguyio-bot-squad/test-team-repo --yes
done

# Delete all issues
for num in $(gh issue list -R devguyio-bot-squad/test-team-repo --state all --json number -q '.[].number'); do
  gh issue delete "$num" -R devguyio-bot-squad/test-team-repo --yes
done
```

## Common Pitfalls

### Forgetting to clean the repo

**Problem:** Test fails because previous test left labels/issues behind.

**Solution:** Always call `super::github::clean_persistent_repo()` at the start.

### Not verifying remote state

**Problem:** Command succeeds but doesn't actually create the label on GitHub.

**Solution:** After the command, call `list_labels()` and assert the label exists.

### Hardcoding repo URLs

**Problem:** Tests break if the repo is renamed or moved.

**Solution:** Always use `super::github::PERSISTENT_REPO` constant.

### Running tests in parallel

**Problem:** Tests interfere with each other (race conditions on shared repo).

**Solution:** Always run with `--test-threads=1`.

## When to Add an E2E Test

Add an E2E test when:
- You're adding a new `gh` CLI command construction
- You're modifying GraphQL payload generation
- You're changing how labels, issues, or projects are created/modified
- You're adding a new GitHub API interaction

See `invariants/e2e-testing.md` for the full invariant.
