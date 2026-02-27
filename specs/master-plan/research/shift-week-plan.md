# Shift Week — February 2026

## Background
Shift Week is a week once every OpenShift release (i.e. every 9 weeks) where we get to spend it doing some deep work, working on writing, learning, doing a relevant creative project, or any kind of innovative, relevant work that's away from the planned product features.

**Dates:** Mon Feb 9 — Sat Feb 14, 2026

---

## Goals

### Goal 1: Establish an Agentic Workflow for HyperShift Development (60%)
- There are a tons of ways for doing agentic, Claude Code assisted development.
- The area is uncharted, there're many techniques, projects, tools, etc. Here're a few keywords
  - Ralph Wiggum workflow
  - GSD
  - Spec driven development
  - Claude Code team
  - Gastown
  - Beads
- By the end of this week, I wanna establish a solid workflow and tooling to be the ideal setup for any HyperShift / OpenShift developer that has the following criteria:
  1. Well defined tools and process to tackle the workflow from requirements, implementation, testing, debugging, troubleshooting.
  2. Enables running agents unattended on a task while not worrying about security implications
    1. Claude Code needs to run either containerized or sandboxed.
    2. I built https://github.com/devguyio/zenzana out of necessity but I believe the way forward is https://nono.sh/
  3. Well established to build skills, automation, persisted context that empowers Claude Code to work in the complicated enviorment of HyperShift with minimal human intervention
  4. Well established way to build, preserve, and share the team knowledge.
  5. Not unique for HyperShift team, but can be standardized and shared with other teams
- Tools to evaluate:
  - https://mikeyobrien.github.io/ralph-orchestrator/ (current favorite, used heavily)
  - https://github.com/dlorenc/multiclaude
  - https://github.com/maxritter/claude-pilot
  - https://github.com/glittercowboy/get-shit-done
  - https://pi.dev/ (Mario Zechner's minimal terminal coding agent — direct Claude Code competitor, see [Pi.dev: Minimal Agent](pi-dev-minimal-agent.md))
  - https://agents.craft.do/ (lower priority — general agent platform)
- Next week, I wanna present the new agentic workflow for my team

### Goal 2: Build My Local AI Assistant (40%)
- I've just gotten a Mac Mini m4 pro 20C / 64GB
- Starting next week, I need to have my fully functional personal, local LLM based AI assistant.
- I need it for:
  - Organizing my mail
  - Organizing my docs
  - Being my empowered Wazir
- As a sub-goal to achieve there, is have a solid personal PC (my Mac Mini) / work laptop setup. I've a usb-c switch in place.

---

## Deliverables
1. **Tool evaluation scorecard** — see [Tool Evaluation & Decision](tool-evaluation-and-decision.md)
2. **Working agentic workflow** for HyperShift development (tested on a real task)
3. **Shareable setup guide** for the team — see [Workflow Guide](workflow-guide-template.md)
4. **Team presentation** (demo-driven, for next week)
5. **Running local LLM** on Mac Mini with one working use case

---

## Day-by-Day Plan

### Day 1 — Mon Feb 9 ✅
- [x] Mac Mini hardware setup
- [x] USB-C switch configured

### Day 2 — Tue Feb 10: Tool Landscape Research
**Original plan was 3-hour tool evaluation. Pivoted to deep landscape research instead (see [Tool Evaluation & Decision](tool-evaluation-and-decision.md)):**

| Block          | Duration | What                                                                                                         |
| -------------- | -------- | ------------------------------------------------------------------------------------------------------------ |
| Setup          | 30 min   | Review the 5 scoring criteria, set up the scorecard                                                          |
| Ralph baseline | 15 min   | Already known — score against criteria, note gaps                                                            |
| multiclaude    | 45 min   | Skim README, try basic setup, score. Key Q: does multi-agent parallelism help HyperShift's test/debug cycle? |
| GSD            | 45 min   | Same drill. Key Q: how does its spec model compare to Ralph?                                                 |
| claude-pilot   | 30 min   | Quick scan. If clearly worse, cut short                                                                      |
| Decision       | 15 min   | Score, compare, pick                                                                                         |

**Rest of the day:** Deep dive on the winner — set it up for a real HyperShift task.

### Day 3 — Wed Feb 11: POC Comparison
- [ ] Pick a real HyperShift task for both POCs
- [ ] POC A: Run the task through Ralph Orchestrator (3-hr time-box)
- [ ] POC B: Run the task through Claude Code native (3-hr time-box)
- [ ] Score both against the 7 workflow aspects (see [Ralph vs Claude Code POC](ralph-vs-claude-code-poc.md))
- [ ] Sandboxing: test nono.sh (time-box 2 hours). If it doesn't work → fall back to Zenzana

### Day 4 — Thu Feb 12: Finalize + Presentation
- [ ] Morning: Finalize the workflow guide — make it reproducible for the team
- [ ] Afternoon: Build team presentation. Demo-driven: show the actual HyperShift task run through the workflow

### Day 5 — Fri Feb 13: Local AI Assistant (Goal 2)
- [ ] Install Ollama or LM Studio on Mac Mini
- [ ] Get a model running (Llama 3.3 70B fits in 64GB, or Qwen 2.5 72B)
- [ ] Pick ONE use case: mail OR docs. Not both
- [ ] Connect to data source, test basic pipeline

### Day 6 — Sat Feb 14: Polish + Buffer
- [ ] Polish whichever goal needs it
- [ ] Refine presentation demo
- [ ] If on track: explore second Goal 2 use case
- [ ] If behind: catch-up day

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| nono.sh doesn't work smoothly | Time-boxed to 2 hours. Zenzana is the fallback |
| Tool setup takes too long | Each tool eval capped at 45 min. If setup alone > 15 min, that's a negative signal |
| Goal 2 rabbit hole | ONE use case only. "Running local LLM + basic prompt" is the deliverable |
| Presentation too abstract | Use real HyperShift task as live demo |

---

## Log
> Running notes, decisions, and observations as the week progresses.

### Day 1 — Mon Feb 9
- Mac Mini + USB-C switch setup complete. Goal 2 hardware unblocked.

### Day 2 — Tue Feb 10
- Conducted deep landscape research — mapped the full agentic dev tooling ecosystem (see [Agentic Tooling Landscape](agentic-tooling-landscape.md))
- **Key decision:** Deprioritize scatter/gather multi-agent orchestrators (multiclaude, Gas Town, etc.) — Claude Code's native sub-agent support covers this. Focus on workflow orchestration (spec-driven, plan→execute→verify)
- Wrote deep-dive reports on Beads ([Beads: Agent Memory System](beads-agent-memory-system.md)) and Pi.dev ([Pi.dev: Minimal Agent](pi-dev-minimal-agent.md))
- Original tool scorecard superseded — the real question narrowed to Ralph Orchestrator vs Claude Code native (see [Tool Evaluation & Decision](tool-evaluation-and-decision.md) for the discussion thread)
- Identified 7 transferable workflow aspects: SDD, context eng, prompt eng, agent memories, agent tasks, team knowledge, agent personas
- Reframed Day 3 around two POCs: Ralph vs Claude Code native, evaluated against the 7 aspects (see [Ralph vs Claude Code POC](ralph-vs-claude-code-poc.md))

### Day 3 — Wed Feb 11

### Day 4 — Thu Feb 12

### Day 5 — Fri Feb 13

### Day 6 — Sat Feb 14
