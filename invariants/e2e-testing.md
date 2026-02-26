# Invariant: E2E Testing for External API Interactions

## Rule

Any code that constructs payloads for external APIs (GitHub GraphQL, REST, `gh` CLI commands) **MUST** have a corresponding E2E test that executes the real API call and verifies the result.

Unit tests and integration tests with mocked/absent credentials are necessary but **not sufficient** for this class of code. Serialization bugs, escaping issues, and payload format errors are invisible to tests that don't hit the real service.

## What qualifies as an external API interaction

- GraphQL mutations constructed as strings (e.g., `updateProjectV2Field`)
- `gh` CLI commands that create, modify, or query GitHub resources
- Any `Command::new("gh")` call that produces side effects

## E2E test requirements

1. **Location:** `crates/bm/tests/e2e/` — feature-gated behind `--features e2e`
2. **Auth gate:** Use `require_gh_auth!()` macro so tests skip gracefully without credentials
3. **Persistent repo cleanup:** Call `super::github::clean_persistent_repo()` at the start of each test to ensure a pristine state
4. **Use persistent repo:** Use `super::github::PERSISTENT_REPO` constant instead of creating temporary repos
5. **Isolation:** Use `tempfile::tempdir()` for HOME — never touch `~/.botminter`
6. **Verify the remote state:** Don't just check exit code and stdout. Query the API to confirm the mutation actually took effect (e.g., `list_project_status_options` after sync, `list_labels` after label creation)
7. **Test idempotency:** Re-run the command and verify it succeeds again

## Persistent Test Repository

**Repository:** `devguyio-bot-squad/test-team-repo`
**Access via:** `super::github::PERSISTENT_REPO` constant

All E2E tests share a single persistent repository. Each test MUST call `super::github::clean_persistent_repo()` at the start to ensure a clean state. This function:
- Deletes all custom labels (keeps GitHub defaults like "bug", "enhancement")
- Closes and deletes all issues
- Deletes all projects

**Rationale:** Using a persistent repo instead of creating/deleting temp repos for each test is:
- Faster (no repo creation API calls)
- More reliable (avoids rate limits)
- Simpler (no RAII cleanup needed)
- More realistic (tests run against a stable repo URL)

## Test Pattern

```rust
#[test]
fn e2e_my_feature() {
    require_gh_auth!();

    // Clean the persistent repo to pristine state
    super::github::clean_persistent_repo();

    // Use temp dir for local state (HOME, workzone)
    let tmp = tempfile::tempdir().unwrap();

    // Set up local team config pointing to persistent repo
    setup_team_with_github(
        tmp.path(),
        "my-test",
        "scrum",
        super::github::PERSISTENT_REPO,
    );

    // Run the CLI operation
    let mut cmd = bm_cmd();
    cmd.args(["my-command", ...])
        .env("HOME", tmp.path());
    assert_cmd_success(&mut cmd);

    // Verify the remote state changed
    let labels = super::github::list_labels(super::github::PERSISTENT_REPO);
    assert!(labels.contains(&"my-label".to_string()));

    // Test idempotency
    let mut cmd = bm_cmd();
    cmd.args(["my-command", ...])
        .env("HOME", tmp.path());
    assert_cmd_success(&mut cmd);  // Should succeed again
}
```

## Rationale

This invariant exists because a GraphQL escaping bug (`\\\"` vs `\"`) shipped past 12 unit tests, 4 integration tests, and clippy — and was only caught by a user running the command manually. An E2E test that hits the real GitHub API catches this class of bug in under 15 seconds.

## Running E2E Tests

```bash
# All E2E tests (requires gh auth)
cargo test -p bm --features e2e --test e2e -- --test-threads=1

# Specific E2E test
cargo test -p bm --features e2e --test e2e e2e_my_feature -- --exact
```

**Prerequisites:**
- `gh auth status` must succeed
- `GH_TOKEN` env var (or gh CLI auth)
- Write access to `devguyio-bot-squad/test-team-repo`

## When to skip E2E tests

E2E tests are required for code that hits external APIs. They are NOT required for:
- Pure business logic
- File system operations (covered by integration tests)
- CLI argument parsing (covered by unit tests)
- Code that only reads configuration
