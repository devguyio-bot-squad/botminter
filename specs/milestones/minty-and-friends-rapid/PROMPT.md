# Minty and Friends — Sprints 4–6

## Objective

Implement skills extraction, the Team Manager role with `bm chat`, and the Minty interactive assistant for the `bm` CLI. Steps 14–20 of the implementation plan, covering 12 tasks across 3 sprints.

## Prerequisites

Sprints 1–3.5 are complete:
- **Sprint 1** (Steps 1–6): Coding-agent-agnostic architecture — `CodingAgentDef`, agent tag filtering, profile restructuring, workspace parameterization
- **Sprint 2** (Steps 7–9): Profile externalization — `bm profiles init`, disk-based profile API, auto-prompt pattern
- **Sprint 3** (Steps 10–12): Workspace repository model — workspace repo creation via `bm teams sync --push`, submodule sync, `bm start`/`bm stop` adaptation, status commands
- **Sprint 3.5** (Step 13): Board-scanner skill migration — `board_scanner` hat replaced with `board-scanner` auto-inject skill, `roles/` directory rename, 149 stale `.botminter/` path references fixed (commit `1f8d406`)

## Spec Directory

`specs/milestones/minty-and-friends-rapid/`

## Required Reading

- **Design:** `specs/milestones/minty-and-friends-rapid/design.md` — read Sprint 4, Sprint 5, Sprint 6 sections before beginning any task
- **Plan:** `specs/milestones/minty-and-friends-rapid/plan.md` (Steps 14–20)
- **Requirements:** `specs/milestones/minty-and-friends-rapid/requirements.md`
- **Research:** `specs/milestones/minty-and-friends-rapid/research/ralph-injected-prompts.md` — critical for Step 14

Read the design document before beginning any task. Each task file references specific design sections.

## Task Execution Order

Tasks are in `specs/milestones/minty-and-friends-rapid/tasks/` organized by step. Execute in step order — each step builds on the previous:

| Step | Dir | Tasks | Sprint |
|------|-----|-------|--------|
| 14 | `step14/` | 1 — Extract Ralph prompts to profiles | 4 |
| 15 | `step15/` | 2 — Status-workflow skill + hat updates + Sprint 4 docs | 4 |
| 16 | `step16/` | 2 — Team Manager profile manifest + skeleton + content | 5 |
| 17 | `step17/` | 2 — `build_meta_prompt()` + `bm chat` CLI subcommand | 5 |
| 18 | `step18/` | 1 — Sprint 5 documentation | 5 |
| 19 | `step19/` | 2 — Minty config/prompt + `bm minty` CLI subcommand | 6 |
| 20 | `step20/` | 2 — Minty skills + Sprint 6 documentation | 6 |

Within each step, execute tasks in filename order (`task-01-*`, `task-02-*`).

## Sprint Summaries

### Sprint 4: Skills Extraction & Ralph Prompt Shipping (Steps 14–15)

Extract Ralph Orchestrator's system prompts into `ralph-prompts/` within each profile — these are reference copies enabling `bm chat` in Sprint 5. Extract status transition helpers into a shared `coding-agent/skills/status-workflow/` skill. Update hat instructions to reference the shared skill instead of inlining mutation logic.

Per design.md "Sprint 4: Skills Extraction & Ralph Prompt Shipping" section.

### Sprint 5: Team Manager Role (Steps 16–18)

Add the `team-manager` role to the scrum profile with `mgr:todo`/`mgr:in-progress`/`mgr:done` statuses. Create the member skeleton with board-scanner skill, executor hat, and context files. Implement `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt]` — builds a meta-prompt from Ralph prompts, guardrails, hat instructions, and PROMPT.md content, then launches the coding agent with `--append-system-prompt-file`.

Per design.md "Sprint 5: Team Manager Role" section.

### Sprint 6: Minty — BotMinter Interactive Assistant (Steps 19–20)

Create Minty's config structure (`minty/` embedded alongside profiles), persona prompt, and `bm minty [-t team]` launch command. Implement four composable skills: `team-overview`, `profile-browser`, `hire-guide`, `workspace-doctor`. Update `bm profiles init` to extract Minty config alongside profiles.

Per design.md "Sprint 6: Minty — BotMinter Interactive Assistant" section.

## DANGER: Nested Claude Code & Process Safety

You are running inside Ralph Orchestrator. Steps 17 and 19 produce code that **launches Claude Code** (`bm chat`, `bm minty`). This creates a Claude-inside-Claude situation.

- **CLAUDECODE env var:** Before launching Claude Code for testing (e.g., running `bm chat` or `bm minty`), you MUST unset the `CLAUDECODE` environment variable. The nested Claude Code instance will refuse to start if it detects it is already inside another Claude Code session. Use `CLAUDECODE= bm chat ...` or `env -u CLAUDECODE bm chat ...`.
- **NEVER kill Ralph:** You are a Ralph-managed process. You MUST NOT run `kill`, `pkill`, `killall`, or any signal-sending command against Ralph or its parent processes. If you need to stop a process you launched for testing, you MUST only kill it by the specific PID returned from your own spawn — never by process name, never by pattern matching.
- **No `bm stop` against yourself:** Do NOT run `bm stop` during implementation — it would terminate Ralph (your own orchestrator). Only use `bm stop` if explicitly told to by the human.

## Constraints

- All profile content MUST use `team/` paths — no `.botminter/` path references
- E2E tests required for GitHub API operations per `invariants/e2e-testing.md`
- Alpha policy: breaking changes expected, no migration paths
- Existing tests MUST continue to pass after each step
- Ralph prompt extraction (Step 14) MUST accurately represent what Ralph injects — compare against Ralph Orchestrator source
- The `board-scanner` auto-inject skill pattern from Step 13 MUST be followed for the Team Manager's board-scanner skill
- `bm chat` MUST use `--append-system-prompt-file` — this gives the meta-prompt higher authority than CLAUDE.md (see design.md Sprint 5 section)
- Minty MUST work without any teams configured (`~/.botminter/` may not exist)
- Documentation updates MUST be delivered per sprint, not batched
