# Research: Modern Rust TUI & CLI Frameworks

> Research for M3 — evaluating Elm-architecture TUI frameworks and modern CLI prompt libraries in Rust. Motivated by the question: "Is there a bubbletea equivalent in Rust?"

---

## The Honest Answer

**No.** There is no Rust framework that matches bubbletea's combination of simplicity, composability, ecosystem (Lip Gloss, Glamour, Huh), maturity (v1.x, 30K+ stars, 10K+ dependents), and developer experience.

The closest candidates:

| Framework | Architecture | Stars | Maturity | Verdict |
|---|---|---|---|---|
| **tui-realm** | React/Elm hybrid on ratatui | 858 | Active (v3.3.0) | Most mature TEA-like, but heavier than bubbletea |
| **tears** | Pure TEA on ratatui | 5 | Active but tiny | Clean design, essentially a solo project |
| **bubbletea-rs** | Direct bubbletea port | 215 | v0.0.9 | Not production-ready |
| **r3bl_tui** | Redux/Elm hybrid, own renderer | ~200 | Active | Heavy deps, small community |

**Why the gap exists:** Rust's ownership model makes TEA simultaneously natural (immutability enforced) and awkward (can't easily clone/pass state like Go). The ecosystem converged on **ratatui** as a powerful but low-level rendering primitive, with the expectation that developers build their own architecture on top. This is very Rust-like — excellent building blocks, not opinionated frameworks.

---

## The Rust TUI Landscape

### ratatui (18.5K stars, 17.4M downloads)

The dominant TUI library. Successor to tui-rs (forked 2023). **Immediate-mode** — you own the event loop and describe the entire UI each frame. ratatui diffs and renders only changes.

- Massive widget ecosystem (charts, tables, gauges, sparklines, code editors)
- Flexbox-like constraint layouts
- Sub-millisecond rendering
- Used by GitUI, Trippy, Television, kubetui, OpenAI Codex terminal agent
- **Not a framework** — no built-in state management, component model, or event routing

### cursive (4.7K stars, 1.1M downloads)

**Retained-mode / callback-based.** Closer to GTK/Qt than Elm. You build a view tree, attach callbacks, call `run()`. Easiest mental model for desktop GUI developers. Mature but less active than ratatui.

---

## What `bm` Actually Needs

`bm` does NOT need a full-screen TUI (no alternate screen buffer, no raw keyboard events, no frame-by-frame rendering). It needs:

1. **Interactive wizard flows** (`bm init`) — sequential prompts with validation
2. **Status dashboards** (`bm status`) — styled table printed to stdout
3. **Progress feedback** (`bm start`) — spinners during git/ralph operations
4. **Styled output** — colors, emoji, formatting

This is an **enhanced CLI**, not a TUI application.

---

## The Modern Option: cliclack

**cliclack** (138K downloads/month) is a Rust port of the popular npm `@clack/prompts` library used by SvelteKit, Astro, and other modern JS tools. It provides the most polished wizard experience in Rust:

- `intro()` / `outro()` framing for cohesive wizard flows
- `input()`, `select()`, `multiselect()`, `confirm()`, `password()` prompts
- Spinner integration
- Theme support
- Visual style is modern and clean — closest to bubbletea's Huh library for forms

vs. **inquire** (previously recommended):
- inquire has more prompt types (date picker, autocomplete, editor)
- cliclack has better visual cohesion (the intro/outro framing)
- Both are production-quality; they serve slightly different aesthetics

---

## Recommended Stack for `bm`

| Concern | Crate | Downloads | Why |
|---|---|---|---|
| Argument parsing | `clap` (derive) | 300M+ | De facto standard |
| Interactive prompts | `cliclack` or `inquire` | 138K/mo or millions | Modern wizard UX |
| Progress/spinners | `indicatif` | 90M+ | Multi-bar, spinner templates |
| Colors/styling | `console` | 100M+ | Cross-platform, emoji fallback |
| Tables | `comfy-table` | 58M+ | Auto-wrapping, ANSI styling |
| Errors | `anyhow` | Millions | Context-rich errors |

### When to escalate to ratatui

If a future feature demands a **live, interactive dashboard** (e.g., `bm monitor` streaming events with keyboard navigation between panes), then ratatui + crossterm becomes the right tool. The recommendation at that point:

- Implement TEA manually following [ratatui's own TEA guide](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
- Or use tui-realm for pre-built component lifecycle
- Do NOT use tears, bubbletea-rs, or r3bl_tui (too immature)

For M3 scope, none of this is needed.

---

## Sources

- [ratatui](https://ratatui.rs/) — 18.5K stars, 267 contributors
- [ratatui TEA pattern guide](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
- [tui-realm](https://github.com/veeso/tui-realm) — 858 stars
- [cursive](https://github.com/gyscos/cursive) — 4.7K stars
- [cliclack](https://github.com/fadeevab/cliclack) — Clack for Rust
- [BubbleTea vs Ratatui comparison](https://www.glukhov.org/post/2026/02/tui-frameworks-bubbletea-go-vs-ratatui-rust/)
- [Comparison of Rust CLI prompts](https://fadeevab.com/comparison-of-rust-cli-prompts/)
