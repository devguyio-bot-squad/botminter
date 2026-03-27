---
status: pending
created: 2026-03-17
started: null
completed: null
---
# Task: `bm bootstrap` Command

## Description

New CLI command that provisions an isolated Fedora VM for running a BotMinter team. Uses Lima for cross-platform VM management with cloud-init for declarative provisioning.

## Background

Currently, operators manually set up environments before using BotMinter. `bm bootstrap` automates this: one command creates a fully provisioned Fedora VM with all tools installed.

## Reference Documentation

- Requirements: `specs/local-formation/requirements.md`
- ADR: `.planning/adrs/0008-local-formation-as-first-class-concept.md`
- Lima project: `/opt/workspace/lima/` (cloned locally)
- Lima Fedora template: `/opt/workspace/lima/templates/fedora.yaml`
- Lima default template (all config options): `/opt/workspace/lima/templates/default.yaml`
- Official install docs: `docs/content/getting-started/prerequisites.md` — install URLs for bm, ralph

## Technical Requirements

### CLI

1. Add `bm bootstrap` subcommand to CLI (`cli.rs`, `commands/mod.rs`)
2. Create `commands/bootstrap.rs` implementing the wizard
3. Support `--non-interactive --name <vm-name>` for CI/testing

### Host prerequisites

4. Check `limactl` is in PATH; if not, show install instructions:
   - macOS: `brew install lima`
   - Linux: link to Lima releases or `brew install lima`
   - Windows: link to Lima WSL2 docs

### Lima template generation

5. Generate a Lima YAML config with:
   - Base image: Fedora Cloud (x86_64 and aarch64 URLs from Lima's fedora template)
   - `provision:` scripts that install all tools (see below)
   - Resource defaults: 4 CPUs, 8GB RAM, 100GB disk (configurable via flags)
   - Home directory mount (Lima default)

### Tool installation via cloud-init provision scripts

6. System packages: `dnf install -y git jq curl gh just gnome-keyring podman`
7. `bm`: cargo-dist installer from `botminter/botminter` GitHub releases (`curl ... | sh`)
8. `ralph`: cargo-dist installer from `botminter/ralph-orchestrator` GitHub releases (`curl ... | sh`). **MUST use botminter fork.**
9. `claude`: native binary installer from claude.ai/code
10. See `docs/content/getting-started/prerequisites.md` for exact install URLs

### VM lifecycle

11. `limactl create --name=<name> <template-path>` to create the VM
12. `limactl start <name>` to start it
13. Verify the VM is running and SSH is accessible

### Config schema

14. Add `vms: Vec<VmEntry>` to `BotminterConfig` (global)
15. `VmEntry { name: String }`
16. Add `vm: Option<String>` to `TeamEntry` for team-to-VM linking

### Idempotency

17. Each step checks if already done:
    - VM exists (`limactl list`)? → skip create
    - VM running? → skip start
    - Config already has this VM? → skip config write

## Dependencies

- `config.rs` — new `VmEntry` struct and `vms` field
- Existing CLI infrastructure (`cli.rs`, `commands/mod.rs`)

## Implementation Approach

1. Add `VmEntry` struct and `vms: Vec<VmEntry>` to `BotminterConfig`
2. Add `Bootstrap` variant to CLI subcommands with `--non-interactive`, `--name`, `--cpus`, `--memory`, `--disk` flags
3. Write Lima template as a Rust string template (or use `serde_yaml` to build it)
4. Shell out to `limactl create` and `limactl start` via `std::process::Command`
5. Each step returns `Result<StepOutcome>` (Completed / AlreadyDone / Skipped)

## Acceptance Criteria

1. **Fresh bootstrap**
   - Given a system with `limactl` installed
   - When the operator runs `bm bootstrap` and provides a VM name
   - Then a Fedora VM is created, started, tools are installed, and config is updated

2. **Idempotent re-run**
   - Given `bm bootstrap` has already completed
   - When the operator runs `bm bootstrap` again with the same name
   - Then it detects the existing VM, skips creation, and exits cleanly

3. **Missing prerequisite**
   - Given `limactl` is not installed
   - When the operator runs `bm bootstrap`
   - Then it shows platform-specific install instructions and exits

4. **Operator config updated**
   - Given `bm bootstrap` completes
   - When `~/.botminter/config.yml` is read
   - Then it contains a `vms` entry with the VM name

5. **Non-interactive mode**
   - Given `limactl` is installed
   - When `bm bootstrap --non-interactive --name bm-test` is run
   - Then bootstrap completes without prompts

6. **Tools available in VM**
   - Given `bm bootstrap` completed
   - When the operator runs `limactl shell <name> -- which bm ralph claude gh git just`
   - Then all tools are found in PATH

## Metadata
- **Complexity**: Medium
- **Labels**: formation, cli, bootstrap, lima
- **Required Skills**: Rust, Lima/cloud-init YAML, CLI design
