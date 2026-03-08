# Flaky Test Policy

Flaky tests must be root-caused and fixed; persistent flakes are tracked, never ignored.

## Rule

All flaky tests **MUST** be root-caused and fixed immediately, even if the flaky test is unrelated to the changes being introduced. A test is flaky if it passes and fails non-deterministically on the same code.

When a flaky test is encountered:

1. **Root-cause it.** Investigate the non-determinism — race condition, timing sensitivity, external service instability, state leakage between tests, etc.
2. **Fix it.** Apply the fix and verify the test passes reliably.
3. **If the flakiness persists for more than 10 minutes of debugging,** stop and track it:
   - Add an entry to `crates/bm/tests/README.md` under a `## Known Flaky Tests` section.
   - Each entry must include: test name, observed failure mode, suspected root cause, and date logged.
   - Do **NOT** mark the test with `#[ignore]` — it must keep running so regressions stay visible.

## Applies To

- All test types: unit tests, integration tests, and E2E tests.
- Any test that exhibits non-deterministic pass/fail behavior on unchanged code.
- Flaky tests discovered while working on unrelated changes — you own what you see.

Does **NOT** apply to:
- Tests that fail deterministically (those are just bugs — fix them normally).
- Tests that fail due to missing credentials or environment setup (those should use skip guards like `require_gh_auth!()`).

## Examples

**Compliant:**

```markdown
<!-- crates/bm/tests/README.md -->
## Known Flaky Tests

| Test | Failure mode | Suspected cause | Date |
|------|-------------|-----------------|------|
| `e2e_init_to_sync` | Timeout on label creation | GitHub API rate limiting under load | 2026-03-04 |
```

The test remains enabled and keeps running.

**Also compliant:** encountering a flaky test while working on an unrelated feature, spending 10 minutes investigating, then adding the README entry and continuing with the original work.

**Violating:**

```rust
#[ignore] // flaky, will fix later
#[test]
fn e2e_init_to_sync() { ... }
```

Silencing the test hides regressions. No README entry means no tracking.

**Also violating:** encountering a flaky test while running tests locally for unrelated changes and skipping it with "not my change, not my problem." The invariant applies regardless of what triggered the test run.

## Rationale

Flaky tests erode trust in the test suite. Leaving them red blocks progress; leaving them untracked lets regressions hide. The 10-minute threshold prevents rabbit-holing on transient issues while ensuring genuine bugs get fixed immediately. Requiring action regardless of change ownership prevents the bystander effect — flaky tests only get fixed when someone takes responsibility, and that someone is whoever encounters them first, whether human or coding agent.
