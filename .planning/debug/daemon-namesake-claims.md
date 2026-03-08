---
status: diagnosed
trigger: "Daemon tests verify their namesake claims — child termination and per-member log creation"
created: 2026-03-05T00:00:00Z
updated: 2026-03-05T00:00:00Z
---

## Current Focus

hypothesis: Two tests have structural defects that prevent them from verifying what their names claim
test: Code review of daemon_lifecycle.rs lines 250-426
expecting: Confirm conditional guards silently skip core assertions
next_action: Document root cause

## Symptoms

expected: daemon_stop_terminates_running_members should reliably verify child process termination. daemon_per_member_log_created should assert per-member log content.
actual: daemon_stop_terminates_running_members wraps child kill verification in `if stub_pid_file.exists()` — if daemon never launches member in 4s window, the distinctive claim is silently skipped. daemon_per_member_log_created has double-conditional block with zero assertions — per-member log mention check is decorative eprintln only.
errors: None — tests pass but don't verify what their names claim
reproduction: Test 12 in UAT. See daemon_lifecycle.rs lines 304 and 409.
started: Discovered during UAT of Phase 01

## Eliminated

(none — root cause identified on first pass)

## Evidence

- timestamp: 2026-03-05T00:00:00Z
  checked: daemon_stop_terminates_running_members (lines 250-317)
  found: |
    Lines 304-316: The entire child-process-termination check is inside `if stub_pid_file.exists()`.
    The stub ralph writes its PID to `$PWD/.ralph-stub-pid` on launch. But the daemon must:
    (1) complete a poll cycle (2s interval), (2) decide to launch a member, (3) actually spawn ralph.
    The test sleeps only 4 seconds (line 279). If the daemon doesn't reach the member-launch step
    in that window (e.g., gh auth fails before launch, or poll timing is off), the stub PID file
    never gets created, and the `if` block is skipped entirely.

    The test then passes — having only verified that the DAEMON process died (lines 288-294),
    NOT that child member processes were terminated. The test name claims "terminates_running_members"
    but the member-termination check is optional.
  implication: The test's distinctive claim (child termination) is race-dependent and silently skippable.

- timestamp: 2026-03-05T00:00:00Z
  checked: daemon_per_member_log_created (lines 387-426)
  found: |
    Lines 409-417: Double-conditional block:
      if daemon_log.exists() {           // outer condition
        if log_content.contains(...) {   // inner condition
          eprintln!("...");              // decorative output only
        }
      }

    Neither conditional has an `else` branch or assertion. The only actual assertion in the
    test is on line 421-425: `assert!(daemon_log.exists(), ...)` — which checks that the
    daemon's OWN log file exists. This has nothing to do with per-member logs.

    The test name is "daemon_per_member_log_created" but it never:
    - Checks if a per-member log file exists
    - Asserts on per-member log content
    - Even constructs the expected per-member log path for assertion

    The double-conditional block on lines 409-417 checks if the daemon log MENTIONS the
    member log filename, but (a) this is inside two if-guards so it's entirely skippable,
    and (b) even if reached, it only eprints — no assertion.
  implication: Test verifies daemon log existence (a different, weaker claim) while its name claims per-member log creation verification.

## Resolution

root_cause: |
  Two independent but structurally identical defects — both tests wrap their namesake verification
  in conditional guards that silently skip when conditions aren't met:

  1. daemon_stop_terminates_running_members (line 304):
     - The child-termination check is inside `if stub_pid_file.exists()`.
     - The stub PID file only exists if the daemon successfully launched a member within the 4s sleep window.
     - Member launch depends on daemon poll timing AND the launch not failing early (e.g., gh auth).
     - When the condition is false, the test passes having only verified daemon death, not member termination.
     - FIX DIRECTION: Replace the 4s sleep + conditional with a polling loop that waits for
       stub_pid_file to appear (with timeout), then unconditionally assert child termination.

  2. daemon_per_member_log_created (line 409):
     - The per-member log check is inside `if daemon_log.exists() { if log_content.contains(...) { eprintln } }`.
     - Zero assertions in the conditional block — only decorative eprintln.
     - The only assertion (line 421) checks daemon log existence, not per-member log.
     - The test verifies something completely different from what its name claims.
     - FIX DIRECTION: Construct the expected per-member log path and assert! it exists.
       If member launch is unreliable in test env, either (a) wait for it with timeout,
       or (b) rename the test to reflect what it actually verifies (daemon_log_created).

fix: ""
verification: ""
files_changed: []
