# Loop Inbox — Brain-to-Loop Feedback Channel

## Objective

Implement a per-loop inbox so the brain process can send feedback messages to coding agents running inside Ralph loops. The brain writes via `bm-agent inbox write`, the coding agent receives messages automatically via a Claude Code PostToolUse hook that invokes `bm-agent claude hook post-tool-use`.

## Spec Directory

`.planning/plans/loop-inbox/`

## Execution Order

Follow the checklist in `implementation/plan.md`. Steps build sequentially:

1. Core domain logic (`brain/inbox.rs`) + unit tests
2. `bm-agent` CLI namespace (inbox + hook) + integration tests
3. Profile settings.json + workspace surfacing + e2e tests
4. Brain & agent context updates
5. Final validation (`just clippy` + `just test` + `just exploratory-test`)

## Key Design Decisions

### `bm-agent` namespace (ADR-0010)
All agent-consumed commands live under `bm-agent`, separated from operator-facing commands. See `.planning/adrs/0010-agent-tools-namespace.md`.

### JSONL file-based inbox at `.ralph/loop-inbox.jsonl`
Consistent with existing `.ralph/events-*.jsonl` pattern. Append-only writes with `flock` for concurrency safety.

### Dedicated hook subcommand instead of shell script
`bm-agent claude hook post-tool-use` guarantees exit 0 and suppresses errors internally. No shell scripts to chmod, extract, or surface.

### `brain/inbox.rs` submodule (not a top-level domain module)
The inbox is the reverse direction of `brain/event_watcher.rs`. Belongs in the same domain module.

### Best-effort consumption (FR-4)
Crash window between hook truncating inbox and agent processing `additionalContext`. Accepted tradeoff.

### Primary loop only in v1
`--loop` flag deferred until worktree model exists.

### Team-level settings.json (not member-level)
Shared hooks apply to all members. Different from member-level `settings.local.json`.

## Requirements

1. Brain MUST send text messages to the primary loop via `bm-agent inbox write` — see `requirements.md` FR-1, FR-2
2. Coding agent MUST receive messages automatically via PostToolUse hook — see `design/detailed-design.md` Section 3
3. Messages MUST be consumed on delivery (best-effort) — see `requirements.md` FR-4
4. Coding agent context MUST include brain feedback priority guidance — see `requirements.md` FR-5
5. Operator MUST inspect pending messages via `bm-agent inbox peek` — see `requirements.md` FR-6
6. Inbox MUST be workspace-scoped — see `requirements.md` FR-7
7. Workspace provisioning MUST surface settings.json without manual steps — see `design/detailed-design.md` Section 4
8. Brain system prompt MUST document inbox usage — see `requirements.md` FR-9
9. Orphaned messages MUST survive loop restarts — see `requirements.md` FR-10
10. Concurrent writers/readers MUST be safe via `flock` — see `requirements.md` NFR-1
11. Empty inbox check MUST add negligible latency — see `requirements.md` NFR-2
12. `hook post-tool-use` MUST always exit 0 — see `requirements.md` NFR-3

## Acceptance Criteria

1. **(Regression)** All existing tests pass — `just clippy`, `just test`, `just exploratory-test` are green
2. **Write and peek**
   - Given a BotMinter workspace
   - When `bm-agent inbox write "fix CI"` is run
   - Then `bm-agent inbox peek` shows the message with timestamp and sender
3. **Read consumes**
   - Given a pending inbox message
   - When `bm-agent inbox read --format json` is run
   - Then the message is returned as JSON and peek shows "No pending messages"
4. **Hook delivers**
   - Given a pending inbox message
   - When `bm-agent claude hook post-tool-use` is run
   - Then stdout contains valid JSON with `additionalContext` key containing the message
5. **Hook is silent when empty**
   - Given no pending messages
   - When `bm-agent claude hook post-tool-use` is run
   - Then stdout is empty and exit code is 0
6. **Hook never fails**
   - Given a directory without `.botminter.workspace`
   - When `bm-agent claude hook post-tool-use` is run
   - Then exit code is 0 and stdout is empty
7. **Empty message rejected**
   - Given a BotMinter workspace
   - When `bm-agent inbox write ""` is run
   - Then the command exits non-zero with an error message
8. **Outside workspace rejected (inbox commands)**
   - Given a directory without `.botminter.workspace`
   - When `bm-agent inbox write "test"` is run
   - Then the command exits non-zero
9. **Settings.json surfaced**
   - Given a workspace created via `bm teams sync`
   - When `<workspace>/.claude/settings.json` is read
   - Then it contains a PostToolUse hook referencing `bm-agent claude hook post-tool-use`
10. **Re-sync preserves messages**
    - Given a pending inbox message
    - When `bm teams sync` is run
    - Then `bm-agent inbox peek` still shows the message
11. **Concurrent writes safe**
    - Given 8 concurrent `write_message` calls
    - When all complete
    - Then `read_messages` returns all 8 with no corruption
12. **Brain prompt documents inbox**
    - Given the scrum-compact brain system prompt
    - Then it contains `bm-agent inbox write` usage
13. **Agent context includes feedback guidance**
    - Given the scrum-compact context.md
    - Then it contains "Brain Feedback" section with priority guidance

## Key References

- Design: `.planning/plans/loop-inbox/design/detailed-design.md`
- Requirements: `.planning/plans/loop-inbox/requirements.md`
- Implementation plan: `.planning/plans/loop-inbox/implementation/plan.md`
- ADR-0010 (agent namespace): `.planning/adrs/0010-agent-tools-namespace.md`
- Brain module: `crates/bm/src/brain/`
- Workspace surfacing: `crates/bm/src/workspace/repo.rs`, `crates/bm/src/workspace/sync.rs`
- Profile: `profiles/scrum-compact/coding-agent/`, `profiles/scrum-compact/brain/system-prompt.md`
- CLI: `crates/bm/src/cli.rs`, `crates/bm/src/commands/mod.rs`
