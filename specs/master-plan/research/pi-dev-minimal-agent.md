# pi.dev — Evaluation Report

> Part of [Shift Week Plan](shift-week-plan.md) Day 2 tool evaluation.
> See also: [Agentic Tooling Landscape](agentic-tooling-landscape.md)

---

> [!warning] Name Confusion
> **pi.dev is NOT Inflection AI's Pi chatbot (pi.ai).** They share nothing but the name.
> - **pi.ai** = Inflection AI's empathic consumer chatbot (Mustafa Suleyman). Pivoted to enterprise B2B after Microsoft acqui-hired the team in March 2024. Effectively dead as a consumer product.
> - **pi.dev** = Mario Zechner's minimal terminal coding agent. A direct Claude Code competitor with 9.8k GitHub stars.
>
> The original [Agentic Tooling Landscape](agentic-tooling-landscape.md) incorrectly labeled pi.dev as "General AI, not dev-specific" — this was based on confusing it with pi.ai. **pi.dev is very much dev-specific.**

---

## Summary

| Field | Value |
|-------|-------|
| **Tool** | Pi (`pi`) |
| **Author** | Mario Zechner (badlogic) |
| **Category** | Alternative Coding Harness (Claude Code competitor) |
| **Website** | [pi.dev](https://pi.dev/) |
| **GitHub** | [badlogic/pi-mono](https://github.com/badlogic/pi-mono) |
| **Stars** | ~9.8k |
| **Install** | `npm install -g @mariozechner/pi` |
| **Language** | TypeScript |
| **License** | MIT |
| **Relationship to Claude Code** | **Direct competitor** — you'd use one or the other, not both |

---

## The Creator: Mario Zechner (badlogic)

Mario Zechner is an Austrian developer best known as the creator of **libGDX** — a cross-platform Java game framework with 24.8k GitHub stars that's been a cornerstone of indie game dev for over a decade.

- **Career arc**: Game engine developer → Sourcegraph-era dev tooling → independent AI agent builder
- **GitHub handle**: `badlogic`
- **Style**: Craftsman-type developer — opinionated, minimalist, skeptical of complexity for complexity's sake
- **Self-deprecating humor**: The alternate domain [shittycodingagent.ai](https://shittycodingagent.ai/) redirects to pi.dev

Pi was born out of frustration with Claude Code's growing complexity. Zechner's thesis: frontier models have been RL-trained so heavily that they inherently understand what a coding agent is. You don't need to spoon-feed them with dozens of specialized tools — you just need to get out of their way.

---

## The Philosophy: Radical Minimalism

Pi ships with exactly **4 tools**: `read`, `write`, `edit`, and `bash`. That's it. The system prompt is under 1,000 tokens. Everything else is your problem — in the best sense.

### What Pi Deliberately Omits (and Why)

| Feature | Claude Code | Pi | Pi's Rationale |
|---------|------------|-----|----------------|
| Built-in tools | ~15+ specialized | 4 (read, write, edit, bash) | "Adding tools just adds tokens without capability" |
| MCP support | First-class | No — use CLI tools + READMEs | Progressive disclosure saves tokens |
| Sub-agents | Built-in Task system | No — spawn pi via tmux, or build your own | Many valid approaches, don't pick one |
| Plan mode | Built-in | No — write plans to files | Not the harness's job |
| Permission popups | Yes | No — run in a container | Security at the infrastructure level |
| System prompt | Large, detailed | <1,000 tokens | Leave room for user context |
| Glob/Grep tools | Dedicated tools | No — use bash | Model knows bash already |

### The Core Argument

> "Frontier models have been RL-trained to be coding agents. They know what bash is. They know what files are. You don't need a specialized `Glob` tool — `find` works. You don't need a `Grep` tool — `rg` works. Every specialized tool you add costs tokens in the system prompt and reduces the context available for actual work."

---

## Architecture

### Four Execution Modes

1. **Interactive** — REPL-style, like Claude Code's default
2. **Print/JSON** — for scripting and CI pipelines (`pi --print "fix the tests"`)
3. **RPC** — for process integration (other tools can drive pi)
4. **SDK** — for embedding in your own apps (`import { Agent } from '@mariozechner/pi-agent-core'`)

### Extensibility System

Pi's minimalism in defaults is balanced by deep extensibility:

| Extension Point | What It Does |
|----------------|--------------|
| **TypeScript Extensions** | Hook into agent lifecycle events, register custom tools, intercept tool execution, add providers, modify behavior |
| **Skills** | Self-contained capability packages loaded on-demand (compatible with Claude Code and Codex CLI too) |
| **Prompt Templates** | Reusable prompt patterns |
| **Themes** | Visual customization |
| **Pi Packages** | Third-party installable bundles of the above |

Extensions are loaded via `jiti` for runtime TypeScript compilation — no build step needed.

### Multi-Provider Support

Pi supports: Anthropic, OpenAI (including Codex models gpt-5.1, gpt-5.2), Google, Azure, Bedrock, Mistral, Groq, Cerebras, xAI, Hugging Face, Kimi, MiniMax, OpenRouter, Ollama, and more.

- Switch models mid-session: `/model` or `Ctrl+L`
- Cycle through favorites: `Ctrl+P`
- Authenticate via API keys or OAuth

### Context Management

**Compaction** — auto-summarizes older messages when approaching the context limit. Customizable via extensions: topic-based compaction, code-aware summaries, or different summarization models.

---

## The "CLI Tools over MCP" Argument

This is Pi's most distinctive technical position and worth understanding in detail:

### MCP Approach (Claude Code)
```
Agent starts → all MCP tools loaded into context → tokens consumed permanently
```

### Pi's CLI Approach
```
Agent starts → minimal context → needs a tool → reads its README → invokes via bash → README context discarded
```

**The key difference**: MCP tools are **always loaded**. CLI tools with READMEs are **loaded lazily** (progressive disclosure). For agents running long sessions, this token efficiency difference compounds.

### Tradeoffs

| | MCP | CLI + README |
|-|-----|-------------|
| **Latency** | Fast (tool already loaded) | Slower (must read README first) |
| **Token cost** | Higher baseline (all tools in context) | Lower baseline, spiky on use |
| **Discoverability** | Agent knows all tools upfront | Agent must be told or discover tools |
| **Structured I/O** | Schema-enforced | Free-form (bash stdout) |
| **Error handling** | Typed errors | Exit codes + stderr |

---

## Traction & Benchmarks

- **9.8k GitHub stars**, 974 forks on the monorepo
- Pi powered **OpenClaw**, which hit **145k+ GitHub stars** in a single week
- Ran on **Terminal-Bench 2.0** against Codex, Cursor, and Windsurf with competitive results (submitted for leaderboard)
- Short CLI aliases added recently: `-ne` (no extensions), `-ns` (no skills), `-np` (no prompt templates) for fast scripting

---

## HyperShift Relevance Assessment

### Potential Fit

- **HyperShift has a rich CLI ecosystem** (`hypershift`, `oc`, `kubectl`, `jq`, `yq`). Pi's "just use bash" philosophy aligns — these tools don't need MCP wrappers
- **Multi-provider support** means you could use cheaper models for simple tasks and Claude for complex ones, within the same harness
- **SDK/RPC modes** enable building custom automation (e.g., a HyperShift-specific CI integration)
- **Extension system** could host HyperShift-specific skills without forking

### Concerns

- **Losing Claude Code's built-in advantages**: Task system, plan mode, sub-agent orchestration, Anthropic-tuned system prompt, native sandboxing — all gone
- **You'd be rebuilding**: Many things Claude Code provides out of the box would need to be built as pi extensions
- **Smaller ecosystem**: 9.8k stars vs Claude Code's massive Anthropic-backed ecosystem
- **You're already deep in Claude Code**: Your entire Shift Week infrastructure (CLAUDE.md, skills, hooks, this vault) is Claude Code native. Switching harnesses is a high-friction move
- **No built-in sandboxing**: Pi says "run in a container" — which means you need to set that up yourself

### Verdict

**Interesting philosophy, wrong time to switch.** Pi's minimalism is intellectually compelling and its "CLI over MCP" argument has merit for CLI-heavy workflows like HyperShift. But switching your primary harness mid-workflow while you're already invested in Claude Code's ecosystem would be high cost, low immediate return.

**Better approach**: Take the lessons (lazy tool loading, minimal system prompts, extension patterns) and apply them within Claude Code — e.g., slim down your CLAUDE.md, use skills for progressive disclosure, avoid loading unnecessary MCP servers.

---

## Key Reading & Watching

| Resource | Link |
|----------|------|
| Website | [pi.dev](https://pi.dev/) |
| GitHub (monorepo) | [badlogic/pi-mono](https://github.com/badlogic/pi-mono) |
| Design philosophy blog | [What I learned building an opinionated and minimal coding agent](https://mariozechner.at/posts/2025-11-30-pi-coding-agent/) |
| Skills repo | [badlogic/pi-skills](https://github.com/badlogic/pi-skills) |
| Architecture deep dive | [DeepWiki: pi-mono](https://deepwiki.com/badlogic/pi-mono) |
| Coding agent README | [packages/coding-agent/README.md](https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/README.md) |
| Skills documentation | [docs/skills.md](https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/docs/skills.md) |
| OpenClaw case study | [Anatomy of a Minimal Coding Agent Powering OpenClaw](https://medium.com/@shivam.agarwal.in/agentic-ai-pi-anatomy-of-a-minimal-coding-agent-powering-openclaw-5ecd4dd6b440) |
| npm package | [@mariozechner/pi](https://www.npmjs.com/package/@mariozechner/pi) |

---

## Appendix: Inflection AI's Pi (pi.ai) — Not This

For the record, since the original research confused them:

- **Inflection AI** was founded in 2022 by Reid Hoffman, Mustafa Suleyman, and Karén Simonyan
- Raised $1.5B at $4B valuation. Built Pi as an empathic consumer chatbot
- In March 2024, Microsoft acqui-hired nearly the entire 70-person team for $650M
- Suleyman became head of Microsoft AI. Pi was left behind
- The remaining team pivoted to enterprise B2B under new CEO Sean White
- Pi.ai still exists but is functionally a dead product — can't code, can't browse, can't analyze files
- Chronicled in the book *AI Valley* by Gary Rivlin
