---
status: pending
created: 2026-03-17
started: null
completed: null
---
# Task: Bootstrap E2E Test

## Description

Automated e2e test that verifies `bm bootstrap` and `bm attach` work end-to-end: creates a Fedora VM via Lima, verifies tools are installed, runs the core BotMinter workflow inside the VM, and tears down.

## Background

With Lima as the VM provider, we can fully automate the bootstrap verification. Lima VMs can be created, queried, and destroyed programmatically via `limactl`. The test uses the existing `TestEnv`/`TestCommand` pattern (ADR-005) and runs as a separate e2e scenario.

## Reference Documentation

- Requirements: `specs/local-formation/requirements.md`
- Lima Docker template (provisioning + probes example): `/opt/workspace/lima/templates/docker.yaml`
- Existing e2e test patterns: `crates/bm/tests/e2e/`
- TestEnv: `crates/bm/tests/e2e/test_env.rs`

## Technical Requirements

### E2E scenario

1. Add a `bootstrap` e2e scenario in `crates/bm/tests/e2e/`
2. The scenario runs as a separate CI job (nightly / on-demand) due to VM creation time (~2-5 min)
3. Requires `limactl` in PATH — this is mandatory, not optional

### Test cases

4. **Bootstrap creates VM**
   - Run `bm bootstrap --non-interactive --name e2e-bootstrap-test`
   - Assert `limactl list --json` shows the VM as "Running"
   - Assert `~/.botminter/config.yml` contains the VM in `vms:`

5. **Tools available in VM**
   - Run `limactl shell e2e-bootstrap-test -- which bm ralph claude gh git just`
   - Assert exit code 0

6. **Idempotent re-run**
   - Run `bm bootstrap --non-interactive --name e2e-bootstrap-test` again
   - Assert exit code 0 (skips, no error)

7. **Attach resolves VM**
   - Run `bm attach` verification (can't test interactive shell, but can verify resolution logic)
   - Assert config resolution finds the VM

8. **BotMinter workflow inside VM**
   - Run `limactl shell e2e-bootstrap-test -- bm init --non-interactive ...` (with test GitHub token/org)
   - Assert team repo is created
   - Run hire, sync, verify workspace exists

9. **Keyring works inside VM**
   - Run bridge identity onboard inside the VM
   - Assert credential is stored and retrievable

### Cleanup

10. `limactl delete --force e2e-bootstrap-test` in test teardown (Drop or explicit cleanup)
11. Clean up any GitHub repos created during the workflow test

### Lima probes

12. The generated Lima template MUST include probes that wait for provisioning to complete:
    ```yaml
    probes:
    - script: |
        #!/bin/bash
        if ! timeout 180s bash -c "until command -v bm >/dev/null 2>&1; do sleep 3; done"; then
          echo >&2 "bm not installed yet"
          exit 1
        fi
      hint: See /var/log/cloud-init-output.log
    ```

## Dependencies

- Tasks 1 and 2 (`bm bootstrap` and `bm attach`) must be complete
- `limactl` installed on the test machine
- `TESTS_GH_TOKEN` and `TESTS_GH_ORG` for the workflow test cases

## Implementation Approach

1. Create `crates/bm/tests/e2e/bootstrap.rs` with the scenario
2. Skip if `limactl` is not in PATH (like tg-mock does with podman)
3. Use `TestEnv` for hermetic environment
4. Each test case runs `limactl shell <vm> -- <command>` via `TestCommand`
5. Cleanup via `limactl delete --force` in Drop guard

## Acceptance Criteria

1. **Bootstrap e2e passes**
   - Given `limactl` is installed
   - When the bootstrap e2e scenario runs
   - Then all test cases pass (VM created, tools installed, workflow works, keyring works)

2. **Clean teardown**
   - Given the e2e test completed (pass or fail)
   - When checking `limactl list`
   - Then the test VM is deleted

4. **No regressions**
   - Given the full test suite
   - When `just test` runs
   - Then all existing tests still pass (bootstrap e2e skipped if no limactl)

## Metadata
- **Complexity**: Medium
- **Labels**: formation, e2e, testing, lima
- **Required Skills**: Rust, e2e test patterns, Lima CLI
