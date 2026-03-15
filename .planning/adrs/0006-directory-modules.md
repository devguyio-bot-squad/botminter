---
status: accepted
date: 2026-03-14
decision-makers: operator (ahmed), claude
---

# Directory Modules as the Only Module Organization

## Problem

Domain modules in `crates/bm/src/` are single files that have grown to contain multiple unrelated concerns. `bridge.rs` (1313 lines) contains bridge manifests, bridge state, bridge lifecycle, credential store traits, local keyring implementation, credential resolution, and bridge provisioning. `profile.rs` (2199 lines) and `workspace.rs` (2045 lines) are similarly monolithic. When a concept like credential storage belongs to the formation domain but lives in `bridge.rs`, the organizational structure obscures ownership — unlike Go packages, single-file Rust modules don't create enough boundary pressure to force domain-aligned grouping.

Single-file modules are the root cause: because Rust allows everything in one crate to freely reference everything else, there is no friction when a type ends up in the wrong module. A directory boundary forces you to think about what a module owns, what it exports, and what belongs elsewhere — the same way Go packages do.

This codebase is primarily developed by LLM coding agents (Claude Code, via Ralph Orchestrator). LLMs process the codebase through file listings, grep results, and file reads — they don't carry an intuitive mental model of module boundaries the way a human developer might after months of working in the code. A directory boundary is a **structural signal** that an LLM can see in any file listing: `formation/` is a domain with sub-concerns, `bridge/` is a different domain. With single files, an LLM sees `bridge.rs` and `formation.rs` as peers at the same level, with no structural hint that `LocalCredentialStore` in `bridge.rs` is actually formation behavior that ended up in the wrong file. Directory modules make domain ownership visible in the filesystem, which is the primary interface LLM agents use to understand codebase organization.

## Constraints

* `commands/` already uses the directory module pattern (`commands/mod.rs` + per-command files) — the convention exists in this codebase
* Rust's module system makes `foo.rs` and `foo/mod.rs` equivalent from the caller's perspective — `bm::formation::FormationConfig` works either way
* 12 domain modules exist as single files today, ranging from 176 lines (`session.rs`) to 2199 lines (`profile.rs`)
* The codebase is primarily developed by LLM coding agents that navigate via file listings and grep — structural signals matter more than in human-only codebases

## Decision

Adopt **directory modules** as the **only** module organization for all domain modules under `crates/bm/src/`. No new single-file modules. Existing single-file modules are converted to directory modules as they are touched.

A directory module replaces `foo.rs` with `foo/mod.rs` + sub-files, where each sub-file owns a cohesive slice of the domain.

### Structure

Every domain module becomes a directory:

```
src/
  formation/          # directory module
    mod.rs            # re-exports, public API
    config.rs         # FormationConfig, load, resolve, list
    local/            # nested directory module for local formation
      mod.rs
      credential.rs
      process.rs
      topology.rs
    ...
  bridge/             # directory module
    mod.rs            # re-exports, shared types
    manifest.rs       # BridgeManifest, BridgeState
    credential.rs     # CredentialStore trait, resolve logic
    lifecycle.rs      # start/stop/health recipes
    provisioning.rs   # identity and room provisioning
    ...
  config/             # directory module
    mod.rs            # Config, TeamEntry, load/save
    ...
  state/              # directory module
    mod.rs            # RuntimeState, MemberRuntime, load/save
    ...
  topology/           # directory module
    mod.rs            # Topology, Endpoint, load/save
    ...
  session/            # directory module
    mod.rs            # interactive and oneshot session launching
    ...
  profile/            # directory module
    mod.rs
    ...
  workspace/          # directory module
    mod.rs
    ...
  commands/           # already a directory module (unchanged)
    mod.rs
    start.rs
    stop.rs
    ...
  cli.rs              # CLI definition (Clap derive) — stays as single file (entry point, not a domain module)
  main.rs             # binary entry point
  lib.rs              # module declarations
```

### Rules

1. **All domain modules are directory modules** — no exceptions. Even small modules like `state` and `topology` become directories. A small module starts as `foo/mod.rs` with all code in `mod.rs` — the directory is the structure, sub-files come when the module grows
2. **New modules** always start as directories
3. **Existing single-file modules** are converted when they are being touched for other work — not as a separate migration task
4. **`mod.rs` is the public API** — it re-exports types and functions. Sub-files can be `pub(crate)` or private
5. **Tests stay with their code** — each sub-file contains its own `#[cfg(test)] mod tests` block, not a separate test file
6. **`cli.rs` and `main.rs` are exempt** — they are entry points, not domain modules

### Small modules

When a module is small (e.g., `state.rs` at 184 lines), conversion to a directory module means moving the content to `state/mod.rs`. The directory exists from day one. If the module grows or gains sub-concerns later, sub-files are added naturally — no restructuring needed.

```
# Small module — all code in mod.rs, directory ready for growth
state/
  mod.rs    # RuntimeState, MemberRuntime, load, save, is_alive, cleanup_stale
```

This is the key difference from "convert only when large": the directory boundary exists from the start, creating the organizational pressure that prevents domain concepts from scattering.

## Rejected Alternatives

### Keep everything as single files

Rejected because: single-file modules create no boundary pressure, allowing domain concepts to scatter across unrelated modules.

- Credential storage ended up in bridge.rs, launch logic in start.rs, state tracking in state.rs — all formation concerns
- LLM agents see no structural signal that these are misplaced

### Directory modules only for large/complex modules (threshold-based)

Rejected because: a threshold means small modules stay as single files and only get restructured when they cross it — by which point the organizational damage is already done.

- The directory boundary should exist from the start so that growth happens within the right structure

### One module per type (micro-modules)

Rejected because: creates excessive indirection without adding clarity.

- A file containing just the `CredentialStore` trait (4 methods) doesn't justify its own file
- Types belong grouped with their implementations in a domain-aligned sub-file

### Workspace-level crate splitting (e.g., `crates/bm-formation/`)

Rejected because: premature for this project's size.

- Crate boundaries add compile-time partitioning and visibility enforcement, but the `bm` crate is a single binary with tightly coupled domain modules
- Directory modules provide sufficient organization without multi-crate dependency management overhead

## Consequences

* Every domain concept is a directory — the filesystem structure mirrors the domain model
* Domain concepts group naturally — formation-related code lives in `formation/`, bridge-related code in `bridge/`
* `commands/` already follows this pattern — the codebase becomes fully consistent
* Module moves are transparent to callers: `bm::formation::FormationConfig` works whether `formation` is a file or directory
* Small modules start as directories with all code in `mod.rs` — zero overhead, ready for growth
* Conversion is incremental — no big-bang migration, modules are converted as they are touched

## Anti-patterns

* **Do NOT** create single-file domain modules — all new domain modules must be directories, even if they start small
* **Do NOT** create sub-files with only one type or function — the overhead of a file isn't worth it for trivial slices. If a sub-concern is under ~50 lines, it belongs in `mod.rs`
* **Do NOT** use directory modules as a substitute for proper domain alignment — moving `LocalCredentialStore` from `bridge.rs` to `bridge/credential.rs` doesn't fix anything if it still belongs in `formation/`
* **Do NOT** batch-convert all existing modules at once — convert incrementally as modules are touched for other work
