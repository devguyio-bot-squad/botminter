---
status: pending
created: 2026-03-17
started: null
completed: null
---
# Task: `bm attach` Command

## Description

New CLI command that opens an interactive shell inside a team's Lima VM. Wraps `limactl shell` with team-aware VM resolution from config.

## Background

After `bm bootstrap` creates a VM, the operator needs a way to enter it. `bm attach` resolves the right VM from config and execs into `limactl shell`.

## Reference Documentation

- Requirements: `specs/local-formation/requirements.md`
- Lima shell implementation: `/opt/workspace/lima/cmd/limactl/shell.go`

## Technical Requirements

1. Add `bm attach` subcommand to CLI with optional `-t`/`--team` flag
2. Create `commands/attach.rs` implementing:
   - Resolve which VM to use:
     1. If `-t <team>` given and that team has `vm` set → use it
     2. If exactly one entry in `vms` → use it
     3. If multiple → prompt (or error in non-interactive)
   - Verify the VM exists and is running (`limactl list --json`)
   - If VM exists but stopped, offer to start it
3. Exec into `limactl shell <vm-name>` using `CommandExt::exec()` (replaces current process)
4. Clear error if no VMs configured ("No VM found. Run `bm bootstrap` first.")
5. Clear error if `limactl` not in PATH

## Dependencies

- Task 1 (`bm bootstrap`) — config fields must exist
- `config.rs` — reading `vms` and `TeamEntry.vm`

## Implementation Approach

1. Add `Attach` variant to CLI subcommands with optional `-t`/`--team`
2. Load config, resolve VM using the 3-step resolution
3. Check VM status via `limactl list --json` (parse JSON output)
4. `CommandExt::exec()` into `limactl shell <name>`

## Acceptance Criteria

1. **Successful attach**
   - Given a running Lima VM from `bm bootstrap`
   - When the operator runs `bm attach`
   - Then an interactive shell opens inside the VM

2. **Team flag resolution**
   - Given a team config with `vm: bm-alpha`
   - When `bm attach -t myteam` is run
   - Then it attaches to the `bm-alpha` VM

3. **No VM configured**
   - Given no `vms` entries in config
   - When `bm attach` is run
   - Then it fails with: "No VM found. Run `bm bootstrap` first."

4. **VM stopped — offer to start**
   - Given a VM exists but is stopped
   - When `bm attach` is run
   - Then it offers to start the VM, then attaches

5. **limactl not found**
   - Given `limactl` is not in PATH
   - When `bm attach` is run
   - Then it fails with platform-specific install instructions

## Metadata
- **Complexity**: Low
- **Labels**: formation, cli
- **Required Skills**: Rust, process exec
