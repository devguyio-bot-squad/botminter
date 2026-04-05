# BotMinter Demo Script: "Your Agents, Your Process"

**Duration:** 10 minutes recorded
**Project:** HyperShift (real, audience knows it well)
**Profile:** scrum-compact (single agent, multiple hats)
**Bridge:** Matrix via Tuwunel (local, zero setup for viewers)

---

## Pre-Recording Checklist

Before recording, have these ready:

1. **A fully provisioned team** with HyperShift as the project, bridge running, workspace synced
2. **A completed epic** that went through the full pipeline (design, plan, breakdown, implementation) -- this is the "reveal" at the end
3. **A clean terminal** with a reasonable font size (14-16pt), dark background
4. **Element (Matrix client)** open with the team room visible, showing agent messages
5. **GitHub** open in a browser with the team repo's Project board
6. **Two browser tabs ready:** (a) the Project board, (b) a completed epic's issue page with full comment history

### Recording Strategy

| Segment | Method | Why |
|---------|--------|-----|
| Opening hook + problem statement | Direct to camera or voiceover | Sets the frame before any tool appears |
| `bm init` wizard | Real-time recording, edited for pace | Shows the interactive experience |
| `bm hire` + `bm projects add` | Real-time | Fast commands, high impact |
| `bm teams sync --all` | Time-lapse or cut | Takes 30-60s, mostly waiting |
| Bridge + Element | Screen recording | Show the chat room with agents |
| `bm start` + board activity | Pre-recorded results shown as walkthrough | Full pipeline takes hours; show the output |
| Epic walkthrough on GitHub | Screen recording with voiceover | The payoff -- show every comment, every transition |

---

## The Script

### ACT 1: The Problem (0:00 - 2:00)

**[SCREEN: Empty terminal, or split with Claude Code on one side]**

> You're using Claude Code. Maybe Copilot, maybe Cursor. And it works -- for one task, in one session, with you driving. But you've started to notice the gaps.

**[BEAT]**

> You paste the same context into every session. "We use Go. Our tests live here. Don't touch this package." Every time. You write it in CLAUDE.md, and then your colleague writes a different one. And when you update a convention -- say, you switch from `make` to `just` -- you update your file, they don't update theirs, and now your agents are doing different things.

> And the bigger problem: when you close that terminal, the work disappears. There's no record of what the agent decided, why it chose that approach, or what it tried and rejected. You review a PR and you're reverse-engineering intent from a diff.

**[PAUSE -- let it land]**

> What if your agents worked like a team? Not a terminal session you babysit, but a team that picks up work from a board, follows your process, and leaves a traceable record of every decision?

> That's what I want to show you. Not a concept. Working software, running against HyperShift.

**[TRANSITION: Terminal comes to focus]**

---

### ACT 2: Stand Up a Team in 2 Minutes (2:00 - 4:30)

**Talking point before typing:** "Let me show you the entire setup. From nothing to a running team."

#### Step 1: Init (show ~45 seconds of the wizard, cut the rest)

```bash
bm init
```

**[Show the wizard flow -- narrate as you go:]**

> `bm init` walks you through it. Pick a name for the team, pick a profile -- I'm using `scrum-compact`, a single agent that wears all the hats: architect, developer, QE, code reviewer. Pick a bridge for chat -- I'll use Matrix, which runs locally, no accounts to create. Point it at a GitHub org, and it creates the repo, the project board, all the labels for the workflow.

**[Show the wizard selecting:]**
- Team name (e.g., `hypershift-agents`)
- Profile: `scrum-compact`
- Bridge: `Matrix (Tuwunel)`
- GitHub org selection
- Repo creation

**[Cut to completed output showing success message]**

#### Step 2: Hire + Add Project (show real-time, ~30 seconds)

```bash
bm hire superman
```

> One command. The agent now has its role, its prompts, its knowledge structure, its workflow definition -- all from the profile.

```bash
bm projects add https://github.com/<org>/hypershift
```

> And now it knows what codebase to work on. This also creates a `project/hypershift` label on the board so issues get routed to the right code.

#### Step 3: Sync (time-lapse or cut, ~15 seconds shown)

```bash
bm teams sync --all
```

**[Narrate over the output:]**

> `teams sync` does three things. It pushes the team repo to GitHub. It creates the agent's workspace -- a dedicated repo with the project as a submodule and all the context files surfaced at the root. And with `--all`, it provisions the bridge -- starts the Matrix server, creates the agent's bot account, creates the team room.

**[Cut to the completed output]**

#### Step 4: Quick Status Check (~15 seconds)

```bash
bm status
```

**[Show the status dashboard with team name, profile, members, projects]**

```bash
bm teams show
```

> Everything is registered. The agent has a workspace, the bridge is running, the board is ready.

---

### ACT 3: What You Actually Get (4:30 - 6:00)

**Talking point:** "Let me show you what just happened, because the setup is not the interesting part. What you get from it is."

#### The Workspace

**[Show the workspace directory tree, either `tree` or `ls -la`]**

```
hypershift-agents/
  team/                          # The control plane -- your process, in Git
  superman-01/                   # The agent's workspace
    team/                        # Submodule -- always current
    projects/
      hypershift/                # Your project code, as a submodule
    CLAUDE.md                    # Agent context -- auto-synced from team/
    PROMPT.md                    # Work instructions
    ralph.yml                    # Workflow config (hats, guardrails)
```

> The agent's working directory is `superman-01/`. It sees your project under `projects/hypershift/`. It reads context from CLAUDE.md. And here's the part that matters --

#### Layered Knowledge

> -- all of this is scoped. Watch.

```bash
bm knowledge list
```

**[Show the output listing knowledge files at different scopes]**

> Team-wide knowledge applies to every agent on every project. "Use conventional commits." "PRs need tests." Put it in `team/knowledge/`, every agent sees it.

> Project knowledge applies only to HyperShift. Architecture decisions, API contracts, deployment constraints. Only agents working on HyperShift pick this up.

> And invariants -- these are not suggestions. "All public functions must have tests." An invariant at the team level means every agent, every project, every time. You add it once. It applies forever.

> When you update a knowledge file, push to the team repo, and every agent picks it up on the next cycle. No copy-pasting. No "did you update your CLAUDE.md?"

---

### ACT 4: The Pipeline -- From Idea to PR (6:00 - 8:30)

**Talking point:** "Now let me show you what happens when work flows through this system."

**[IMPORTANT: This section uses pre-recorded results. The presenter walks through a completed epic on GitHub, not a live run.]**

#### The Board

**[Switch to browser: GitHub Project board]**

> This is the project board. Each column is a pipeline stage. `po:triage`, `arch:design`, `dev:implement`, `qe:verify`. The agent pulls work by scanning for issues in columns that match its current hat.

> I created an issue yesterday -- a real feature for HyperShift. Let me show you what happened.

#### The Issue Walkthrough

**[Switch to browser: The completed epic's issue page. Scroll through the comments slowly.]**

> I created this issue: "[whatever the epic title is]". I labeled it `kind/epic` and `project/hypershift`, set the status to `po:triage`.

> The agent picked it up. First comment -- the PO hat triaged it, posted a summary, and asked me to approve.

**[Show the triage comment with the emoji header: "### PO -- timestamp"]**

> I approved. The agent moved it to `arch:design`. Now the architect hat activated -- same agent, different context. It read the HyperShift source code, read the project knowledge, and produced a design document.

**[Show the design comment with the architecture summary]**

> Look at the detail here. It references real modules, real interfaces. This is not a template. It read the code.

> Then the lead hat reviewed the design, caught [whatever it caught], and the architect revised. Then it came to me for approval.

**[Show the rejection and revision cycle if one occurred, or the approval]**

> After I approved the design, the architect broke it into stories -- each with acceptance criteria, each as a separate GitHub issue linked to this epic.

**[Show a story issue briefly]**

> Then execution. The QE hat wrote test stubs from the acceptance criteria -- tests first, before any implementation. The dev hat implemented against those tests. The code review hat verified. Every step is a comment on the issue. Every decision is on the record.

**[Scroll through the story comments showing the hat transitions]**

> This is the part people miss about agentic SDLC. It's not about making agents faster. It's about making their work auditable. When this PR lands on your desk, you don't have to reverse-engineer what happened. You read the issue and you see every design decision, every review comment, every revision.

#### The Chat (Bridge)

**[Switch to Element (Matrix client)]**

> And while all of that was happening on GitHub, the agent was also here -- in a Matrix room. You can follow along in real time. Each agent has its own bot identity, so you see who said what.

**[Show the room with messages from the agent -- status transitions, summaries, or whatever is visible]**

> This is a Tuwunel server running locally in a Podman container. No accounts to create, no SaaS to sign up for. `bm teams sync --bridge` set it all up.

---

### ACT 5: The Takeaway (8:30 - 9:30)

**[Back to terminal or direct to camera]**

> Three things to take away.

> **One.** Agentic SDLC is not "let the AI write code and hope for the best." It's a structured pipeline where every step produces a reviewable artifact, every decision is traceable, and you control the gates.

> **Two.** The conventions problem is solved. Knowledge, invariants, and constraints live in Git, scoped to the right level, and propagate automatically. You define them once. Every agent follows them.

> **Three.** This runs today. Against real projects. I showed you HyperShift -- not a toy demo. The same setup works for any project you point it at.

---

### ACT 6: One More Thing (9:30 - 10:00)

**[PAUSE -- the "one more thing" beat]**

> One more thing.

> Everything I showed you -- the profiles, the hats, the knowledge scoping, the pipeline stages, the review gates -- all of it is customizable. The `scrum-compact` profile is a starting point. You can add roles, change the pipeline, tighten invariants, add project-specific knowledge. The profile is a Git repo. Your process evolves with `git push`.

> If you want to try it: `cargo install bm`, run `bm init`, and you're five minutes from having your own team running against your project.

> I'll drop the link in the thread.

**[END]**

---

## Production Notes

### Pacing Guidelines

| Segment | Target Duration | Pacing |
|---------|----------------|--------|
| Problem statement | 2:00 | Deliberate, let silences work |
| Setup demo | 2:30 | Brisk, show competence |
| Knowledge/workspace explanation | 1:30 | Medium, this is the "aha" |
| Pipeline walkthrough | 2:30 | Slow at the reveals, fast between |
| Takeaway | 1:00 | Confident, declarative |
| One more thing | 0:30 | Pause before, punch at the end |

### Editing Notes

1. **The init wizard** can be 2x speed with narration at 1x -- shows the full flow without boring the viewer
2. **`teams sync`** should be a hard cut from command to result -- nobody needs to watch git clone
3. **The epic walkthrough** is the centerpiece. Do NOT rush this. Let people read the comments. Pause on the design doc. This is where skeptics become believers.
4. **Element/Matrix** is a brief visual proof point, not a deep dive. 15-20 seconds maximum.
5. **No background music.** This is for engineers. Silence and terminal sounds carry authority.

### What NOT to Show

- Do not show `ralph.yml` internals or hat configuration -- too much detail, loses the narrative
- Do not explain the two-layer runtime model (inner loop / outer loop) -- save for a follow-up
- Do not mention alpha status or caveats -- this is a demo, not a disclaimer
- Do not show `bm start` and wait for the agent to run -- the pipeline takes hours, show the results
- Do not compare to other tools by name -- let the work speak

### Recovery: If Something Breaks During Recording

Because this is recorded, not live:

- Record each act as a separate segment and stitch in post
- Have the completed epic pre-recorded as a backup -- if you can't create a new one, walk through the existing one
- The bridge container should be pre-started and verified before recording Act 2
- Run `bm status` before recording to confirm everything is green

### Pre-Recording Dry Run

Before the final recording, do one full dry run:

1. `bm init` through the full wizard -- verify it completes without errors
2. `bm hire superman` -- verify the member is created
3. `bm projects add <hypershift-fork-url>` -- verify the project is registered
4. `bm teams sync --all` -- verify workspace creation and bridge provisioning
5. Open Element and verify the room exists with the agent's bot user
6. Create a test epic and run it through at least `arch:design` to verify the pipeline works
7. Time each segment -- aim for 9:30 total to leave buffer

### HyperShift-Specific Prep

- Fork HyperShift to the demo org before recording
- Add at least one project-level knowledge file (e.g., `projects/hypershift/knowledge/architecture-overview.md`) so the knowledge list shows real scoping
- The epic should be something the audience recognizes as real HyperShift work, not a generic "add health check" -- pick something from actual issues or backlog items
- If the design doc references real HyperShift packages (`hypershift/control-plane-operator`, `hypershift/hypershift-operator`), it demonstrates that the agent actually read the code

---

## Narrative Arc Summary

```
The problem you feel           "Same context everywhere, no record, no process"
     |
The setup                      "Five commands, two minutes, done"
     |
What you get                   "Scoped knowledge that propagates automatically"
     |
The pipeline in action         "Every decision on the record, every step auditable"
     |
The takeaway                   "This is what agentic SDLC looks like"
     |
One more thing                 "Everything is customizable. Your process, in Git."
```

The audience walks in thinking agentic SDLC is vaporware or chaos.
They walk out knowing it's a structured pipeline they can run tomorrow.
