# Add Shell Completions to `bm` CLI

## Objective

Add a `bm completions <shell>` subcommand that generates shell completion scripts (bash, zsh, fish, PowerShell, elvish) using the `clap_complete` crate. This is a developer-experience feature — no changes to existing command behavior.

## Task Files

Execute in order:

1. `specs/tasks/task-01-add-completions-command.code-task.md` — Add `clap_complete` dependency and implement the `completions` subcommand
2. `specs/tasks/task-02-completions-integration-test.code-task.md` — Add integration tests for all supported shells

Each task has detailed acceptance criteria in Given-When-Then format. Update the YAML frontmatter (`status`, `started`, `completed`) as you begin and finish each task.

## Key References

- **CLI definitions:** `crates/bm/src/cli.rs` — Clap 4 derive-based `Cli` struct and `Command` enum
- **Command handlers:** `crates/bm/src/commands/` — one module per command group, re-exported in `mod.rs`
- **Entry point:** `crates/bm/src/main.rs` — match on `Command` variants and dispatch
- **Existing tests:** `crates/bm/tests/integration.rs` — subprocess-based integration tests
- **Dependencies:** `crates/bm/Cargo.toml`

## Implementation Constraints

- Follow existing code patterns exactly (derive macros, `anyhow::Result`, module structure)
- The subcommand MUST be named `completions` (not `completion` or `complete`)
- The shell argument MUST use `clap_complete::Shell` as its type (Clap handles validation automatically)
- Use `clap_complete::generate()` to produce output — do not hand-write completion scripts
- One commit per task, conventional format: `feat(cli): <subject>`

## Verification

After each task, run all three gates:

```bash
just build    # cargo build -p bm
just test     # cargo test -p bm
just clippy   # cargo clippy -p bm -- -D warnings
```

All three MUST pass before committing.
