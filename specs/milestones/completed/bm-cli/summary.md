# Summary — `bm` CLI (Milestone 3)

> PDD session summary. All planning artifacts for Milestone 3 are complete.

---

## Artifacts

| File | Description | Status |
|------|-------------|--------|
| [rough-idea.md](rough-idea.md) | Initial concept — replace Justfiles with a proper CLI | Complete |
| [UX.md](UX.md) | Operator interaction sketch — first 10 minutes, command patterns | Complete |
| [requirements.md](requirements.md) | 28 Q&A pairs covering scope, architecture, workspace model, process lifecycle, profiles | Complete |
| [research/multiclaude-prior-art.md](research/multiclaude-prior-art.md) | Analysis of multiclaude's patterns — PID tracking, tmux, daemon model | Complete |
| [research/rust-cli-frameworks.md](research/rust-cli-frameworks.md) | Evaluation of clap, cliclack, comfy-table, and alternatives | Complete |
| [research/rust-tui-modern-cli.md](research/rust-tui-modern-cli.md) | Survey of modern Rust TUI/CLI libraries (ratatui, dialoguer, indicatif) | Complete |
| [research/rust-asset-embedding.md](research/rust-asset-embedding.md) | Comparison of include_dir, rust-embed, and build.rs approaches | Complete |
| [design.md](design.md) | Standalone design — 12 commands, 5 data models, workzone layout, profile model, acceptance criteria, testing strategy | Complete (2 review rounds, all findings resolved) |
| [design-review.md](design-review.md) | Round 1 review — 7 gaps, 5 concerns identified and resolved | Complete |
| [design-review-round-2.md](design-review-round-2.md) | Round 2 review — 1 critical, 4 high, 18 medium findings, all resolved | Complete |
| [plan.md](plan.md) | 7-step implementation plan — profile restructuring through integration testing | Complete |

---

## Overview

Milestone 3 replaces the Justfile-based tooling with `bm`, a Rust CLI binary that serves as the single operator interface for managing agentic teams. The CLI embeds profiles at compile time, manages teams via a local config (`~/.botminter/`), and provisions workspaces where Ralph instances run.

### What M3 Delivers

- **12 commands** covering the full operator workflow: `init`, `hire`, `start`, `stop`, `status`, `teams list`, `teams sync`, `members list`, `roles list`, `profiles list`, `profiles describe`, `projects add`
- **Versioned profile model** — `botminter.yml` + `.schema/v1.yml` establishing the foundation for future `bm upgrade`
- **Workzone structure** — discoverable, multi-team workspace management
- **Stored credentials** — no more passing tokens as CLI flags every time
- **Schema version guards** — prevent cross-version contamination across team repos

### What M3 Does NOT Deliver

`bm wake`, `bm knowledge` (Claude-assisted), `bm upgrade`, `bm validate`, auto-restart on crash, laptop sleep recovery, daemon process.

### Key Architectural Decisions

| Decision | Choice |
|----------|--------|
| Language | Rust (aligns with Ralph ecosystem) |
| CLI framework | clap (derive) + cliclack |
| Profile storage | Embedded in binary via `include_dir` |
| State model | Git = source of truth; `~/.botminter/` = convenience index |
| Process management | Direct PID tracking, no daemon |

---

## Implementation Plan Summary

The plan has **7 sequential steps**, each building on the previous and ending with demoable functionality:

1. **Profile Restructuring** — collapse `skeletons/` into `profiles/`, create `botminter.yml` and `.schema/v1.yml`
2. **Cargo Workspace + CLI + `bm profiles`** — first compilable binary, profile embedding verified
3. **Config + `bm init` + `bm teams list`** — first operational command, team creation wizard
4. **`bm hire` + `bm projects add` + list commands** — team composition management
5. **`bm teams sync`** — workspace provisioning (clone + symlink + `.claude/` assembly)
6. **`bm start` + `bm stop` + `bm status`** — process lifecycle (PID tracking, Ralph launch)
7. **Integration Testing + Cleanup** — full lifecycle tests, old skeleton removal

---

## Suggested Next Steps

1. **Implement via code tasks:** Generate `.code-task.md` files from `plan.md` steps using `/code-task-generator`
2. **Autonomous implementation:** Create a `PROMPT.md` and run via `ralph run --config presets/pdd-to-code-assist.yml`
3. **Manual implementation:** Work through steps sequentially using `/code-assist`
