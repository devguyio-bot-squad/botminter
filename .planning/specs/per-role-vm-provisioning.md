# Per-Role VM Provisioning — Rough Idea

## The Problem

`bm bootstrap` creates a VM with the BotMinter runtime (bm, ralph, claude, gh, git, just). But team members need project-specific tooling too — a Rust developer needs rustup, a docs writer needs mkdocs, a Python team needs pip and venv. Today there's no way to express these requirements.

## Two-Layer Model

**Layer 1 — BotMinter Runtime (embedded, ships with `bm`)**
The base VM template: Fedora cloud image, system packages, bm, ralph, claude, gh. This is what `bm bootstrap` does today. Every VM gets this regardless of team or role.

**Layer 2 — Team/Role Tooling (profile-defined, per member)**
Project-specific tools and configuration. Defined per-role in the profile's `roles/<role>/` directory. Applied after the base VM is running.

## Where It Lives

Each role in the profile gets a provisioning definition:

```
profiles/scrum-compact/roles/superman/provision.yml
profiles/scrum/roles/developer/provision.yml
profiles/scrum/roles/architect/provision.yml
```

Uses a cloud-init-compatible YAML subset:

```yaml
packages:
  - python3-pip
  - rustup

write_files:
  - path: ~/.config/some-tool/config.yml
    content: |
      key: value

runcmd:
  - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  - rustup default stable
```

## How It Composes

Two-phase:

1. `bm bootstrap` — creates the base VM (Layer 1). No team context needed.
2. A second command (TBD — could be `bm setup`, or part of `bm teams sync`) — reads the team repo, finds each member's role, looks up `provision.yml` from the profile, and applies it into the running VM.

The second phase runs inside the VM via `limactl shell <vm> -- <script>` or similar.

## Open Questions

- **Cloud-init subset**: Which directives do we support? `packages`, `write_files`, `runcmd` seem essential. What about `users`, `groups`, `snap`, `apt`, `yum_repos`?
- **No Rust cloud-init crate exists**: We'd define serde structs for the subset we support, or use `serde_yml::Value` for flexibility.
- **Idempotency**: Cloud-init runs once at first boot. Our provisioning may run multiple times (re-sync, role change). How do we handle that?
- **Lima translation**: Do we translate our cloud-init subset into Lima `provision` blocks and re-create the VM? Or exec scripts into a running VM?
- **Per-member vs per-role**: The profile defines per-role provisioning. When a member is hired into a role, they inherit it. Can individual members override?
- **Cross-platform**: This is Lima/VM-specific. How does it interact with the Formation trait from ADR-0008? Is provisioning part of `Formation::setup()`?
