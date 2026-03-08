---
phase: 05-team-manager-chat
plan: 01
subsystem: cli
tags: [team-manager, bm-chat, meta-prompt, interactive-sessions, ralph-config]

requires:
  - phase: 01-coding-agent-agnostic
    provides: CodingAgentDef for agent binary resolution
  - phase: 03-workspace-repository
    provides: Workspace repo model with .botminter.workspace marker
  - phase: 04-skills-extraction
    provides: Skills and ralph-prompts for meta-prompt assembly
provides:
  - Team Manager role definition with minimal statuses (mgr:todo/in-progress/done)
  - Full Team Manager skeleton in all profiles
  - bm chat command launching interactive coding agent sessions
  - build_meta_prompt() function assembling context-aware system prompts
affects: [06-minty]

tech-stack:
  added: []
  patterns: [meta-prompt-assembly, exec-replacement, interactive-framing, guardrail-numbering]

key-files:
  created:
    - crates/bm/src/commands/chat.rs
    - crates/bm/src/chat.rs
    - profiles/scrum/roles/team-manager/
  modified:
    - crates/bm/src/cli.rs
    - crates/bm/src/commands/mod.rs
    - profiles/scrum/botminter.yml
    - profiles/scrum-compact/botminter.yml
    - profiles/scrum-compact-telegram/botminter.yml

key-decisions:
  - "exec() to replace bm process with coding agent — no parent process overhead"
  - "Guardrails numbered from 999 — prevents collision with Ralph's internal numbering"
  - "Interactive framing replaces loop framing in meta-prompt — different mental model for chat vs autonomous"
  - "Hat instructions included as capabilities, not as active directives — user picks context"
  - "Team Manager's default project is the team repo itself — process improvement focus"

patterns-established:
  - "MetaPromptParams struct for meta-prompt assembly — single parameter object"
  - "build_meta_prompt() pattern: identity → capabilities → guardrails → role context → reference"
  - "exec() for CLI-to-agent handoff — clean process replacement"
  - "--render-system-prompt flag for debugging meta-prompts"

one-liner: "Team Manager role with full skeleton and bm chat command with build_meta_prompt() context-aware system prompts"

requirements-completed: [TMGR-01, TMGR-02, TMGR-03, CHAT-01, CHAT-02, CHAT-03]

completed: 2026-03-04
---

# Phase 5: Team Manager + Chat Summary

**Team Manager role with mgr:todo/in-progress/done statuses and full skeleton, bm chat command launching interactive coding agent sessions with build_meta_prompt() assembling context-aware system prompts from workspace data**

## Performance

- **Tasks:** 3 (steps 16-18 from original plan)
- **Files modified:** ~15

## Accomplishments
- Defined Team Manager role in all 3 profile manifests with minimal statuses (`mgr:todo`, `mgr:in-progress`, `mgr:done`)
- Built full Team Manager skeleton at `profiles/scrum/roles/team-manager/` — .botminter.yml, context.md, ralph.yml (executor hat, board-scanner auto-inject, RObot config), PROMPT.md, knowledge/, invariants/, coding-agent/
- Implemented `bm chat <member> [-t team] [--hat h]` command (273 lines) — verifies workspace, reads ralph.yml and PROMPT.md, assembles meta-prompt, execs coding agent
- Built `build_meta_prompt()` in `chat.rs` (362 lines) — assembles markdown system prompt with identity, capabilities (all hats or single hat), guardrails (numbered from 999), role context, and reference material paths
- Added `MetaPromptParams` struct as single parameter object for meta-prompt assembly
- Added `--render-system-prompt` flag for debugging — prints assembled prompt and exits
- Used `exec()` for process replacement — `bm` process becomes the coding agent with no overhead
- Sprint 5 documentation updated

## Files Created/Modified
- `crates/bm/src/commands/chat.rs` — Chat command: workspace verification, ralph.yml parsing, hat instruction extraction, meta-prompt assembly, agent exec
- `crates/bm/src/chat.rs` — MetaPromptParams, build_meta_prompt() with 5-section assembly
- `profiles/scrum/roles/team-manager/` — Full role skeleton (.botminter.yml, context.md, ralph.yml, PROMPT.md, hats/executor/, knowledge/, coding-agent/)
- `profiles/scrum-compact/roles/team-manager/` — Same skeleton
- `profiles/scrum-compact-telegram/roles/team-manager/` — Same skeleton
- `crates/bm/src/cli.rs` — Chat and Minty command definitions

## Decisions Made
- `exec()` over subprocess — clean handoff, no zombie parent process, coding agent inherits terminal directly
- Guardrails numbered from 999 to avoid collision with Ralph's internal 1-N numbering scheme
- Interactive framing ("You are in an interactive session with the human") replaces Ralph's loop framing — fundamentally different interaction model
- Hat instructions presented as "capabilities" in chat — user can access any hat's knowledge without the dispatch loop
- Team Manager's default project is the team repo — focuses on process improvement rather than product code

## Deviations from Plan

### Implementation decisions beyond plan specification

**1. Guardrails numbered from 999**
- **Issue:** Plan does not specify a numbering scheme for guardrails in the meta-prompt
- **Decision:** Numbered from 999 to avoid collision with Ralph's internal 1-N numbering
- **Impact:** Defensive convention; prevents guardrail ID conflicts in nested sessions

**2. `MetaPromptParams` struct**
- **Issue:** Design shows `build_meta_prompt()` with inline parameters; implementation wrapped into a struct
- **Rationale:** Improves testability and parameter management
- **Impact:** API shape differs from design pseudocode but behavior matches

## Next Phase Readiness
- Chat infrastructure complete, ready for Minty (which reuses the same exec + system prompt pattern)
- Meta-prompt assembly pattern established for any future interactive entry points
- Team Manager role available for hiring via `bm hire team-manager`

---
*Phase: 05-team-manager-chat*
*Completed: 2026-03-04*
