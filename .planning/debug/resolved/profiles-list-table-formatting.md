---
status: diagnosed
trigger: "bm profiles list table formatting broken"
created: 2026-03-09T00:00:00Z
updated: 2026-03-09T00:00:00Z
---

## Current Focus

hypothesis: Table lacks terminal-width-aware content arrangement, causing it to overflow narrow terminals
test: Run with COLUMNS=80 and observe output width
expecting: Table should wrap or truncate to fit terminal; instead renders at full ~170 char width
next_action: Return diagnosis

## Symptoms

expected: Table output formatted to fit terminal width
actual: Table renders at full content width (~170+ chars), overflows on standard 80-column terminals
errors: No errors — visual formatting issue only
reproduction: Run `bm profiles list` in any terminal narrower than ~170 columns
started: Present since implementation — no width management was ever configured

## Eliminated

(none needed — root cause identified on first hypothesis)

## Evidence

- timestamp: 2026-03-09T00:00:00Z
  checked: profiles.rs list() function (crates/bm/src/commands/profiles.rs)
  found: Table created with UTF8_FULL_CONDENSED preset and UTF8_ROUND_CORNERS modifier, but no ContentArrangement or width constraint is set
  implication: comfy_table defaults to ContentArrangement::Disabled, meaning columns expand to fit content without wrapping

- timestamp: 2026-03-09T00:00:00Z
  checked: All other table-rendering commands (teams, bridge, status, members, projects, roles)
  found: None of the 7 table-using commands set ContentArrangement or width constraints
  implication: Systemic issue across all commands, but most visible in profiles list due to long Description strings

- timestamp: 2026-03-09T00:00:00Z
  checked: Actual output with COLUMNS=80
  found: Table renders at ~170 chars wide regardless of terminal width
  implication: Confirms comfy_table is not detecting or respecting terminal width

## Resolution

root_cause: comfy_table's ContentArrangement defaults to Disabled. Without calling `table.set_content_arrangement(ContentArrangement::DynamicFullWidth)`, the table never wraps content to fit the terminal. The Description column (~140 chars) forces the entire table well beyond standard terminal widths.
fix: (not applied — diagnosis only)
verification: (not applied)
files_changed: []
