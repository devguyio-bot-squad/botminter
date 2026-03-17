# Requirements — Local Formation

## Core User Story

**From fresh system to running in two commands:**
```
bm bootstrap    # Creates a Fedora VM via Lima, provisions tools
bm attach       # Shell into the VM
bm init ...     # Create team, hire members, start working
```

Everything else (VM creation, tool installation, keyring, dev environment) is handled by `bm bootstrap`.

## Key Constraints

1. **VM-based isolation** — each team runs in its own Fedora VM managed by Lima
2. **Full dev environment** — the VM is a proper development environment where coding agents work
3. **Keyring** — gnome-keyring works natively inside the VM (own systemd, own D-Bus, own PAM)
4. **No regressions** — all existing features must keep working
5. **Cross-platform** — Lima runs on Linux (QEMU/KVM), macOS (Apple VZ / QEMU), Windows (WSL2)
6. **Fedora Cloud** — starting image; can be swapped later

## Architecture

### Why Lima

Lima provides:
- Cross-platform VMs with native hypervisors per platform
- Built-in Fedora template (`template:fedora`)
- Cloud-init provisioning via `provision:` scripts in YAML
- Automatic file sharing (home directory mounted via virtiofs/9p)
- Automatic port forwarding
- SSH built in (`limactl shell`)
- Apache 2.0 license

No user accounts, no polkit, no machinectl, no toolbox, no D-Bus plumbing, no keyring scripts. The VM is a complete, self-contained environment.

### VM model

- **One VM per team** — all members run inside the same VM
- VM name: operator chooses, `bm bootstrap` recommends `bm-<name>`
- Fedora Cloud image as the base
- Cloud-init handles all provisioning declaratively

### `bm bootstrap`

Idempotent environment provisioning. Checks `limactl` is installed, generates a Lima YAML template, creates the VM.

Prerequisites checked on the **host**:
- `limactl` installed (if not, show install instructions per platform)

What it does:
1. Checks host prerequisites
2. Asks for VM name (recommends `bm-<name>`)
3. Generates a Lima YAML config with `provision:` scripts that install:
   - System packages: `git`, `jq`, `curl`, `gh`, `just`, `gnome-keyring`, `podman`
   - `bm`: cargo-dist installer from `botminter/botminter` GitHub releases
   - `ralph`: cargo-dist installer from `botminter/ralph-orchestrator` GitHub releases (**must use botminter fork**)
   - `claude`: native binary installer from claude.ai/code
4. Creates the VM via `limactl create --name=<name> <generated-template.yaml>`
5. Starts the VM via `limactl start <name>`
6. Stores VM name in operator config

Supports `--non-interactive` mode with `--name <vm-name>` flag.

### Operator config schema

```yaml
# ~/.botminter/config.yml
workzone: ~/workzone
default_team: myteam
vms:
  - name: bm-alpha
  - name: bm-beta
teams:
  - name: myteam
    # ... existing fields ...
    vm: bm-alpha    # optional: links team to a VM
```

`bm attach` resolves which VM to use:
1. If `-t <team>` is given and that team has `vm` set, use it
2. If exactly one VM, use it
3. Otherwise, prompt the operator to choose

### `bm attach`

One-shot command. Resolves the VM, then execs into:

```bash
limactl shell <vm-name>
```

The operator gets an interactive shell inside a full Fedora VM. Everything works natively.

```
operator$ bm attach
[vm]$ bm init ...
[vm]$ bm start
[vm]$ exit
operator$
```

### Execution model

`bm attach` lands inside the VM via SSH (Lima's built-in mechanism). All commands run there — no proxying, no forwarding. The VM has its own systemd, D-Bus, keyring, podman — a complete OS.

### File sharing

Lima automatically mounts the operator's home directory into the VM (read-only by default, writable with `:w` suffix). This means project code can optionally be shared between host and VM without git clone.

### `bm bootstrap` ↔ teams

No assumed relationship. One VM can host multiple teams, or one VM per team. `bm bootstrap` is team-agnostic. Teams can optionally link to a VM via `vm` in the team config.

## Local Reference Repos

- Lima source code: `/opt/workspace/lima/` — cloned from `github.com/lima-vm/lima`
- KCLI source code: `/opt/workspace/kcli/` — cloned from `github.com/karmab/kcli` (evaluated, not used)

## Planning Priority

User-facing features first, refactoring second:
1. `bm bootstrap` — VM creation and provisioning via Lima
2. `bm attach` — shell into the VM
3. Verify everything works inside the VM (existing commands)
4. Formation trait refactor — architectural cleanup after features work
