---
status: diagnosed
trigger: "Session tests assert the error path without launching real Claude/Ralph when binaries are present"
created: 2026-03-05T09:00:00Z
updated: 2026-03-05T09:05:00Z
---

## Current Focus

hypothesis: Tests don't restrict PATH, so which::which() finds real binaries; `if let Err` silently accepts Ok
test: Read session.rs test code and production code
expecting: Confirm tests have no PATH manipulation and use non-asserting error check
next_action: Return diagnosis

## Symptoms

expected: Unit tests `interactive_session_missing_claude_errors` and `oneshot_session_missing_ralph_errors` should test that the functions error when the binary is missing, without launching real processes
actual: Both tests use `if let Err(e) = result` which silently passes when claude/ralph are in PATH. Real processes were launched — Ralph ran 8 iterations ($0.16, 191s), Claude made a real API call.
errors: No errors — silent pass on Ok path is the problem
reproduction: Run `cargo test -p bm --lib -- session::tests` when claude and ralph are in PATH
started: Present since tests were written

## Eliminated

(none needed — root cause is immediately evident from code)

## Evidence

- timestamp: 2026-03-05T09:02:00Z
  checked: session.rs lines 87-132 (test module)
  found: |
    Two defects work together to cause real process launches:
    1. NO PATH RESTRICTION: Neither test modifies PATH before calling the production function.
       `which::which("claude")` and `which::which("ralph")` search the real system PATH.
       When binaries exist (dev environment), the check passes and execution continues.
    2. NON-ASSERTING ERROR CHECK: Both tests use `if let Err(e) = result { ... }` with no
       `else` branch. When result is Ok (binary found, process launched), the if-let doesn't
       match and the test body completes successfully — a silent no-op.
    Combined: binary found -> process launched -> Ok returned -> test silently passes.
  implication: Tests named "missing_X_errors" never actually assert that an error occurs

- timestamp: 2026-03-05T09:03:00Z
  checked: session.rs lines 14-18 and 56-60 (production code binary checks)
  found: |
    Production code uses `which::which("claude")` / `which::which("ralph")` to check PATH.
    This is correct production behavior. The problem is purely in the tests — they don't
    control the environment to ensure which::which fails.
  implication: Production code is fine; only tests need fixing

- timestamp: 2026-03-05T09:04:00Z
  checked: Test comments (lines 93-95, 121)
  found: |
    Comments explicitly acknowledge the flaw:
    - "In test environment, 'claude' is unlikely to be in PATH"
    - "But if it is, this test is still valid — it will try to run"
    - "Either 'claude not found' or it runs — both are valid"
    - "ralph might be in PATH in the test environment"
    These comments reveal the tests were written with awareness that they'd be no-ops
    when binaries are present, but incorrectly treat that as acceptable.
  implication: The design intent was wrong — tests that don't assert are not tests

## Resolution

root_cause: |
  Two co-occurring defects in session.rs test module (lines 91-131):

  **Defect 1 — No PATH isolation:** Tests call production functions without restricting
  PATH. When `claude` or `ralph` binaries exist on the system (common in dev environments),
  `which::which()` succeeds, and the functions proceed to launch real processes. The test
  for `oneshot_session_missing_ralph_errors` launched a real 8-iteration Ralph orchestration
  loop; `interactive_session_missing_claude_errors` made a real Claude API call.

  **Defect 2 — Non-asserting error check:** Both tests use `if let Err(e) = result { ... }`
  with no else branch. When result is Ok (binary found and process ran), the pattern doesn't
  match and the test completes with zero assertions — a silent pass that tests nothing.

  These are test-only defects. The production code in `interactive_claude_session` and
  `oneshot_ralph_session` is correct.

fix: (diagnose only — not applied)
verification: (diagnose only)
files_changed: []
