# Zero Test Failures

All tests MUST pass with zero failures and zero warnings before any change is considered done.

## Rule

Every non-documentation change MUST produce zero test failures across all test layers. No exceptions for pre-existing failures, unrelated breakage, or "will fix later." A single failure at any layer blocks the change from being considered complete.

## How to Confirm

Run the full verification pipeline in order:

```bash
just clippy            # Stage 1: zero warnings
just test              # Stage 2: zero failures (unit + conformance + e2e)
just exploratory-test  # Stage 3: zero failures (real-infrastructure integration)
```

After running:

1. All three stages MUST exit 0 with no failures.
2. If any stage fails, fix the issue and re-run before the change is done.
3. Pre-existing failures MUST be fixed — they are not exemptions.

Skip the pipeline only for **documentation-only changes**: files exclusively in `docs/content/`, `*.md` outside `crates/`, or `.planning/`.

## Applies To

- Any non-documentation change to the codebase

## Examples

**Compliant:**
```
- clippy: PASS
- test: PASS (430 unit + 18 conformance + 6 e2e)
- exploratory-test: PASS (78 cases, 0 failures)
```

**Violating:**
```
- clippy: PASS
- test: PASS
- exploratory-test: not run
```

**Violating:**
```
- clippy: PASS
- test: PASS
- exploratory-test: FAIL — "Pre-existing, not my change" — skipped
```

## Rationale

The project has three verification layers catching different bug classes: clippy (compile-time), `just test` (logic), and `just exploratory-test` (real-infrastructure integration). Historically, only the first two were run consistently, missing integration regressions. Zero failures across all layers is the minimum bar for shipping.
