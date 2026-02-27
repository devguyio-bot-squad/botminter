# Agentic Tooling Research — Feb 2026

> Research conducted on Day 2 of [Shift Week Plan](shift-week-plan.md).
> Purpose: Discover the full landscape of agentic dev tools to inform the [Tool Evaluation & Decision](tool-evaluation-and-decision.md).

---

> [!important] Decision — Day 2
> **Scatter/gather (multi-instance parallel orchestration) → Claude Code native only.**
> Claude Code's built-in sub-agent orchestration (Opus 4.6) handles spawning multiple parallel agents natively. External tools that just multiplex Claude Code instances (multiclaude, Gas Town, Claude Squad, Conductor, claude-flow, Agent Farm, etc.) are **deprioritized**.
>
> **Ralph Wiggum-style orchestrators remain in scope.** These structure *how* an agent autonomously works through tasks (plan → execute → verify loops) — a fundamentally different problem from scatter/gather.

---

## The Big Picture

The agentic dev tooling space has exploded since late 2025. The ecosystem now has clear categories:

1. ~~**Scatter/Gather Orchestrators** — spawn multiple Claude Code instances in parallel~~ → **DEPRIORITIZED: using Claude Code native**
2. **Workflow Orchestrators** — Ralph Wiggum-style autonomous task execution (plan → execute → verify) ← **EVALUATE**
3. **Workflow Frameworks** — structure how you plan/execute with Claude Code (spec-driven, phased) ← **EVALUATE**
4. **Memory Systems** — persistence across sessions (the "50 First Dates" problem) ← **EVALUATE**
5. **Sandboxes** — safe unattended execution ← **EVALUATE**
5. **Meta / Configs** — curated skills, hooks, slash commands, CLAUDE.md collections

The most significant platform shift: **Claude Code itself now ships native multi-agent support** (Opus 4.6). A lead agent can coordinate sub-agents, assign subtasks, and merge results. This makes many external orchestrators less necessary for simpler use cases — but doesn't eliminate the need for structured workflows, persistence, or sandboxing.

### The Core Design Tension

**Brownian Ratchet** (multiclaude, Gas Town) vs **Spec-Driven** (GSD, SDD, Ralph).

- Brownian: throw many agents at a problem, let CI/tests filter results. Chaotic but parallel. Expensive.
- Spec-Driven: invest in planning upfront (requirements → design → tasks), then execute with confidence. Less waste, more control.

For HyperShift's long test/debug cycles, the likely sweet spot is **spec-driven planning + controlled parallelism** — not pure chaos.

---

## Category 1: Multi-Agent Orchestrators

### Gas Town
- **Author:** Steve Yegge
- **URL:** https://github.com/steveyegge/gastown
- **Install:** `brew install gastown` / `npm install -g @gastown/gt` / from source (Go)
- **What it is:** "Kubernetes for AI agents." A Go-based orchestrator managing 20-30 parallel agents via tmux + git worktrees, built on the Beads framework.
- **Architecture:**
  - Mayor orchestrates, Polecats execute in parallel, Witness and Deacon monitor
  - Seven well-defined worker roles
  - Persistent agent identity — sessions are cattle, agents are pets
  - Git worktree-based persistent storage
  - GUPP protocol: "If there is work on your hook, YOU MUST RUN IT" — solves the "agent stops" problem
- **Supports:** claude, gemini, codex, cursor, auggie, amp
- **Cost warning:** ~$100/hr in Claude tokens. ~10x a normal session per unit time.
- **Maturity:** Very early (weeks old as of Jan 2026). Wild but directionally correct.
- **Key reading:**
  - [Welcome to Gas Town](https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04)
  - [A Day in Gas Town (DoltHub)](https://www.dolthub.com/blog/2026-01-15-a-day-in-gas-town/)
  - [Gas Town's Agent Patterns (Maggie Appleton)](https://maggieappleton.com/gastown)
  - [GasTown and the Two Kinds of Multi-Agent](https://paddo.dev/blog/gastown-two-kinds-of-multi-agent/)

### multiclaude
- **Author:** Dan Lorenc (dlorenc)
- **URL:** https://github.com/dlorenc/multiclaude
- **Stars:** 468 | **Forks:** 45 | **Commits:** 227
- **Install:** `go install github.com/dlorenc/multiclaude/cmd/multiclaude@latest`
- **What it is:** Go-based orchestrator. Spawns independent Claude Code instances in tmux windows + git worktrees. Brownian Ratchet philosophy.
- **Architecture:**
  - Supervisor coordinates, Merge Queue auto-merges on CI pass, PR Shepherd for multiplayer
  - Worker = one task = one branch = one PR
  - Built-in Reviewer agent for automated code review
  - Two modes: Single Player (auto-merge) and Multiplayer (human reviewers on forks)
- **Strengths:** Clean design, self-hosting (agents wrote the codebase), good docs including Gastown comparison
- **Exports reusable Go packages:** `pkg/tmux`, `pkg/claude`

### Claude Squad
- **Author:** smtg-ai
- **URL:** https://github.com/smtg-ai/claude-squad
- **Install:** `brew install claude-squad` (runs as `cs`)
- **What it is:** Terminal TUI for managing multiple Claude Code / Codex / Aider / Gemini sessions. tmux + git worktrees.
- **Key differentiator:** Simplest onboarding. One terminal window to manage all instances. Supports YOLO/auto-accept mode for unattended work.
- **Supports:** Claude Code, Aider, Codex, OpenCode, Amp, Gemini

### Conductor
- **Author:** Melty Labs (Charlie Holtz, Jackson de Campos)
- **URL:** https://www.conductor.build/
- **What it is:** Mac-native GUI app for running multiple Claude Code agents in parallel with a visual dashboard.
- **Architecture:**
  - Git worktree isolation per agent
  - Uses your existing Claude Code auth (API key or Pro/Max plan)
  - Real-time monitoring of agent status and changes
- **Free tier:** Conductor itself is free; you pay Claude/OpenAI for API usage.
- **Limitation:** Mac-only.

### claude-flow
- **Author:** ruvnet (Reuven Cohen)
- **URL:** https://github.com/ruvnet/claude-flow
- **What it is:** MCP-based orchestration platform. 87 specialized tools, swarm topologies (hierarchical, mesh, ring, star), self-learning routing.
- **Claims:** 500K downloads, 100K monthly active users, 84.8% SWE-Bench solve rate, 32.3% token reduction
- **Architecture:**
  - V3 is a full rewrite — 250K+ lines TypeScript + WASM
  - Self-learning neural capabilities — learns from task execution
  - Cost optimization — routes simple tasks to cheaper models or skips LLM entirely
  - 64-agent system for enterprise orchestration
- **Caveat:** Very ambitious claims. "Ranked #1" is self-declared. Worth evaluating with skepticism.

### claude-code-by-agents
- **Author:** baryhuang
- **URL:** https://github.com/baryhuang/claude-code-by-agents
- **What it is:** Desktop app + API for multi-agent Claude Code orchestration via @mentions. Supports local + remote agents (Mac Mini, cloud instances).
- **Interesting:** No API key required (uses Claude CLI auth). Designed for distributed setups.

### Agent Farm
- **Author:** Dicklesworthstone
- **URL:** https://github.com/Dicklesworthstone/claude_code_agent_farm
- **Stars:** 619
- **What it is:** Run 20-50 Claude Code agents simultaneously with lock-based coordination and real-time monitoring dashboards.
- **Use cases:** Bug fixing, best practices sweeps, coordinated multi-agent development.

### sandboxed.sh
- **Author:** Th0rgal
- **URL:** https://github.com/Th0rgal/sandboxed.sh
- **What it is:** Self-hosted orchestrator with containerized workspaces (systemd-nspawn), web dashboard (Next.js) + iOS app (SwiftUI with PiP).
- **Key features:**
  - Dual runtime: Claude Code or OpenCode agents
  - Mission Control: start/stop/monitor agents remotely with real-time streaming
  - Git-backed library for skills, tools, rules, agents, MCPs
  - Supports multi-day unattended operations (e.g., give agent SSH to home GPU for training)
- **Why it stands out:** Combines orchestration + sandboxing + remote monitoring. Closest to the "unattended + secure + mobile monitoring" vision.

### Others Noted

| Tool | URL | One-liner |
|------|-----|-----------|
| Multi-Agent Shogun | https://github.com/yohey-w/multi-agent-shogun | Samurai-themed hierarchy (shogun → karo → ashigaru). YAML-file coordination |
| Tmux Orchestrator | https://github.com/Jedward23/Tmux-Orchestrator | Self-triggering agents, project manager coordination, persists across laptop close |
| CCManager | https://github.com/kbwo/ccmanager | Like Claude Squad but no tmux dependency, self-contained |
| Agent of Empires | https://github.com/njbrake/agent-of-empires | Rust-based, tmux + worktrees, optional Docker sandboxing |
| myclaude | https://github.com/cexll/myclaude | Multi-agent orchestration for Claude/Codex/Gemini/OpenCode with modular installer |
| Claude-Code-Workflow | https://github.com/catlog22/Claude-Code-Workflow | JSON-driven multi-agent framework with Gemini/Qwen/Codex CLI orchestration |
| Agent-Fusion | (via resource list) | 46 stars. Multi-agent: Claude Code + Codex + Amazon Q + Gemini bidirectional collaboration |
| Agentrooms | https://claudecode.run/ | Route tasks to specialized agents, @mentions coordination |

---

## Category 2: Workflow Frameworks (Spec-Driven / Planning)

### GSD (Get Shit Done)
- **Author:** glittercowboy
- **URL:** https://github.com/glittercowboy/get-shit-done (now github.com/gsd-build/get-shit-done)
- **Stars:** 12.9K | **Forks:** 1.2K
- **Install:** `npx get-shit-done-cc@latest`
- **What it is:** A meta-prompting and context engineering system. Solves "context rot" through systematic 6-phase cycles.
- **Workflow:**
  1. **Discuss** — capture decisions before planning
  2. **Plan** — parallel researchers investigate, planners create atomic XML tasks, checkers verify iteratively
  3. **Execute** — run plans in parallel waves, fresh 200K context per execution wave, commit per task
  4. **Verify** — manual UAT with automated diagnostics
  5. **Complete Milestone** — archive and tag
  6. **New Milestone** — repeat
- **Context engineering:**
  - Structured files: `PROJECT.md`, `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`, `PLAN.md`, `SUMMARY.md`
  - Size-limited docs prevent context window degradation
  - XML task format with `<name>`, `<files>`, `<action>`, `<verify>`, `<done>` tags
- **Key features:**
  - Quick Mode for small fixes (skips research/verify)
  - Brownfield support (`/gsd:map-codebase` for existing projects)
  - Session management (`/gsd:pause-work` / `/gsd:resume-work`)
  - Model profiles: quality/balanced/budget
- **Supports:** Claude Code, OpenCode, Gemini CLI
- **Community:** Discord, used by engineers at Amazon, Google, Shopify, Webflow
- **Recommendation:** `--dangerously-skip-permissions` for unattended use

### Spec Driven Development (SDD)
- **URL:** https://medium.com/@universe3523/spec-driven-development-with-claude-code-206bf56955d0
- **Not a tool — a pattern.** Inspired by Kiro's spec-driven approach.
- **Architecture:**
  1. Requirements Definition Agent — gathers requirements, asks probing questions, creates spec
  2. Design/Architecture Agent — translates to technical design, data models, roadmap
  3. Implementation Agent — executes with minimal interruptions
- **Key insight:** Review 3 documents upfront (requirements, design, tasks) instead of dozens of approvals during implementation. Approval count drops dramatically.

### AB Method
- **Author:** Ayoub Bensalah
- **Via:** awesome-claude-code
- **What it is:** Principled spec-driven workflow using Claude Code's native sub-agents. Transforms large problems into focused, incremental missions.

### Ralph Orchestrator
- **Author:** mikeyobrien
- **URL:** https://mikeyobrien.github.io/ralph-orchestrator/
- **Already on your shortlist and used heavily.** Implements the "Ralph Wiggum" technique for autonomous task completion.

### claude-pilot (Claude CodePro)
- **Author:** Max Ritter (maxritter)
- **URL:** https://github.com/maxritter/claude-pilot
- **What it is:** Professional dev environment with spec-driven workflow, TDD enforcement, cross-session memory, semantic search, quality hooks, and modular rules integration.

### Pi (pi.dev)
- **Author:** Mario Zechner (badlogic) — creator of libGDX (24.8k stars)
- **URL:** https://pi.dev/ | https://github.com/badlogic/pi-mono
- **Stars:** ~9.8K | **Forks:** 974
- **Install:** `npm install -g @mariozechner/pi`
- **What it is:** A radically minimal terminal coding agent. Direct Claude Code competitor. Ships with only 4 tools (read, write, edit, bash) and a <1,000 token system prompt.
- **Philosophy:** Frontier models are already RL-trained to be coding agents. Don't spoon-feed them with specialized tools — get out of their way.
- **Key differentiator:** "CLI tools over MCP" — instead of loading all MCP tools into context permanently, agents discover CLI tools via READMEs on demand (progressive disclosure, lower token baseline).
- **Extensible via:** TypeScript Extensions, Skills (cross-compatible with Claude Code), Prompt Templates, Themes, Pi Packages.
- **Supports:** Anthropic, OpenAI (incl. Codex gpt-5.x), Google, Azure, Bedrock, Mistral, Groq, xAI, Ollama, and more.
- **4 execution modes:** Interactive, Print/JSON (scripting), RPC (process integration), SDK (embedding).
- **Notable:** Powered OpenClaw (145k+ GitHub stars in one week). Competitive on Terminal-Bench 2.0.
- **Assessment:** Interesting philosophy but switching primary harness mid-workflow is high friction. Better to take the lessons (lazy loading, minimal system prompts) and apply within Claude Code.
- **Full report:** [Pi.dev: Minimal Agent](pi-dev-minimal-agent.md)

---

## Category 3: Memory & Persistence

### Beads
- **Author:** Steve Yegge
- **URL:** https://github.com/steveyegge/beads
- **What it is:** A git-backed JSONL issue tracker designed for agents. Solves the "50 First Dates" problem — agents waking with no memory of yesterday.
- **Architecture:**
  - Issues stored as JSONL in `.beads/`, versioned/branched/merged with code
  - Hash-based IDs (bd-a1b2) prevent merge collisions in multi-agent workflows
  - SQLite local cache for speed, background daemon for auto-sync
  - Dependency-aware task graph — auto-detects ready tasks
  - Semantic "memory decay" compaction summarizes old closed tasks to save context window
- **Why it matters:**
  - When you branch code, you automatically branch the agent's context and task graph
  - When you merge, you merge the agent's memory too
  - Queryable (unlike markdown plans), structured, explicit semantics
- **Multi-agent:** Naturally distributed. Workers on multiple machines share the same beads database via git.
- **Pair with:** MCP Agent Mail for inter-agent messaging. "Beads gives agents shared memory, Agent Mail gives them messaging — that's all they need."
- **Ecosystem:** Rust port exists (beads_rust), viewer tool (beads_viewer)
- **Key reading:**
  - [Introducing Beads](https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a)
  - [Beads Best Practices](https://steve-yegge.medium.com/beads-best-practices-2db636b9760c)
  - [The Beads Revolution](https://steve-yegge.medium.com/the-beads-revolution-how-i-built-the-todo-system-that-ai-agents-actually-want-to-use-228a5f9be2a9)
  - [Beads: Git-Backed Memory (YUV.AI)](https://yuv.ai/blog/beads-git-backed-memory-for-ai-agents-that-actually-remembers)

### Continuous-Claude-v2
- **Stars:** 2.2K
- **What it is:** Context management via hooks — maintains state through ledgers and handoffs. MCP execution without context pollution. Agent orchestration with isolated context windows.

---

## Category 4: Sandboxing & Security

### Claude Code Native Sandbox
- **URL:** https://code.claude.com/docs/en/sandboxing
- **Blog:** https://www.anthropic.com/engineering/claude-code-sandboxing
- **What it is:** Built into Claude Code. Uses Linux bubblewrap / macOS seatbelt for OS-level filesystem + network isolation.
- **Results:** Reduced permission prompts by 84% in Anthropic's internal usage.
- **Boundaries:**
  - Filesystem: read/write to CWD and subdirs, read-only elsewhere (with some blocked dirs)
  - Network: only approved servers
  - Even a prompt-injected Claude is fully isolated
- **Open source:** Sandbox runtime available as npm package for use in custom agents.
- **Platforms:** macOS, Linux, WSL2 (not WSL1, native Windows planned)

### nono.sh
- **Author:** Luke Hinds
- **URL:** https://nono.sh/ | https://github.com/lukehinds/nono
- **License:** Apache 2.0
- **What it is:** Kernel-enforced capability sandbox. Landlock (Linux) / Seatbelt (macOS).
- **Key properties:**
  - Agent-agnostic (Claude, GPT, opencode, any process)
  - Once restrictions applied, no API to escape — not even for nono itself
  - Children inherit all restrictions (no subprocess escalation)
  - Blocks SSH keys, AWS creds, shell configs, destructive commands (rm, dd, chmod) by default
- **Status:** Early, security auditing ongoing, ready for experimentation.

### Docker Sandboxes (Feb 2026)
- **URL:** https://www.docker.com/blog/docker-sandboxes-run-claude-code-and-other-coding-agents-unsupervised-but-safely/
- **What it is:** Docker's official solution for running coding agents unattended. Full container isolation.
- **Supports:** Claude Code, Codex CLI, Copilot CLI, Gemini CLI, Kiro
- **Removes:** permission prompts, system risk, Docker-in-Docker limitations

### sandbox-agent
- **Author:** Rivet
- **URL:** https://github.com/rivet-dev/sandbox-agent
- **What it is:** Run coding agents in sandboxes, control over HTTP. Supports Claude Code, Codex, OpenCode, Amp.

### Zenzana
- **Author:** Ahmed (you)
- **URL:** https://github.com/devguyio/zenzana
- **What it is:** Your own containerized Claude Code execution solution.
- **Note:** With native Claude Code sandboxing and Docker Sandboxes now available, Zenzana's niche has narrowed. May still have value for custom HyperShift-specific constraints.

---

## Category 5: Meta-Resources & Curated Lists

| Resource | URL | What |
|----------|-----|------|
| awesome-claude-code | https://github.com/hesreallyhim/awesome-claude-code | Primary curated list — skills, hooks, commands, orchestrators, plugins |
| awesome-claude-code-toolkit | https://github.com/rohitg00/awesome-claude-code-toolkit | 135 agents, 35 skills, 42 commands, 120 plugins, 19 hooks, 15 rules, 7 templates |
| awesome-claude-skills | https://github.com/ComposioHQ/awesome-claude-skills | Practical skills for Claude.ai, Claude Code, and API |
| everything-claude-code | https://github.com/affaan-m/everything-claude-code | Anthropic hackathon winner's battle-tested configs — agents, skills, hooks, commands, MCPs |
| awesome-claude-code-subagents | https://github.com/VoltAgent/awesome-claude-code-subagents | 100+ specialized subagents with isolated context spaces |
| awesome-skills.com | https://awesome-skills.com/ | Web directory of 100+ curated skills and plugins |
| Agentic Workflow Patterns | by ThibautMelen (via awesome-claude-code) | Collection of patterns from Anthropic docs: Subagent Orchestration, Progressive Skills, Parallel Tool Calling, Master-Clone Architecture |

---

## Category 6: Platform-Level Developments

### Claude Code Native Multi-Agent (Opus 4.6)
Claude Code can now spawn agent teams natively. A lead agent coordinates, assigns subtasks, merges results. For simpler multi-agent needs, this may make external orchestrators unnecessary.

### Claude Agent SDK
Build custom agents with full control over orchestration, tool access, and permissions. Composable with Microsoft Agent Framework for mixing Claude agents with Azure OpenAI, GitHub Copilot, etc.

### GitHub Agent HQ (Feb 2026)
GitHub now supports Claude, Codex, and Copilot as first-class agents within repo workflows. Developers can assign different agents to different tasks within the same repo.

### Anthropic Agentic Coding Trends Report
Official report on how coding agents are reshaping development.
URL: https://resources.anthropic.com/hubfs/2026%20Agentic%20Coding%20Trends%20Report.pdf

---

## Key Reading List

| Article | Author | URL |
|---------|--------|-----|
| The future of agentic coding: conductors to orchestrators | Addy Osmani | https://addyosmani.com/blog/future-agentic-coding/ |
| Welcome to Gas Town | Steve Yegge | https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04 |
| The Future of Coding Agents | Steve Yegge | https://steve-yegge.medium.com/the-future-of-coding-agents-e9451a84207c |
| Introducing Beads | Steve Yegge | https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a |
| Making Claude Code more secure and autonomous | Anthropic | https://www.anthropic.com/engineering/claude-code-sandboxing |
| Docker Sandboxes for coding agents | Docker | https://www.docker.com/blog/docker-sandboxes-run-claude-code-and-other-coding-agents-unsupervised-but-safely/ |
| Gas Town's Agent Patterns | Maggie Appleton | https://maggieappleton.com/gastown |
| Multi-agent orchestration for Claude Code in 2026 | Shipyard | https://shipyard.build/blog/claude-code-multi-agent/ |
| The Rise of Coding Agent Orchestrators | Aviator | https://www.aviator.co/blog/the-rise-of-coding-agent-orchestrators/ |
| GasTown and the Two Kinds of Multi-Agent | paddo.dev | https://paddo.dev/blog/gastown-two-kinds-of-multi-agent/ |
| Spec Driven Development with Claude Code | Wataru Takahashi | https://medium.com/@universe3523/spec-driven-development-with-claude-code-206bf56955d0 |
| Best AI Coding Agents for 2026 | Faros AI | https://www.faros.ai/blog/best-ai-coding-agents-2026 |

---

## Suggested Additions to Evaluation Shortlist

Based on this research, tools worth adding to the [Tool Evaluation & Decision](tool-evaluation-and-decision.md) beyond the original four:

1. **Beads** — Not an orchestrator but solves the persistence problem. Works with any orchestrator. Standalone value.
2. **Claude Squad** — Simplest path to multi-agent. Lower friction than multiclaude.
3. **sandboxed.sh** — Unique combo of orchestration + sandboxing + remote monitoring.
4. **GSD** — Already on the list but the 12.9K stars and structured spec-driven approach make it the strongest workflow framework.
5. **Claude Code Native Sandbox + Docker Sandboxes** — Platform-level solutions that may eliminate the need for custom sandboxing.

## Tools to Deprioritize

- ~~**pi.dev** — General AI, not dev-specific. Not competitive in this space.~~ **CORRECTED:** pi.dev is Mario Zechner's (badlogic) minimal terminal coding agent with 9.8k stars — a **direct Claude Code competitor**, not Inflection AI's Pi chatbot (pi.ai). It IS dev-specific. Recategorized below. See [Pi.dev: Minimal Agent](pi-dev-minimal-agent.md).
- **agents.craft.do** — General agent platform, not focused on code development.
- **claude-flow** — Ambitious claims but self-declared "Ranked #1", very high complexity. Evaluate with skepticism.
