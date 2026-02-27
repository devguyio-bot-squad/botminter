# Rough Idea — Spike: Autonomous Ralph Runner

## Problem

Ralph Orchestrator is designed for single-objective sessions: you give it a PROMPT.md with a specific task, it works on it, emits LOOP_COMPLETE, and exits. Our team members need the opposite — they need to run indefinitely, pulling work from a board (`.github-sim/issues/`), working on one issue at a time, and moving to the next.

## Three Core Questions

1. **Work discovery:** How should the hatless Ralph (or a board_scanner hat) determine the next work item? PROMPT.md sets the reusable role identity, but the actual objective comes from a GitHub issue. How does Ralph discover and bind to a specific issue each cycle?

2. **Loop lifetime:** How do we prevent Ralph from dying? `persistent: true` suppresses LOOP_COMPLETE, but there are other termination triggers (max_iterations, max_runtime, consecutive_failures, fallback exhaustion). What's the right configuration and event flow to keep an agent alive indefinitely?

3. **State hygiene between work items:** Ralph tracks progress in scratchpad, memories, and tasks. When an agent finishes one issue and moves to the next, how do we reset working state without losing accumulated knowledge? PROMPT.md is already read-only, but scratchpad/tasks carry context from the previous issue.

## Proposed Direction: External Runner

An external runner (bash script or Justfile recipe) that wraps Ralph:

1. Ralph starts fresh with a reusable PROMPT.md (role identity + board scanning instructions)
2. Ralph discovers work from the board, works on it
3. Ralph completes (LOOP_COMPLETE or similar)
4. External runner: commits changes, archives `.ralph/` state, restarts Ralph fresh
5. Fresh Ralph: reads same PROMPT.md, discovers next work item from the board

This gives clean session boundaries between work items while keeping the agent alive at the system level.

## Constraints

- This is a spike — answer the three questions with working prototypes, not production code
- No changes to ralph-orchestrator source code — work within the existing configuration surface
- Findings inform M2's ralph.yml and workspace design
