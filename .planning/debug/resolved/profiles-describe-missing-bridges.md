---
status: diagnosed
trigger: "bm profiles describe missing Bridges section despite profile having bridge declarations"
created: 2026-03-09T00:00:00Z
updated: 2026-03-09T00:00:00Z
---

## Current Focus

hypothesis: The profiles describe command handler never renders ProfileManifest.bridges
test: Read the describe command handler and check for bridges rendering
expecting: No bridges rendering code exists
next_action: Read profile.rs for BridgeDef and the describe command handler

## Symptoms

expected: `bm profiles describe scrum-compact` shows a Bridges section
actual: No Bridges section appears in output
errors: none
reproduction: Run `bm profiles describe scrum-compact`
started: Since Phase 09-01 added bridges to ProfileManifest

## Eliminated

## Evidence

- timestamp: 2026-03-09T00:01:00Z
  checked: crates/bm/src/commands/profiles.rs describe() function (lines 33-99)
  found: Function renders Roles (line 44), Labels (line 61), Coding Agents (line 66-83), and agent tags (line 85-96). No mention of bridges anywhere in the file.
  implication: Bridges rendering was never added when BridgeDef was introduced in Phase 09-01.

- timestamp: 2026-03-09T00:01:30Z
  checked: crates/bm/src/profile.rs ProfileManifest struct (lines 219-221) and BridgeDef struct (lines 225-232)
  found: ProfileManifest.bridges is Vec<BridgeDef> with fields name, display_name, description, bridge_type
  implication: The data model is complete; only the rendering in the describe command is missing.

- timestamp: 2026-03-09T00:02:00Z
  checked: profiles/scrum-compact/botminter.yml
  found: Has bridges section with one entry (telegram, display_name "Telegram", type external)
  implication: Profile data exists and would parse correctly; it's just never displayed.

## Resolution

root_cause: The `describe()` function in `crates/bm/src/commands/profiles.rs` was never updated to render `manifest.bridges`. It renders Roles, Labels, Coding Agents, and agent tags but has no block for Bridges.
fix:
verification:
files_changed: []
