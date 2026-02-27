# Requirements — Spike: Autonomous Ralph Runner

> Q&A record from requirements clarification.
> Context: [rough-idea.md](rough-idea.md), [master-plan design.md](../master-plan/design.md).

---

## Q1: How should scratchpad state be reset between work items?

**Q:** When an agent finishes one issue and moves to the next, the scratchpad carries stale context from the previous work item. Three approaches exist:

- **(a) External runner:** Each work item is a fresh `ralph run`. The external runner commits changes, archives state, and restarts Ralph. Fresh run clears the scratchpad automatically. Memories (accumulated knowledge) persist across runs.
- **(b) Persistent mode + self-clear:** Agent stays alive via `persistent: true`. Hat instructions tell the agent to manually clear its own scratchpad (via `ralph tools`) at the start of each new work item. No external process needed, but relies on the agent following instructions correctly.
- **(c) Persistent mode + accept accumulation:** Agent stays alive, scratchpad accumulates. Rely on the scratchpad's natural structure and the agent's ability to ignore stale context. Risk: context pollution, growing token cost, eventual confusion.

Which approach should the spike validate?

**A:** Option (b) — persistent mode with self-clear. The agent stays alive via `persistent: true` and is instructed to clear its own scratchpad when starting a new work item. No external runner needed. This keeps the architecture simpler (one process, no wrapper script) and relies on Ralph's existing capabilities.

---

## Q2: Is self-clearing the scratchpad technically feasible?

**Q:** Option (b) requires the agent to clear its own scratchpad. Research into Ralph's internals confirms:

- The scratchpad (`.ralph/agent/scratchpad.md`) is **read fresh from disk every iteration** — no in-memory caching. Overwriting the file takes effect on the next iteration.
- There is **no `ralph tools scratchpad clear` command**. The agent would overwrite the file directly using standard file write operations.
- Tasks (`.ralph/agent/tasks.jsonl`) are also read fresh from disk each iteration — same overwrite strategy applies.
- Memories (`.ralph/agent/memories.md`) persist and are shared across loops — these are accumulated knowledge and should be **kept** between work items.

The self-clear strategy: when the board scanner dispatches to a new work item, the agent overwrites `scratchpad.md` with a fresh context (e.g., "Starting work on issue #N") and clears `tasks.jsonl`. Memories survive.

Is this the right boundary — clear scratchpad + tasks, keep memories?

**A:** Yes. Clear scratchpad + tasks when starting a new work item. Keep memories — they represent accumulated knowledge that should carry across work items.

---

## Q3: What event flow keeps the agent alive between work items?

**Q:** In `persistent: true` mode, when an agent emits LOOP_COMPLETE after finishing an issue, Ralph suppresses termination and injects `task.resume`. This `task.resume` event needs to route back to the board scanner to start the next cycle.

Two options:
- **(a)** Board scanner subscribes to `task.resume` in addition to `board.scan` / `board.rescan`. When persistent mode injects `task.resume`, the board scanner fires, clears scratchpad/tasks, and scans for next work.
- **(b)** Skip LOOP_COMPLETE entirely. After finishing an issue, the work hat publishes `board.scan` directly — no completion signal, just loop back.

**A:** Option (a) — the board scanner subscribes to `task.resume` (hats can subscribe to multiple events). This way LOOP_COMPLETE signals a clean work-item boundary, persistent mode converts it to `task.resume`, and the board scanner picks it up to start fresh.

---

## Q4: What does the spike produce?

**Q:** Two options for deliverables:
- **(a) Minimal working prototype:** A single-hat agent that polls a dummy board, picks up a fake issue, works on it, completes, self-clears, and picks up the next one. Proves the pattern end-to-end.
- **(b) Research/design spike:** Produces findings and a recommended ralph.yml pattern for M2 to implement. No running code.

**A:** Option (a) — a minimal working prototype. Prove the pattern works end-to-end with a running agent.

---

## Q5: What should the prototype work on?

**Q:** The prototype needs fake work items to cycle through:
- **(a) Fake `.github-sim/` issues:** Markdown files with frontmatter, scanner reads them, worker does something trivial (appends comment, transitions status). Closer to M2.
- **(b) Plain text files:** Simple files in a `work/` directory. Agent reads one, writes a result, marks it done. Avoids coupling to `.github-sim/` format.

**A:** Option (b) — plain text files. Keep the spike decoupled from `.github-sim/` format. Focus on proving the autonomous loop pattern, not the issue format.

---

## Q6: Where does the prototype live?

**Q:** Should the prototype be a throwaway directory or inside the repo?

**A:** Inside the repo at `specs/milestone-1.5-autonomous-ralph/artifacts/`. The prototype runtime directory is throwaway (`/tmp/spike-autonomous/`), but the spike's artifacts — ralph.yml, PROMPT.md template, captured diagnostics, and findings — are preserved under the spike milestone directory for reference.

---

## Q7: How many work items should the prototype cycle through?

**Q:** How many items does the prototype need to process to prove the pattern?

**A:** 3 items. Enough to prove: (1) first item pickup works, (2) self-clear + second item proves clean handoff, (3) third item proves the pattern is repeatable. After the third item, the scanner finds no work and idles — proving the idle behavior too.

---

## Q8: What does the worker hat actually do?

**Q:** The work hat needs to do something observable but trivial. What action proves "work was done"?

**A:** Read the work item file, write a corresponding result file (e.g., `work/item-1.txt` → `done/item-1.txt` with a summary), and delete/rename the source. Observable, verifiable, minimal.

---

## Q9: Should the prototype use Telegram (RObot)?

**Q:** Should the prototype include HIL via Telegram, or run fully autonomous?

**A:** No Telegram. Run fully autonomous. The spike is about the loop lifecycle, not HIL. Training mode and Telegram are validated separately in M1/M2.

---

## Q10: What are the success criteria?

**Q:** What must be true for the spike to be considered successful?

**A:**
1. Ralph starts with `persistent: true`, picks up the first work item, processes it.
2. After completing the first item, Ralph emits LOOP_COMPLETE. Persistent mode keeps it alive.
3. Board scanner fires on `task.resume`, clears scratchpad + tasks, picks up the second item.
4. Scratchpad after item 2 contains only item-2 context (no item-1 pollution).
5. After processing all 3 items, the scanner finds no work and idles (publishes `board.rescan` on cooldown).
6. Ralph does not terminate during the entire run (no fallback exhaustion, no crashes).
7. Memories persist across all 3 items.

---

## Q11: What observability is needed during the spike?

**Q:** The spike needs to capture enough data to prove each success criterion and diagnose failures. What should be captured?

**A:** Full observability across the run. Capture the following for each work item transition:

**Per-iteration state snapshots:**
- Scratchpad content (`.ralph/agent/scratchpad.md`) — captured before and after each work item to prove self-clear works and no context pollution occurs.
- Task state (`ralph tools task list --format json`) — captured before, during, and after each work item.
- Events (`ralph events --format json`) — full event history showing the `board.scan` → `work.execute` → `LOOP_COMPLETE` → `task.resume` cycle.

**Continuous captures:**
- Run Ralph with `RALPH_DIAGNOSTICS=1` to get structured diagnostics in `.ralph/diagnostics/<timestamp>/` — includes agent output, orchestration decisions (hat selection, event publishing), performance metrics, and full traces.
- Run with `-v` (verbose) to stream tool results.
- Rotate log files in `.ralph/diagnostics/logs/`.

**Post-run artifacts to preserve:**
- Copy the full `.ralph/` directory to `specs/milestone-1.5-autonomous-ralph/artifacts/ralph-state/` after the run.
- Copy `done/` directory (completed work items) to `specs/milestone-1.5-autonomous-ralph/artifacts/results/`.
- Write a `specs/milestone-1.5-autonomous-ralph/artifacts/findings.md` summarizing observations and the validated ralph.yml pattern.

---

## Q12: Proposal A vs B — which approach?

**Q:** The research report ([continuous-agent-design.md](research/continuous-agent-design.md)) recommends Proposal A (outer loop with fresh `ralph run` per issue) over Proposal B (persistent mode). Proposal A has better error isolation, clean issue boundaries, cost isolation, and zero-cost idle. Proposal B has scratchpad bleed, the idle problem, and error propagation. Should we reconsider?

**A:** Stay with Proposal B (persistent mode). The end-game is a hybrid approach where Ralph processes a batch of issues in one persistent session, then dies, and is relaunched later on demand. Proposal B validates the persistent-mode mechanics that this hybrid model requires. Proposal A's strengths (error isolation, clean boundaries) are noted as things the board scanner's self-clear must handle explicitly.

Key tradeoffs accepted:
- **Idle problem:** `cooldown_delay_seconds` applies to ALL transitions, not just idle. For the spike this is acceptable. M2 will need a smarter solution (possibly RObot turning idle into a feature).
- **Orphaned events:** Events emitted before LOOP_COMPLETE in the same turn carry over. The board scanner must handle stale pending events alongside `task.resume`.
- **No per-issue cost isolation:** Accepted for the spike. M2 can add cost tracking per work item via memories/logging.

---

## Requirements Complete

All questions for the spike have been clarified. The spike validates Proposal B (persistent mode): `persistent: true` + self-clearing scratchpad/tasks + `task.resume` → board scanner. Deliverable is a minimal working prototype with 3 plain-text work items, full diagnostic capture, and preserved artifacts. Findings feed directly into M2's ralph.yml and hat design.

Ready for design.
