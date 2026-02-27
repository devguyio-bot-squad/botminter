# Ralph-Orchestrator Capabilities Audit

> Research findings from deep exploration of `/opt/workspace/ralph-orchestrator/`.
> Assessed against the rough-idea requirements for the HyperShift Agentic Team POC.

---

## 1. Architecture & Execution Model

Ralph is a Rust CLI that wraps AI agents (Claude Code, Kiro, Gemini, Codex, Amp) in an orchestration loop.

**Core concepts:**
- **Hats** = personas/phases. Each hat has: name, description, triggers (events it listens to), publishes (events it emits), instructions (system prompt).
- **Events** = signals between hats. Stored as JSONL in `.ralph/events-{timestamp}.jsonl`. Hats chain by publishing events that trigger other hats.
- **Sequential execution** — one hat active at a time (Ralph-Wiggum style). No swarm.
- **Per-hat backends** — each hat can use a different AI tool (Claude for coding, Kiro for research, Gemini for review, etc.).

**Config structure:**
```yaml
event_loop:
  starting_event: "spec.start"
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 100
  max_runtime_seconds: 14400
  persistent: false  # true = loop stays alive after completion

cli:
  backend: "claude"  # default backend

hats:
  hat_name:
    name: "Display Name"
    description: "What this hat does"
    triggers: ["event.a", "event.b"]
    publishes: ["event.c", "event.d"]
    instructions: |
      System prompt for this hat...
    backend: "kiro"  # optional per-hat override
    default_publishes: "event.c"  # fallback if hat forgets to emit
    max_activations: 3  # limit retries
```

**Verdict:** Architecture maps well to the team concept. Hats = team members. Events = handoffs. Sequential = one person works at a time.

---

## 2. Phase System (Hats as Phases)

Phases ARE hats. The phase chain is defined by event routing:

```
spec.start → SpecWriter → spec.ready → SpecCritic → spec.approved → Implementer → implementation.done → Verifier → task.complete
```

**Existing presets:**

| Preset | Flow | Use Case |
|--------|------|----------|
| `feature.yml` | Builder → Reviewer | Standard feature dev |
| `spec-driven.yml` | SpecWriter → SpecCritic → Implementer → Verifier | Spec-first pipeline |
| `code-assist.yml` | Planner → Builder → Validator → Committer | TDD from any input |
| `bugfix.yml` | PlotAuthorization → Builder | Bug fixes |
| `review.yml` | Reviewer (multi-perspective) | Code review |
| `research.yml` | Researcher | Investigation |
| `refactor.yml` | Refactorer with quality gates | Refactoring |

**Supports:**
- Prompt-driven (instructions in YAML)
- Spec-driven (hat reads a spec file as input)
- Hybrid (conversation fills a template)

**Verdict:** Phase system is fully configurable via YAML. Can define any SDLC pipeline. No code changes needed for new phase chains.

---

## 3. Personas & Tool Sandboxing

**Personas = Hats.** Each hat gets its own:
- Instructions (system prompt)
- Backend (AI tool)
- Event triggers/publishes
- `extra_instructions` with YAML anchors for shared blocks

**Per-hat backend flexibility:**
```yaml
hats:
  dev:
    backend: "claude"       # Claude for coding
  researcher:
    backend:
      type: "kiro"
      agent: "researcher"   # Kiro with custom agent config
  reviewer:
    backend: "gemini"       # Different model for review
```

**Tool sandboxing:**
- Ralph disables Claude Code's TodoWrite/TaskCreate/TaskUpdate tools (uses its own task system)
- No per-hat tool restriction config in ralph itself — relies on backend-level controls
- Kiro agents have per-agent `tools`, `allowedTools`, `mcpServers` config
- Claude Code has `.claude/settings.json` for permissions

**Gap:** No declarative per-hat tool sandboxing in ralph YAML. Would need backend-level config (Kiro agents, Claude settings) or a ralph enhancement.

---

## 4. Backpressure & Gating

**Event routing IS the gating mechanism:**
- Hat B only triggers when hat A publishes the right event
- Reject/retry loops: `spec.rejected` → SpecWriter retries
- `default_publishes`: safety net if hat forgets to emit an event
- `max_activations`: limit how many times a hat can fire (prevents infinite loops)

**Safety limits:**
- `max_iterations`: hard cap on total loop iterations
- `max_runtime_seconds`: wall clock limit
- `max_cost_usd`: cost cap
- `max_consecutive_failures`: stop after N failures

**Human-in-the-loop (Telegram RObot):**
- `human.interact`: agent asks a question, loop blocks until reply or timeout
- `human.response`: human's reply injected into next iteration
- `human.guidance`: proactive message from human injected into agent's prompt
- Per-loop routing for parallel loops

**Gap:** No formal confidence scoring, evidence reports, or review triage. Would need to be built as hat instructions + eval prompts. No automated CI gating — agents can run `make test` but ralph doesn't enforce test passage as a hard gate.

---

## 5. Knowledge & Memory

**Agent memories:**
- Stored in `.ralph/agent/memories.md`
- Persistent across sessions (survives loop restarts)
- Auto/Manual/None inject modes
- `ralph tools memory add "learning"` — agent can add memories
- Filter by type, tags, time window
- Token budget control

**Scratchpad:**
- `.ralph/agent/scratchpad.md` — ephemeral per-loop working notes
- Cleared on fresh runs, preserved on resume

**Context engineering:**
- CLAUDE.md auto-loaded as project context
- Hat instructions injected per phase
- Skills system (`.claude/skills/`) for reusable capability packages

**Gap:** No team-shared knowledge base. Memories are per-agent, not per-team. For team knowledge (shared patterns, troubleshooting guides), would need: git-tracked knowledge files + instructions that tell agents to check them. This is doable without code changes — just file convention + hat instructions.

---

## 6. Task System

**`ralph task` CLI:**
```bash
ralph task add "Title" --priority 2 --description "desc"
ralph task list [--status open|in_progress|closed|failed]
ralph task ready [--all]   # unblocked tasks
ralph task close <id>
ralph task fail <id>
ralph task show <id>
```

**Storage:** `.ralph/agent/tasks.jsonl` (append-only JSONL, file-locked for concurrency)

**Features:**
- Priority 1-5
- `blocked_by` dependencies
- `loop_id` tagging (which loop owns this task)
- Status: Open → InProgress → Closed / Failed

**Gap:** No hierarchical planning (Feature → Epic → Story). Tasks are flat work items. The JIRA hierarchy would need to be mapped externally (GitHub issues for team-facing, ralph tasks for internal agent work).

---

## 7. GitHub Integration

**No native GitHub API integration.** Ralph doesn't create PRs, issues, or manage branches via API.

**What exists:**
- Git worktree system for parallel loops (each loop gets an isolated branch)
- Merge queue for tracking and sequencing merges to main
- Auto-commit changes before merge
- Works with forks implicitly (clone fork → run ralph in fork directory)

**Gap:** For team fork workflow (branches, issues, PRs), agents would need to use `gh` CLI or GitHub MCP server via their backend. This is a configuration task, not a code change.

---

## 8. Prompt System

- `PROMPT.md` = the task prompt (what to do)
- Hat `instructions` = system prompt per phase (how to do it)
- `extra_instructions` with YAML anchors = shared instruction blocks across hats
- Skills (`.claude/skills/`) = reusable capability packages (PDD, TDD, etc.)
- Guardrails in `core.guardrails` = rules injected into every hat

**Verdict:** Flexible enough. Custom prompts per hat, shared instruction blocks, skill system for reusable patterns.

---

## 9. Team Process Storage & Evolution

**The problem:** The rough-idea requires the team to *"self-correct, explore process optimization"* and support retrospective sessions where *"the process gets modified."* But in Ralph, the team process — how work flows, what gates apply, how personas interact — is spread across static config files:

| Artifact | Stores | Agents can modify? |
|----------|--------|-------------------|
| `ralph.yml` | Workflow config (hats, events, gates) | No |
| Hat `instructions` | Per-persona behavior rules | No |
| `core.guardrails` | Rules injected into every hat | No |
| `.ralph/agent/memories.md` | Per-agent learning | Yes — but per-agent, not team-wide |
| `.claude/skills/` | Reusable capability packages | No |
| `CLAUDE.md` | Project context | No |

**There is no living "team process document"** that agents read, follow, and propose changes to. The process IS the config, and config is static.

**Design options:**
1. **Living document** (e.g., `PROCESS.md`) — agents read as instructions, can propose changes, PO gates
2. **ralph.yml as the process** — process changes = config changes, also PO-gated
3. **Both** — human-readable process doc + machine-readable ralph.yml, kept in sync

**Verdict:** This is a design decision for the POC. Needs to be addressed in the design phase.

---

## Gap Analysis: What Needs Building vs Configuring

| Need | Ralph Status | Action |
|------|-------------|--------|
| Multiple personas (dev, QE, architect, reviewer) | **Ready** — hats | Configure YAML |
| Team fork with branches/PRs | **Partial** — git worktrees exist, no GH API | Configure `gh` CLI access |
| Feature → Epic → Story breakdown | **Not built** — flat tasks only | Use GH issues externally + ralph tasks internally |
| Cooperative sprint planning | **Not built** — no interactive planning session | Build as a hat + human.interact via Telegram |
| PO/Tech Lead gating | **Ready** — human.interact + Telegram | Configure RObot |
| Knowledge accumulation | **Partial** — memories exist but per-agent | Add team knowledge files in repo + hat instructions |
| Eval/confidence system | **Not built** | Build as reviewer hat with eval prompts |
| CI integration | **Not built** — agents can run commands | Configure agents to run tests, report results |
| Configurable tools per persona | **Partial** — per-hat backends, not per-hat tool restrictions | Use backend-level config (Kiro agents, Claude settings) |
| Team process storage & evolution | **Not built** — process is static config (ralph.yml + hat instructions), no living process document | Design needed: living PROCESS.md + ralph.yml sync, gated by PO |

### What requires NO code changes (config only):
- Defining 4 personas as hats (dev, QE, architect, dev-reviewer)
- Phase chain via event routing
- Human-in-the-loop via Telegram
- Agent memories
- Team knowledge as repo files + instructions
- Running tests/linting via agent bash access

### What might require ralph modifications:
- Interactive sprint planning session (cooperative breakdown with human)
- Formal eval system with scored dimensions
- Team-level (not agent-level) knowledge accumulation
- GitHub issue creation/management as first-class concept
- Team process as a living, evolvable document (not just static config)
