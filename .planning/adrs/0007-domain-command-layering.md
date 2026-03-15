---
status: accepted
date: 2026-03-15
decision-makers: operator (ahmed), claude
---

# Domain Modules and Command Layering

## Problem

Commands and domain logic have no defined boundary. Without a clear rule, business logic ends up wherever it was first needed — YAML parsing in command files, process management inline in CLI handlers, credential resolution repeated across commands. This makes domain logic untestable without a terminal, unreusable across commands, and invisible to code reviewers who expect commands to be thin.

## Constraints

* The crate has a lib + bin split — `lib.rs` declares public domain modules, `main.rs` is the binary entry point
* `commands/` is a directory module organized by subcommand (ADR-0006)
* Domain modules are directory modules (ADR-0006)
* Domain operations inherently involve side effects (process spawning, keyring access, shell commands) — the goal is not purity but proper encapsulation behind domain interfaces

## Decision

The `bm` crate has two layers. All code belongs to exactly one of them, with two exceptions: `cli.rs` (Clap argument definitions) and `main.rs` (binary entry point) are CLI infrastructure that serves both layers.

### The command pattern

A command does exactly four things:

1. **Load config and resolve context** — `config::load()`, `config::resolve_team()`
2. **Construct domain objects** (dependency injection) — `formation::create()`, `bridge::for_team()`
3. **Call one domain method** — `formation.start()`, `bridge.add_identity()`
4. **Display the result** — `println!`, tables, error messages

When reading command code, you should see CLI-relevant logic: argument handling, user prompts, output formatting, error presentation. If you see YAML parsing, process spawning, PID management, credential resolution, or external tool invocation — that is domain logic in the wrong layer.

### What well-layered commands look like

```rust
// bm start
pub fn run(team_flag: Option<&str>, formation_flag: Option<&str>, ...) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let formation = formation::create(&team_repo, &resolved_name)?;

    let result = formation.start(&team, &cfg)?;

    for m in &result.launched { println!("{}: started (PID {})", m.name, m.pid); }
    for m in &result.skipped  { println!("{}: already running", m.name); }
    for m in &result.errors   { eprintln!("{}: {}", m.name, m.error); }
    println!("\nStarted {}, skipped {}, {} error(s).",
        result.launched.len(), result.skipped.len(), result.errors.len());
    Ok(())
}
```

The formation's `start()` handles everything: prerequisite checks, bridge auto-start, member discovery, credential resolution, process spawning, topology writing. The command displays the structured result.

```rust
// bm stop
pub fn run(team_flag: Option<&str>, force: bool, ...) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let formation = formation::create(&team_repo, "local")?;

    let result = formation.stop(&team, &cfg, force)?;

    for m in &result.stopped { println!("{}: stopped", m.name); }
    println!("\nStopped {} member(s), {} error(s).",
        result.stopped.len(), result.errors.len());
    Ok(())
}
```

```rust
// bm bridge identity add
pub fn run(username: &str, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let bridge = bridge::for_team(&team_repo, &team, &cfg)?;

    let result = bridge.add_identity(username)?;

    println!("Identity '{}' added (user_id: {})", result.username, result.user_id);
    Ok(())
}
```

The pattern: **resolve context → construct domain object → call one method → display result.** If a command makes more than one domain call, prefer composing them inside a single domain method. Two domain calls are acceptable only when they are genuinely independent operations (e.g., loading formation and loading bridge for display).

Interactive wizards (e.g., `bm init`) are an exception — they naturally interleave user prompts with multiple domain calls. Each prompt-domain-call pair should still follow the pattern: the prompt gathers input, then a single domain call processes it.

### Command layer

Command modules (`commands/start.rs`, `commands/stop.rs`, etc.) contain:

- **Argument handling** — receiving parsed CLI args from `cli.rs`
- **Context resolution** — loading config, resolving team, constructing domain objects
- **User output** — `println!`, `eprintln!`, tables, progress indicators, interactive prompts
- **Error presentation** — converting domain errors into user-facing messages with actionable hints
- **CLI infrastructure** — some command-layer modules serve infrastructure purposes (shell completions, help generation) rather than following the four-step pattern. These are valid command-layer modules because they are purely about the CLI interface

Command modules have no public API. They do not export to other command modules. If two commands need the same logic, that logic belongs in a domain module.

Commands do NOT:

- Contain functions that parse domain-specific YAML/JSON — that is domain persistence
- Contain reusable helper functions consumed by other commands — that is a missing domain module
- Contain type definitions (structs, enums) used outside the command — that is a domain type
- Contain functions that wrap external tools (`gh`, `kubectl`, `ralph`) — those are domain infrastructure
- Duplicate logic that exists in another command — duplication signals a missing domain module
- Exceed ~100 lines of non-test code — thickness signals domain logic that has not been extracted
- Export `pub` items consumed by other command modules — that is cross-command coupling

### Domain layer

Domain modules (`formation/`, `bridge/`, `config/`, `profile/`, `workspace/`, `topology/`, `state/`, `session/`, etc.) contain:

- **Types** — structs, enums, traits that model the domain (`FormationConfig`, `BridgeManifest`, `CredentialStore`, `Topology`)
- **Implementations** — trait impls, constructors, methods that operate on domain types
- **Domain operations** — business logic that a command calls (`formation.start()`, `bridge.add_identity()`, `workspace::sync()`)
- **Persistence** — loading and saving domain state (`topology::load()`, `state::save()`, `config::load()`)
- **Infrastructure** — wrappers for external tools, shell commands, system APIs — behind domain interfaces

Domain modules do NOT:

- Parse CLI arguments
- Format output for the terminal (no `println!`, `eprintln!`, or table formatting)
- Reference `clap`, `comfy_table`, `dialoguer`, `cliclack`, or other CLI libraries
- Know about the command that called them

Domain operations return `Result<T>` with structured data. The command decides what to print.

**Error handling:** domain modules use `anyhow::Result` for errors. If a command needs to handle specific domain errors differently (e.g., showing different hints for "keyring locked" vs "member not found"), the domain module should define typed error variants. Commands convert domain errors into user-facing messages with actionable hints.

### Domain module structure

Domain modules are not bags of free functions. They model concepts with types and methods:

- **Model with types** — domain concepts get structs with methods. A group of related operations becomes methods on a struct, not free functions that pass the same context parameters around.
- **Return structured types** — domain operations return named result structs (e.g., `StartResult`, `IdentityAddResult`), not raw tuples, strings, or booleans. Commands use these structs to format output.
- **One concept per sub-file** — within a directory module (ADR-0006), each sub-file owns a cohesive slice describable in one phrase. If describing a sub-file requires "and," it should be split.
- **`mod.rs` is the public API** — it re-exports the types and functions that commands use. Sub-files can be `pub(crate)` or private.
- **Encapsulate context** — if multiple functions take the same parameters (e.g., `token`, `owner`, `team_name`), those parameters are fields on a struct. Free functions passing the same context around signal a missing struct.
- **No sub-file exceeds ~300 lines** — if it does, it likely contains multiple concepts that should be split.

### Domain modules may have side effects

Domain operations like `formation.start()` spawn processes. `credential_store.store()` writes to the system keyring. `bridge.add_identity()` runs shell commands. These are not pure functions.

The rule is not "domain modules are pure." The rule is: **domain modules expose their side effects through domain interfaces** (traits, methods that return structured results), not through terminal output. A domain module can spawn a process — but it returns the result as data, it does not print it.

### Command thickness test

A command should read like a short script. If a command file has private helper functions that do not contain `println!`/`eprintln!`/table rendering/interactive prompts, those functions are domain logic that should be extracted to a domain module.

**Mechanical test:** count functions in a command file. Functions without output formatting are domain logic in the wrong layer.

### Duplication signals a missing domain module

If the same logic appears in two or more commands, a domain module is missing. The duplicated code should be extracted into a domain module that both commands call.

### Domain module completeness

Every significant domain concept needs its own domain module. If business logic for a concept exists only inside command files, the domain module for that concept is missing — even if no one has created it yet.

### Testing

Domain modules are the primary test surface. The whole point of the layering is that domain logic is testable without a terminal — if domain logic can only be tested by calling a command's `run()` function, the extraction is incomplete.

**What domain tests look like:**

- Construct a domain object (e.g., `LocalFormation`, `Bridge`, `GitHubClient`)
- Call a method
- Assert on the structured return type — field by field, not just `is_ok()`

```rust
#[test]
fn launch_member_returns_pid_and_workspace() {
    let formation = LocalFormation::new();
    let result = formation.launch_member(&params).unwrap();
    assert!(result.pid > 0);
    assert_eq!(result.workspace, expected_path);
}
```

Domain tests do NOT:

- Capture stdout/stderr to verify output — that is a command concern
- Parse CLI arguments — domain methods take typed parameters
- Require a running terminal or interactive session

**Tests live with the code.** Each domain sub-file contains its own `#[cfg(test)] mod tests` block. When domain logic moves from a command to a domain module, any existing tests for that logic move too.

**Structured return types must be assertable.** If a domain operation returns `StartResult`, tests assert on `result.launched.len()`, `result.errors[0].name`, etc. A test that only checks `result.is_ok()` is not testing the domain operation — it is testing that nothing panicked.

**Commands are not the test surface for domain logic.** If a domain operation is only exercised through a command's integration test, the domain module is missing unit tests. Command tests verify output formatting and CLI behavior. Domain tests verify business logic.

### Compliance checklist

When writing or reviewing a command, verify:

1. Does the command file have private helper functions without `println!`/`eprintln!`/table rendering/prompts? → Extract to domain module
2. Does the command file define structs/enums used by other modules? → Move to domain module
3. Does the command file exceed ~100 non-test lines? → Look for extractable domain logic
4. Does the command export `pub` items consumed by other commands? → Move shared logic to domain module
5. Does the command wrap external tools (`gh`, `ralph`, `just`)? → Move to domain infrastructure

When writing or reviewing a domain module, verify:

1. Does the module use `println!`/`eprintln!` or reference CLI libraries? → Return structured data instead
2. Does the module contain free functions that share 3+ context parameters? → Encapsulate in a struct
3. Does any public function return `Result<()>` when it produces meaningful output? → Return a named result struct
4. Does any sub-file exceed ~300 lines? → Split into cohesive sub-files
5. Does the module have unit tests that assert on structured return types? → If not, add them

## Rejected Alternatives

### Extract a separate library crate (`crates/bm-lib/`)

Rejected because: the lib + bin split already exists within the single `bm` crate (`lib.rs` + `main.rs`). A separate crate adds dependency management overhead without benefit.

- If a second consumer appears (e.g., a web API), crate extraction becomes worthwhile

### Pure domain modules — no side effects in domain layer

Rejected because: many domain operations are inherently effectful (spawning processes, writing to keyring, running shell commands). Forcing purity would push all I/O into commands, making commands thick with infrastructure code.

- The boundary is about domain interfaces vs terminal output, not about purity

### Domain modules return display-ready strings

Rejected because: coupling domain operations to a specific output format prevents reuse.

- Domain operations return structured data (`Result<StartResult>`)
- Commands format that data for the terminal

## Consequences

* Commands are short scripts: load, construct, call, display
* Domain modules become testable without a terminal — tests call domain methods and assert on returned data
* Domain operations are reusable across commands
* New commands can be added by orchestrating existing domain operations — no domain module changes needed
* The `comfy_table`, `dialoguer`, `cliclack` dependencies are confined to the command layer
* Thick commands are a code smell — they signal domain logic that has not been extracted

## Anti-patterns

* **Do NOT** use `println!` or `eprintln!` in domain modules — return structured data, let the command format it
* **Do NOT** define types, helper functions, or infrastructure wrappers in command files — if it does not do output formatting, it belongs in a domain module
* **Do NOT** duplicate logic across commands — extract it to a domain module that both commands call
* **Do NOT** pass `&dyn Write` or output sinks into domain methods as a workaround for the no-println rule — this is the same coupling in disguise. Return data, do not write it
* **Do NOT** treat "no println in domain modules" as the complete layering rule — the equally important rule is "no domain logic in commands"
* **Do NOT** let command files grow past ~100 lines of non-test code — thickness means domain logic is hiding in the command layer
* **Do NOT** create domain modules that are bags of free functions — model concepts with structs and methods
* **Do NOT** return `Result<()>` from domain operations that produce meaningful output — return a named result struct the command can display
* **Do NOT** dump all extracted code into `mod.rs` — split into cohesive sub-files per ADR-0006
