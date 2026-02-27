# RH: Dream UX Proposal

> Fresh-eyes proposal for the ideal agentic SDLC workflow.
> RH-O = Orchestrator. RH-A = AI Agent.
> Written for the Pre-POC Requirements Discovery of [Ralph vs Claude Code POC](ralph-vs-claude-code-poc.md)

---

## The Core Insight

**The best agentic SDLC feels like delegating to a senior engineer, not like programming a CI pipeline.**

When you hand a Feature to a senior engineer, they don't ask you for a YAML pipeline definition. They read the requirements, ask smart questions, produce a plan, break it down, implement incrementally, test as they go, and produce PRs that tell a story. When they get stuck, they don't silently fail — they come to you with context and a proposed path forward. When they learn something, the whole team benefits.

RH should feel exactly like that. You hand it a Feature. It acts like the best engineer on the team — one who never forgets, never gets tired, and writes everything down.

The difference from a real engineer: **RH produces an evidence chain at every step.** You don't trust it because of reputation. You trust it because of proof.

---

## The Experience

### Act 1: Intake

```
$ rh intake OCPSTRAT-1751
```

That's it. One command. RH-O reads the JIRA Feature, its linked RFE, any linked design docs, and the relevant areas of the codebase. Then it comes back with a briefing:

```
Feature: External DNS Support for HyperShift on Azure
Source:  OCPSTRAT-1751
RFE:     RFE-4521

I've read the feature request, the linked design doc, and scanned the
relevant packages in hypershift/. Here's what I understand:

  - Azure hosted clusters need ExternalDNS for ingress record management
  - This touches: control-plane-operator, azure platform code, NodePool
  - Similar work was done for AWS in v4.14 (PR #3421, #3455)

Ready to plan? I have some questions first.

  1. The RFE mentions "private clusters" — should the first cut include
     private DNS zones, or public only?
  2. There's an existing ExternalDNS operator in the cluster. Should we
     reuse it or deploy a dedicated instance in the control plane namespace?
  3. The AWS implementation used a credentials secret per hosted cluster.
     Same pattern for Azure, or should we use workload identity?

Answer these, or say "plan" and I'll make my best assumptions
(documented, reviewable).
```

Notice: RH-A didn't just regurgitate the JIRA issue. It **cross-referenced the codebase**, found the prior art (AWS implementation), and asked targeted questions that a senior engineer would ask. The questions aren't generic — they're grounded in what it found in the code.

The human can answer the questions, or skip them:

```
$ rh respond 1:public-only 2:dedicated-instance 3:workload-identity
```

Or:

```
$ rh plan --auto
```

The `--auto` flag means: make your best judgment, document your assumptions, I'll review the output. This is the "100% unattended with optional human-in-the-loop" knob. **It's not a global setting. It's per-command, per-phase, per-feature.** The human modulates their involvement fluidly.

---

### Act 2: Planning

RH-O drives RH-A (in planner persona) to produce the breakdown. The output is a **planning document** — not JIRA tickets yet. A document that humans and agents can both read:

```
$ rh status

Feature: OCPSTRAT-1751 — External DNS for HyperShift on Azure
Phase:   planning (awaiting approval)

Proposed Breakdown:
  Epic 1: Core ExternalDNS integration
    ├── Story 1.1: Add ExternalDNS deployment to CPO for Azure [M]
    ├── Story 1.2: Wire Azure workload identity credentials    [S]
    └── Story 1.3: Create DNS records for default ingress      [M]

  Epic 2: Testing & validation
    ├── Story 2.1: Unit tests for ExternalDNS reconciliation   [S]
    ├── Story 2.2: E2e test: create cluster, verify DNS        [L]
    └── Story 2.3: E2e test: destroy cluster, verify cleanup   [M]

  Epic 3: Documentation & graduation
    ├── Story 3.1: Update API docs for Azure DNS fields        [S]
    └── Story 3.2: Runbook: troubleshooting DNS for Azure HCs  [S]

Assumptions made (review these):
  → Public DNS zones only (no private cluster support in v1)
  → Dedicated ExternalDNS instance per hosted cluster namespace
  → Azure workload identity (not shared credentials secret)
  → Based on AWS pattern from PR #3421 with Azure adaptations

$ rh approve planning
```

On `approve`, RH-O can optionally push these to JIRA as linked Epics/Stories under the Feature. Or not — that's config. The point is: **the planning artifact is a document first, JIRA tickets second.** The document is the source of truth. JIRA is the sync target.

```yaml
# .rh/config.yaml (relevant section)
planning:
  sync_to_jira: true
  jira_project: HOSTEDCP
  hierarchy:
    feature: OCPSTRAT    # source of truth
    epic: HOSTEDCP        # created by RH
    story: HOSTEDCP       # created by RH
  approval_required: true # human must approve before implementation starts
```

---

### Act 3: Implementation

This is where RH earns its keep. For each Story, RH-O runs it through a **phase chain**. The default chain:

```
discovery → design → implement → verify → pr → review
```

Each phase has three properties:

| Property | What it controls |
|----------|-----------------|
| **Mode** | `prompt` (conversational), `spec` (fill a template), `hybrid` (conversation → spec) |
| **Persona** | Which RH-A persona runs this phase (researcher, architect, builder, tester, reviewer) |
| **Gate** | What evidence must exist before advancing to the next phase |

Example config:

```yaml
# .rh/phases.yaml
phases:
  discovery:
    mode: prompt
    persona: researcher
    prompt: prompts/discovery.md
    gate:
      - output_file_exists: ".rh/stories/{story_id}/discovery.md"
    tools:
      allow: [jira, grep, read, web_search, github]

  design:
    mode: hybrid
    persona: architect
    prompt: prompts/design.md
    template: templates/design-doc.md   # conversation fills this
    gate:
      - output_file_exists: ".rh/stories/{story_id}/design.md"
      - field_not_empty: ["approach", "api_changes", "risks"]
    tools:
      allow: [grep, read, web_search, github]

  implement:
    mode: spec
    persona: builder
    spec: ".rh/stories/{story_id}/design.md"  # reads the design as input
    gate:
      - code_compiles: true
      - tests_pass: "unit"
      - diff_size_under: 500  # lines. if over, split into sub-PRs
    tools:
      allow: [grep, read, write, edit, bash, github]
      bash:
        allow: [make, go, git]  # whitelist — no rm -rf

  verify:
    mode: spec
    persona: tester
    spec: ".rh/stories/{story_id}/design.md"
    gate:
      - tests_pass: "unit"
      - tests_pass: "integration"  # optional, depends on story size
      - coverage_delta: ">= 0"     # don't decrease coverage
    tools:
      allow: [grep, read, write, edit, bash]
      bash:
        allow: [make, go, git]

  pr:
    mode: spec
    persona: builder
    gate:
      - pr_created: true
      - pr_description_complete: true
      - ci_green: true
    tools:
      allow: [github, bash]

  review:
    mode: prompt
    persona: reviewer
    prompt: prompts/review.md
    gate:
      - review_comments_addressed: true
      - confidence_score: ">= 0.7"
    tools:
      allow: [grep, read, github]
```

**The key insight here: phases are not code. They're config.** A team lead configures the phase chain once. Every team member gets the same workflow. When someone discovers that "the design phase needs to also check for API compatibility," they update the phase config, push it, and everyone benefits.

The human watches progress with:

```
$ rh status --live

Story 1.1: Add ExternalDNS deployment to CPO for Azure
  ✓ discovery    (2m 14s)  — found AWS reference impl, 3 relevant packages
  ✓ design       (4m 32s)  — design.md complete, 2 API changes proposed
  ◉ implement    (11m 03s) — writing reconciler, 3/5 functions done
  ○ verify
  ○ pr
  ○ review

Story 1.2: Wire Azure workload identity credentials
  ◉ discovery    (1m 45s)  — reading Azure identity docs
  ○ design
  ○ implement
  ○ verify
  ○ pr
  ○ review
```

Stories run in dependency order. RH-O manages the sequencing — Story 1.2 might depend on 1.1's API changes, so RH-O holds it until 1.1's design phase is done (so 1.2's discovery can reference the new API). This is the **Ralph-Wiggum style**: one orchestrator, sequential-by-default, parallel only when explicitly safe.

---

### Act 4: The Evidence Chain

This is what makes agentic PRs trustworthy without line-by-line human review.

Every phase produces **evidence artifacts** stored alongside the code:

```
.rh/stories/HOSTEDCP-5678/
  ├── discovery.md          # what was learned, prior art, references
  ├── design.md             # approach, API changes, risks, alternatives considered
  ├── implementation.log    # agent's reasoning trace (compressed)
  ├── test-results.json     # unit + integration test results
  ├── confidence.json       # breakdown of confidence score
  └── review.md             # self-review findings, addressed issues
```

The **confidence score** is not a magic number. It's a transparent breakdown:

```json
{
  "confidence": 0.82,
  "factors": {
    "tests_written": { "score": 0.9, "detail": "12 new tests, 100% of new functions covered" },
    "tests_passing": { "score": 1.0, "detail": "all 12 pass" },
    "design_adherence": { "score": 0.8, "detail": "implemented 4/5 design decisions, 1 deferred with rationale" },
    "prior_art_consistency": { "score": 0.7, "detail": "follows AWS pattern, 2 Azure-specific deviations documented" },
    "diff_coherence": { "score": 0.85, "detail": "427 lines, single concern, no unrelated changes" },
    "self_review_clean": { "score": 0.75, "detail": "found and fixed 3 issues during self-review, 1 known limitation documented" }
  },
  "recommendation": "Ready for human review. Focus areas: Azure-specific deviations in reconciler (lines 142-198)"
}
```

This changes the review dynamic. Instead of "I need to read every line because an AI wrote this," it becomes: **"The confidence score is 0.82. Tests are solid. Let me focus on the two Azure-specific deviations the agent flagged."**

The PR description itself includes this evidence chain summary. The reviewer isn't starting from zero — they're starting from a curated, honest assessment.

---

### Act 5: When Things Go Wrong

CI fails. It will. Here's the UX:

```
$ rh status

Story 1.1: Add ExternalDNS deployment to CPO for Azure
  ✓ discovery
  ✓ design
  ✓ implement
  ✓ verify (local)
  ✓ pr (created: #4521)
  ✗ review — CI failed: e2e-azure-ovn

  RH-A is investigating. Current hypothesis:
    The ExternalDNS container image isn't available in the CI registry.
    Checking image mirroring config...

  Options:
    [wait]       Let RH-A continue investigating (ETA: ~5 min)
    [intervene]  Drop into a conversation with RH-A about this failure
    [pause]      Park this story, continue with others
```

```
$ rh intervene story/HOSTEDCP-5678

You're now in a conversation with RH-A (builder persona) about Story 1.1.
Context: CI failure in e2e-azure-ovn for PR #4521.

RH-A: I've identified the issue. The ExternalDNS image reference uses
       registry.redhat.io but CI uses a mirrored registry. I found 3
       similar fixes in git history:
       - PR #3890 (AWS): added imageContentSourcePolicy
       - PR #4102 (GCP): same pattern
       - PR #4201 (PowerVS): same pattern

       I can apply the same pattern. Should I push the fix and re-trigger CI?

You: yes, go ahead

RH-A: Done. Pushed commit abc123, CI re-triggered.
       I'm also adding this to the team knowledge base — "ExternalDNS
       requires ICSP entry for CI environments."

You: good catch. exit

Back to orchestrator view.
```

The `intervene` command is the human-in-the-loop at the finest grain. You're not watching a dashboard. You're **having a conversation with the agent that's doing the work,** with full context already loaded. When you're done, you leave, and RH-O continues orchestrating.

---

### Act 6: Knowledge Accumulation

This is requirement #6, and it's the sleeper feature that compounds over time.

Knowledge lives in git, in the repo, in a shared location:

```
.rh/
  ├── config.yaml            # team workflow config
  ├── phases.yaml             # phase chain definition
  ├── personas/               # persona definitions
  │   ├── researcher.md
  │   ├── architect.md
  │   ├── builder.md
  │   ├── tester.md
  │   └── reviewer.md
  ├── prompts/                # reusable prompts per phase
  │   ├── discovery.md
  │   ├── design.md
  │   └── review.md
  ├── templates/              # spec templates
  │   └── design-doc.md
  ├── knowledge/              # <-- the accumulation layer
  │   ├── patterns/
  │   │   ├── azure-workload-identity.md
  │   │   ├── image-mirroring-ci.md      # <-- added from Act 5
  │   │   └── nodepool-test-debugging.md
  │   ├── pitfalls/
  │   │   ├── etcd-backup-timing.md
  │   │   └── cpo-restart-race.md
  │   └── references/
  │       ├── hypershift-api-conventions.md
  │       └── test-framework-patterns.md
  └── stories/                # per-story artifacts (evidence chain)
```

The scenario from the requirements:

> Team member A runs the workflow and finds that RH-A isn't good at debugging NodePool tests.
> Team member B pushes an improvement to `.rh/knowledge/patterns/nodepool-test-debugging.md`.
> Team member A pulls, and they're unblocked.

This works because:
1. Knowledge files are **in the repo, in git.** Normal git workflows apply — push, pull, branch, PR.
2. RH-A loads relevant knowledge files at the start of each phase. The `researcher` persona searches `.rh/knowledge/` for files relevant to the current story's domain.
3. When RH-A learns something during a run (like the image mirroring fix in Act 5), it **proposes a knowledge file addition.** The human approves or edits.

```
$ rh knowledge list

Recent additions (last 7 days):
  + image-mirroring-ci.md          by ahmed    2 days ago
  + azure-workload-identity.md     by ahmed    3 days ago
  ~ nodepool-test-debugging.md     by toni     1 day ago  (updated)

$ rh knowledge propose

RH-A has 2 pending knowledge proposals from recent runs:
  1. "Azure DNS zone delegation requires explicit role assignment"
     Source: Story HOSTEDCP-5678, discovery phase
  2. "CPO reconciler ordering: DNS before ingress controller"
     Source: Story HOSTEDCP-5678, implementation phase

  [approve 1] [approve 2] [approve all] [edit 1] [dismiss all]
```

The knowledge isn't a wiki nobody reads. It's **operational intelligence that directly improves the next agent run.** Every knowledge file is context that gets loaded when relevant. The team's collective debugging experience, architectural decisions, and domain expertise accumulate as a living, version-controlled, agent-consumable knowledge base.

---

## Architecture

### Component Model

```
┌─────────────────────────────────────────────────────────┐
│                     Human (CLI / TUI)                    │
│         rh intake | status | approve | intervene         │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                      RH-O (Orchestrator)                 │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │ Feature   │  │ Phase    │  │ Evidence │              │
│  │ State     │  │ Engine   │  │ Tracker  │              │
│  │ Machine   │  │          │  │          │              │
│  └──────────┘  └──────────┘  └──────────┘              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │ Knowledge │  │ Tool     │  │ JIRA     │              │
│  │ Index     │  │ Sandbox  │  │ Sync     │              │
│  │           │  │ Manager  │  │          │              │
│  └──────────┘  └──────────┘  └──────────┘              │
└──────────────────────────┬──────────────────────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
     ┌──────────┐  ┌──────────┐  ┌──────────┐
     │  RH-A    │  │  RH-A    │  │  RH-A    │
     │ (planner)│  │ (builder)│  │ (tester) │
     └──────────┘  └──────────┘  └──────────┘
         │              │              │
    Only 1 active at a time (Ralph-Wiggum style)
    Each gets: persona + phase prompt + knowledge + tools
```

### RH-O: The Orchestrator

RH-O is **not an AI.** It's a deterministic state machine. It:
- Tracks the Feature → Epic → Story hierarchy and their states
- Runs the phase engine: for each story, advance through phases, check gates
- Manages tool sandboxing: each persona gets only the tools its phase allows
- Indexes knowledge files for relevance matching
- Syncs state to JIRA (if configured)
- Handles human interaction (CLI commands)

RH-O doesn't think. It schedules, gates, and tracks. This is critical: **the orchestrator must be predictable and auditable.** No AI in the control loop. AI is in the execution loop.

### RH-A: The Agent

RH-A is an AI agent (Claude, or whatever model). It runs one phase of one story at a time. When RH-O invokes RH-A, it provides:

1. **Persona definition** — from `.rh/personas/builder.md`
2. **Phase prompt** — from `.rh/prompts/implement.md`
3. **Story context** — the story description + all prior phase outputs (discovery.md, design.md)
4. **Relevant knowledge** — RH-O searches `.rh/knowledge/` and injects relevant files
5. **Tool permissions** — only the tools allowed for this phase
6. **Gate requirements** — what RH-A must produce before this phase is considered complete

RH-A runs, produces artifacts, and exits. RH-O checks the gate. If the gate passes, advance. If not, RH-O can retry (with feedback), escalate to human, or park the story.

### Why This Split Matters

| Concern | Owner | Why |
|---------|-------|-----|
| "What runs next?" | RH-O | Deterministic, auditable, no hallucination risk |
| "How do I implement this?" | RH-A | Creative, contextual, benefits from AI |
| "Is this good enough?" | RH-O (gates) | Objective checks, not AI self-assessment |
| "What tools can be used?" | RH-O | Security boundary, not left to AI judgment |
| "What knowledge is relevant?" | RH-O (index) + RH-A (usage) | RH-O surfaces it, RH-A consumes it |

---

## The Confidence System (Requirement #7)

The goal: **increase confidence in agentic PRs without requiring line-by-line review.**

Confidence is not a single number. It's a **multi-dimensional evidence profile** that humans can interpret at a glance.

### Confidence Dimensions

| Dimension | How It's Measured | Gate Threshold |
|-----------|-------------------|----------------|
| **Test coverage** | New code coverage % | Configurable (default: 80%) |
| **Test passage** | All tests pass | Required |
| **Design adherence** | Agent self-check: did implementation match design.md? | Configurable |
| **Prior art consistency** | How closely does this follow established patterns in the codebase? | Advisory |
| **Diff coherence** | Single concern? Reasonable size? No unrelated changes? | Configurable |
| **Self-review** | Agent reviews its own PR with reviewer persona, fixes issues | Required |
| **Knowledge alignment** | Were relevant pitfalls from `.rh/knowledge/pitfalls/` checked? | Required |

### The Review Triage

Based on confidence, RH-O recommends a review level:

| Confidence | Review Level | What It Means |
|------------|-------------|---------------|
| 0.9+ | **Scan** | Tests are solid, follows established patterns, self-review clean. Human skims diff, checks for obvious issues. ~5 min. |
| 0.7-0.9 | **Focused** | Generally good but has flagged areas. Human reviews specific sections the agent highlighted. ~15 min. |
| 0.5-0.7 | **Thorough** | New patterns, complex logic, or known pitfall areas. Full review needed. ~30 min. |
| <0.5 | **Pair** | Agent is uncertain. Human should review with agent in conversation (`rh intervene`). |

This isn't about trusting AI blindly. It's about **directing human attention where it matters most.** The agent does the boring verification (tests pass, diff is coherent, patterns match). The human does the judgment calls (is this the right approach? does this API make sense?).

---

## Tool Sandboxing (Requirement #8)

Tools are configured at three levels, with inheritance:

```yaml
# Level 1: Team defaults (in .rh/config.yaml, committed to repo)
tools:
  defaults:
    mcp_servers:
      - jira
      - github
    bash:
      allow: [make, go, git, kubectl]
      deny: [rm, curl, wget]  # deny takes precedence

# Level 2: Per-phase overrides (in .rh/phases.yaml)
phases:
  discovery:
    tools:
      allow: [jira, grep, read, web_search]
      # no bash, no write — discovery is read-only
  implement:
    tools:
      allow: [grep, read, write, edit, bash, github]
      bash:
        allow: [make, go, git]

# Level 3: Per-persona restrictions (in .rh/personas/reviewer.md)
# The reviewer persona gets read-only access even in phases that allow writes
tools:
  allow: [grep, read, github]
  deny: [write, edit, bash]
```

Resolution order: persona restrictions > phase config > team defaults. Most restrictive wins on deny, most specific wins on allow.

**Why this matters for HyperShift/OpenShift:** These codebases have real infrastructure implications. A builder agent shouldn't be able to `kubectl delete` anything. A reviewer shouldn't be able to modify code. Tool sandboxing is a safety boundary, and it must be configured by the team, not left to the agent's judgment.

---

## Configuration Philosophy

Everything is in `.rh/` in the repo. Everything is git.

```
.rh/
  ├── config.yaml       # team-level: JIRA sync, approval gates, defaults
  ├── phases.yaml        # phase chain + gates + tool permissions
  ├── personas/          # who the agent pretends to be in each phase
  ├── prompts/           # what the agent is told to do in each phase
  ├── templates/         # spec templates for hybrid/spec-driven phases
  ├── knowledge/         # team's accumulated wisdom
  └── stories/           # per-story evidence artifacts (gitignored or separate branch)
```

This means:
- **Team onboarding = git clone.** You get the workflow, the knowledge, the personas, everything.
- **Workflow changes are PRs.** "I improved the review prompt" is a code-reviewable change.
- **Multiple teams can fork.** Team A's HyperShift workflow is different from Team B's MCE workflow. Fork `.rh/`, adapt, done.
- **No external state.** No databases, no SaaS dashboards, no vendor lock-in. It's files in a repo.

---

## Addressing Each Requirement

| # | Requirement | How RH Addresses It |
|---|-------------|---------------------|
| 1 | Red Hat planning process (RFE → Feature → Epic → Story) | `config.yaml` maps the hierarchy. `rh intake` reads the Feature from JIRA, `rh approve planning` creates Epics/Stories in JIRA. The hierarchy is configurable, not hardcoded. |
| 2 | 100% unattended with optional human-in-the-loop | Every command has `--auto`. Every phase gate can be set to `approval_required: false`. The human modulates involvement fluidly — from full auto to per-phase approval. |
| 3 | Feature breakdown to Epics/Stories | The `planning` phase uses the planner persona to produce a structured breakdown. Output is a reviewable document before becoming JIRA tickets. |
| 4 | Configurable SDLC phases, evidence-based, PR/CI, K8s-suitable | Phase chain is fully configurable in `phases.yaml`. Each phase has mode (prompt/spec/hybrid), persona, gate conditions, and tool permissions. Evidence artifacts are stored per-story. |
| 5 | Ralph-Wiggum style, not swarm | RH-O runs one RH-A at a time per story. No concurrent agents racing. Sequential by default. Parallel only across independent stories when explicitly configured. |
| 6 | Knowledge accumulation between team members | `.rh/knowledge/` directory in git. Agent proposes additions from learnings. Team members push improvements. `rh knowledge` CLI for managing the knowledge base. |
| 7 | Confidence in agentic PRs | Multi-dimensional confidence score with transparent breakdown. Review triage levels (scan/focused/thorough/pair) based on evidence. Self-review phase catches issues before human sees the PR. |
| 8 | Configurable tools per phase/persona/team | Three-level tool sandboxing: team defaults → phase overrides → persona restrictions. Most restrictive wins. Bash commands are whitelisted, not blacklisted. |

---

## The Wedge: What to Build First

Don't build all of this. Build the smallest thing that proves the biggest thesis.

**Wedge: `rh intake` + `rh plan` + single-story implementation with evidence chain.**

Scope:
1. `rh intake OCPSTRAT-XXXX` — reads JIRA, scans codebase, asks questions
2. `rh plan` — produces Epic/Story breakdown as a document (no JIRA sync yet)
3. `rh implement STORY-1` — runs through discovery → design → implement → verify → pr with evidence artifacts
4. Confidence score on the PR

Skip for v1:
- JIRA sync (use documents only)
- Knowledge accumulation (hardcode knowledge files, don't auto-propose yet)
- Parallel stories (one at a time)
- TUI/live status (use polling `rh status`)

This wedge validates: "Can an orchestrator + agent workflow produce a PR with higher confidence than ad-hoc Claude Code usage, for a real HyperShift story?"

If yes, everything else is just making it better.

---

## The Anti-Vision: What If We Don't Build This?

Without RH, the team's agentic adoption looks like this:

- Each engineer figures out their own Claude Code setup
- Prompts are private, ephemeral, never shared
- Knowledge dies in Slack threads
- Every agentic PR gets full line-by-line review because there's no evidence chain
- There's no way to tell a "good" agentic PR from a "bad" one until you've read every line
- The team lead has no visibility into what the agent did or why
- When someone discovers a trick (better prompt, knowledge file, tool config), the discovery benefits only them
- The planning process stays manual — agents are used only for implementation
- There's no confidence accumulation — the team never gets faster at reviewing agentic work

That's not agentic development. That's individual AI-assisted coding with extra steps. The compound returns never materialize because there's no system.

**RH is the system.**
