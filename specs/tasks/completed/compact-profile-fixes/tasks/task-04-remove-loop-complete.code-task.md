---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Remove all LOOP_COMPLETE usage from all profiles

## Description
Both the compact and rh-scrum profiles use `LOOP_COMPLETE` as `completion_promise`, `default_publishes`, and in hat instruction text. `LOOP_COMPLETE` must never appear as `default_publishes` on any hat or as `completion_promise` in the event loop config. All occurrences must be removed across both profiles — Ralph's persistent loop restarts from `starting_event` automatically.

## Background
`LOOP_COMPLETE` is a Ralph event that terminates the orchestration loop. In a persistent poll-based loop (`persistent: true`), the loop should never terminate — it should keep cycling back to the board scanner. Having `default_publishes: LOOP_COMPLETE` on every hat means that any hat that doesn't explicitly publish an event will accidentally terminate the entire loop. Similarly, `completion_promise: LOOP_COMPLETE` tells Ralph that publishing this event means "we're done" — wrong for a persistent agent.

The fix is to remove these entirely. When a hat completes without publishing an explicit event, Ralph's persistent loop will automatically restart from `starting_event: board.scan`.

## Reference Documentation
**Required:**

Compact profile:
- `profiles/compact/members/superman/ralph.yml` — 37 LOOP_COMPLETE occurrences
- `profiles/compact/members/superman/PROMPT.md` — 3 LOOP_COMPLETE references

rh-scrum profile:
- `profiles/rh-scrum/members/architect/ralph.yml` — lines 3, 29, 30, 71, 96
- `profiles/rh-scrum/members/human-assistant/ralph.yml` — lines 3, 22, 23, 61, 81
- `profiles/rh-scrum/members/human-assistant/PROMPT.md` — line 34

## Technical Requirements

### Compact profile (`profiles/compact/members/superman/`)
1. Remove `completion_promise: LOOP_COMPLETE` from the `event_loop` config block
2. Remove every `default_publishes: LOOP_COMPLETE` line from all 15 hats
3. Remove `LOOP_COMPLETE` from the board_scanner's `publishes` list
4. Update all hat instruction text that says "Publish `LOOP_COMPLETE`" — replace with "Return control to the orchestrator" or equivalent
5. Update board_scanner instruction "No work found → publish `LOOP_COMPLETE`" — replace with "No work found → return control (the persistent loop will re-scan automatically)"
6. Update board_scanner rules section that references LOOP_COMPLETE
7. Update `PROMPT.md` references (~3 occurrences)

### rh-scrum profile
8. `profiles/rh-scrum/members/architect/ralph.yml` — remove `completion_promise: LOOP_COMPLETE` (line 3), `default_publishes: LOOP_COMPLETE` (line 30), `- LOOP_COMPLETE` from publishes (line 29), and instruction references (lines 71, 96)
9. `profiles/rh-scrum/members/human-assistant/ralph.yml` — remove `completion_promise: LOOP_COMPLETE` (line 3), `default_publishes: LOOP_COMPLETE` (line 23), `- LOOP_COMPLETE` from publishes (line 22), and instruction references (lines 61, 81)
10. `profiles/rh-scrum/members/human-assistant/PROMPT.md` — remove LOOP_COMPLETE reference (line 34)

## Dependencies
- None — standalone fix (profile content only)

## Implementation Approach
1. For each profile and each member's `ralph.yml`:
   - Remove `completion_promise: LOOP_COMPLETE` from event_loop block
   - Remove all `default_publishes: LOOP_COMPLETE` lines
   - Remove `- LOOP_COMPLETE` from any hat's publishes list
   - In each hat's instructions section, replace "Publish `LOOP_COMPLETE`" with context-appropriate wording (e.g., "Return control to the orchestrator.")
   - Replace "No work found → publish `LOOP_COMPLETE`" with "No work found → return control (persistent loop re-scans automatically)"
2. Update all PROMPT.md files that reference LOOP_COMPLETE
3. Verify no references remain: `grep -r LOOP_COMPLETE profiles/`

## Acceptance Criteria

1. **No LOOP_COMPLETE in any profile's YAML config keys**
   - Given any `ralph.yml` under `profiles/`
   - When searching for `completion_promise` and `default_publishes` values
   - Then neither contains `LOOP_COMPLETE`

2. **No LOOP_COMPLETE in any publishes lists**
   - Given any `ralph.yml` under `profiles/`
   - When searching for `LOOP_COMPLETE` in any hat's `publishes` array
   - Then zero matches are found

3. **No LOOP_COMPLETE in any instruction text**
   - Given any `ralph.yml` under `profiles/`
   - When searching instruction text for `LOOP_COMPLETE`
   - Then zero matches are found

4. **No LOOP_COMPLETE in any PROMPT.md**
   - Given any `PROMPT.md` under `profiles/`
   - When searching for `LOOP_COMPLETE`
   - Then zero matches are found

5. **Zero total occurrences across all profiles**
   - Given `profiles/`
   - When running `grep -r LOOP_COMPLETE profiles/`
   - Then zero matches are found

6. **YAML remains valid**
   - Given all modified `ralph.yml` files
   - When parsed as YAML
   - Then they parse without errors and all hat definitions are structurally intact

7. **Hat instructions still convey correct behavior**
   - Given each hat's on-failure block
   - When reading the instructions
   - Then it's clear the hat should stop processing and let the orchestrator handle the next cycle

## Metadata
- **Complexity**: Medium
- **Labels**: bugfix, profile, compact, rh-scrum, ralph
- **Required Skills**: YAML, markdown
