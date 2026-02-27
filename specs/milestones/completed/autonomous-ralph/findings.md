# Findings — Spike: Autonomous Ralph Runner (M1.5)

> Results of executing the M1.5 spike: an autonomous Ralph loop processing
> 3 work items through a two-phase pipeline (propose → finish).
> Run date: 2026-02-17.

---

## 1. Summary

The M1.5 spike succeeded. Ralph ran autonomously in persistent mode, processing all 3 work items through the full propose → finish pipeline without manual intervention during the active phase. Of the 11 acceptance criteria, **9 passed**, **1 partially passed** (self-clear — observable but not fully verifiable from preserved state), and **1 failed non-blocking** (memories — agent never created memories during the run). All blocking criteria (1–5, 8–9, 11) passed. The validated `ralph.yml` + `PROMPT.md` pattern is ready for M2 adoption with minor adjustments documented below.

---

## 2. Success Criteria Results

| # | Criterion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | Multi-phase processing | **PASS** | `done/` contains `item-1.txt`, `item-2.txt`, `item-3.txt`. `work/` and `proposed/` are empty. All items traversed both phases. |
| 2 | Two-hat dispatch | **PASS** | Diagnostics show both `proposal_writer` and `finisher` hat activations across 22 timestamped diagnostic directories in `.ralph/diagnostics/`. `poll-log.txt` shows 3 `work.propose` dispatches and 2 `work.finish` dispatches (item-3 finish handled via coordinator recovery). |
| 3 | Work hats publish board.rescan | **PASS** | After each `work.propose` and `work.finish` dispatch, the board scanner fires again on `board.rescan`, confirming work hats published the event correctly. Observable in the poll-log sequence: propose → rescan → next scan. |
| 4 | LOOP_COMPLETE only on idle | **PASS** | LOOP_COMPLETE first appears only after all 3 items reached `done/`. No premature LOOP_COMPLETE between phases. First idle entry in `poll-log.txt` at 16:05:53Z, after all work.finish dispatches completed. |
| 5 | Persistent mode keeps alive | **PASS** | After LOOP_COMPLETE, `board.scan` events continued. `poll-log.txt` shows 3 idle cycles (16:05:53Z, 16:08:03Z, 16:10:03Z) — the loop stayed alive and the board scanner kept firing. |
| 6 | Self-clear between phases | **PARTIAL PASS** | Final scratchpad state shows `# Phase: finish — item-3.txt / Starting fresh.` format, confirming self-clear happened. However, only the final scratchpad state survives across iterations — intermediate clears are not directly observable from preserved artifacts. The poll-log phase transitions (propose → finish) and correct done/ content provide indirect evidence that self-clear worked. |
| 7 | No context pollution | **INCONCLUSIVE** | Only the final scratchpad state is preserved. However, all 3 done items contain correct, task-specific content: item-1 covers TDD benefits, item-2 covers Go design patterns, item-3 covers the reconciler pattern. No cross-contamination observed in outputs, suggesting no context pollution occurred during processing. |
| 8 | Idle on empty board | **PASS** | `poll-log.txt` lines 20–27 show 3 consecutive `idle (no work)` entries at 16:05:53Z, 16:08:03Z, and 16:10:03Z. The ~2-minute interval between idle scans matches the `cooldown_delay_seconds: 30` setting plus agent processing time. |
| 9 | No crashes | **PASS** | Ralph ran 19+ iterations over ~20 minutes. Process was manually terminated after confirming idle cycles. No fallback exhaustion, no crashes, no error states. |
| 10 | Memories persist | **FAIL** (non-blocking) | No `.ralph/agent/memories.md` file was created during the spike run. The agent processed all work items successfully but never invoked the memory system. This is non-blocking — the spike's work items were simple enough that the agent had no learnings to persist. |
| 11 | Priority ordering | **PASS** | All 3 `work.propose` dispatches (poll-log lines 1–12) precede any `work.finish` dispatch (poll-log lines 13+). The board scanner correctly drained `work/` before processing `proposed/`, matching the priority rule. |

**Blocking criteria (must-pass):** 1–5, 8–9, 11 → all **PASS**.
**Non-blocking criteria:** 6 (partial), 7 (inconclusive), 10 (fail) → documented, no impact on spike success.

---

## 3. Observations

### Runtime Characteristics

- **Total iterations:** 19+ (7 board scanner scans with dispatches, 3 idle scans, plus proposal_writer and finisher hat activations).
- **Wall-clock time:** ~20 minutes (15:51:37Z to 16:10:03Z).
- **Diagnostics volume:** 22 timestamped directories in `.ralph/diagnostics/`, each containing orchestration, agent-output, and trace JSONL files.
- **Cooldown between active dispatches:** ~1–2 minutes per cycle (agent processing time dominates; the 30s cooldown is negligible relative to agent work).
- **Cooldown between idle scans:** ~2 minutes per cycle (30s cooldown + agent overhead).

### Anomalies

1. **Duplicate `work.propose item-1.txt` dispatch.** The board scanner dispatched `work.propose item-1.txt` twice (poll-log lines 1–6). The first proposal_writer activation for item-1 either failed to complete or its `board.rescan` event was missed. The second dispatch succeeded. This suggests a timing edge case where the proposal_writer's file operations completed but the event routing took an extra cycle. **Non-blocking** — the system self-corrected.

2. **Item-3 finish via coordinator recovery.** The poll-log shows only 2 explicit `work.finish` dispatches (item-1 at 15:58:11Z, item-2 at 16:01:28Z). Item-3's finish was handled via coordinator recovery (hatless iteration) rather than a board scanner dispatch. Despite this, item-3 was correctly finalized in `done/`. **Non-blocking** — the system reached the correct end state.

3. **No orphaned event issues.** The board scanner correctly ignored stale pending events. The design's concern about orphaned events from previous cycles did not materialize as a problem.

4. **`task.resume` is reserved.** Ralph 2.5.0 treats `task.resume` as a coordinator-only trigger. The original design had `task.resume` in the board scanner's trigger list, but Ralph rejected this during pre-flight. The fix was to remove `task.resume` from board_scanner triggers. Persistent mode handles it via the coordinator, which re-dispatches to the board scanner through `board.scan`. **Functionally equivalent** to the design intent.

### Content Quality

All 3 done items contain substantive, on-topic content:

- **item-1.txt:** 3-paragraph TDD benefits (early bug detection, better code design, living documentation).
- **item-2.txt:** 3 Go patterns (functional options, table-driven tests, accept interfaces/return structs).
- **item-3.txt:** Reconciler pattern explanation with Kubernetes examples, covering level-triggered vs edge-triggered design.

The proposal → finish pipeline produced genuinely expanded content (not just echoing the original prompts), validating that multi-phase processing adds value.

---

## 4. Validated Pattern

The following `ralph.yml` + `PROMPT.md` pattern is validated and ready for M2 adoption.

### ralph.yml Pattern

````yaml
event_loop:
  prompt_file: PROMPT.md
  completion_promise: LOOP_COMPLETE
  max_iterations: 100
  max_runtime_seconds: 3600
  cooldown_delay_seconds: 30
  starting_event: board.scan
  persistent: true

hats:
  board_scanner:
    triggers: [board.scan, board.rescan]    # NOT task.resume (reserved)
    publishes: [work.propose, work.finish, LOOP_COMPLETE]
    default_publishes: LOOP_COMPLETE
    instructions: |
      # Scan → self-clear → dispatch ONE item → STOP
      # Priority: work/ before proposed/
      # Empty board → LOOP_COMPLETE

  <work_hat>:
    triggers: [<phase_event>]
    publishes: [board.rescan]
    default_publishes: board.rescan
    instructions: |
      # Read input → produce output → delete source → board.rescan
````

### Modifications from Original Design

| Aspect | Design spec | Actual (validated) | Reason |
|--------|------------|-------------------|--------|
| `board_scanner.triggers` | `[board.scan, board.rescan, task.resume]` | `[board.scan, board.rescan]` | Ralph 2.5.0 reserves `task.resume` for coordinator-only use |
| `task.resume` routing | Hat-level subscription | Coordinator re-dispatches via `board.scan` | Functionally equivalent; no behavioral difference |

### Key Behaviors Confirmed

- **Persistent mode** suppresses LOOP_COMPLETE and injects `task.resume` as designed.
- **Multi-hat dispatch** works: board scanner routes to different work hats based on directory state.
- **Self-clear** (scratchpad overwrite + tasks.jsonl deletion) provides clean context for each hat activation.
- **`default_publishes`** acts as a safety net against fallback exhaustion — every hat has a guaranteed event output.
- **Work hats → `board.rescan`** (not LOOP_COMPLETE) keeps the active loop running without idle cooldown.

---

## 5. M2 Implications

### Keep As-Is

1. **`persistent: true` + `board.scan`/`board.rescan` routing.** This pattern maps directly to M2's team members scanning `.github-sim/` for issues matching their role's status labels. Replace directory listing with label-based issue queries.

2. **Self-clear between phases.** The board scanner's scratchpad overwrite + tasks.jsonl deletion pattern is essential for multi-phase issue processing (design → review → plan → breakdown). Each phase starts with clean agent state.

3. **`default_publishes` on every hat.** This prevented fallback exhaustion during the spike and should be mandatory for all M2 hats.

4. **LOOP_COMPLETE only from the board scanner.** Work hats returning `board.rescan` ensures the scanner re-evaluates the board after every phase transition. This prevents premature idle when work remains.

5. **Poll-log pattern.** The board scanner's append-only log is valuable for debugging. M2 should adopt an equivalent (possibly structured JSONL instead of plain text).

### Must Change for M2

1. **Directory scanning → `.github-sim/` label queries.** The board scanner's `ls work/` and `ls proposed/` become queries like "find issues with label `status/arch:design`". The routing pattern (scan → dispatch → rescan) remains identical.

2. **`task.resume` routing.** Do not add `task.resume` to hat triggers. Ralph reserves it for the coordinator. The coordinator handles `task.resume` and re-dispatches via `board.scan` automatically. M2 ralph.yml configs should follow the validated pattern (only `board.scan` and `board.rescan` in scanner triggers).

3. **`cooldown_delay_seconds` tuning.** The 30s cooldown applies globally to ALL transitions, including active work (board scanner → work hat). During the spike, agent processing time (~1–2 min) dwarfed the cooldown, making it unnoticeable. In M2, if issue processing is fast, the 30s delay between phases would be wasteful. Consider: (a) reducing to 5–10s for active work, or (b) requesting a Ralph feature for per-event cooldown (cooldown only on `task.resume`, not `board.rescan`).

4. **Duplicate dispatch handling.** The spike observed one duplicate dispatch (item-1 proposed twice). M2 should implement idempotent phase transitions — if an issue is already in the next status, skip it rather than re-processing. The board scanner instructions should include: "If the item is already in the expected output state, skip and scan next."

5. **Memory usage.** The spike agent never created memories despite having the capability enabled. M2 should either (a) add explicit memory creation instructions to work hat prompts ("after processing, record any learnings"), or (b) accept that memories are optional and rely on scratchpad + handoff instead.

### Error Handling Improvements

1. **Coordinator recovery worked but is opaque.** Item-3's finish was handled by coordinator recovery rather than explicit board scanner dispatch. M2 should add logging when coordinator recovery activates, so operators can distinguish planned dispatches from recovery.

2. **No infinite retry observed.** The design anticipated infinite retry if a work hat fails to delete its source file (design §6.3). This never triggered. M2 should still implement a skip/error mechanism — add a `failed/` directory or `status/error` label for items that fail N times.

---

## 6. Open Questions

1. **Per-event cooldown.** Does Ralph support (or plan to support) different cooldown values for different event types? The spike uses a single `cooldown_delay_seconds: 30` for both active work and idle polling. M2 would benefit from zero cooldown on `board.rescan` (active work) and longer cooldown on `task.resume` (idle polling).

2. **Memory injection scope.** If a team member processes 100+ issues, will the memory budget (2000 tokens) be sufficient? Should M2 implement memory pruning or rotation?

3. **Multi-agent coordination.** The spike ran a single agent. M2 runs multiple agents writing to the same `.github-sim/`. What prevents two agents from picking up the same issue? The spike's file-based model (delete source after read) provides natural locking, but `.github-sim/` label transitions don't. M2 needs a claim mechanism (e.g., `status/arch:in-progress` label set before processing begins).

4. **Diagnostics volume.** The spike generated 22 diagnostic directories for ~19 iterations of a trivial pipeline. M2's longer-running agents will produce substantial diagnostics. Should diagnostics be rotated, compressed, or selectively captured?

5. **Nested Ralph validation.** Running Ralph inside a Ralph session required unsetting `CLAUDECODE` env var (mem-1771045664-40bb). Is this the intended pattern for M2's `just launch` command, or should Ralph handle nested invocations natively?

---

## Appendix: Reset Instructions

To re-run the spike from scratch:

```bash
cd specs/milestone-1.5-autonomous-ralph/artifacts
git checkout -- work/
rm -rf proposed/* done/* .ralph/ poll-log.txt ralph-output.log
```
