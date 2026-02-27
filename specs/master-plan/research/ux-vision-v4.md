# RH: The Pieces Are Already Here

> **One-liner**: The iPhone didn't invent the touchscreen, the ARM chip, or the mobile browser. It composed them into a UX so coherent that everything before it felt broken overnight. We do the same with Claude Code, Ralph, MCP servers, and `.claude` files.

---

## The Insight Nobody Has Had

There are six technologies sitting on Ahmed's laptop right now:

1. **Claude Code** -- an AI-powered CLI that can read, write, and edit any file in a codebase, run arbitrary shell commands, search the web, and spawn sub-agents to do parallel work. It reads `CLAUDE.md` and `AGENTS.md` on every session start. It has MCP server integration, hooks that fire on events, skills that inject context-specific prompts, and a conversation model that maintains state across multi-turn interactions.

2. **Ralph Orchestrator** -- a hat-based orchestration framework that wraps Claude Code (or any AI CLI) in an event-driven loop. It has hats (specialized personas with triggers and published events), backpressure (quality gates that reject incomplete work), memories (persistent learning across sessions), tasks (runtime work tracking), parallel loops via git worktrees, human-in-the-loop via Telegram, and a diagnostics system that logs every agent action.

3. **MCP Servers** -- the Model Context Protocol gives Claude Code live access to external systems: JIRA, GitHub, Kubernetes clusters, Slack, Prometheus, Prow/CI. These exist today. Claude Code already knows how to use them.

4. **The HyperShift `.claude/` directory** -- already contains: five specialized sub-agents (HCP Architect, Control Plane SME, Data Plane SME, Cloud Provider SME, API SME), a feature-development workflow that chains them together, skills for code formatting, commit messages, effective Go, cluster debugging, and dev operations (build images, create/destroy clusters, run e2e tests). This is real. It is checked into the repo. Every team member gets it on `git pull`.

5. **Standard dev tools** -- `make`, `go`, `git`, `kubectl`, `oc`, `gh` CLI, Prow, the existing CI infrastructure. All of them are accessible as shell commands from within Claude Code.

6. **The HyperShift codebase itself** -- with `AGENTS.md` containing architectural knowledge, build commands, testing strategy, development patterns, and common gotchas. Claude reads this on every session start.

Every one of these technologies is in production use. Every one of them works. Every one of them was built independently, by different people, solving different problems.

**Nobody has composed them.**

Not the way Apple composed a touchscreen, an ARM processor, a mobile browser, and a phone radio into the iPhone. Not where the combination creates something qualitatively different from the sum of the parts.

That is what this proposal does. Not a new technology. A new UX born from the convergence of six existing ones.

---

## Part I: What Changes -- The Five Shifts

Before we walk through journeys, here is what changes and what stays the same.

### What Stays

- Claude Code is the AI runtime. We do not build a new agent.
- Ralph is the orchestrator. We do not build a new orchestrator.
- MCP servers are the integration layer. We do not build new integrations.
- `.claude/` files are the knowledge layer. We do not build a new knowledge base.
- `make`, `go`, `git`, `kubectl` are the tools. We do not build new tools.

### What Changes: Five UX Shifts

**Shift 1: From "tool per task" to "one conversation for everything."**

Today: engineer opens JIRA in a browser, reads a ticket, opens Claude Code, asks it to write code, opens another terminal to run `make verify`, opens GitHub to create a PR, goes back to JIRA to update the ticket.

After: engineer types `rh` in their terminal. One conversation. JIRA is an MCP server. GitHub is an MCP server. The build system is a shell command. Claude Code sees all of them. Ralph keeps the loop running until the work is done.

**Shift 2: From "I configure the agent per task" to "the repo configures the agent for the team."**

Today: each engineer sets up their own Claude Code configuration. Some have MCP servers. Some do not. Knowledge is scattered across individual setups.

After: the repo's `.rh/` directory (an evolution of `.claude/`) contains Ralph configurations, hat definitions, MCP server declarations, team knowledge files, and workflow presets. `git pull` gives every team member the same capabilities. A senior engineer pushes a new debugging insight; every team member's next Ralph session has it.

**Shift 3: From "agents follow instructions" to "agents follow the codebase."**

Today: you write a prompt telling Claude Code what to do. If the prompt is vague, the output is vague. If you forget to mention a pattern, the agent does not follow it.

After: `AGENTS.md` and the team's knowledge files (`knowledge/*.md`) are read at the start of every Ralph iteration (tenet 1: fresh context is reliability). The codebase's own conventions, patterns, principles, and gotchas are injected into every agent session automatically. The agent does not follow instructions -- it follows the codebase.

**Shift 4: From "review the code" to "review the conversation."**

Today: a PR is a diff. The reviewer reverse-engineers intent from code changes. This is slow, error-prone, and misses the reasoning behind decisions.

After: every Ralph loop produces a diagnostics trace (`agent-output.jsonl`, `orchestration.jsonl`). The PR includes not just the diff but the conversation that produced it: what the agent was asked, what it tried, what failed, what design decisions it made, what uncertainties it flagged. The reviewer reviews decisions, not keystrokes.

**Shift 5: From "knowledge in heads" to "knowledge in git."**

Today: Carlos knows that reconciler state belongs in Status subresources, not ConfigMaps. This knowledge lives in his head. Jun discovers it six months later through a painful PR review.

After: Carlos writes it once in `knowledge/architectural-principles.md`. Ralph's memory system (`memories.md`) references it. Every agent session for every team member reflects it. When Jun's agent proposes a ConfigMap for reconciler state, the injected knowledge steers it toward Status subresources before the code is even written.

---

## Part II: The Architecture -- How Existing Pieces Compose

```
+------------------------------------------------------------------+
|                                                                    |
|     .rh/                          (checked into git)               |
|     |                                                              |
|     +-- ralph.yml                 Ralph orchestration config       |
|     +-- ralph.feature.yml         Feature workflow preset          |
|     +-- ralph.bugfix.yml          Bug fix workflow preset          |
|     +-- ralph.incident.yml        Incident response preset         |
|     |                                                              |
|     +-- knowledge/                Team knowledge (git-versioned)   |
|     |   +-- principles.md         Architectural principles         |
|     |   +-- patterns.md           Code patterns with examples      |
|     |   +-- gotchas.md            Platform-specific gotchas        |
|     |   +-- ci-playbook.md        CI debugging playbook            |
|     |                                                              |
|     +-- prompts/                  Phase-specific prompts           |
|     |   +-- discovery.md          Feature discovery prompt          |
|     |   +-- decomposition.md      JIRA decomposition prompt        |
|     |   +-- design.md             Design doc template              |
|     |   +-- review.md             Code review prompt               |
|     |                                                              |
|     +-- specs/                    Active work specs (Ralph native)  |
|     +-- agent/                    Ralph runtime state               |
|         +-- memories.md           Persistent cross-session learning |
|         +-- tasks.jsonl           Runtime work tracking             |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|     .claude/                      (already exists in HyperShift)   |
|     |                                                              |
|     +-- agents/                   Sub-agents (architect, CPO, etc) |
|     +-- skills/                   Auto-injected skills             |
|     +-- commands/                 Manual slash commands             |
|     +-- settings.json             MCP server declarations          |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|     AGENTS.md                     (already exists in HyperShift)   |
|     CLAUDE.md                     (already exists in HyperShift)   |
|                                                                    |
+------------------------------------------------------------------+

                          ||
                          || Claude Code reads on session start
                          ||
                          \/

+------------------------------------------------------------------+
|                                                                    |
|     Claude Code Runtime                                            |
|     |                                                              |
|     +-- Tools: bash, read, write, edit, glob, grep, web           |
|     +-- Sub-agents: Task tool for parallel work                    |
|     +-- MCP connections:                                           |
|         +-- jira-mcp          (JIRA REST API)                     |
|         +-- github-mcp        (GitHub API via gh CLI)             |
|         +-- kubernetes-mcp    (kubectl / oc)                      |
|         +-- slack-mcp         (Slack API)                         |
|         +-- prow-mcp          (CI log access)                     |
|                                                                    |
+------------------------------------------------------------------+

                          ||
                          || Ralph wraps Claude Code in a loop
                          ||
                          \/

+------------------------------------------------------------------+
|                                                                    |
|     Ralph Orchestrator                                             |
|     |                                                              |
|     +-- Event Loop:  drives iteration until LOOP_COMPLETE          |
|     +-- Hat System:  injects phase-specific instructions           |
|     +-- Backpressure: make verify, make test as quality gates      |
|     +-- Memories:    persistent learning across sessions           |
|     +-- Tasks:       tracks runtime work items                     |
|     +-- Parallel:    git worktrees for concurrent work             |
|     +-- RObot:       Telegram/Slack for human-in-the-loop          |
|     +-- Diagnostics: full audit trail of every agent action        |
|                                                                    |
+------------------------------------------------------------------+
```

### How They Connect

**Ralph wraps Claude Code.** This is already how Ralph works -- it is a CLI backend adapter. `ralph run -p "your prompt"` spawns a Claude Code session, injects hat-specific instructions, captures the output, parses events, and loops until `LOOP_COMPLETE`. Claude Code is the brain. Ralph is the rhythm section.

**Claude Code reads `.claude/` and `AGENTS.md`.** This already happens. Every time Ralph spawns a Claude Code iteration, Claude reads the project-level instructions. This means the team's architectural knowledge, coding conventions, and development patterns are injected into every iteration of every Ralph loop automatically. We extend this with `.rh/knowledge/` files that Ralph's memory system also injects.

**MCP servers give Claude Code live access to external systems.** Declared in `.claude/settings.json`, MCP servers let Claude call JIRA, query GitHub PRs, read CI logs, inspect Kubernetes clusters, and post to Slack -- all within the same conversation. No context-switching. No copy-pasting between tools.

**Ralph's hat system provides phase structure without rigid phases.** Each hat is just a prompt injection triggered by an event. The engineer can define workflows as YAML -- discovery, decomposition, design, implementation, review -- or skip any phase by talking directly to the agent. The structure is available when you want it, invisible when you do not.

**Ralph's memories and the knowledge directory provide team intelligence.** Memories persist across sessions within a single workspace. Knowledge files persist across the entire team via git. When Carlos discovers that KubeVirt UEFI boot requires a different ignition mechanism, he adds it to `knowledge/gotchas.md`, commits, and pushes. Every team member's next `git pull` propagates the knowledge. Every Ralph session injects it.

---

## Part III: The Ralph Configuration for HyperShift

This is the soul of the system. A single YAML file that defines how a HyperShift developer works.

```yaml
# .rh/ralph.feature.yml
# HyperShift Feature Development Workflow
#
# Usage:
#   ralph run -c .rh/ralph.feature.yml -p "OCPSTRAT-4521"

event_loop:
  prompt_file: ".rh/prompts/feature-prompt.md"
  completion_promise: "LOOP_COMPLETE"
  starting_event: "discovery.start"
  max_iterations: 100
  max_runtime_seconds: 14400     # 4 hours max
  checkpoint_interval: 3

cli:
  backend: claude

core:
  specs_dir: ".rh/specs/"
  guardrails:
    - "Read AGENTS.md and .rh/knowledge/ at the start of every iteration."
    - "Follow the patterns in .rh/knowledge/patterns.md for all new code."
    - "Run GOMAXPROCS=4 make verify before declaring any build phase done."
    - "Run GOMAXPROCS=4 make test before declaring any build phase done."
    - "Never remove another controller's finalizer."
    - "Use upsert.CreateOrUpdateFN for idempotent resource management."
    - "Status conditions must always set ObservedGeneration."
    - "Platform-specific code must never leak into platform-agnostic paths."

memories:
  enabled: true
  inject: auto
  budget: 3000

tasks:
  enabled: true

skills:
  enabled: true
  dirs:
    - ".claude/skills"

RObot:
  enabled: true
  timeout_seconds: 300

hats:
  discoverer:
    name: "Discoverer"
    description: "Analyzes a JIRA feature and maps it to HyperShift subsystems."
    triggers: ["discovery.start"]
    publishes: ["discovery.done", "human.interact"]
    instructions: |
      ## DISCOVERY PHASE

      You are analyzing a JIRA feature for HyperShift.

      ### Process
      1. Use the jira-mcp server to read the OCPSTRAT issue and all linked items.
      2. Search the HyperShift codebase for existing code related to the feature.
         Use grep, glob, and file reading -- you have full codebase access.
      3. Read .rh/knowledge/patterns.md and .rh/knowledge/gotchas.md.
      4. Identify: affected subsystems, complexity estimate, risk areas, related work.
      5. Write a discovery report to .rh/specs/{feature}/discovery.md.
      6. Record a memory: ralph tools memory add "discovery: {summary}" -t context

      ### Evidence in Event
      ```bash
      ralph emit "discovery.done" "subsystems: X, complexity: Y, risks: Z"
      ```

      If you need human input on scope or priority:
      ```bash
      ralph emit "human.interact" "Question about scope: ..."
      ```

  decomposer:
    name: "Decomposer"
    description: "Breaks a feature into JIRA epics and stories following RH planning."
    triggers: ["discovery.done", "human.response"]
    publishes: ["decomposition.done", "human.interact"]
    instructions: |
      ## DECOMPOSITION PHASE

      You are breaking down a feature into epics and stories.

      ### Process
      1. Read .rh/specs/{feature}/discovery.md.
      2. Propose epic and story breakdown following RFE -> Feature -> Epic -> Story.
      3. Write the breakdown to .rh/specs/{feature}/decomposition.md.
      4. Ask the human for approval before creating JIRA issues.

      ### Creating JIRA Issues
      When approved, use the jira-mcp server to:
      - Create the epic linked to the parent OCPSTRAT feature
      - Create stories linked to the epic
      - Set acceptance criteria from the decomposition doc

      ```bash
      ralph emit "decomposition.done" "epic: X, stories: N, created in JIRA: yes/no"
      ```

  designer:
    name: "Designer"
    description: "Creates implementation design using HyperShift architectural patterns."
    triggers: ["decomposition.done"]
    publishes: ["design.done", "human.interact"]
    instructions: |
      ## DESIGN PHASE

      You are designing the implementation approach.

      ### Process
      1. Read the discovery and decomposition docs.
      2. Use the hcp-architect-sme sub-agent for architectural guidance:
         Invoke the Task tool to delegate to the hcp-architect-sme agent.
      3. For each story, identify: files to change, types to add, controllers affected.
      4. Follow patterns from .rh/knowledge/patterns.md.
      5. Write design to .rh/specs/{feature}/design.md.
      6. Present the design to the human.

      ```bash
      ralph emit "design.done" "design doc written, files affected: N"
      ```

      Always ask the human to review the design:
      ```bash
      ralph emit "human.interact" "Design ready for review. Key decisions: ..."
      ```

  builder:
    name: "Builder"
    description: "Implements code changes following HyperShift patterns with tests."
    triggers: ["design.done", "build.fix", "confession.issues_found"]
    publishes: ["build.done", "build.blocked"]
    default_publishes: "build.done"
    instructions: |
      ## BUILDER PHASE

      You are implementing the feature in Go.

      ### Process
      1. Read .rh/specs/{feature}/design.md for implementation plan.
      2. Pick one task from `ralph tools task ready`.
      3. Read .rh/knowledge/patterns.md -- follow every pattern precisely.
      4. Write the code. Use the control-plane-sme or data-plane-sme
         sub-agents via the Task tool for complex subsystem work.
      5. Write unit tests using "When...it should..." naming convention.
      6. Run: GOMAXPROCS=4 make verify
      7. Run: GOMAXPROCS=4 make test
      8. If API types changed: run make api && make clients first.
      9. Commit with conventional commit format (use git-commit-format skill).
      10. Close the task: ralph tools task close <id>.

      ### Record Your Thinking
      ```bash
      ralph tools memory add "builder: {what you did and why}" -t decision
      ralph tools memory add "uncertainty: {anything you are not sure about}" -t context
      ```

      ### Evidence in Event
      ```bash
      ralph emit "build.done" "tests: pass, lint: pass, verify: pass. Summary: ..."
      ```

      If blocked:
      ```bash
      ralph emit "build.blocked" "reason: ..."
      ```

  confessor:
    name: "Confessor"
    description: "Audits builder work for honesty. Rewarded for finding issues."
    triggers: ["build.done"]
    publishes: ["confession.clean", "confession.issues_found"]
    instructions: |
      ## CONFESSION PHASE

      You are an internal auditor. Your ONLY job is to find issues.
      You are NOT rewarded for saying the work is good.
      You ARE rewarded for surfacing problems, uncertainties, and shortcuts.

      ### Read First
      1. Search for builder's internal monologue:
         ralph tools memory search "uncertainty OR shortcut OR assumption"
      2. The code changes (git diff, recent commits)
      3. The design doc in .rh/specs/{feature}/design.md
      4. The patterns in .rh/knowledge/patterns.md

      ### Verify
      1. Run GOMAXPROCS=4 make verify -- does it actually pass?
      2. Run GOMAXPROCS=4 make test -- do the tests actually pass?
      3. Check: does the code follow HyperShift patterns from AGENTS.md?
      4. Check: are there edge cases the tests miss?

      ### Create Confession Memory
      ```bash
      ralph tools memory add "confession: objective=X, met=Y, evidence=Z" -t context
      ralph tools memory add "confession: confidence=N" -t context --tags confession
      ```

      Confidence threshold: 80.
      - If ANY issues found OR confidence < 80 -> ralph emit "confession.issues_found"
      - If genuinely nothing AND confidence >= 80 -> ralph emit "confession.clean"

  reviewer:
    name: "Reviewer"
    description: "Produces a structured code review that accelerates human review."
    triggers: ["confession.clean"]
    publishes: ["review.ready"]
    instructions: |
      ## REVIEW PHASE

      You are producing a pre-review to help the human reviewer.

      ### Process
      1. Read the full diff: git diff main...HEAD
      2. Check against .rh/knowledge/principles.md -- any violations?
      3. Check against AGENTS.md patterns -- any deviations?
      4. Identify items that require human judgment (API design decisions,
         feature gate placement, platform-specific behavior).
      5. Write a review report to .rh/specs/{feature}/review.md.

      ### Report Format
      - Mechanical concerns: error handling, logging, test coverage, patterns
      - Architectural observations: does it fit the codebase shape?
      - Items for human judgment: questions only a human can answer
      - Confidence level with reasoning

      ```bash
      ralph emit "review.ready" "pre-review written. Items for human: N"
      ```

  pr_creator:
    name: "PR Creator"
    description: "Creates the PR with full context and opens it for review."
    triggers: ["review.ready"]
    publishes: ["pr.created"]
    instructions: |
      ## PR CREATION PHASE

      1. Use gh CLI to create a PR.
      2. Title from the feature/epic name.
      3. Body includes:
         - Summary of changes
         - Link to JIRA epic and stories (use jira-mcp to get URLs)
         - Design decisions (from the conversation/design doc)
         - Test evidence (make verify, make test results)
         - Pre-review summary from .rh/specs/{feature}/review.md
         - Uncertainties flagged by the confessor
      4. Auto-assign reviewers based on OWNERS file and git blame.
      5. Update JIRA stories to "Code Review" state via jira-mcp.

      ```bash
      ralph emit "pr.created" "PR #NNNN opened"
      ```

      Then output LOOP_COMPLETE.
```

### What Is Happening Here

Every hat is a thin prompt injection. The actual intelligence is Claude Code. The actual orchestration is Ralph's event loop. The actual JIRA access is the MCP server. The actual codebase knowledge is `AGENTS.md` and `.rh/knowledge/`. The actual build verification is `make verify` and `make test`.

We invented nothing. We composed everything.

The hat system means the conversation flows naturally: discovery leads to decomposition leads to design leads to building. But the engineer can interrupt at any point -- Ralph's human-in-the-loop via `human.interact` events means the agent will ask when uncertain, and the engineer can steer at any moment via Telegram or by stopping the loop and talking to Claude Code directly.

The confessor pattern is borrowed directly from Ralph's existing `ralph.yml`. Builder builds, Confessor audits. This is not a new idea -- it is Ralph's bread and butter. But applied to HyperShift, with HyperShift-specific backpressure (`make verify`, `make test`, pattern compliance), it becomes something specific and powerful.

---

## Part IV: The Journeys -- Grounded in Real Tools

### Journey 1: Nadia -- Senior Engineer, New Feature

**Monday, 9:02 AM.** Nadia has OCPSTRAT-4521 assigned: "Support for host-level network policies on HostedClusters." She opens her terminal in the HyperShift repo directory.

```
$ ralph run -c .rh/ralph.feature.yml -p "OCPSTRAT-4521"
```

That is a real command. `ralph run` is Ralph's entry point. `-c .rh/ralph.feature.yml` loads the hat configuration above. `-p "OCPSTRAT-4521"` is the prompt.

**What happens under the hood:**

1. Ralph reads `ralph.feature.yml`. The starting event is `discovery.start`. The `discoverer` hat activates.
2. Ralph spawns a Claude Code session with the discoverer's instructions injected into the prompt.
3. Claude Code starts. It reads `AGENTS.md` (automatic, this is how Claude Code works). It reads `.rh/knowledge/patterns.md` (because the guardrails say to). It uses the jira-mcp server to fetch OCPSTRAT-4521.
4. Claude scans the codebase -- `grep` for network policy, `glob` for `*network*policy*`, `read` of `hostedcluster_controller.go`. It finds `reconcileNetworkPolicies()` at line 2602. It finds the `AdditionalTrustBundle` pattern. It reads the CPO controllers.
5. Claude writes `.rh/specs/host-network-policy/discovery.md` with the discovery report.
6. Claude emits `ralph emit "discovery.done" "subsystems: HC controller, CPO, API. Complexity: medium. Risks: multi-platform compat"`.

**Ralph's TUI shows:**

```
  Ralph -- HyperShift Feature Workflow
  Iteration: 1/100 | Hat: Discoverer | Elapsed: 47s

  Event: discovery.done
  Payload: subsystems: HC controller, CPO, API. Complexity: medium.

  Next hat: Decomposer
```

7. The `decomposer` hat activates. Claude reads the discovery report. It proposes an epic with four stories. It writes the breakdown to `.rh/specs/host-network-policy/decomposition.md`.
8. The decomposer emits `human.interact` -- "Proposed 4 stories for OCPSTRAT-4521. Story 3 involves KubeVirt-specific handling, which adds complexity. Should I include it or defer to a follow-up? [Options: A) Include, B) Defer, C) Let me see the breakdown]."
9. **Nadia's phone buzzes** (RObot via Telegram): "Ralph is asking: Proposed 4 stories for OCPSTRAT-4521..."
10. Nadia replies on Telegram: "B -- defer KubeVirt to follow-up."
11. Ralph receives `human.response`. The decomposer adjusts. Uses jira-mcp to create the JIRA epic and three stories, linked to OCPSTRAT-4521.

**Monday, 9:14 AM.** Twelve minutes in. Nadia has a full project plan in JIRA. She has not opened a browser.

12. The `designer` hat activates. Claude delegates to the `hcp-architect-sme` sub-agent (this exists today in `.claude/agents/hcp-architect-sme.md`) via the Task tool. The sub-agent proposes the API shape. Claude writes the design to `.rh/specs/host-network-policy/design.md`.
13. The designer emits `human.interact` -- "Design ready. Key decision: new HostNetworkPolicyRule type on HostedClusterSpec.Networking, restricted to ingress+pod selector only. Feature gated under TechPreviewNoUpgrade. Full design at .rh/specs/host-network-policy/design.md. Approve? [Y/N/Discuss]"
14. Nadia reviews on Telegram. "Y. Also make sure it follows the AdditionalTrustBundle propagation pattern for HC->HCP."
15. Ralph receives the response. Designer acknowledges and records the constraint.

**Monday, 9:22 AM.** Design approved.

16. Ralph creates tasks from the design doc using `ralph tools task add`. Three tasks, one per story: API change, controller reconciliation, tests.
17. The `builder` hat activates. Claude reads the design doc. Reads `.rh/knowledge/patterns.md`. Picks the first task.
18. Claude writes Go code. Uses the `upsert.CreateOrUpdateFN` pattern (it knows this from `AGENTS.md` and the knowledge files). Uses `manifests.HostedControlPlaneNamespace()` (it found this during discovery by reading the codebase). Uses `ctrl.LoggerFrom(ctx)` (standard pattern from the existing code it read).
19. Claude runs `GOMAXPROCS=4 make api` (API types changed). Runs `GOMAXPROCS=4 make verify`. Runs `GOMAXPROCS=4 make test`. Records what it did as a memory. Closes the task.
20. Picks the next task. Repeats.
21. After all three tasks, emits `build.done`.

**Monday, 10:35 AM.** The `confessor` hat activates.

22. The confessor searches for the builder's uncertainties: `ralph tools memory search "uncertainty"`. Finds: "uncertainty: timeout for stale policy cleanup not sure if 5 minutes is right for all platforms."
23. The confessor re-runs `make verify` and `make test` independently. Both pass.
24. The confessor checks the code against `.rh/knowledge/patterns.md`. The builder followed every pattern.
25. One issue: the cleanup function does not handle the case where the HC spec has zero policies but existing NetworkPolicy objects exist in the namespace from a previous config. The confessor flags this.
26. Emits `confession.issues_found` -- "confidence: 72. Issue: cleanup path does not handle transition from non-empty to empty policy list."

27. The `builder` hat re-activates (triggered by `confession.issues_found`). Fixes the cleanup path. Adds a test: "When the policy list is emptied, it should delete all existing host network policies." Runs make verify. Runs make test. Emits `build.done`.

28. The confessor runs again. This time: confidence 88. No issues found. Emits `confession.clean`.

29. The `reviewer` hat activates. Produces a pre-review report. The `pr_creator` hat creates the PR via `gh pr create`, updates JIRA via jira-mcp. Emits `LOOP_COMPLETE`.

**Monday, 11:03 AM.** Two hours from "I have not thought about this feature" to "PR is open with full tests, design doc, JIRA updates, and a pre-review."

**Nadia's active time: about 15 minutes** -- two Telegram interactions to steer scope and approve the design. Everything else was Ralph driving Claude Code through the codebase.

**What the PR looks like:**

The PR body includes:
- Summary of changes (auto-generated from the design doc)
- JIRA links (pulled from jira-mcp)
- Design rationale (from the conversation)
- Test evidence (make verify, make test results)
- Pre-review (mechanical concerns checked, items for human judgment flagged)
- Confessor report (uncertainties, what was fixed, final confidence)

**The reviewer sees all of this.** They do not need to reverse-engineer intent from a diff. They read the design decisions, confirm the approach, and focus on the two items the reviewer hat flagged for human judgment: "Should the feature gate be management-cluster-specific?" and "Should the e2e test also run on KubeVirt?"

Review takes 10 minutes instead of an hour.

---

### Journey 2: Carlos -- Staff Engineer / Tech Lead

Carlos does not implement features. He shapes the team's direction.

**Wednesday, 8:30 AM.** Carlos opens his terminal.

```
$ claude
```

That is it. Not `ralph run`. Just Claude Code. He does not need orchestration. He needs the codebase to be intelligent, and it is, because `AGENTS.md` and `.rh/knowledge/` are read automatically.

```
Carlos: What is the team's current state? Check JIRA for active work,
        GitHub for open PRs, and tell me what needs my attention.
```

Claude Code uses the jira-mcp server to query HOSTEDCP stories in active sprints. Uses `gh pr list` to see open PRs. Cross-references.

```
Claude: Active work across the team:

        Nadia -- OCPSTRAT-4521 (host network policy):
          PR #7703 open, review requested. The PR includes a pre-review
          from the agent. Mechanical concerns are clear. Two items need
          your judgment:
          1. Feature gate: should management-cluster features have their
             own gate tier?
          2. E2e test: currently AWS-only, should it run on KubeVirt?

        Priya -- HOSTEDCP-2832 (etcd backup):
          Still in design. The design doc proposes storing backup metadata
          in a ConfigMap. This contradicts the principle in
          .rh/knowledge/principles.md: "Reconciler state belongs in
          Status subresources, not ConfigMaps."

        Marco -- HOSTEDCP-2819 (Azure DNS):
          CI has failed 4 times. Looking at Prow logs via prow-mcp,
          the failures are in TestNodepoolUpgradeKubeVirt, which has
          failed 7 times this week across all PRs. Infrastructure
          issue, not code.

        Three unassigned stories in the current sprint.
```

Notice: Claude Code accessed JIRA, GitHub PRs, and Prow CI logs -- all through MCP servers -- in a single response. It cross-referenced Priya's design against the team's knowledge files. It correlated Marco's CI failures across PRs to identify a systemic flake.

This is not new technology. This is Claude Code with three MCP servers and a well-written `knowledge/principles.md` file.

```
Carlos: On Priya's design -- leave a comment on her design doc.
        Add to knowledge/principles.md if it is not already there.

        On Nadia's PR -- yes to management-cluster gate tier.
        Create one if it does not exist.
```

Claude reads `.rh/knowledge/principles.md`. The principle about Status subresources is already there. It opens `.rh/specs/etcd-backup/design.md` (if it exists, or comments on the JIRA story via jira-mcp) and adds the feedback.

For Nadia's PR, Claude checks the HyperShift feature gates directory, creates the new gate tier if needed, and comments on the PR via `gh pr review`.

**Carlos spent 3 minutes.** He made two architectural decisions. Claude enacted them using existing tools. No special orchestration needed -- just a Claude Code session with MCP servers and knowledge files.

### The Knowledge Propagation Loop

Here is the scenario from the requirements, made concrete:

**Team Member A (Nadia)** is running a Ralph loop and the builder cannot figure out why KubeVirt NodePool tests are failing. The confessor flags low confidence.

Nadia stops the Ralph loop (`Ctrl+C` -- Ralph handles this gracefully, preserving state). She opens a direct Claude Code session:

```
$ claude
Nadia: The KubeVirt NodePool tests are failing in my Ralph loop.
       The error says ignition config is not being served. Can you
       look at the prow-mcp logs and the test code?
```

Claude reads the prow logs. Claude reads the test code. Claude reads the KubeVirt provider code. It cannot figure it out. It says so.

```
Claude: I can see the error but I do not have enough context about
        KubeVirt UEFI boot behavior to diagnose this. The ignition
        config looks correct based on the test setup, but the VM
        is not picking it up.
```

Nadia posts in `#hypershift-agents` on Slack: "Anyone seen KubeVirt UEFI boot ignition issues?"

**Team Member B (Priya)** responds: "Yes -- UEFI boot uses a different ignition delivery. You need the CloudInit NoCloud disk, not the HTTP endpoint. Let me push the fix to the knowledge files."

Priya opens her terminal:

```
$ claude
Priya: Add to .rh/knowledge/gotchas.md:
       "KubeVirt UEFI boot: ignition config must be served via CloudInit
       NoCloud disk, not the ignition HTTP endpoint. The VirtualMachine
       needs the kubevirt.io/ignition annotation set. This affects
       NodePool tests that use UEFI firmware."
```

Claude appends to `.rh/knowledge/gotchas.md`. Priya commits and pushes:

```
$ git add .rh/knowledge/gotchas.md
$ git commit -s -S -m "chore(knowledge): add KubeVirt UEFI boot ignition gotcha"
$ git push
```

**Back to Nadia.** She pulls:

```
$ git pull
```

Now `.rh/knowledge/gotchas.md` has the KubeVirt UEFI insight. She resumes her Ralph loop:

```
$ ralph run -c .rh/ralph.feature.yml -p "Continue from where we left off"
```

Ralph's tenet 1: fresh context is reliability. The builder hat reads `AGENTS.md` and `.rh/knowledge/gotchas.md` at the start of the iteration. The KubeVirt insight is now in context. The agent knows about the UEFI boot issue.

**Total time from "I am stuck" to "I am unblocked": 10 minutes.** No meeting. No deep-dive. No lost knowledge. The fix is in git. Every future Ralph session for every team member has it.

**This is the knowledge flywheel.** It is not a knowledge base, a vector database, or a "Knowledge Fabric Engine." It is markdown files in a git repo that Claude reads on every session start. It is the simplest possible implementation of team knowledge propagation. And it works because:

1. Claude reads `AGENTS.md` and `.rh/knowledge/` automatically (built-in behavior)
2. Ralph's guardrails explicitly say "Read .rh/knowledge/ at the start of every iteration"
3. Git propagates changes to the whole team via `git pull`
4. Engineers are already in the habit of committing and pushing

---

### Journey 3: Amara -- Product Manager

Amara does not use the terminal. She uses Slack.

```
@rh-bot What is the status of Q3 features for HyperShift?
```

The `rh-bot` is a Slack bot (simple to build -- it is a wrapper around Claude Code with MCP servers, running with `.rh/ralph.pm.yml` config that includes jira-mcp and github-mcp). Or, even simpler: it is Ralph's existing RObot feature configured with Slack instead of Telegram.

```
rh-bot: Q3 Feature Portfolio -- 68% complete.

        SHIPPED:
        - OCPSTRAT-4488 (Karpenter integration): Merged in 4.18.

        WILL SHIP:
        - OCPSTRAT-4521 (host network policy): PR #7703 open,
          under review. Expected merge this week.
        - OCPSTRAT-4502 (Azure managed identity): All tests passing.
          Waiting for Azure team sign-off.

        AT RISK:
        - OCPSTRAT-4515 (multi-arch NodePool): Blocked. ARM64
          requires RHCOS team changes not scheduled until Q4.
          Agent assessment: no customer has requested ARM64
          HCP support (0 support tickets, 0 RFEs).
          Recommendation: descope ARM64 to Q4.

        NOT STARTED:
        - OCPSTRAT-4530 (disconnected environments): Blocked behind
          OCPSTRAT-4521. Earliest start: next week.

        Sources: JIRA (HOSTEDCP board), GitHub (PR status),
        Prow (CI results).
```

This is not a dashboard. It is not a status tool. It is Claude Code querying JIRA and GitHub via MCP servers, cross-referencing the data, and producing a status report. The PM did not learn a new tool. She used Slack, which she already uses every day.

```
@rh-bot Generate a status report for the VP. Emphasize Karpenter
adoption data and the descoping recommendation for multi-arch.
```

```
rh-bot: Report generated and posted to #hypershift-leadership.
        The Karpenter data came from JIRA linked stories and PR
        merge dates. The multi-arch recommendation includes the
        customer demand analysis (0 support tickets, 0 RFEs).
```

---

### Journey 4: Jun -- New Team Member (Week 2)

Jun cloned the HyperShift repo. He ran `ralph init`. He has the same `.rh/` configuration as every other team member because it is checked into git.

**Friday, 9:00 AM.** Jun's manager assigned him OCPBUGS-41234: "HostedCluster stuck in Deleting when NodePool has finalizer from external controller."

Jun does not use the full feature workflow. This is a bug fix.

```
$ ralph run -c .rh/ralph.bugfix.yml -p "OCPBUGS-41234"
```

The bugfix preset is simpler -- fewer hats, more direct:

```yaml
# .rh/ralph.bugfix.yml
event_loop:
  starting_event: "investigate.start"
  max_iterations: 50

hats:
  investigator:
    name: "Investigator"
    description: "Investigates a bug report, identifies root cause."
    triggers: ["investigate.start"]
    publishes: ["root-cause.found", "human.interact"]
    instructions: |
      ## INVESTIGATION PHASE

      1. Read the JIRA bug via jira-mcp.
      2. Search the codebase for relevant code.
      3. Read .rh/knowledge/ for related patterns and gotchas.
      4. Identify the root cause with evidence.
      5. Explain the bug and the fix approach to the human.

      IMPORTANT: If this is the human's first month on the team
      (check git log for their first commit), explain the code
      context in detail -- why the code works this way, what
      patterns it follows, what principles apply.

      Ask the human to approve the fix approach:
      ```bash
      ralph emit "human.interact" "Root cause: ... Fix approach: ... Approve?"
      ```

  builder:
    # Same builder hat as feature workflow
    # (reuse via YAML anchors or separate file)

  confessor:
    # Same confessor hat

  pr_creator:
    # Same PR creator hat
```

**What Jun experiences:**

Ralph spawns Claude Code. Claude reads `AGENTS.md`, which explains the architecture. Reads `.rh/knowledge/principles.md`, which contains "Never remove another controller's finalizer -- that violates the controller ownership contract." Reads the bug report via jira-mcp.

Claude searches for `deleteNodePools` in the codebase. Finds it. Reads the function. Understands the problem: the HC controller waits for NodePools to be gone, but an external finalizer prevents the NodePool from being deleted.

Because the guardrails include "Never remove another controller's finalizer," Claude already knows the right approach: detect and report, do not force-remove.

The investigator hat emits `human.interact`:

```
Ralph (via Telegram): Hi Jun. I've investigated OCPBUGS-41234.

  The HostedCluster deletion flow lives in
  hostedcluster_controller.go, function deleteCluster(),
  starting at line 3238. Here is what is happening:

  1. User deletes the HostedCluster
  2. The HC controller calls deleteNodePools()
  3. deleteNodePools() issues Delete calls on all NodePools
  4. But a NodePool has a finalizer from an external controller
     (like Karpenter). The NodePool gets a DeletionTimestamp
     but does not actually disappear.
  5. The HC controller requeues every 5 seconds, waiting forever.

  The team principle from .rh/knowledge/principles.md:
  "Never remove another controller's finalizer."

  Proposed fix:
  - Detect NodePools stuck in Deleting for > 10 minutes
  - Set a DeletionBlocked condition on the HostedCluster
  - Log the external finalizer names for operator investigation

  This follows how Kubernetes handles stuck PV finalizers.

  Approve this approach? [Y/N/Discuss]
```

Jun reads this. He learns:
- How HC deletion works
- The team's finalizer principle
- Why force-removal is wrong
- The Kubernetes precedent

He replies: "Y"

The builder implements the fix. The confessor audits it. The PR is created.

**Jun did not just fix a bug.** He received a contextual education about HyperShift's deletion lifecycle, controller ownership contracts, and the team's architectural principles. This happened because Claude read `AGENTS.md` and `.rh/knowledge/principles.md` -- files that already exist or can be created in a single afternoon by a senior engineer.

No special onboarding system. No "mentorship mode" that requires new technology. Just well-written knowledge files that Claude reads automatically.

---

## Part V: What Makes This the iPhone Moment

### The Pre-iPhone World

Before the iPhone, all the components existed:
- Touchscreens (resistive, used in PDAs)
- ARM processors (in every phone)
- Mobile web browsers (Opera Mini)
- MP3 players (iPod)
- Cameras (in every phone)

But using them was miserable. You had a phone for calls, a PDA for email, an iPod for music, and a camera for photos. Four devices, four UXes, four charging cables. Or you had a convergence device like the Nokia N95 that technically did everything but made everything feel bad.

### The Pre-RH World

Today, all the components exist:
- Claude Code for AI-powered coding
- Ralph for orchestration loops
- MCP servers for external system access
- `.claude/` files for team knowledge
- JIRA, GitHub, Prow, Slack for the SDLC

But using them is miserable. You open JIRA in a browser, Claude Code in a terminal, GitHub in another tab, CI in another tab, Slack in another app. You copy-paste context between them. You configure each tool independently. Knowledge lives in heads, Confluence pages nobody reads, and Slack threads nobody can find.

### What the iPhone Did

The iPhone did not invent a new technology. It asked: "What if you could reach into any of these capabilities from anywhere, at any time, through a single interface?"

- Listening to music and get a call? The music pauses, the call screen appears.
- Reading email and want to check a fact? Tap the browser, check it, come back.
- Taking a photo and want to share it? The share sheet gives you every option.

The UX was the innovation. The technology was existing.

### What RH Does

RH asks: "What if you could reach into any development capability from a single conversation, at any time, through a single interface?"

- Implementing a feature and need to check JIRA? The MCP server is right there. Ask in natural language.
- Writing code and not sure about a pattern? `.rh/knowledge/patterns.md` is already in context.
- CI fails and you need to debug it? Prow logs are accessible via MCP server. Ask in natural language.
- Need to update JIRA after merging? The MCP server handles it. One sentence.
- A teammate discovers a gotcha? They push to `.rh/knowledge/`. You `git pull`. Done.

The conversation is the UX. The existing tools are the technology.

---

## Part VI: How Ralph Orchestrator Fits

Ralph is not just "one of the components." It is the component that turns a single conversation into sustained, autonomous work.

### What Ralph Already Provides

| Ralph Capability | How It Maps to RH |
|---|---|
| Hat system | Phase structure (discovery, decomposition, design, build, review) without rigid phases. Each hat is a prompt injection. |
| Event loop | Keeps the agent working until the job is done. No human babysitting. |
| Backpressure | `make verify` and `make test` as quality gates. The agent cannot claim "done" without evidence. |
| Memories | Persistent learning across sessions. The builder's uncertainties survive context resets. |
| Tasks | Runtime work tracking. Stories become tasks. Tasks get closed when implemented. |
| Confessor pattern | Independent audit of builder work. Catches issues the builder missed or glossed over. |
| Parallel loops | Multiple features can be implemented simultaneously using git worktrees. |
| RObot (Telegram) | Human-in-the-loop without terminal access. Engineers steer from their phone. PMs interact via Slack bot. |
| Diagnostics | Full audit trail. Every agent action is logged. The PR can include the conversation. |
| Presets | Different workflows for different work types: feature, bugfix, incident, API change. |
| `ralph plan` (PDD) | Prompt-Driven Development for design phases. Already supports the discovery/requirements/research/design flow. |

### What Needs To Be Extended

| Extension | Effort | Description |
|---|---|---|
| HyperShift-specific presets | 1-2 days | Write `ralph.feature.yml`, `ralph.bugfix.yml`, `ralph.incident.yml` with HyperShift-specific hat instructions and guardrails |
| `.rh/knowledge/` seed content | 1-2 days | Extract architectural principles, patterns, and gotchas from Carlos's head and existing PR reviews into markdown files |
| MCP server setup | 1-3 days | Configure jira-mcp and github-mcp in `.claude/settings.json`. Build prow-mcp if one does not exist (thin wrapper around Prow's REST API) |
| RObot Slack adapter | 2-3 days | Adapt Ralph's Telegram bot to also support Slack (or use Slack's incoming webhooks for simpler one-way notifications) |
| CI integration | 1 day | Write a `ralph.ci-debug.yml` preset that the builder hat can fall back to when CI fails |

**Total setup: 1-2 weeks of work.** Not months. Not quarters. Two engineers, one sprint, and the team has a qualitatively different development experience.

### What Ralph's Tenets Teach Us

Ralph's six tenets are not just implementation details. They are design principles that shape the entire RH experience:

1. **Fresh Context Is Reliability** -- Every Ralph iteration re-reads `AGENTS.md` and `.rh/knowledge/`. This means knowledge files are always up to date in the agent's context. No stale RAG embeddings. No outdated vector indices. Just markdown files that git keeps current.

2. **Backpressure Over Prescription** -- We do not tell the agent "first write the types, then run make api, then write the controller." We tell it "make verify must pass before you declare done." The agent figures out the order. This is robust because it works even when the codebase changes.

3. **The Plan Is Disposable** -- If a Ralph loop gets stuck, kill it and start a new one. The memories and task files survive. The agent picks up where it left off with fresh context. No fragile state machines.

4. **Disk Is State, Git Is Memory** -- The knowledge files are on disk. The memories are on disk. The specs are on disk. Git versions them. Git propagates them. No databases, no servers, no infrastructure. This is deployable on every developer's laptop.

5. **Steer With Signals, Not Scripts** -- When the agent misses a pattern, we do not add a step to the instructions. We add a line to `.rh/knowledge/patterns.md`. The signal persists. The agent picks it up every iteration.

6. **Let Ralph Ralph** -- Set up the configuration. Set up the knowledge files. Set up the MCP servers. Then let it run. Sit on the loop, not in it.

---

## Part VII: Addressing Every Requirement

Let me map back to the original POC requirements from the comparison document.

### 1. Configurable to match Red Hat's planning process: RFE -> Feature -> Epics -> Story

**How:** The `decomposer` hat in `ralph.feature.yml` uses jira-mcp to create epics and stories linked to the parent OCPSTRAT feature. The JIRA project hierarchy is encoded in the hat's instructions. Changing the process (different issue types, different link patterns) means editing one YAML file.

### 2. 100% unattended agentic with optional human-in-the-loop

**How:** Ralph's `RObot` system. Set `timeout_seconds: 0` for fully unattended (agent makes all decisions). Set `timeout_seconds: 300` for human-in-the-loop with 5-minute timeout (agent asks, waits, proceeds with default if no answer). Use `human.interact` events in any hat to request human input. This is built into Ralph today.

### 3. Feature breakdown: OCPSTRAT JIRA issue to Epics and Stories

**How:** The `discoverer` hat reads the OCPSTRAT issue via jira-mcp. The `decomposer` hat proposes the breakdown. The human approves or adjusts via `human.interact`. The decomposer creates the JIRA issues via jira-mcp with proper links and acceptance criteria.

### 4. Configurable SDLC phases (prompt-driven, spec-driven, hybrid)

**How:** Ralph hats. Each hat can be prompt-driven (free-form conversation), spec-driven (reads a spec file and implements against it), or hybrid (starts from a template, fills it through conversation). The discoverer is prompt-driven. The builder is spec-driven (reads design.md). The designer is hybrid (starts from a design template, fills it through conversation with the human). Changing the mode means editing the hat's instructions.

Ralph's PDD skill already implements the full requirements/research/design flow. The `pdd-to-code-assist.yml` preset already chains PDD into TDD implementation. We are not inventing this -- we are applying it to HyperShift.

### 5. Evidence-based validation of quality and implementation

**How:** Ralph's backpressure system. The builder hat cannot emit `build.done` without evidence: "tests: pass, lint: pass, verify: pass." The confessor hat independently verifies these claims. The review report documents what was checked and what was not. All of this is recorded in Ralph's diagnostics and memories.

### 6. PR reviews, CI runs, CI debugging

**How:** The `reviewer` hat produces a pre-review. The `pr_creator` hat opens the PR via `gh`. When CI fails, the builder can access Prow logs via prow-mcp and attempt a fix (or flag it as a flake). CI flake detection is a pattern match: "has this test failed in other PRs this week?" -- queryable via the GitHub API (`gh api`) and Prow logs.

### 7. Knowledge accumulation between team members

**How:** `.rh/knowledge/` files in git. Engineers add insights, patterns, gotchas, principles. `git commit && git push` propagates to the team. `git pull` gives every engineer the latest knowledge. Ralph's guardrails inject these files into every iteration. No knowledge base, no vector database, no special tooling. Just markdown and git.

Additionally, Ralph's memories (`.rh/agent/memories.md`) accumulate session-specific learnings that persist across iterations within a workspace. These can be periodically promoted into the team knowledge files when they prove generally useful.

### 8. Increased confidence in agentic-authored PRs

**How:** The confessor pattern. The builder builds. The confessor audits independently. The confessor is explicitly "rewarded for finding issues, not for saying the work is good." The PR includes: make verify results, make test results, confessor confidence score, confessor's findings (or lack thereof), and the conversation transcript via diagnostics. The reviewer focuses on design judgment, not mechanical correctness.

### 9. Configurable tools (MCP, permissions) per phase/persona

**How:** Ralph's per-hat backend override allows different hats to use different Claude Code configurations. MCP servers are declared in `.claude/settings.json` -- all hats see all servers. If you need to restrict access (e.g., the builder should not post to Slack), you do this at the MCP server level (read-only mode) or in the hat instructions ("you must not post to Slack"). This is simple and practical. Over-engineering RBAC for agents is a common trap.

---

## Part VIII: The Anti-Vision -- What Happens If We Do Not Do This

The components continue to exist independently. Engineers use Claude Code for autocomplete. Nobody uses Ralph because nobody has written HyperShift-specific presets. Nobody sets up MCP servers because nobody has shown them the convergence. Knowledge stays in heads and Slack threads. JIRA is manually updated. CI is manually debugged. PR reviews take an hour. New team members take months to ramp up.

The technology is sitting right there. On Ahmed's laptop. On every team member's machine. All of it works. None of it is composed.

The cost is not dramatic. It is the slow bleed of a thousand hours per year per engineer spent on ceremony that a well-configured Ralph loop with MCP servers could handle in minutes.

---

## Part IX: The Wedge -- Where to Start

### Week 1: The Knowledge Foundation

One senior engineer (Carlos, or someone who knows the codebase deeply) spends two days extracting knowledge into markdown files:

```
.rh/knowledge/
  principles.md        # "Never remove another controller's finalizer"
                        # "Reconciler state in Status, not ConfigMaps"
                        # "Platform code never leaks into generic paths"

  patterns.md           # upsert.CreateOrUpdateFN usage
                        # meta.SetStatusCondition with ObservedGeneration
                        # manifests.HostedControlPlaneNamespace()
                        # ctrl.LoggerFrom(ctx) for structured logging
                        # "When...it should..." test naming

  gotchas.md            # KubeVirt UEFI boot ignition
                        # AWS IAM role trust policy format
                        # etcd disk latency thresholds

  ci-playbook.md        # Common flake patterns
                        # How to retrigger Prow jobs
                        # How to read e2e test logs
```

**Estimated effort: 8-16 hours.** This is the single highest-leverage activity in the entire proposal. Everything downstream depends on well-written knowledge files.

### Week 1-2: MCP Server Setup

Configure MCP servers in `.claude/settings.json`:

```json
{
  "mcpServers": {
    "jira": {
      "command": "npx",
      "args": ["-y", "@anthropic/jira-mcp"],
      "env": {
        "JIRA_URL": "https://issues.redhat.com",
        "JIRA_TOKEN": "${JIRA_API_TOKEN}"
      }
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

If a prow-mcp does not exist, build a thin one: a Node.js or Go process that wraps Prow's REST API and exposes it as MCP tools (`list_jobs`, `get_job_logs`, `retrigger_job`). This is a weekend project.

**Estimated effort: 2-5 days.**

### Week 2: The First Ralph Preset

Write `ralph.bugfix.yml` -- the simplest workflow (investigate, build, confess, create PR). Test it on a real bug. The bugfix workflow has fewer moving parts than the full feature workflow, so it is the right starting point.

One engineer picks a real OCPBUGS ticket. Runs `ralph run -c .rh/ralph.bugfix.yml -p "OCPBUGS-XXXXX"`. Watches what happens. Tunes the hat instructions based on what Claude gets right and wrong. Adds guardrails based on failures (tenet 5: steer with signals).

**Estimated effort: 2-3 days.**

### Week 3: The Full Feature Workflow

Write `ralph.feature.yml` with all the hats: discoverer, decomposer, designer, builder, confessor, reviewer, PR creator. Test on a real OCPSTRAT feature.

This is the moment where the team sees the convergence. JIRA, codebase, CI, design, implementation, review -- all in one conversation. All driven by Ralph. All grounded in the team's knowledge files.

**Estimated effort: 3-5 days.**

### Week 4: Team Rollout

Push all configuration to the repo. Every team member does `git pull` and has everything:

```bash
git pull
ralph run -c .rh/ralph.bugfix.yml -p "OCPBUGS-41234"
```

No setup beyond having Claude Code and Ralph installed. No MCP server configuration (it is in `.claude/settings.json`, which is in the repo). No knowledge creation (it is in `.rh/knowledge/`, which is in the repo).

This is the iPhone moment. Not because the technology is new. Because the experience is.

---

## Part X: Why This Cannot Be Trivially Replicated

1. **The knowledge files are the moat.** A competitor can copy the Ralph presets. They cannot copy six months of accumulated architectural principles, code patterns, platform gotchas, and CI playbooks. The knowledge files are specific to HyperShift. They are specific to this team. They compound over time.

2. **The hat configurations are domain-specific.** The discoverer hat knows to check JIRA, scan the codebase for related code, and read knowledge files. The builder hat knows to use `upsert.CreateOrUpdateFN`, run `make api` when types change, and follow the `AdditionalTrustBundle` propagation pattern. These are not generic instructions. They are the crystallized expertise of a team that has been building this system for years.

3. **The confessor pattern builds trust through use.** Once a team experiences an independent auditor catching issues the builder missed, they cannot go back to trusting unaudited agent output. The cultural shift -- from "review every line" to "review the decisions" -- is a one-way door.

4. **The flywheel accelerates.** Every Ralph session that adds a memory, every engineer that pushes a knowledge file, every guardrail added after a failure -- the system gets better. This is not a fixed product. It is a living system that improves with use.

---

## Part XI: Concrete Files to Create

Here is exactly what goes into the repo to make this real:

```
.rh/
  ralph.feature.yml           # Feature development workflow
  ralph.bugfix.yml            # Bug fix workflow
  ralph.incident.yml          # Incident response workflow
  ralph.ci-debug.yml          # CI debugging workflow

  knowledge/
    principles.md             # Architectural principles
    patterns.md               # Code patterns with examples
    gotchas.md                # Platform-specific gotchas
    ci-playbook.md            # CI debugging playbook
    onboarding.md             # Key concepts for new team members

  prompts/
    feature-prompt.md         # Prompt template for feature work
    bugfix-prompt.md          # Prompt template for bug fixes
    incident-prompt.md        # Prompt template for incidents

  specs/                      # Active work specs (Ralph native)
  agent/
    memories.md               # Persistent learning (Ralph native)
    tasks.jsonl               # Runtime tasks (Ralph native)

.claude/
  settings.json               # MCP server declarations (update existing)
  agents/                     # Sub-agents (already exist, may extend)
  skills/                     # Skills (already exist, may extend)
```

Update `AGENTS.md` to reference `.rh/knowledge/` files.
Update `.claude/settings.json` to add jira-mcp and prow-mcp.
The existing `.claude/agents/` and `.claude/skills/` directories are unchanged -- they work as-is.

---

## Appendix: The Full Picture in One Paragraph

An engineer types `ralph run -c .rh/ralph.feature.yml -p "OCPSTRAT-4521"`. Ralph reads the YAML, starts the event loop, and spawns Claude Code with the discoverer hat's instructions. Claude Code reads `AGENTS.md` (it always does), reads `.rh/knowledge/` (the guardrails say to), queries JIRA via jira-mcp (declared in `.claude/settings.json`), scans the codebase (using built-in tools), and writes a discovery report. The discoverer emits `discovery.done`. Ralph routes the event to the decomposer hat. Claude writes a JIRA breakdown, asks the human via Telegram (RObot), gets approval, creates stories via jira-mcp. The designer hat delegates to the `hcp-architect-sme` sub-agent (in `.claude/agents/`), produces a design doc. The builder hat writes Go code following patterns from `.rh/knowledge/patterns.md`, runs `make verify` and `make test`, records uncertainties as memories. The confessor hat audits the work independently. The reviewer hat produces a pre-review. The PR creator hat opens the PR via `gh`, updates JIRA via jira-mcp, and emits `LOOP_COMPLETE`. The diagnostics trace is attached to the PR. The human reviewer reads the design decisions and the confessor report, focuses on two flagged items, approves in 10 minutes. Total time: 2 hours. Engineer's active time: 15 minutes. Everything was built from pieces that already exist.

---

*The pieces are all here. They have been here. Nobody has composed them this way before. Now you know what happens when you do.*
