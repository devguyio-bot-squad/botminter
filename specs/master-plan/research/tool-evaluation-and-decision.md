# Agentic Workflow Tool Evaluation Scorecard

> Time-box: 3 hours total. Score each tool 1-5 on each criterion.
> Part of [Shift Week Plan](shift-week-plan.md)

## Evaluation Criteria
>>> this is no longer needed, since the only contestant left from the tools we selected are ralph-orchestrator (see decision on top [Agentic Tooling Landscape](agentic-tooling-landscape.md)), I don't have time now to explore other tools that might fit.
>>> so currently here're my thoughts
>>> 1) the tools are evolving rapidly. what might fit best today can be obsoleted next week by a new, game changer feature in claude code itself.
>>> 2) so it's better to focus on:
>>>  - the workflow that fits the team best.
>>>  - ideally that workflow covers the following aspects:
>>>      - spec-driven-development
>>>      - context engineering
>>>      - prompt engineering
>>>      - agent memories
>>>      - agent tasks
>>>      - team knowledge
>>>      - agent personas
>>> 3) so the goal now is to decide on the following three options:
>>>       A) Use ralph-orchestrator
>>>       B) Use Claude Code + plugins / workflows etc
>>>       C) Fork ralph-orchestrator
>>>   4) I can see two POCs that can help me decide:
>>>       A) Full POC using ralph-orchestrator
>>>       B) Full POC using Claude Code
(CC-->) [2026-02-11 08:55] Agreed — the multi-tool scorecard is dead weight now. Your reframing is sharper.
(CC--)   Here's my read on the A/B/C decision:
(CC--)
(CC--)   **Option C (Fork Ralph) — eliminate first.** Maintenance burden, upstream divergence, and
(CC--)   if Claude Code ships features that overlap (which it will), the fork becomes dead weight
(CC--)   you own forever. Forking is the "build it yourself" trap disguised as pragmatism.
(CC--)
(CC--)   **Option A (Ralph as-is) vs B (Claude Code native) — this is the real question.**
(CC--)   Ralph's actual value isn't the tool — it's the *workflow pattern*: plan → execute → verify,
(CC--)   with structured task decomposition. That pattern can be encoded as Claude Code skills.
(CC--)
(CC--)   Your 7 workflow aspects map to Claude Code native like this:
(CC--)   | Aspect | Claude Code Native Mechanism |
(CC--)   |--------|------------------------------|
(CC--)   | spec-driven-development | Skill (plan mode + structured spec templates) |
(CC--)   | context engineering | CLAUDE.md + skills + hooks |
(CC--)   | prompt engineering | Skills + prompt templates in repo |
(CC--)   | agent memories | Beads (add-on) or custom persistence skill |
(CC--)   | agent tasks | Built-in Task system + Beads for cross-session |
(CC--)   | team knowledge | Git-committed CLAUDE.md, skills, hooks — already shareable |
(CC--)   | agent personas | CLAUDE.md persona sections (you're literally doing this with Wazir) |
(CC--)
(CC--)   The gap in Option B is the autonomous workflow loop — Ralph gives you plan→execute→verify
(CC--)   out of the box. But building that as a Claude Code skill is a bounded task, not a moonshot.
(CC--)
(CC--)   **My lean: B, with Ralph's workflow patterns extracted as skills.** Zero external dependency,
(CC--)   everything compounds with Claude Code updates, and the team adopts Claude Code — not
(CC--)   Claude Code + Ralph.
(CC--)
(CC--)   **On the POCs** — smart. Same HyperShift task for both, apples to apples. I'd suggest
(CC--)   time-boxing each to 3 hours max and evaluating against your 7 aspects afterward.
(CC--)
(CC--)   **Proposed action:** Restructure this document — replace the dead scorecard with the
(CC--)   A vs B POC framework. Want me to do that?
>>> The reasoning behind the fork is :
>>>     A) RO (Ralph Orchestrator) vs Claude Code: RO abstracts claude code to being an agent, not an orchestrator. From an orchestrator POV, the workflow is easy to build and compose in a single yaml file via the hats and events planner --publishes plan.done-> builder -build.done->tester--test.failed->planner etc. It also added all the configurations bundled in a single tool (tasks tracking, SDP, ralph loop etc) and using Claude to implement that will require mix and matching tons of plugins etc.
>>>     B) Why fork? The trend currently is that tool development is easy given the state of vibe coding now. Everyone is building the best tool that suits their needs and reeping that benefit. Specially for tooling, the old argument of maintenance burden etc is largely mitigated. And given our focus on the workflow, the tool can be thrown away if we end up finding the better tool, as long as those transferable items mentioned in point 2 above is well maintained.
>>>     C) the POC scope will be around comparing using Claude Code (hooks, skills, plugins, task lists, orchestrator?) vs Ralph (with a possibility of forking) to have a ralph-wiggum orchestrator that includes all the items in point 2 in a configureble way
(CC-->) [2026-02-11 09:00] Fair — I was too quick to dismiss the fork. You're making a different
(CC--)   argument than the one I was countering.
(CC--)
(CC--)   The key distinction: **the workflow is the asset, the tool is disposable.** If you
(CC--)   separate those cleanly — the 7 transferable aspects (SDD, context eng, memories, etc.)
(CC--)   live in git-portable artifacts, while the orchestrator is just a runner — then the fork
(CC--)   risk is genuinely low. You can throw away the tool without losing the knowledge.
(CC--)
(CC--)   Point A is the strongest argument: Ralph's hat/event composition
(CC--)   (`planner --plan.done→ builder --build.done→ tester --test.failed→ planner`) is a
(CC--)   fundamentally different abstraction layer than Claude Code's skill/hook system. Claude
(CC--)   Code doesn't have a native event-driven orchestration loop — you'd be duct-taping one
(CC--)   together from hooks, and that's fragile.
(CC--)
(CC--)   Revised position: **POC A (Ralph/fork) vs POC B (Claude Code native) is the right
(CC--)   comparison.** Let the POCs settle it empirically. Option C (fork) stays viable.
(CC--)
(CC--)   Archiving this doc as Day 2 artifact now. New Day 3 doc will frame the POC comparison.

| #   | Criterion                            | What to look for                                                                                                           |
| --- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| C1  | **Workflow Coverage**                | Does it handle the full loop: requirements → implementation → testing → debugging → troubleshooting? Or only part of it?   |
| C2  | **Unattended / Sandboxed Execution** | Can agents run safely without babysitting? Containerized/sandboxed support? How does it handle failures?                   |
| C3  | **Context & Skills Persistence**     | Can you build and persist skills, CLAUDE.md context, automation that survives across sessions? Does it compound over time? |
| C4  | **Team Knowledge Sharing**           | How easy is it to share the setup, workflows, and accumulated context with the team? Git-friendly?                         |
| C5  | **Portability / Standardization**    | Not locked to HyperShift. Can other OpenShift teams adopt this without major customization?                                |

## Scoring Guide
- **1** — Doesn't support this at all
- **2** — Barely supports it, major gaps
- **3** — Adequate, works but with friction
- **4** — Good support, minor gaps
- **5** — Excellent, this is a strength

---

## Ralph Orchestrator
> https://mikeyobrien.github.io/ralph-orchestrator/
> Status: **Baseline** (already used heavily)

| Criterion | Score | Notes |
|-----------|-------|-------|
| C1 — Workflow Coverage | /5 | |
| C2 — Unattended Execution | /5 | |
| C3 — Context Persistence | /5 | |
| C4 — Team Knowledge | /5 | |
| C5 — Portability | /5 | |
| **Total** | **/25** | |

**Strengths:**

**Gaps:**

---

## multiclaude
> https://github.com/dlorenc/multiclaude
> Key question: Does multi-agent parallelism add value for HyperShift's test/debug cycle?

| Criterion | Score | Notes |
|-----------|-------|-------|
| C1 — Workflow Coverage | /5 | |
| C2 — Unattended Execution | /5 | |
| C3 — Context Persistence | /5 | |
| C4 — Team Knowledge | /5 | |
| C5 — Portability | /5 | |
| **Total** | **/25** | |

**Strengths:**

**Gaps:**

---

## GSD (Get Shit Done)
> https://github.com/glittercowboy/get-shit-done
> Key question: How does its spec-driven model compare to Ralph?

| Criterion | Score | Notes |
|-----------|-------|-------|
| C1 — Workflow Coverage | /5 | |
| C2 — Unattended Execution | /5 | |
| C3 — Context Persistence | /5 | |
| C4 — Team Knowledge | /5 | |
| C5 — Portability | /5 | |
| **Total** | **/25** | |

**Strengths:**

**Gaps:**

---

## claude-pilot
> https://github.com/maxritter/claude-pilot
> Quick eval — cut short if clearly outclassed

| Criterion | Score | Notes |
|-----------|-------|-------|
| C1 — Workflow Coverage | /5 | |
| C2 — Unattended Execution | /5 | |
| C3 — Context Persistence | /5 | |
| C4 — Team Knowledge | /5 | |
| C5 — Portability | /5 | |
| **Total** | **/25** | |

**Strengths:**

**Gaps:**

---

## Comparison Summary

| Tool | C1 | C2 | C3 | C4 | C5 | Total | Notes |
|------|----|----|----|----|----|----|-------|
| Ralph | | | | | | /25 | Baseline |
| multiclaude | | | | | | /25 | |
| GSD | | | | | | /25 | |
| claude-pilot | | | | | | /25 | |

## Decision

**Winner:**

**Rationale:**

**What to combine:** (e.g., "Use Ralph's workflow model + multiclaude's parallelism")

**Next step:** Deep dive on the winner — run a real HyperShift task through it.
