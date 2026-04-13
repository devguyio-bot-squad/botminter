# Your First Journey

You've completed the [Bootstrap Your Team](bootstrap-your-team.md) guide — your team is set up, workspaces are provisioned, and the board is ready. This page walks you through your first end-to-end experience: creating an epic, launching your agent, watching it work, and reviewing its output.

We'll use the `agentic-sdlc-minimal` profile with its three-member model — an `engineer` (handles PO, architect, developer, and QE hats), a `chief-of-staff` (team coordination), and a `sentinel` (automated gates) — working on a project called `my-project`.

## Verify your setup

Before you begin, confirm everything is in place.

### On disk

Your workzone should look like this:

```
workzone/
  my-team/
    team/                            # Team repo (control plane)
      members/engineer-01/            # Member config
      projects/my-project/           # Project-specific dirs
    engineer-01/                     # Workspace repo (agent CWD)
      team/                          # Submodule → team repo
      projects/
        my-project/                  # Submodule → project fork
          src/                       # Your project's source code
            main.py
            config.py
          tests/
            test_main.py
          README.md
          pyproject.toml
      PROMPT.md                      # Copied from team/members/engineer-01/
      CLAUDE.md                      # Copied from team/members/engineer-01/
      ralph.yml                      # Copied from team/members/engineer-01/
      .botminter.workspace           # Workspace marker file
```

Run a quick check:

```bash
bm status
bm members list
bm projects list
```

### On GitHub

Open your team repo on GitHub. You should see:

- **Issue types** — The profile uses GitHub native issue types (Epic, Task, Bug) instead of kind labels. You may also see a `kind/docs` label used as a modifier on any issue type.
- **Labels** — `project/my-project` (created by `bm projects add`)
- **Project board** — linked to the repo, with a Status field containing all the pipeline statuses (`eng:po:triage`, `eng:arch:design`, etc.)
- **Views** — if you followed the `bm projects sync` instructions, you should have role-based views (Engineer, Human Gates, Sentinel, Chief of Staff). The Engineer view shows the engineer agent's work across all its hats; Human Gates shows statuses requiring operator input; Sentinel shows automated gate activity; Chief of Staff shows team coordination work.

If any of these are missing, re-run `bm projects sync` to sync statuses and get the view setup instructions.

## Create your first epic

Go to your team repo on GitHub and create a new issue:

1. Click **New issue**
2. **Issue type**: Select **Epic** from the issue type dropdown (the profile uses GitHub native issue types)
3. **Title**: Something concrete, e.g., `Add health check endpoint`
4. **Body**: Describe what you want. Be as specific or as high-level as you like — the agent's architect hat will produce a design doc from this. For example:

    ```markdown
    Add a `/healthz` endpoint that returns the application's health status.

    - Should check database connectivity
    - Should check external service availability
    - Return 200 with JSON body when healthy, 503 when unhealthy
    ```

5. **Labels**: Apply:
    - `project/my-project` — associates it with your project

6. **Status**: On the Project board, set the Status field to `eng:po:triage`

Your epic is now on the board, waiting for the agent to pick it up.

## Launch the agent

Start your team:

```bash
bm start
```

The agent launches as a Claude Code instance in its workspace. It runs on a loop — scanning the board, picking up work, processing it, and rescanning. There may be a short delay before the first scan cycle picks up your issue.

You can watch the agent's progress:

```bash
bm status -v
```

## The triage gate (your first interaction)

The agent's first action is to pick up your epic at `eng:po:triage`. The PO hat reads the epic, evaluates it, and posts a **triage request comment** on the issue:

```markdown
### po — 2026-02-27T14:00:00Z

**Triage request**

New epic in triage: "Add health check endpoint"

Summary: [agent's evaluation of the epic]

Please respond on this issue:
- `Approved` — accept into backlog
- `Rejected: <reason>` — close this epic
```

!!! warning "You need to respond"
    The agent does NOT auto-approve. The epic stays at `eng:po:triage` until you respond. On each scan cycle, the agent checks for your response — if none is found, it moves on and checks again next cycle.

**To approve:** Add an issue comment:

```
@bot Approved
```

**To reject:** Add an issue comment with your feedback:

```
@bot Rejected: The scope is too broad. Let's focus on the /healthz endpoint only, without external service checks.
```

Once you approve, the agent moves the epic to `eng:po:backlog`.

Alternatively, you can move the issue's Status to `eng:po:backlog` directly on the Project board — the agent will pick it up on the next scan.

!!! note "Why `@bot`?"
    Each agent has its own GitHub App identity and posts as a bot user (e.g., `team-engineer[bot]`). The `@bot` prefix on your comments helps the agent reliably identify human input in contexts where comment parsing is ambiguous.

## Backlog activation

At `eng:po:backlog`, the agent posts a backlog report comment. The epic parks here until you're ready to start work on it.

**To activate:** Add an issue comment:

```
@bot start
```

Or move the issue's Status to `eng:arch:design` directly on the Project board.

The agent picks this up on the next scan and begins the design phase.

## Watch the pipeline

From here, the agent drives the epic through the pipeline — switching hats at each stage. Here's what to expect:

### Design phase

1. **`eng:arch:design`** — The architect hat reads the epic and project codebase, produces a design document (stored in the team repo under `projects/my-project/knowledge/designs/`), and posts a summary comment.

2. **`eng:lead:design-review`** — The lead reviewer hat reviews the architect's design. This is an automated quality gate — the lead hat checks the design for completeness and either approves (advancing to human review) or rejects (sending back to the architect with feedback).

3. **`human:po:design-review`** — Your review gate. The PO reviewer hat posts a review request comment:

    ```markdown
    ### po — 2026-02-27T15:00:00Z

    **Design review request**

    Epic: "Add health check endpoint"

    [Summary of the design]

    Please respond on this issue:
    - `Approved` — proceed to planning
    - `Rejected: <feedback>` — revise the design
    ```

    **To approve:**

    ```
    @bot Approved
    ```

    **To reject:**

    ```
    @bot Rejected: The design doesn't address error handling for when the database is unreachable.
    ```

    If you reject, the agent reverts to `eng:arch:design` and incorporates your feedback. You can also move the Status directly on the Project board to approve or reject.

### Planning phase

4. **`eng:arch:plan`** — The architect hat breaks the approved design into stories with acceptance criteria and posts the story breakdown as a comment.

5. **`eng:lead:plan-review`** — The lead reviewer hat reviews the story breakdown.

6. **`human:po:plan-review`** — Your review gate. Same approve/reject flow as the design review (`@bot Approved` or `@bot Rejected: <feedback>`), or move the Status on the board.

### Breakdown and execution

7. **`eng:arch:breakdown`** — The architect creates individual story issues on the team repo. Stories are created using the **Task** issue type and linked to the parent epic via GitHub's native sub-issue relationship.

8. **`eng:lead:breakdown-review`** — The lead reviewer hat reviews the created story issues.

9. **`eng:po:ready`** — The epic parks here. Stories are created, and the epic waits for you to activate execution.

    **To activate:** Comment on the epic issue:

    ```
    @bot start
    ```

    Or move the Status to `eng:arch:in-progress` directly on the Project board.

10. **`eng:arch:in-progress`** — The architect hat monitors story execution. Each story goes through its own pipeline:

    ```
    eng:qe:test-design -> eng:dev:implement -> eng:dev:code-review -> eng:qe:verify -> eng:arch:sign-off -> snt:gate:merge -> done
    ```

    - **`eng:qe:test-design`** — The QE hat designs tests and writes test stubs *before* implementation (test-first approach)
    - **`eng:dev:implement`** — The developer hat implements the story against the test stubs
    - **`eng:dev:code-review`** — The code review hat reviews the implementation. Can reject back to `eng:dev:implement` with feedback.
    - **`eng:qe:verify`** — The QE hat verifies the implementation against acceptance criteria. Can reject back to `eng:dev:implement` with feedback.
    - **`eng:arch:sign-off`** and **`snt:gate:merge`** — Auto-advance gates handled by the board scanner. No manual action needed.

### Final acceptance

11. **`human:po:accept`** — When all stories are complete, the epic reaches your final review gate. Review the completed work and comment to close or revise:

    ```
    @bot Approved
    ```

    Or to send it back:

    ```
    @bot Rejected: The health check doesn't return the expected JSON schema. See the acceptance criteria.
    ```

12. **`done`** — Epic complete.

## Summary of human interaction points

Throughout the epic lifecycle, you'll interact at these gates:

| Gate | Status | Comment | Or move Status to |
|------|--------|---------|-------------------|
| Triage | `eng:po:triage` | `@bot Approved` or `@bot Rejected: <reason>` | `eng:po:backlog` |
| Backlog activation | `eng:po:backlog` | `@bot start` | `eng:arch:design` |
| Design review | `human:po:design-review` | `@bot Approved` or `@bot Rejected: <feedback>` | `eng:arch:plan` or `eng:arch:design` |
| Plan review | `human:po:plan-review` | `@bot Approved` or `@bot Rejected: <feedback>` | `eng:arch:breakdown` or `eng:arch:plan` |
| Ready activation | `eng:po:ready` | `@bot start` | `eng:arch:in-progress` |
| Final acceptance | `human:po:accept` | `@bot Approved` or `@bot Rejected: <feedback>` | `done` or `eng:arch:in-progress` |

The agent never auto-approves at any of these gates. You can interact via comments or by moving the Status directly on the Project board.

!!! tip "Comment attribution"
    Each agent posts as its own GitHub App bot user (e.g., `team-engineer[bot]`), making it easy to distinguish agent comments from your own. The `@bot` prefix on your comments provides an additional signal for reliable parsing.

## Monitor progress

While the agent works, you can check in anytime:

```bash
# Agent status
bm status -v

# See the issue activity on GitHub
gh issue list -R <your-org>/team-repo --state open
```

On the GitHub Project board, switch between views (Engineer, Human Gates, Sentinel, Chief of Staff) to see where issues sit in the pipeline.

## What to do when things go wrong

### Nothing happens after creating the epic

Make sure the epic has:
- The **Epic** issue type set
- The `project/<name>` label
- Its Status field set to `eng:po:triage` on the Project board

Then check that the agent is running with `bm status -v`.

### The epic is stuck at a review gate

If an issue sits at `eng:po:triage`, `human:po:design-review`, `human:po:plan-review`, `human:po:accept`, `eng:po:backlog`, or `eng:po:ready`, the agent is waiting for your comment. Check the issue for a review request comment and respond with `@bot Approved`, `@bot Rejected: <feedback>`, `@bot start`, or `@bot activate` as appropriate. Don't forget the `@bot` prefix. See the [human interaction table](#summary-of-human-interaction-points) above.

### Agent gets stuck on an issue

If an issue reaches `error` status, the agent has failed to process it 3 times. Check the issue comments for error details, fix the underlying problem, then manually reset the Status field on the GitHub Project board to the appropriate pipeline stage.

### Agent picks up the wrong issue

Issue types and labels matter. Make sure every issue has the correct issue type (Epic, Task, or Bug) and the correct `project/<name>` label.

## Stop the agent

When you're done:

```bash
bm stop
```

Or to force-stop immediately:

```bash
bm stop --force
```

## Next steps

- Read [The Agentic Workflow](../workflow.md) to understand the philosophy behind the process
- Learn about [Knowledge & Invariants](../concepts/knowledge-invariants.md) to customize what your agent knows
- Explore the [Coordination Model](../concepts/coordination-model.md) for details on board scanning and handoffs
- See [Profiles](../concepts/profiles.md) to understand how to switch to a multi-member team
