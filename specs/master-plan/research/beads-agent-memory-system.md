# Beads — Evaluation Report

> Part of [Shift Week Plan](shift-week-plan.md) Day 2 tool evaluation.
> See also: [Agentic Tooling Landscape](agentic-tooling-landscape.md) (Category 3: Memory & Persistence)

---

## Summary

| Field | Value |
|-------|-------|
| **Tool** | Beads (`bd`) |
| **Author** | Steve Yegge |
| **Category** | Memory & Persistence (agent session continuity) |
| **GitHub** | [steveyegge/beads](https://github.com/steveyegge/beads) |
| **Install** | `brew tap steveyegge/beads && brew install beads` |
| **Language** | Go (~130k LOC, vibe-coded in 6 days) |
| **License** | MIT |
| **Relationship to Claude Code** | Complementary — persistence layer that works alongside any harness |

---

## The Creator: Steve Yegge

Steve Yegge is a veteran software engineer with ~40 years of coding experience:

- **GeoWorks** (1992) → **Amazon** (1998–2005, Senior Manager) → **Google** (2005–2018, Senior Staff SWE) → **Grab** (2018) → **Sourcegraph** (2022, Head of Engineering)
- Famous for "Stevey's Drunken Blog Rants" and an accidentally-public Google+ memo (2011) criticizing Google's platform strategy
- US Navy veteran (nuclear reactor operator) before CS degree at UW
- Built a graphical MUD called Wyvern; attempted "Rhino on Rails" at Google
- Post-Sourcegraph: became a leading voice in the vibe coding movement, co-authored *Vibe Coding* with Gene Kim
- Now fully focused on the agentic coding ecosystem (Beads → Gas Town)

---

## The Problem: "50 First Dates"

Every time you start a new Claude Code session, the agent wakes up with **zero memory** of what happened before. The context window fills up, the session ends, and the next agent starts from scratch. For any task spanning multiple sessions — i.e., anything real — this is crippling.

Existing workarounds (CLAUDE.md notes, TODO files, markdown plans) are:
- **Unstructured** — natural language, hard to query programmatically
- **Implicit dependencies** — agent must infer order from prose
- **Context-heavy** — entire spec files loaded into the window whether needed or not

---

## What Beads Is

A **git-native, agent-first issue tracker** — persistent working memory for coding agents.

### Core Properties

- **Single Go binary** — no server, no daemon, no Docker
- **Git IS the database** — JSONL files in `.beads/` committed to the repo. Branch code = branch the agent's memory. Merge code = merge memory
- **Hash-based IDs** (e.g., `bd-a1b2`) — prevent merge collisions in multi-agent workflows
- **SQLite local cache** — fast queries without parsing JSONL each time
- **DAG dependency tracking** — 4 types of issue links form a directed acyclic graph. `bd ready` returns the next unblocked task
- **Agent-first** — `--json` is the primary interface, not an afterthought. Designed for LLMs to consume
- **Semantic memory decay** — compaction auto-summarizes old closed tasks to save context window space. The database "forgets" fine-grained details while preserving essential context

### The 4 Dependency Types

Issues are linked together — like beads on a string — that agents follow to execute work in the right order:

1. **blocks / blocked-by** — task ordering
2. **parent / child** — hierarchical decomposition
3. **relates-to** — informational links
4. **duplicates** — dedup

### Practical Flow

```
1. Install bd
2. Add one line to CLAUDE.md pointing at it
3. Create issues:  bd create "Implement feature X" --depends-on 42
4. Agent runs:     bd ready  →  picks next unblocked task
5. Agent completes: bd close bd-a1b2
6. Session ends. New session starts. Agent runs bd ready again.
7. State survives because it's in the repo, not the context window.
```

---

## Beads vs. Alternatives

### vs. Spec-Driven Approaches (GSD, SDD, Ralph)

| Aspect | Spec-Driven | Beads |
|--------|-------------|-------|
| State format | Large markdown files | Structured DAG in JSONL |
| Dependencies | Implicit (natural language) | Explicit (graph edges) |
| Context cost | Heavy — whole spec loaded | Light — only current task |
| Nature | **Planning** tool | **Execution memory** tool |
| Queryable | Grep/regex | Structured queries (`bd ready`, `bd list`) |

They're complementary: planning happens in specs/PRDs/design docs. Beads tracks what you're actively building.

### vs. Claude Code Built-in Tasks

Anthropic's Claude Code team directly cited Beads as inspiration for the built-in Tasks system (`TaskCreate`/`TaskUpdate`).

| Aspect | Claude Code Tasks | Beads |
|--------|-------------------|-------|
| Scope | Session-level | Project-level (persists across sessions) |
| Storage | In-memory (dies with session) | Git-committed (lives forever) |
| Multi-agent | Within one session's sub-agents | Across machines via git |
| Dependency tracking | Basic blocking | Full DAG with 4 link types |
| Agent-agnostic | Claude Code only | Works with Amp, Codex, Cursor, Gemini CLI, etc. |

### vs. Plain TODO/Markdown Files

| Aspect | Markdown TODOs | Beads |
|--------|----------------|-------|
| Structure | Free-form | Schema-enforced |
| Dependencies | "Do X before Y" in prose | Explicit graph edges |
| Ready work | Agent must read & reason | `bd ready` returns actionable list |
| Multi-agent | Manual conflict resolution | Hash IDs + git merge driver |
| Memory management | Manual pruning | Automatic semantic decay |

---

## The Ecosystem

### Gas Town (the bigger project)

Beads is the **foundation layer** for Yegge's larger project: **Gas Town** — a multi-agent orchestration system coordinating 20–30 parallel Claude Code agents via tmux, git worktrees, and Beads as the work queue.

- Gas Town uses Beads as its task database
- Agents pick work from Beads, execute in isolated worktrees, merge results
- **GUPP protocol**: "If there is work on your hook, YOU MUST RUN IT" — solves the "agent stops" problem
- Cost: ~$100/hr in Claude tokens
- Explicitly for advanced users (Yegge's "Stage 7-8")

### Community Tools

| Tool | What |
|------|------|
| [beads_viewer](https://github.com/Dicklesworthstone/beads_viewer) | Web-based viewer for Beads databases |
| [beads_rust](https://github.com/nicholasgasior/beads-rs) | Rust port of Beads |
| [Spec Kit](https://github.com/jmanhype/speckit) | Bridges spec-driven planning with Beads execution tracking |

---

## Known Limitations

1. **Passive infrastructure** — agents won't proactively check Beads unless told to. You need `bd ready` in your CLAUDE.md or hook setup. Without it, the agent "forgets" Beads exists
2. **Context rot in long sessions** — even with hooks, long sessions can drift and the agent stops consulting Beads
3. **Not a planning tool** — doesn't replace specs, PRDs, or design docs. Only tracks execution
4. **Repo pollution** — `.beads/` JSONL files committed to the repo can be noise if the team isn't bought in
5. **Yegge-speed evolution** — the project moves fast and breaks things. API stability is not guaranteed

---

## Real-World Usage

- **DoltHub**: Used Beads to refactor **315 frontend files in a single 12-hour session** — the developer was doing leisure activities while the agent kept picking up work
- **Gas Town users**: Coordinating swarms of 20+ agents via Beads as the shared task queue
- **Anthropic inspiration**: Claude Code's built-in Tasks system was directly influenced by Beads' design

---

## HyperShift Relevance Assessment

### Potential Fit

- HyperShift tasks often span multiple sessions (long CI cycles, complex debugging)
- Git-native approach aligns with existing PR workflow
- DAG dependencies could model HyperShift's complex task chains (e.g., "fix controller → update tests → verify e2e → update docs")
- Multi-agent potential: different agents working on different components with shared state

### Concerns

- `.beads/` files in the HyperShift repo would need team buy-in (or use a separate tracking repo)
- HyperShift's test cycles are 1-2 hours — Beads doesn't speed up the feedback loop itself
- Passive nature means the agent still needs explicit prompting to use it
- Team adoption friction — only useful if other HyperShift contributors also use it

### Verdict

**Complementary tool, not a primary workflow driver.** Beads solves a real problem (session persistence) but doesn't address the core workflow challenges (sandboxing, autonomous execution, test feedback loops). Best evaluated as an add-on to whatever primary orchestrator wins.

---

## Key Reading & Watching

| Resource | Link |
|----------|------|
| Introducing Beads (blog) | [steve-yegge.medium.com](https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a) |
| The Beads Revolution (blog) | [steve-yegge.medium.com](https://steve-yegge.medium.com/the-beads-revolution-how-i-built-the-todo-system-that-ai-agents-actually-want-to-use-228a5f9be2a9) |
| Beads Best Practices (blog) | [steve-yegge.medium.com](https://steve-yegge.medium.com/beads-best-practices-2db636b9760c) |
| DoltHub real-world use | [dolthub.com](https://www.dolthub.com/blog/2026-01-27-long-running-agentic-work-with-beads/) |
| BetterStack guide | [betterstack.com](https://betterstack.com/community/guides/ai/beads-issue-tracker-ai-agents/) |
| Ian Bull overview | [ianbull.com](https://ianbull.com/posts/beads/) |
| Pragmatic Engineer interview | [newsletter.pragmaticengineer.com](https://newsletter.pragmaticengineer.com/p/amazon-google-and-vibe-coding-with) |
| From Beads to Tasks (Anthropic influence) | [paddo.dev](https://paddo.dev/blog/from-beads-to-tasks/) |
| GitHub repo | [github.com/steveyegge/beads](https://github.com/steveyegge/beads) |
| Gas Town (the orchestrator built on Beads) | [steve-yegge.medium.com](https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04) |
