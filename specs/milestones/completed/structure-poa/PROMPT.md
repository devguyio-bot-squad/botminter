# PROMPT — Implement Milestone 1: Structure + human-assistant

## Objective

Build the generator repo (`botminter`) that stamps out GitOps-style agentic team repos, and validate the first team member (human-assistant) running with HIL via Telegram.

## Context

- Design: `specs/master-plan/design.md` (Section 3: generator architecture, Section 4: M1 design)
- Plan: `specs/master-plan/plan.md` (Steps 1–9)
- Ralph orchestrator: `/opt/workspace/ralph-orchestrator/`
- Requirements: `specs/master-plan/requirements.md`
- M1 Spec: `specs/milestone-1-structure-poa/m1-spec.md`

## Key Requirements

1. **Persistent polling** — Ralph's event loop must stay alive indefinitely when no work is found. Validate this FIRST (plan Step 1) before building anything else.
2. **Generator skeleton** — `skeletons/team-repo/` with bare directory structure + Justfile. Process-agnostic.
3. **`just init` recipe** — layers skeleton + profile into a self-contained team repo instance at a target path. Copies generator content into `.team-template/` for `add-member`.
4. **RH Scrum profile** — `skeletons/profiles/rh-scrum/` with PROCESS.md, CLAUDE.md, team knowledge/invariants, and human-assistant member skeleton.
5. **Team repo Justfile** — `add-member`, `create-workspace`, `launch` recipes baked into every generated repo.
6. **Workspace model** — each member runs in an isolated workspace repo with the team repo as a submodule. Files surfaced (copied) from team repo to workspace root.
7. **human-assistant member** — single `board_scanner` hat, persistent polling, training mode (observe & report), poll-log.txt timestamped logging.
8. **HIL via Telegram** — RObot configured, full round-trip: human-assistant sends → human receives → human responds → human-assistant processes.

## Acceptance Criteria

- Given a clean machine, when `just init --repo=/tmp/test-team --profile=rh-scrum project=hypershift` is run from the generator repo, then a self-contained team repo is created with Justfile, PROCESS.md, CLAUDE.md, knowledge/, invariants/, projects/hypershift/, .github-sim/, and .team-template/.
- Given a generated team repo, when `just add-member human-assistant` is run, then `team/human-assistant/` is created with ralph.yml, PROMPT.md, CLAUDE.md, knowledge/, invariants/.
- Given a team repo with human-assistant added, when `just create-workspace human-assistant` is run, then `../workspace-human-assistant/` is created with the team repo as submodule and member files surfaced to root.
- Given a workspace, when `just launch human-assistant` is run, then Ralph starts, enters the board_scanner hat, scans the empty board, and reports via training mode format.
- Given a running human-assistant, when 5+ minutes elapse with no work, then `poll-log.txt` shows timestamped START/result/END triplets at regular intervals (persistent polling validated).
- Given a running human-assistant with RObot configured, when the human-assistant sends a training-mode message, then the human receives it on Telegram and can respond, completing the full HIL round-trip.

## Implementation Order

Follow `plan.md` Steps 1–9 sequentially. Each step has detailed guidance, test requirements, and a demo description. Step 1 (persistent polling) is the critical validation — if it fails, revise the event loop design before proceeding.

## Constraints

- You MUST NOT kill all ralph processes because you are running inside ralph.
- YOU MUST use a pid or store pid number
- You MUST kill ONLY a pid number of a process that you launched
- Use ralph-orchestrator as-is where possible; fork/modify only where necessary
- Ralph-Wiggum style (sequential) within each team member
- No code work on any project repo in M1
- No .github-sim issues populated in M1 (board is empty)
- **Step 9 (HIL via Telegram):** `RALPH_TELEGRAM_BOT_TOKEN` env var MUST be set. If not set, abort Step 9 with a clear error. The token is NEVER stored in committed files. During bot onboarding, the human must send a message to the bot on Telegram to establish the chat_id — wait for this before proceeding. After launching the human-assistant, the human will respond to the training-mode message on Telegram — verify the round-trip by checking events and poll-log.txt.
