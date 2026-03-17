# Local Formation — Bootstrap & Attach

## Objective

Implement `bm bootstrap` and `bm attach` commands that provision and connect to isolated Fedora VMs via Lima. After this work, an operator can go from a fresh system to a running team in two commands.

## Spec Directory

`specs/local-formation/`

## Execution Order

1. `tasks/task-01-bm-bootstrap.code-task.md` — `bm bootstrap` command
2. `tasks/task-02-bm-attach.code-task.md` — `bm attach` command
3. `tasks/task-03-vm-verification.code-task.md` — automated e2e test: creates VM, verifies tools, runs BotMinter workflow, tests keyring, tears down

## Key Design Decisions

### Lima as the VM provider
Lima provides cross-platform VMs (Linux: KVM, macOS: Apple VZ, Windows: WSL2) with built-in cloud-init, file sharing, port forwarding, and SSH. This eliminates user accounts, polkit, machinectl, toolbox, D-Bus plumbing, and keyring scripts from the previous design.

### Cloud-init provisioning
All tool installation happens declaratively via Lima's `provision:` scripts in the template YAML. No imperative wizard steps for tool installation — cloud-init runs once on first boot.

### Template generation
`bm bootstrap` generates a Lima YAML template at runtime (not a static file). This allows:
- Embedding the correct install URLs for `bm` and `ralph` (from the current release)
- Setting resource defaults (CPUs, memory, disk)
- Future: customizing the base image

### Config schema
- `vms: Vec<VmEntry>` on `BotminterConfig` (global, not per-team)
- `vm: Option<String>` on `TeamEntry` (links team to VM)
- No password storage, no keyring entries on the host

### `bm attach` is just `limactl shell`
Uses `CommandExt::exec()` to replace the process. No wrapper, no proxy — just SSH into the VM.

## Requirements

1. `bm bootstrap` MUST create a Fedora VM via `limactl create` with cloud-init provisioning
2. Cloud-init MUST install: `git`, `jq`, `curl`, `gh`, `just`, `gnome-keyring`, `podman`, `bm`, `ralph`, `claude`
3. `bm` and `ralph` MUST be installed from `botminter/` GitHub releases (cargo-dist installers), NOT upstream
4. `bm bootstrap` MUST be idempotent — re-running skips existing VMs
5. `bm bootstrap` MUST support `--non-interactive --name <vm-name>`
6. `bm attach` MUST resolve the VM from config (team flag → single VM → prompt)
7. `bm attach` MUST exec into `limactl shell` (process replacement, not spawn+wait)
8. Both commands MUST show clear errors with install instructions if `limactl` is not found
9. `just test` MUST pass after all changes

### E2E testing
- Bootstrap e2e test MUST create a VM, verify tools, run the BotMinter workflow inside it, and tear down
- `limactl` is a mandatory prerequisite — the test MUST fail if it's not available
- The Lima template MUST include probes that wait for provisioning to complete before tests run
- The test SHOULD run as a separate CI job (nightly/on-demand) due to VM creation time (~2-5 min)
- Use existing TestEnv/TestCommand patterns; run commands inside VM via `limactl shell <vm> -- <command>`

## Acceptance Criteria

1. **(Regression)** All existing tests pass — `just test` is green
2. **Bootstrap creates VM**
   - Given `limactl` is installed
   - When `bm bootstrap --non-interactive --name bm-test` is run
   - Then a Fedora VM named `bm-test` is created and running
3. **Tools available**
   - Given a bootstrapped VM
   - When `limactl shell bm-test -- which bm ralph claude` is run
   - Then all three are found
4. **Attach works**
   - Given a running VM
   - When `bm attach` is run
   - Then an interactive shell opens in the VM
5. **Config persisted**
   - Given bootstrap completed
   - When `~/.botminter/config.yml` is read
   - Then it contains the VM entry
6. **E2E passes**
   - Given `limactl` is installed and `TESTS_GH_TOKEN`/`TESTS_GH_ORG` are set
   - When the bootstrap e2e scenario runs
   - Then VM is created, tools are verified, BotMinter workflow completes, keyring works, VM is cleaned up
7. **limactl is mandatory**
   - Given `limactl` is NOT installed
   - When the e2e suite runs
   - Then the bootstrap scenario fails with a clear error

## Key References

- Lima project: `/opt/workspace/lima/`
- Lima Fedora template: `/opt/workspace/lima/templates/fedora.yaml`
- Lima image URLs: `/opt/workspace/lima/templates/_images/fedora-43.yaml`
- Lima default template (all options): `/opt/workspace/lima/templates/default.yaml`
- Lima provisioning example: `/opt/workspace/lima/templates/docker.yaml`
- BotMinter install docs: `docs/content/getting-started/prerequisites.md`
- Config module: `crates/bm/src/config/mod.rs`
- CLI module: `crates/bm/src/cli.rs`
- Commands module: `crates/bm/src/commands/mod.rs`
