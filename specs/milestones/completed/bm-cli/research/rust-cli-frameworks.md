# Research: Rust CLI Frameworks for `bm`

> Research for M3 — CLI framework selection, interactive prompt libraries, terminal output formatting, and project structure within a non-Rust repo.

---

## 1. Argument Parsing: clap Derive API

**clap** (v4) is the de facto standard. The **derive API** maps perfectly to `bm`'s noun-verb pattern via nested enums:

```rust
#[derive(Subcommand)]
enum Commands {
    Teams {
        #[command(subcommand)]
        action: Option<TeamsAction>,  // None = interactive fallback
    },
    Hire { role: String },
    Start {
        #[arg(short, long)]
        team: Option<String>,
    },
    Status,
    Init,
    // ...
}
```

Key advantage: `Option<SubcommandEnum>` directly supports the "no verb → interactive mode" pattern. When the user types `bm teams` with no verb, `action` is `None` and we branch into interactive mode.

**Verdict:** clap derive. No contest — nested enums = noun-verb; compile-time type safety; auto-generated `--help` at every level.

---

## 2. Interactive Prompts: inquire

Compared **dialoguer** and **inquire**. Both are mature; **inquire** has a slight edge for `bm`:

| Feature | dialoguer | inquire |
|---|---|---|
| Built-in help messages | No | Yes (`.with_help_message()`) |
| Ctrl-C handling | Returns `io::Error` | Returns `InquireError::OperationCanceled` |
| Fuzzy select | Feature-gated | Built-in on all select types |
| Placeholders | No | Yes (`.with_placeholder()`) |

The `bm init` wizard benefits from inline help messages at each step. The `InquireError::OperationCanceled` variant cleanly handles mid-wizard abort (vs. pattern-matching on `io::Error`).

**Verdict:** inquire. Better wizard UX out of the box.

---

## 3. Terminal Output: console + indicatif

These are complementary:

- **console** — styled text output (`style("Running").green().bold()`), emoji with fallback (`Emoji("✔ ", "* ")`), terminal size detection. Used for `bm status` dashboard.
- **indicatif** — progress bars and spinners. Used for multi-step operations (`bm init` creation steps, `bm start` launching members).

**Verdict:** Use both.

---

## 4. Additional Crates

| Crate | Purpose |
|---|---|
| `serde` + `serde_yaml` | Parsing ralph.yml, botminter.yml, profile configs |
| `tabled` | Table formatting for `bm teams list`, `bm status` |
| `anyhow` | Error handling with `.context()` |
| `dirs` | XDG-compliant paths (`~/.botminter/`) |
| `which` | Check for `ralph`, `gh`, `git` in PATH |

---

## 5. Recommended Stack

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
cliclack = "0.3"
console = "0.15"
indicatif = "0.17"
comfy-table = "7"
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
dirs = "5"
which = "7"
```

**Decision: cliclack over inquire.** cliclack provides cohesive wizard framing (`intro()`/`outro()`, connected line art between prompts) that makes `bm init` feel like a product. inquire has more prompt types (autocomplete, date picker, editor) but cliclack's unified visual flow is the better fit for a CLI that aims to be a polished operator experience.

**Decision: comfy-table over tabled.** 58M+ downloads, auto-wrapping, ANSI styling. Better fit for `bm status` and list commands.

---

## 6. Project Structure: Cargo Workspace at Root

**Decision: Cargo workspace.**

The repo is becoming a monorepo. M3 starts with the `bm` CLI crate, but future milestones may add a daemon crate, a TUI crate, or shared library crates. A workspace at the root supports this from day one — single `Cargo.lock`, shared `target/`, `cargo build` from root builds everything.

```
botminter/
├── Cargo.toml                     # [workspace] members = ["crates/*"]
├── Cargo.lock
├── crates/
│   └── bm/                       # CLI binary crate
│       ├── Cargo.toml             # [package] name = "bm"
│       └── src/
│           ├── main.rs
│           ├── cli.rs             # clap derive definitions
│           └── commands/          # one module per command group
├── profiles/                      # embedded into binary at compile time
├── specs/
├── docs/
└── Justfile
```

Workspace `Cargo.toml`:

```toml
[workspace]
members = ["crates/*"]
resolver = "2"
```

Future crates slot in naturally:

```
crates/
├── bm/          # CLI binary (M3)
├── bm-daemon/   # daemon binary (future)
├── bm-tui/      # TUI dashboard (future)
└── bm-core/     # shared library (future)
```

Justfile recipes:

```just
build:
    cargo build --release

install:
    cargo install --path crates/bm
```

---

## 7. Interactive Fallback Pattern

The noun-without-verb → interactive mode pattern in clap derive:

```rust
Commands::Teams { action } => match action {
    Some(TeamsAction::List) => commands::teams::list()?,
    Some(TeamsAction::Show { name }) => commands::teams::show(&name)?,
    None => commands::teams::interactive()?,  // drop into interactive
},
```

The interactive handler uses cliclack to present a menu:

```rust
use cliclack::{intro, outro, select};

pub fn interactive() -> Result<()> {
    intro("teams")?;
    let action: &str = select("What would you like to do?")
        .item("list", "List all teams", "")
        .item("show", "Show team details", "")
        .interact()?;
    match action {
        "list" => list()?,
        "show" => { /* prompt for name, then show() */ }
        _ => unreachable!(),
    }
    outro("Done")?;
    Ok(())
}
```

---

## Sources

- [clap documentation](https://docs.rs/clap/latest/clap/)
- [cliclack documentation](https://docs.rs/cliclack/latest/cliclack/)
- [console documentation](https://docs.rs/console/latest/console/)
- [indicatif documentation](https://docs.rs/indicatif/latest/indicatif/)
- [comfy-table documentation](https://docs.rs/comfy-table/latest/comfy_table/)
