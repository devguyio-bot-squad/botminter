---
status: diagnosed
trigger: "UAT Test 6: bm teams sync --bridge says 'No bridge configured'"
created: 2026-03-09T00:00:00Z
updated: 2026-03-09T00:00:00Z
---

## Current Focus

hypothesis: Three sub-issues in bm teams sync --bridge flow
test: Code review and existing test execution
expecting: Root causes identified for all three
next_action: Return diagnosis

## Symptoms

expected: bm teams sync --bridge --repos should provision bridge identities and create workspace repos
actual: "No bridge configured -- skipping bridge provisioning" + repo creation fails for existing repos + error message references removed --push flag
errors: "No bridge configured -- skipping bridge provisioning" and "HTTP 502: 502 Bad Gateway" and stale "--push" in error message
reproduction: bm init (with bridge Telegram) -> bm hire superman -> bm teams sync --bridge --repos
started: Phase 9 (bridge wiring)

## Eliminated

- hypothesis: record_bridge_in_manifest writes wrong key name
  evidence: Test init_bridge_records_in_manifest passes; function uses serde_yml::Value::String("bridge") which matches discover()'s value.get("bridge")
  timestamp: 2026-03-09

- hypothesis: bridge directory not extracted from profile
  evidence: extract_profile_to skip filter only skips "roles" and ".schema"; bridges/ directory exists in profiles/scrum-compact/bridges/telegram/
  timestamp: 2026-03-09

- hypothesis: bm hire modifies botminter.yml dropping bridge key
  evidence: hire.rs only reads manifest (line 22-26), never re-serializes it; git add only stages members/{dir}/
  timestamp: 2026-03-09

- hypothesis: agent tag filtering corrupts bridge.yml during extraction
  evidence: bridge.yml has no agent tags; filtering passes content through unchanged
  timestamp: 2026-03-09

## Evidence

- timestamp: 2026-03-09
  checked: record_bridge_in_manifest (init.rs:1556-1576)
  found: Uses raw serde_yml::Value manipulation to insert bridge key -- correct approach
  implication: Bridge key IS written correctly during init

- timestamp: 2026-03-09
  checked: bridge::discover (bridge.rs:380-408)
  found: Reads botminter.yml as serde_yml::Value, calls value.get("bridge") -- correct approach
  implication: Discovery logic is correct IF the key exists in the file

- timestamp: 2026-03-09
  checked: ProfileManifest struct (profile.rs:198-222)
  found: Has `bridges: Vec<BridgeDef>` (plural) but NO `bridge: Option<String>` (singular) field
  implication: Any deserialization into ProfileManifest silently drops the bridge key

- timestamp: 2026-03-09
  checked: bm projects add (projects.rs:135-172)
  found: Reads botminter.yml into ProfileManifest, modifies, re-serializes -- this DROPS the bridge key
  implication: Running bm projects add after bm init --bridge would silently remove bridge config

- timestamp: 2026-03-09
  checked: augment_manifest_with_projects (init.rs:837-854)
  found: Same pattern -- reads into ProfileManifest, re-serializes. BUT called BEFORE record_bridge_in_manifest in init flow
  implication: Not the cause during init, but same latent bug pattern

- timestamp: 2026-03-09
  checked: workspace.rs create_workspace_repo (workspace.rs:50-86)
  found: Calls `gh repo create` without checking if repo already exists; on failure, error msg references --push
  implication: No idempotent repo creation; stale flag name in error message

- timestamp: 2026-03-09
  checked: workspace.rs error message (workspace.rs:81)
  found: Says "re-run `bm teams sync --push`" but --push was replaced by --repos in Phase 9
  implication: Stale error message after flag rename

- timestamp: 2026-03-09
  checked: workspace.rs comment (workspace.rs:778)
  found: Comment also references --push mode
  implication: Second stale reference

- timestamp: 2026-03-09
  checked: Test init_bridge_records_in_manifest (integration.rs:3283-3327)
  found: Test passes -- verifies bridge key exists in botminter.yml after init
  implication: The init flow itself is correct for the direct init->sync path

## Resolution

root_cause: |
  **Sub-issue 1: "No bridge configured"**
  ROOT CAUSE: ProfileManifest (profile.rs:198-222) lacks a `bridge: Option<String>` field.
  The `bridge` key written by record_bridge_in_manifest is silently dropped whenever
  botminter.yml is round-tripped through ProfileManifest deserialization/re-serialization.
  Known affected code path: `bm projects add` (projects.rs:135-172). The init->hire->sync
  path itself does not trigger this, but any project addition between init and sync would.
  This is a latent data-loss bug even if not the immediate trigger for the reported scenario.

  If the user ran `bm projects add` between init and sync, the bridge key was dropped.
  If not, the cause may be environmental (profile re-extraction, manual edit, etc.).

  **Sub-issue 2: Repo creation for existing repo**
  ROOT CAUSE: create_workspace_repo (workspace.rs:50-86) unconditionally runs
  `gh repo create` without first checking if the GitHub repo already exists.
  The workspace existence check (teams.rs:338) only checks for the LOCAL
  `.botminter.workspace` marker file. If the GitHub repo exists but the local
  workspace doesn't (e.g., after cleanup/re-run), it tries to create an already-existing
  repo and fails.

  **Sub-issue 3: Stale --push flag in error message**
  ROOT CAUSE: workspace.rs:81 error message says "re-run `bm teams sync --push`"
  but --push was renamed to --repos in Phase 9. Also stale comment at workspace.rs:778.

fix: (research only - not applied)
verification: (research only - not applied)
files_changed: []
