---
status: proposed
date: 2026-03-27
decision-makers: operator (ahmed), bob
---

# Formation Manager — Minty Skill with Formation Contract

## Problem

ADR-0008 defines the `Formation` trait with a `setup()` method for environment provisioning. The current local formation implementation of `setup()` only checks if `ralph` is in PATH — it doesn't actually install anything. This means the operator must manually install all prerequisites (ralph, coding agent, keyring, etc.) before `bm env create` does anything useful.

Hardcoding platform-specific installation logic in Rust (dnf vs apt vs brew, systemd vs launchd, D-Bus vs Keychain) creates a maintenance burden that grows with every new distro, OS version, or tool update.

The `bm env create` command should take the operator from a bare machine to a fully provisioned environment ready to run team members. Today it cannot.

## Constraints

* Must work across Linux distros (Fedora, Ubuntu, Arch, etc.) and macOS without pre-coding each variation
* Must be idempotent — running the provisioning twice produces the same end state
* Must work without elevated permissions by default, with optional permission escalation
* Must not require additional dependencies beyond what ships with `bm` and the coding agent
* Must integrate with the existing `Formation` trait — not replace it
* The operator must remain in control — the manager proposes actions and the operator approves
* Must produce a verifiable end state — deterministic verification passes after provisioning
* Must not introduce a new CLI command — Minty (`bm minty`) already exists as the operator's interactive AI assistant
* Must be coding-agent-agnostic — the contract and invocation must work with any supported coding agent
* Minty must be formation-agnostic — it reads formation contracts, not formation-specific logic
* Formations must expose a structured contract that any AI agent (not just Minty) can consume

## Decision

### Two-layer architecture: Formation Contract + Minty Skill

The design has two cleanly separated layers:

1. **Formation contract** — a structured, machine-readable specification that each formation exposes, declaring its dependencies, desired state, verification commands, and optional hints. The contract is formation-specific but agent-agnostic.

2. **Minty skill** — a formation-agnostic skill that reads any formation's contract, understands the gap between current state and desired state, and provisions the environment. The skill is agent-specific (lives in Minty) but formation-agnostic.

This is the first consumer of the structured prompt protocol defined in [ADR-0013](0013-structured-prompt-protocol.md). The formation contract is a domain-specific structured document; `bm env create` launches Minty with a structured launch document; and `bm-agent env check` provides deterministic verification via structured JSON responses.

### The Formation Contract

Each formation exposes a `contract.yml` file that declares what the formation needs. This is the interface between the deterministic formation layer (Rust) and the AI provisioning layer (Minty).

```yaml
# formations/local/contract.yml
name: local
description: "Run members as local processes on the operator's machine"

# ── Dependencies ──────────────────────────────────────────────
# What must be installed and available. Each dependency has:
# - a verification command (deterministic check)
# - a human-readable purpose
# - optional installation hints per platform
dependencies:
  - name: ralph
    purpose: "Ralph Orchestrator — runs member work loops"
    verify:
      command: "which ralph"
      expect: exit_code_0
    install_hints:
      cargo: "cargo install ralph-orchestrator"

  - name: coding-agent
    purpose: "Coding agent for AI-driven sessions"
    verify:
      command: "which ${CODING_AGENT_BINARY}"
      expect: exit_code_0
    note: "Resolved from profile's coding_agents config"

  - name: gh
    purpose: "GitHub CLI for repository and project operations"
    verify:
      command: "gh auth status"
      expect: exit_code_0
    install_hints:
      dnf: "sudo dnf install gh"
      apt: "sudo apt install gh"
      brew: "brew install gh"

  - name: gh-project-scope
    purpose: "GitHub project scope for Projects v2 operations"
    verify:
      command: "gh auth status 2>&1 | grep -q project"
      expect: exit_code_0
    fix_hint: "gh auth refresh -s project"

  - name: just
    purpose: "Just command runner for bridge lifecycle recipes"
    verify:
      command: "which just"
      expect: exit_code_0
    install_hints:
      cargo: "cargo install just"

  - name: keyring
    purpose: "System keyring for secure credential storage"
    verify:
      command: "bm env check-keyring"
      expect: exit_code_0
    platform_notes:
      linux: "Requires gnome-keyring-daemon and D-Bus Secret Service"
      macos: "Uses macOS Keychain — available by default"

# ── Desired State (Truths) ────────────────────────────────────
# What must be TRUE after successful provisioning.
# These are invariants the AI must verify, not just install.
truths:
  - "All dependencies pass their verification commands"
  - "Operator's gh auth session is valid and has project scope"
  - "System keyring is unlocked and accessible for credential storage"
  - "bm env check reports all checks passed"

# ── Verification ──────────────────────────────────────────────
# The deterministic command that proves the formation is ready.
# This is what the AI runs AFTER provisioning to confirm success.
verification:
  command: "bm env check"
  description: "Runs Formation::check_environment() and reports results"

# ── Optional Hints ────────────────────────────────────────────
# Contextual hints that help the AI but don't constrain it.
# These are NOT required — the AI can figure things out without them.
hints:
  package_manager:
    description: "Preferred package manager (if known)"
    # Not set by default — the AI detects the platform
    # Can be set per-team or per-formation for specialized setups
    # e.g., value: "dnf" for a Fedora-based formation
```

#### Contract design principles

The contract is **declarative** — it says what must be true, not how to make it true. The AI figures out the "how" based on the current platform.

- Each dependency has a **verification command** — a deterministic check the CLI can run without AI
- **Truths** are observable invariants, not install steps — "keyring is accessible" not "install gnome-keyring"
- **Install hints** are optional guidance, not mandates — the AI can figure things out without them
- **Goal-backward verification** — `bm env check` proves the desired state was achieved, not that install steps ran

### The `bm env check` / `bm-agent env check` command

A new deterministic command that runs the formation's `check_environment()` and outputs structured results. Available as both `bm env check` (operator-facing) and `bm-agent env check` (agent-facing, per [ADR-0013](0013-structured-prompt-protocol.md)). This is the bridge between the Rust verification layer and the AI provisioning layer.

```
$ bm env check
Formation: local
Platform: linux (Fedora 43)

  ✓ ralph         ralph-orchestrator found in PATH
  ✓ coding-agent  claude found in PATH
  ✓ gh            gh CLI authenticated
  ✗ gh-project    project scope missing — run: gh auth refresh -s project
  ✓ just          just found in PATH
  ✓ keyring       Secret Service accessible, default collection unlocked

Result: 5/6 checks passed (1 failed)
```

The output is human-readable AND machine-parseable (structured JSON with `--json` flag). Minty reads this output to understand the gap.

### The Minty Skill

The formation-manager skill lives in Minty's skill set. It is formation-agnostic — it reads any formation's contract and provisions accordingly.

```
minty/.claude/skills/formation-manager/
  SKILL.md                    # Skill definition with trigger phrases
  knowledge/
    contract-schema.md        # How to read and interpret formation contracts
    provisioning-strategy.md  # General provisioning approach (check → plan → install → verify)
    platform-patterns.md      # Common patterns for Linux distros, macOS
```

```yaml
# SKILL.md
---
name: formation-manager
description: >-
  Provisions and manages formation environments by reading the formation's
  contract.yml and ensuring all dependencies are satisfied. Formation-agnostic —
  works with any formation type (local, Docker, k8s). Use when the operator asks
  to "bootstrap", "set up my environment", "install prerequisites", "check if my
  machine is ready", "fix my setup", "bm env create", or "formation health check".
metadata:
  author: botminter
  version: 1.0.0
  category: formation
  tags: [formation, bootstrap, environment, prerequisites, setup]
---
```

#### Skill execution flow

```
1. Read contract:    cat formations/<name>/contract.yml
2. Check state:      bm env check [--json]
3. Identify gaps:    Compare contract dependencies against check results
4. Plan actions:     Determine what to install/configure for this platform
5. Execute:          Run install commands (with operator approval)
6. Verify:           Run bm env check again — all checks must pass
```

The skill's `knowledge/contract-schema.md` teaches Minty how to interpret the contract:

```markdown
# Formation Contract Schema

## Reading a contract

The contract file at `formations/<name>/contract.yml` declares:

### dependencies[]
Each dependency has:
- `name`: identifier
- `purpose`: why this dependency exists
- `verify.command`: shell command that returns exit 0 if satisfied
- `verify.expect`: expected result (exit_code_0, contains_string, etc.)
- `install_hints`: optional per-platform installation commands
- `platform_notes`: platform-specific guidance
- `fix_hint`: single command to fix a known issue
- `note`: context about how this dependency is resolved

### truths[]
Invariants that must hold after provisioning. Verify each one.

### verification
The final deterministic command that proves everything is ready.
Run this AFTER all dependencies are satisfied.

### hints
Optional context. Don't rely on these — detect the platform yourself.
Only use hints when they're explicitly set (not null/empty).
```

### Permission model

Minty's default permission behavior is changed in this ADR:

```
bm minty                        # permissions OFF (default — operator is present)
bm minty --enable-permissions   # permissions ON (for sensitive operations)
```

The old `-a`/`--autonomous` flag is removed. Rationale: Minty is an interactive session — the operator is always present. Permission prompts add friction without safety value.

`bm env create` assembles a structured launch document (per [ADR-0013](0013-structured-prompt-protocol.md)) containing the formation contract and current environment state, then launches Minty with it as the system prompt and permissions enabled (since it may need `sudo`):

```rust
fn create(team: Option<&str>, formation_flag: Option<&str>) -> Result<()> {
    // Assemble structured launch document with formation context
    let doc = build_launch_document(LaunchType::SkillSession {
        skill: "formation-manager",
        context: formation_contract_and_env_state(team, formation_flag)?,
    })?;
    launch_minty_with_document(team, &doc, EnablePermissions::Yes)?;
    Ok(())
}
```

### Coding-agent-agnostic implementation

Minty's implementation must not hardcode coding-agent-specific flags. The current implementation directly passes `--append-system-prompt-file` and `--dangerously-skip-permissions` — these are Claude Code-specific.

The coding agent abstraction (from the profile's `coding_agents` config) must provide session-launching capabilities:

| Capability | What it does |
|-----------|-------------|
| `build_session_command(workdir)` | Constructs the base command for an interactive session |
| `append_system_prompt(path)` | Adds a system prompt file to the session |
| `skip_permissions()` | Disables the permission system |
| `enable_permissions()` | Enables the permission system |

Minty calls these trait methods, never hardcodes agent-specific flags.

### Integration with Formation trait

The `Formation` trait gains a new method for exposing the contract, and `setup()` changes to launch Minty:

```rust
pub trait Formation {
    // ... existing methods ...

    /// Returns the path to this formation's contract.yml.
    fn contract_path(&self) -> Option<PathBuf>;
}

// LinuxLocalFormation implementation:
fn setup(&self, params: &SetupParams) -> Result<()> {
    // Launch Minty with formation-manager skill
    // Falls back to check_prerequisites() if no coding agent available
    if coding_agent_available() {
        launch_minty_with_skill("formation-manager", EnablePermissions::Yes)?;
    }
    self.check_prerequisites()  // Final deterministic verification
}
```

### What the formation owns vs what Minty owns

| Concern | Formation (Rust) | Minty (AI Skill) |
|---------|-----------------|------------------|
| **What** must be installed | `contract.yml` — declarative dependencies | — |
| **How** to install it | — | Adapts to platform, package manager, OS |
| **Verify** state | `check_environment()` → `bm env check` | Calls verification, interprets results |
| **Edge cases** | — | Handles missing repos, version conflicts, permissions |
| **Platform detection** | Reports platform in check output | Uses detection to choose install strategy |

## Rejected Alternatives

### New `bm manage` command

Rejected because: duplicates Minty's role as the operator's AI assistant. Two AI entry points create confusion about which to use.

### Hardcoded Rust setup per platform (ADR-0008's original approach)

Rejected because: requires pre-coding every distro x package manager x version combination. The maintenance burden grows linearly with platform support.

### Ansible playbooks

Rejected because: introduces a Python dependency and a new DSL. Bootstrap-the-bootstrapper problem.

### Nix flakes

Rejected because: requires the Nix package manager installed first. Same bootstrap problem. Steep learning curve.

### OpenTofu/Terraform

Rejected because: designed for cloud infrastructure provisioning, not local machine configuration. Wrong abstraction level.

### Shell scripts with platform detection

Rejected because: brittle across distro variations, hard to make idempotent. Already proven insufficient by the current `setup()` implementation.

### Keep permissions ON by default for Minty

Rejected because: Minty is an interactive session — the operator is always present. Permission prompts for every tool call add friction without safety value.

### Unstructured CLAUDE.md as the contract

Rejected because: a prose document is not machine-parseable. The formation needs a structured contract that Rust code can also read (for `bm env check`) and that any AI agent can consume. YAML provides both human readability and machine parseability.

## Consequences

* First consumer of the structured prompt protocol ([ADR-0013](0013-structured-prompt-protocol.md)) — introduces the `skill-session` launch document type and `bm-agent env check`
* Each formation exposes a structured `contract.yml` — the single source of truth for its requirements
* `bm env check` provides deterministic, fast verification without an AI session
* `bm env create` launches Minty with the formation-manager skill — takes the operator from zero to working
* Minty is formation-agnostic — reads any formation's contract, provisions accordingly
* Formations are agent-agnostic — the contract can be consumed by any AI, not just Minty
* No platform-specific Rust code for installation — the AI adapts
* New formations declare their requirements in `contract.yml` — Minty knows how to provision them automatically
* `check_environment()` remains in Rust, now aligned with the contract's verification commands
* Minty defaults to permissions-skipped; `--enable-permissions` opts in
* Minty must be refactored to use coding-agent-agnostic abstractions

## Anti-patterns

* **Do NOT** put installation logic in the formation's Rust code — the AI handles platform-specific installation. Rust code should only verify state.
* **Do NOT** put formation-specific knowledge in the Minty skill — the skill reads the contract. Formation-specific details belong in `contract.yml` and its `install_hints`/`platform_notes`.
* **Do NOT** make `contract.yml` a prose document — it must be structured YAML that both Rust and AI can parse.
* **Do NOT** require Minty for `bm env check` — verification must be fast and deterministic, no AI session needed.
* **Do NOT** hardcode coding-agent-specific flags in Minty — use the coding agent abstraction.
* **Do NOT** assume the contract is only consumed by Minty — other tools, scripts, or agents may read it.
* **Do NOT** make `install_hints` mandatory — they are optional guidance. The AI can figure out installation without them. They exist to speed things up, not to constrain.
* **Do NOT** skip the final `bm env check` after provisioning — always verify deterministically.
