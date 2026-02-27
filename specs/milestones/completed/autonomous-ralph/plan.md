# Implementation Plan — Spike: Autonomous Ralph Runner

> Incremental implementation steps for the M1.5 spike.
> Inputs: [design.md](design.md), [requirements.md](requirements.md).

---

## Checklist

- [ ] Step 1: Scaffold artifacts directory and work items
- [ ] Step 2: Write PROMPT.md
- [ ] Step 3: Write ralph.yml
- [ ] Step 4: Validate configuration (pre-flight)
- [ ] Step 5: Execute the spike
- [ ] Step 6: Verify success criteria
- [ ] Step 7: Document findings
- [ ] Step 8: Preserve artifacts and close out

---

## Step 1: Scaffold artifacts directory and work items

**Objective:** Create the on-disk directory structure the spike operates in, with the 3 seed work items.

**Implementation guidance:**

Create the directory tree under `specs/milestone-1.5-autonomous-ralph/artifacts/`:

```
artifacts/
├── work/
│   ├── item-1.txt
│   ├── item-2.txt
│   └── item-3.txt
├── proposed/          # empty at start
└── done/              # empty at start
```

Work item contents (verbatim from design §4.3):

- `work/item-1.txt` — `Summarize the benefits of test-driven development in 3 bullet points.`
- `work/item-2.txt` — `List 3 common design patterns used in Go projects and briefly describe each.`
- `work/item-3.txt` — `Explain what a reconciler pattern is in Kubernetes controllers in 2-3 sentences.`

Add `.gitkeep` files to `proposed/` and `done/` so empty directories are tracked by git.

**Test requirements:**
- `ls -R specs/milestone-1.5-autonomous-ralph/artifacts/` shows the expected tree.
- Each work item file contains exactly the specified text.
- `proposed/` and `done/` exist and are empty (except `.gitkeep`).

**Integration notes:** This is the foundation. All subsequent steps place files into this directory.

**Demo:** Run `tree specs/milestone-1.5-autonomous-ralph/artifacts/` — shows the clean starting state.

---

## Step 2: Write PROMPT.md

**Objective:** Create the reusable role identity document that Ralph injects into every agent iteration.

**Implementation guidance:**

Write `specs/milestone-1.5-autonomous-ralph/artifacts/PROMPT.md` with the content from design §4.2. The prompt defines:

- Role: autonomous worker agent in a persistent loop
- Workspace layout: `work/`, `proposed/`, `done/`, `poll-log.txt`
- Item lifecycle: work → proposed → done
- State management: scratchpad + tasks cleared between phases, memories persist
- Constraints: one item per hat, only board scanner publishes LOOP_COMPLETE

Keep it exactly as specified in the design — this is a reference artifact for M2.

**Test requirements:**
- File exists and matches design §4.2 content.
- No implementation-specific paths or hardcoded values that would break portability.

**Integration notes:** Referenced by `ralph.yml` via `prompt_file: PROMPT.md`. Must exist before ralph runs.

**Demo:** `cat specs/milestone-1.5-autonomous-ralph/artifacts/PROMPT.md` — review the prompt for correctness.

---

## Step 3: Write ralph.yml

**Objective:** Create the event loop configuration with all three hats (board_scanner, proposal_writer, finisher).

**Implementation guidance:**

Write `specs/milestone-1.5-autonomous-ralph/artifacts/ralph.yml` with the content from design §4.1. Key settings:

- `persistent: true` — suppresses LOOP_COMPLETE, injects `task.resume`
- `starting_event: board.scan` — first event fires the board scanner
- `max_iterations: 100` — safety cap
- `max_runtime_seconds: 3600` — 1 hour safety cap
- Three hats with their triggers, publishes, default_publishes, and instructions (verbatim from design)

The hat instructions contain the critical behavioral logic:
- **board_scanner:** scan priority (work/ before proposed/), self-clear sequence, poll-log.txt logging, orphaned event handling
- **proposal_writer:** read from work/, write to proposed/, delete source, publish board.rescan
- **finisher:** read from proposed/, write to done/, delete source, publish board.rescan

**Test requirements:**
- YAML is syntactically valid (`python3 -c "import yaml; yaml.safe_load(open('ralph.yml'))"` or equivalent).
- All hat triggers and publishes form a complete event graph with no dead ends.
- `default_publishes` is set for every hat (safety net against fallback exhaustion).
- Cross-check: every event that any hat publishes is either a trigger for another hat or is `LOOP_COMPLETE`.

**Integration notes:** This is the core deliverable of the spike. The ralph.yml pattern — persistent mode, multi-hat dispatch, self-clear instructions, directory-based routing — is what M2 adopts.

**Demo:** Review the ralph.yml and trace the event flow manually: `board.scan` → board_scanner → `work.propose` → proposal_writer → `board.rescan` → board_scanner → `work.finish` → finisher → `board.rescan` → board_scanner → `LOOP_COMPLETE` → `task.resume` → board_scanner (idle loop).

---

## Step 4: Validate configuration (pre-flight)

**Objective:** Verify the configuration is correct before running.

**Implementation guidance:**

Run pre-flight checks from within the artifacts directory:

1. **YAML validation:** Parse `ralph.yml` and confirm it loads without errors.
2. **Event graph audit:** Manually (or via script) trace every published event to a hat trigger. Verify:
   - `board.scan` → board_scanner ✓
   - `board.rescan` → board_scanner ✓
   - `task.resume` → board_scanner ✓
   - `work.propose` → proposal_writer ✓
   - `work.finish` → finisher ✓
   - `LOOP_COMPLETE` → (persistent mode handles) ✓
3. **Path check:** Confirm `PROMPT.md` exists at the path referenced by `prompt_file`.
4. **Ralph dry-run (if available):** Run `ralph validate` or `ralph run --dry-run` if Ralph supports it. If not, skip — the YAML parse is sufficient.
5. **Work item sanity:** Confirm the 3 work items are readable and non-empty.

**Test requirements:**
- All pre-flight checks pass.
- Event graph is fully connected — no published event lacks a subscriber (except LOOP_COMPLETE which is handled by persistent mode).

**Integration notes:** This step catches config errors before the expensive run. If ralph has no `--dry-run` mode, rely on manual review + YAML parsing.

**Demo:** Output of each check. Green/pass for all items.

---

## Step 5: Execute the spike

**Objective:** Run Ralph in persistent mode and observe the full autonomous lifecycle: 3 items × 2 phases + idle.

**Implementation guidance:**

Launch from the artifacts directory using the command from design §5.1:

```bash
cd specs/milestone-1.5-autonomous-ralph/artifacts
RALPH_DIAGNOSTICS=1 ralph run -v --no-tui -P PROMPT.md 2>&1 | tee ralph-output.log
```

**Expected lifecycle (13 hat activations: 7 board_scanner + 3 proposal_writer + 3 finisher, then idle):**

| Cycle | Board scanner sees | Dispatches | Hat fires |
|-------|-------------------|------------|-----------|
| 1 | work/{item-1,item-2,item-3}.txt | work.propose item-1 | proposal_writer |
| 2 | work/{item-2,item-3}.txt, proposed/item-1.txt | work.propose item-2 (work/ priority) | proposal_writer |
| 3 | work/item-3.txt, proposed/{item-1,item-2}.txt | work.propose item-3 (work/ priority) | proposal_writer |
| 4 | proposed/{item-1,item-2,item-3}.txt | work.finish item-1 | finisher |
| 5 | proposed/{item-2,item-3}.txt | work.finish item-2 | finisher |
| 6 | proposed/item-3.txt | work.finish item-3 | finisher |
| 7 | (empty) | LOOP_COMPLETE | (idle — persistent mode → task.resume) |
| 8+ | (empty) | LOOP_COMPLETE | (continues idling) |

**Note:** The exact ordering depends on whether the board scanner finds items between phases or whether all proposals are written before any finishing begins. The priority rule (work/ before proposed/) means the scanner drains work/ first. The table above shows the most likely sequence.

**Monitoring during the run:**
- Watch `poll-log.txt` grow in another terminal: `tail -f poll-log.txt`
- Watch file movements: `watch -n 2 'echo "work:"; ls work/; echo "proposed:"; ls proposed/; echo "done:"; ls done/'`

**Termination:** After observing at least one idle cycle (LOOP_COMPLETE → task.resume → board_scanner → LOOP_COMPLETE), stop Ralph manually (Ctrl-C or wait for `max_iterations`/`max_runtime_seconds`).

**Test requirements:**
- Ralph starts without errors.
- All 3 items progress through both phases (work → proposed → done).
- Ralph does not crash or fallback-exhaust during the run.

**Integration notes:** This is the moment of truth. If it fails, diagnose using the diagnostics and iterate on ralph.yml/PROMPT.md. Common failure modes: hat not matching (check triggers), self-clear not happening (check board_scanner instructions), infinite retry (check file deletion in work hats).

**Demo:** Live `tail -f poll-log.txt` showing scan/dispatch/idle cycles. `ls done/` showing 3 completed items.

---

## Step 6: Verify success criteria

**Objective:** Systematically verify each of the 11 acceptance criteria (design §5.5 + priority ordering).

**Implementation guidance:**

Run verification checks after the spike run completes (or is manually stopped). Use the verification table from design §5.5:

| # | Criterion | Verification command / check |
|---|-----------|------------------------------|
| 1 | Multi-phase processing | `ls done/` shows 3 files; `ls work/ proposed/` shows empty |
| 2 | Two-hat dispatch | Grep `.ralph/diagnostics/*/orchestration.jsonl` for `proposal_writer` and `finisher` activations |
| 3 | Work hats publish board.rescan | Grep `.ralph/events-*.jsonl` for `board.rescan` after each `work.propose` and `work.finish` |
| 4 | LOOP_COMPLETE only on idle | Grep `.ralph/events-*.jsonl` — LOOP_COMPLETE appears only after all items processed |
| 5 | Persistent mode keeps alive | `.ralph/events-*.jsonl` shows `task.resume` after LOOP_COMPLETE |
| 6 | Self-clear between phases | Grep `.ralph/diagnostics/*/agent-output.jsonl` for scratchpad overwrites before each hat dispatch |
| 7 | No context pollution | Grep `.ralph/diagnostics/*/agent-output.jsonl` — scratchpad at finisher start has no proposal_writer content from a different item |
| 8 | Idle on empty board | `poll-log.txt` shows "idle (no work)" entries |
| 9 | No crashes | Ralph process exited cleanly (manual stop or max_iterations) |
| 10 | Memories persist | `.ralph/agent/memories.md` contains entries spanning multiple items/phases |
| 11 | Priority ordering | Grep `.ralph/events-*.jsonl` — all `work.propose` events precede any `work.finish` event (work/ drained before proposed/) |

Record pass/fail for each criterion. If any criterion fails, note the failure mode and whether it's blocking or acceptable for the spike.

**Test requirements:**
- All 11 criteria evaluated.
- Criteria 1–5, 8–9, and 11 must pass for the spike to be considered successful.
- Criteria 6–7 (self-clear, no pollution) and 10 (memories) are important but failure is non-blocking — document the observation and note implications for M2.

**Integration notes:** The pass/fail results feed directly into findings.md (Step 7).

**Demo:** A table of 11 criteria with pass/fail status and evidence.

---

## Step 7: Document findings

**Objective:** Write `findings.md` summarizing what worked, what didn't, and implications for M2.

**Implementation guidance:**

Create `specs/milestone-1.5-autonomous-ralph/findings.md` with these sections:

1. **Summary** — One-paragraph result: did the spike succeed? How many criteria passed?
2. **Success criteria results** — The pass/fail table from Step 6 with evidence and links to diagnostic files.
3. **Observations** — Notable behaviors observed during the run:
   - Did orphaned events cause any issues?
   - How many iterations did the full lifecycle take?
   - Did any work hat fail to read/write, triggering the infinite retry path (design §6.3)?
   - Any unexpected behaviors or edge cases.
4. **Validated pattern** — The ralph.yml + PROMPT.md pattern that works, ready for M2 adoption. Note any modifications made during the spike (if ralph.yml was iterated).
5. **M2 implications** — Concrete recommendations:
   - What to keep as-is from the spike pattern.
   - What needs to change for `.github-sim/` status labels (vs. directory scanning).
   - Error handling improvements needed.
6. **Open questions** — Anything the spike couldn't answer that M2 needs to address.

**Test requirements:**
- All sections are present and substantive (not placeholder text).
- Success criteria table matches Step 6 results.
- M2 implications are actionable, not vague.

**Integration notes:** This is the primary output of the spike for M2 consumption. The design.md appendix B ("How This Feeds Into M2") should be cross-referenced — findings.md confirms or revises those assumptions.

**Demo:** Review findings.md — a complete narrative of the spike from setup through results.

---

## Step 8: Preserve artifacts and close out

**Objective:** Commit all spike artifacts and ensure the spike directory is self-contained for future reference.

**Implementation guidance:**

1. **Verify artifact completeness.** After a successful run the artifacts directory should contain:
   - Source files (unchanged): `ralph.yml`, `PROMPT.md`
   - Pipeline result: `work/` (empty — items consumed by proposal_writer), `proposed/` (empty — items consumed by finisher), `done/item-{1,2,3}.txt`
   - Logs: `poll-log.txt`, `ralph-output.log`
   - Ralph state: `.ralph/` directory (diagnostics, events, agent state)
   - The original work items (`work/item-{1,2,3}.txt`) are preserved in git history and restored via `git checkout -- work/` when re-running the spike.

2. **Add reset instructions.** Ensure the design §5.4 reset command is documented in findings.md so anyone can re-run the spike:
   ```bash
   cd specs/milestone-1.5-autonomous-ralph/artifacts
   git checkout -- work/
   rm -rf proposed/* done/* .ralph/ poll-log.txt ralph-output.log
   ```

3. **Commit.** Stage and commit all milestone-1.5 files — both the `artifacts/` directory and `findings.md`:
   ```
   feat(specs): complete M1.5 autonomous ralph runner spike
   ```

**Test requirements:**
- All files (source, runtime output, full `.ralph/` diagnostics) are committed.
- Reset instructions work (dry-run: verify the commands are correct).

**Integration notes:** After this step, M1.5 is complete. The spike findings and validated ralph.yml pattern are available for M2 design work.

**Demo:** `git log --oneline -1` shows the commit. `tree specs/milestone-1.5-autonomous-ralph/` shows the complete milestone directory.
