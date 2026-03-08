# Plan: Replace Board Scanner Hat with Auto-Inject Skill

## Context

The board_scanner hat and the hatless Ralph coordinator are functionally redundant — both PLAN and DELEGATE. This burns an extra LLM iteration every scan cycle (Ralph → board.scan → board_scanner → dispatch). By moving the board scanning logic into an auto-inject Ralph skill, the hatless Ralph becomes the board scanner directly, eliminating one iteration per cycle and dissolving the closed-loop routing problem entirely.

## Approach

1. **Create a Ralph auto-inject skill** at `.botminter/agent/skills/board-scanner/SKILL.md` containing the board scanning procedure (currently the board_scanner hat's `instructions` block in ralph.yml)
2. **Remove the board_scanner hat** from ralph.yml
3. **Configure the skill as auto-inject** via `skills.overrides` in ralph.yml
4. **Remove `starting_event: board.scan`** from ralph.yml (no hat subscribes to it anymore)
5. **Update terminal hats** — they no longer need to emit `board.rescan`; just returning control is fine since the hatless Ralph takes over automatically via fallback

## Changes

### New file: `.botminter/agent/skills/board-scanner/SKILL.md`

A Ralph skill following the SKILL.md format (YAML frontmatter + markdown body). Content adapted from the current board_scanner hat instructions in ralph.yml (lines 37-150). The skill teaches the hatless Ralph coordinator how to:
- Scan the GitHub Projects v2 board
- Cache project field IDs
- Handle auto-advance transitions
- Dispatch based on priority table
- Log to poll-log.txt
- Handle error escalation

### Modify: `ralph.yml`

1. **Remove** the entire `board_scanner` hat definition (lines 16-150)
2. **Remove** `starting_event: board.scan` (line 5)
3. **Add** skill override:
   ```yaml
   skills:
     overrides:
       board-scanner:
         auto_inject: true
   ```
4. **Remove** `board.rescan` from any hat that references it (none currently do — this is preemptive)
5. **Update** other hats that reference "board scanner" in comments (lead_reviewer line 403)

### No changes needed for terminal hats

Terminal hats (po_backlog, po_reviewer, arch_monitor, sre_setup) can continue returning control without emitting events. The fallback mechanism injects `task.resume` → hatless Ralph → scans board → dispatches next work. This is now the intended flow, not a recovery path.

### No changes needed for unmatched events

Unmatched events (lead.approved, lead.rejected, qe.approved, cw.approved) route to the hatless Ralph catch-all. Ralph sees the board-scanner skill, scans the board, and dispatches based on the new project statuses (which the hats already set before publishing).

## Verification

1. The skill file loads correctly: check `ralph tools skill list` shows `board-scanner`
2. Ralph coordinator mode includes the skill content in its prompt
3. Ralph scans the board and dispatches to the correct hat on the first iteration
4. Terminal hats returning control triggers Ralph re-scan via fallback
5. Idle cycle: Ralph emits LOOP_COMPLETE → persistent mode restarts
