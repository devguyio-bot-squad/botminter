---
status: resolved
phase: 09-profile-integration-cleanup
source: [09-01-SUMMARY.md, 09-02-SUMMARY.md, 09-03-SUMMARY.md, 09-04-SUMMARY.md]
started: 2026-03-09T00:00:00Z
updated: 2026-03-09T06:30:00Z
---

## Current Test

[testing complete]

## Tests

### 1. CLI Sync Flags Redesigned
expected: Run `bm teams sync --help`. Output shows `--repos`, `--bridge`, and `--all` (or `-a`) as available flags. The old `--push` flag does NOT appear. Running `bm teams sync --push` produces an unrecognized flag error.
result: pass

### 2. Init Bridge Flag (Non-Interactive)
expected: Run `bm init --help`. Output shows `--bridge` as an available flag. The flag description mentions selecting a bridge from the profile's supported bridges.
result: pass

### 3. Invalid Bridge Name Rejected
expected: Run `bm init --non-interactive --profile <tab>` auto-completes profile names. `bm init --non-interactive --bridge <tab>` auto-completes bridge names.
result: pass (re-test after 09-05 gap closure)

### 4. Profile Bridge Declarations Visible
expected: Run `bm profiles describe scrum-compact`. Output includes a bridges section showing "telegram" as an available bridge with display name and description.
result: pass (re-test after 09-05 gap closure)

### 5. scrum-compact-telegram Profile Removed
expected: Run `bm profiles list`. Output shows only `scrum` and `scrum-compact` profiles. Table formatting respects terminal width.
result: pass (re-test after 09-05 gap closure)

### 6. Sync Bridge Provisions Identities
expected: After init with `--bridge telegram` and hiring a member, running `bm teams sync --bridge` provisions bridge identities for each hired member. The bridge state file tracks provisioned identities. Console output mentions bridge provisioning activity.
result: pass (re-test after 09-06 gap closure)

### 7. RObot.enabled Injected After Sync
expected: After sync with a bridge configured and member credentials available, the member's surfaced ralph.yml in the workspace contains `RObot.enabled: true` (or the equivalent YAML path). Without credentials, RObot.enabled is false or absent.
result: skipped
reason: Bridge provisioning works but no credentials were added (external bridge requires bm bridge identity add)

### 8. Bridge Documentation Pages Exist
expected: Files `docs/content/concepts/bridges.md` and `docs/content/how-to/bridge-setup.md` exist. The concepts page covers bridge types (local vs external), per-member identity model, and credential flow. The setup guide covers the init->hire->sync->start journey.
result: pass

### 9. CLI Reference Documents Bridge Commands
expected: `docs/content/reference/cli.md` contains a `bm bridge` section documenting subcommands: `start`, `stop`, `status`, `identity add/rotate/remove/list`, `room create/list`. The `bm init` section mentions the `--bridge` flag.
result: pass

### 10. Test Suite Passes
expected: `cargo test -p bm` passes all 573+ tests with no failures. `cargo clippy -p bm -- -D warnings` produces no warnings.
result: pass

## Summary

total: 10
passed: 9
issues: 0
pending: 0
skipped: 1

## Residual Issues

- truth: "bm teams show displays bridge configuration and table respects terminal width"
  status: resolved
  reason: "User reported: the team show doesn't show the bridge nor the tables respect the width"
  severity: minor
  test: n/a (discovered during re-test)
  resolution: "Bridge display added to teams show (e7c1dd1). Tables already had DynamicFullWidth from 09-05. E2e bridge lifecycle test verifies Bridge: line in output."

## Gaps

- truth: "bm init --non-interactive --profile <tab> auto-completes profile names"
  status: resolved
  reason: "User reported: the dynamic auto completion isn't working. pressing tab after bm init --non-interactive --profile <tab> isn't auto completing"
  severity: minor
  test: 3
  root_cause: "build_cli_with_completions() in completions.rs has mut_subcommand blocks for 13 subcommands but NOT for init. CompletionContext::profile_names() data source exists but is not connected. Missing feature, not a regression."
  artifacts:
    - path: "crates/bm/src/completions.rs"
      issue: "Missing .mut_subcommand('init', ...) block (lines 106-207)"
    - path: "crates/bm/src/cli.rs"
      issue: "Command::Init --profile and --bridge have no value hints (lines 14-50)"
  missing:
    - "Add mut_subcommand('init') block wiring --profile to profile_names() candidates"
    - "Add --bridge completion (requires bridge-name lister or profile-derived values)"
    - "Strengthen guard test to verify args with dynamic values have candidates"
  debug_session: ".planning/debug/tab-completion-init-profile.md"

- truth: "bm profiles describe shows bridge declarations from profile"
  status: resolved
  reason: "User reported: didn't work. Profile describe output shows Roles, Labels, and Coding Agents but no Bridges section. Telegram bridge declaration not surfaced."
  severity: major
  test: 4
  root_cause: "describe() function in profiles.rs renders four sections (Roles, Labels, Coding Agents, Agent Tags) but has zero references to bridges or BridgeDef. ProfileManifest.bridges is parsed correctly but never displayed."
  artifacts:
    - path: "crates/bm/src/commands/profiles.rs"
      issue: "describe() function (lines 33-99) missing bridges rendering block"
    - path: "crates/bm/src/profile.rs"
      issue: "ProfileManifest.bridges and BridgeDef correctly defined (lines 219-232), just unused by describe"
  missing:
    - "Add if !manifest.bridges.is_empty() block after Coding Agents section (line 83) displaying name, display_name, description, bridge_type"
  debug_session: ".planning/debug/profiles-describe-missing-bridges.md"

- truth: "bm profiles list output is properly formatted in a table"
  status: resolved
  reason: "User reported: pass but the output is not wrapped in the table and the table is broken"
  severity: minor
  test: 5
  root_cause: "comfy_table ContentArrangement defaults to Disabled. No table-using command in the codebase sets ContentArrangement or respects terminal width. Description column alone is ~140 chars, pushing total width to ~170+ chars. Systemic across all 7 table commands."
  artifacts:
    - path: "crates/bm/src/commands/profiles.rs"
      issue: "Table missing set_content_arrangement(DynamicFullWidth) (lines 11-15)"
  missing:
    - "Add .set_content_arrangement(ContentArrangement::DynamicFullWidth) to all 7 table constructions"
  debug_session: ".planning/debug/profiles-list-table-formatting.md"

- truth: "bm teams sync --bridge provisions bridge identities for hired members"
  status: resolved
  reason: "User reported: bm teams sync --bridge --repos says 'No bridge configured -- skipping bridge provisioning'. Also tried to create workspace repo that already exists (regression). Error message still references old --push flag."
  severity: major
  test: 6
  root_cause: "Three sub-issues: (1) ProfileManifest struct has bridges (plural, profile list) but NOT bridge (singular, selected bridge). Any code that round-trips botminter.yml through ProfileManifest silently drops the bridge key — e.g., bm projects add. (2) create_workspace_repo unconditionally calls gh repo create without checking if the GitHub repo already exists. (3) Two stale --push references remain in workspace.rs error messages."
  artifacts:
    - path: "crates/bm/src/profile.rs"
      issue: "ProfileManifest missing bridge: Option<String> field (lines 198-222) — round-trip drops selected bridge"
    - path: "crates/bm/src/workspace.rs"
      issue: "create_workspace_repo has no pre-existence check before gh repo create (line 68-86)"
  missing:
    - "Add bridge: Option<String> with serde(default, skip_serializing_if) to ProfileManifest"
    - "Add gh repo view check before gh repo create in create_workspace_repo"
    - "Replace --push with --repos in workspace.rs lines 81 and 778"
  debug_session: ".planning/debug/uat6-sync-bridge-issues.md"
