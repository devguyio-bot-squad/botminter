# POC Comparison: Ralph Orchestrator vs Claude Code Native

> Day 3 of [Shift Week Plan](shift-week-plan.md)
> Decision input: Which approach best delivers the 7 workflow aspects for HyperShift agentic development?
> Background: [Agentic Tooling Landscape](agentic-tooling-landscape.md), [Tool Evaluation & Decision](tool-evaluation-and-decision.md)
> Target UX: [Karim's First Week UX](ux-karims-first-week.md)

---

## The Question

**Ralph Orchestrator (possibly forked)** vs **Claude Code (skills, hooks, plugins, native orchestration)**

Both approaches use Claude Code as the underlying agent. The difference is the orchestration layer:

- **Ralph**: Claude Code is an *agent* inside Ralph's orchestrator. Workflow is composed via YAML — hats, events, loops (`planner --plan.done→ builder --build.done→ tester --test.failed→ planner`). All 7 aspects configured in one place.
- **Claude Code Native**: Claude Code is *both* agent and orchestrator. Workflow is composed via skills, hooks, CLAUDE.md, and the built-in Task system. Aspects are distributed across multiple mechanisms.

---

## The 7 Workflow Aspects

These are the transferable assets — they must survive regardless of which tool wins.

| #   | Aspect                      | What It Means                                                                     |
| --- | --------------------------- | --------------------------------------------------------------------------------- |
| 1   | **Spec-Driven Development** | Requirements → design → tasks before code. Structured specs, not ad-hoc prompting |
| 2   | **Context Engineering**     | CLAUDE.md, project context, domain knowledge loaded efficiently                   |
| 3   | **Prompt Engineering**      | Reusable, tested prompts for common tasks (review, debug, implement, test)        |
| 4   | **Agent Memories**          | Persistent state across sessions — what was done, what failed, what's next        |
| 5   | **Agent Tasks**             | Structured task tracking with dependencies, status, assignment                    |
| 6   | **Team Knowledge**          | Shared patterns, troubleshooting guides, domain expertise — git-portable          |
| 7   | **Agent Personas**          | Role-specific behavior profiles (planner, builder, tester, reviewer)              |


## Pre-POC Requirements Discovery
Before starting, we need to make some discovery about how an agentic-workflow should look like that satisfies the following requirements:

1. Can be configured to match Red Hat's planning process: RFE -> Feature -> Epics -> Story
2. 100% unattended agentic with optional human-in-the-loop when needed (configurable)
3. For the planning
    1. Covers breakdown of a Feature (OCPSTRAT JIRA issue) to a Epics and stories
4. For the implementation
    1. Can be configured per each phase of the SDLC. A phase can be prompt-driven (i.e. prompt-driven for planning, discovery, design, implementation plan, etc), or spec driven, or hybrid (start a conversation with a spec template that gets filled based on conversation output)
    2. Allows evidence-based validation of quality and implementation.
    3. Covers the PR reviews, CI runs, CI debugging, etc.
    4. Suitable for the nature of HyperShift, OpenShift, Kubernetes.
5. Ralph-Wiggum style, not swarm style.
6. Includes knowledge accumulation between team members. E.g.:
    1. Team member A runs the workflow and find that the agentic workflow isn't good in debugging nodepool tests.
    2. When checking slack, he finds that team member B just pushed an improvement of debugging nodepool tests to the agentic workflow setup/repo/.claude/whatever
    3. team member A pulls that somehow, and he's unblocked.
7. Increases the confidence of agentic-authored PRs without requiring the team members to review every line of code of such PRs
8. Since all agents can run with tools, it's required that those tools are configurable (mcp, tools, permissions) per phase, persona, or shared team config.

---

## POC Structure

1. **Same HyperShift task for both POCs** — apples to apples.

2. **Task:** https://issues.redhat.com/browse/OCPSTRAT-1751

3. **Time-box:** 3 hours per POC



### POC A: Ralph Orchestrator

TBD

#### Setup
- install / configure Ralph 
- define hats and event flow in YAML
- sandboxing approach 
#### Planning

#### Run
- <!-- execute the task through Ralph's workflow -->

#### Observations
- <!-- what worked, what didn't, friction points -->

### POC B: Claude Code Native

TBD

**Setup:**
- <!-- skills, hooks, CLAUDE.md config -->
- <!-- task tracking approach (built-in Tasks? Beads?) -->
- <!-- sandboxing approach -->

**Run:**
- <!-- execute the task through Claude Code's native mechanisms -->

**Observations:**
- <!-- what worked, what didn't, friction points -->

---

## Evaluation

Score each POC 1-5 on each workflow aspect after running.

| Aspect | POC A (Ralph) | POC B (Claude Code) | Notes |
|--------|:---:|:---:|-------|
| Spec-Driven Development | /5 | /5 | |
| Context Engineering | /5 | /5 | |
| Prompt Engineering | /5 | /5 | |
| Agent Memories | /5 | /5 | |
| Agent Tasks | /5 | /5 | |
| Team Knowledge | /5 | /5 | |
| Agent Personas | /5 | /5 | |
| **Total** | **/35** | **/35** | |

### Additional Factors

| Factor | POC A | POC B | Notes |
|--------|:---:|:---:|-------|
| Setup friction | /5 | /5 | How long to get running? |
| Team onboarding ease | /5 | /5 | Could a teammate adopt this in a day? |
| Composability | /5 | /5 | How easy to add/modify workflow steps? |
| Dependency risk | /5 | /5 | What happens if the tool dies/diverges? |

---

## Decision

**Winner:**

**Rationale:**

**What's transferable regardless of winner:**
<!-- The 7 aspects artifacts — specs, prompts, knowledge, personas — should be tool-agnostic -->

**Next step:**
<!-- Build the team workflow guide using the winning approach -->
