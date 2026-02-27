---
status: done
created: 2026-02-21
started: 2026-02-21
completed: 2026-02-21
---
# Task: Add Shell Completions Command

## Description
Add a `bm completions <shell>` subcommand that generates shell completion scripts for the `bm` CLI. This enables tab-completion for all commands, subcommands, flags, and arguments across bash, zsh, fish, PowerShell, and elvish.

## Background
The `bm` CLI uses Clap 4 with derive macros. Clap's ecosystem provides `clap_complete`, a companion crate that introspects the `Command` definition at runtime and generates shell-native completion scripts. Adding a `completions` subcommand is the standard pattern — users run `bm completions <shell>` and pipe/redirect the output to the appropriate shell config location.

## Technical Requirements
1. Add `clap_complete` as a dependency in `crates/bm/Cargo.toml`
2. Add a `Completions` variant to the `Command` enum in `cli.rs` with a positional `shell` argument of type `clap_complete::Shell`
3. Implement the handler in `commands/completions.rs` that generates the completion script and writes it to stdout
4. Register the new module in `commands/mod.rs` and dispatch in `main.rs`
5. The subcommand should be named `completions` (not `completion` or `complete`)

## Dependencies
- `clap_complete` crate (version 4.x to match clap 4)
- Existing `Cli` struct in `cli.rs` (used by `clap_complete::generate` to introspect the command tree)

## Implementation Approach
1. Add `clap_complete = "4"` to `crates/bm/Cargo.toml` dependencies
2. In `cli.rs`, add a new variant to the `Command` enum:
   - `Completions { shell: clap_complete::Shell }` with appropriate help text
3. Create `commands/completions.rs`:
   - Use `clap_complete::generate()` with `Cli::command()`, the shell type, and `stdout`
4. Add `pub mod completions;` to `commands/mod.rs`
5. Add the match arm in `main.rs` to dispatch `Command::Completions { shell }` to the handler

## Acceptance Criteria

1. **Generates bash completions**
   - Given the `bm` binary is built
   - When the user runs `bm completions bash`
   - Then a valid bash completion script is printed to stdout and the process exits with code 0

2. **Generates zsh completions**
   - Given the `bm` binary is built
   - When the user runs `bm completions zsh`
   - Then a valid zsh completion script is printed to stdout and the process exits with code 0

3. **Generates fish completions**
   - Given the `bm` binary is built
   - When the user runs `bm completions fish`
   - Then a valid fish completion script is printed to stdout and the process exits with code 0

4. **Supports all clap_complete shells**
   - Given the `bm` binary is built
   - When the user runs `bm completions <shell>` for any of bash, zsh, fish, powershell, elvish
   - Then a non-empty completion script is produced for each

5. **Rejects invalid shell names**
   - Given the `bm` binary is built
   - When the user runs `bm completions invalidshell`
   - Then clap reports an error with the list of valid shell options

6. **Help text is accurate**
   - Given the `bm` binary is built
   - When the user runs `bm completions --help`
   - Then the help text explains the command generates shell completion scripts

## Metadata
- **Complexity**: Low
- **Labels**: cli, dx, shell-completions
- **Required Skills**: Rust, Clap 4, clap_complete
