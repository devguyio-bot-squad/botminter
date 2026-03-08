---
status: resolved
phase: 05-team-manager-chat
source: 05-01-SUMMARY.md
started: 2026-03-07T00:00:00Z
updated: 2026-03-07T07:50:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Build & Tests Pass
expected: Run `just build` and `just test`. Both complete without errors.
result: pass

### 2. Team Manager Role in Profile Manifests
expected: Check `profiles/scrum/botminter.yml`, `profiles/scrum-compact/botminter.yml`, and `profiles/scrum-compact-telegram/botminter.yml`. Each has a `team-manager` role defined with minimal statuses (mgr:todo, mgr:in-progress, mgr:done). Run `bm roles list` to see it listed.
result: pass

### 3. Team Manager Skeleton Complete
expected: The `profiles/scrum/roles/team-manager/` directory contains a complete role skeleton: .botminter.yml, context.md (with agent tags), ralph.yml (with executor hat, board-scanner auto-inject), PROMPT.md, knowledge/, invariants/, and coding-agent/ directories. Not empty stubs — real configuration content.
result: pass

### 4. BM Chat Command Exists
expected: Run `bm chat --help`. The command exists and shows usage with `<member>` argument, `-t/--team` flag, `--hat` flag for selecting a specific hat, and `--render-system-prompt` flag for debugging. The help text is clear about what the command does.
result: pass

### 5. BM Chat Render System Prompt
expected: With a team and member set up, run `bm chat <member> --render-system-prompt`. Instead of launching an interactive session, it prints the assembled meta-prompt (system prompt) and exits. The output should show sections for identity, capabilities (hats), guardrails (numbered from 999), role context, and reference material.
result: issue
reported: "Identity section says 'You are superman-testbot, a superman on the uat-ws-test team' — the role name 'superman' has no description injected. The AI receiving this prompt won't understand what a 'superman' is or what it's supposed to do."
severity: major

### 6. BM Chat Hat Selection
expected: Run `bm chat <member> --hat executor --render-system-prompt`. The meta-prompt should show only the executor hat's capabilities, not all hats. Compare with no `--hat` flag to confirm the difference.
result: issue
reported: "Passing --hat executor (which doesn't exist in scrum-compact superman) produces no error — silently renders an empty '## Your Capabilities' section. Should validate the hat name against available hats and error if not found."
severity: major

### 7. BM Chat Launches Interactive Session
expected: Run `bm chat <member>` (without --render-system-prompt). The command should attempt to launch the coding agent (Claude Code) as an interactive session using exec() — replacing the bm process. If Claude Code is not available, it should fail with a clear error about the missing binary. It should NOT hang or silently do nothing.
result: issue
reported: "Error: Input must be provided either through stdin or as a prompt argument when using --print. The --print bug is in session.rs (used by bm knowledge), not chat.rs. chat.rs correctly uses --append-system-prompt-file."
severity: minor

### 8. BM Chat Surfaces Skills
expected: The meta-prompt assembled by `bm chat` should include a skills table listing available skills from the workspace's `team/coding-agent/skills/` directory — same as Ralph does for hat sessions. The AI needs to know what skills exist and how to find them (e.g., "read team/coding-agent/skills/status-workflow/SKILL.md").
result: issue
reported: "Meta-prompt has no skills section. Ralph's hat prompt includes a skills table with available skills and load commands. bm chat reads ralph.yml (which has skills.dirs config) but doesn't scan for SKILL.md files or inject a skills table into the meta-prompt."
severity: major

## Summary

total: 8
passed: 4
issues: 4
pending: 0
skipped: 0

## Gaps

- truth: "Chat identity section includes role description so the AI understands its purpose"
  status: resolved
  reason: "User reported: Identity says 'a superman' with no description. AI won't understand what the role is."
  severity: major
  test: 5
  root_cause: "MetaPromptParams struct in chat.rs has no role_description field. Identity line formats 'You are {}, a {} on the {} team' using only role_name. RoleDef in profile.rs already has the description field (e.g., 'All-in-one member — PO, architect, dev, QE, SRE, content writer') but it's not threaded through."
  artifacts:
    - path: "crates/bm/src/chat.rs"
      issue: "MetaPromptParams missing role_description field, identity format string at lines 27-32 doesn't include it"
    - path: "crates/bm/src/profile.rs"
      issue: "RoleDef.description exists (line 221-225) but not passed to chat"
  missing:
    - "Add role_description to MetaPromptParams struct"
    - "Look up role description from manifest in chat command"
    - "Inject description into identity format string"
  debug_session: ""

- truth: "Invalid --hat name produces a clear error instead of silently rendering empty capabilities"
  status: resolved
  reason: "User reported: --hat executor on scrum-compact superman silently produces empty capabilities section, no error."
  severity: major
  test: 6
  root_cause: "chat.rs lines 39-45: if let Some(instructions) = params.hat_instructions.get(hat_name) silently falls through when hat doesn't exist. No validation before build_meta_prompt() call."
  artifacts:
    - path: "crates/bm/src/chat.rs"
      issue: "No hat name validation at lines 39-45 or before MetaPromptParams construction at lines 70-80"
  missing:
    - "Validate --hat value against hat_instructions keys before calling build_meta_prompt()"
    - "Bail with available hats list if hat not found"
  debug_session: ""

- truth: "bm chat launches an interactive coding agent session, not a --print mode session"
  status: resolved
  reason: "User reported: Error: Input must be provided either through stdin or as a prompt argument when using --print. The exec() passes --print instead of launching interactively."
  severity: minor
  test: 7
  root_cause: "Diagnosis found chat.rs:112-116 uses --append-system-prompt-file correctly. The --print bug is in session.rs:39 (interactive_claude_session function) used by bm knowledge. The user's error may have come from bm knowledge or a stale build. Needs verification — if bm chat truly errors, the binary may differ from current source."
  artifacts:
    - path: "crates/bm/src/commands/chat.rs"
      issue: "Lines 112-116 appear correct (uses --append-system-prompt-file)"
    - path: "crates/bm/src/session.rs"
      issue: "Line 39 uses --print for interactive_claude_session(), should use --append-system-prompt-file"
  missing:
    - "Verify bm chat error is reproducible with current source (rebuild and retest)"
    - "Fix session.rs:39 to use --append-system-prompt-file instead of --print"
  debug_session: ""

- truth: "bm chat meta-prompt includes a skills table so the AI knows what skills are available"
  status: resolved
  reason: "User reported: Meta-prompt has no skills section. Ralph includes a skills table with available skills. bm chat reads ralph.yml (which has skills.dirs) but doesn't scan for SKILL.md files or inject skills into the prompt."
  severity: major
  test: 8
  root_cause: "chat.rs deserializes ralph.yml into a minimal RalphConfig struct that only reads core.guardrails and hats. It doesn't read the skills section. build_meta_prompt() in chat.rs has no skills parameter or section. Ralph scans skills.dirs, reads SKILL.md frontmatter (name, description), and injects a table into the hat prompt."
  artifacts:
    - path: "crates/bm/src/commands/chat.rs"
      issue: "RalphConfig struct (line 151) doesn't deserialize skills section. No skills scanning or injection logic."
    - path: "crates/bm/src/chat.rs"
      issue: "MetaPromptParams has no skills field, build_meta_prompt() has no skills table section"
  missing:
    - "Add skills section to RalphConfig deserialization in chat.rs"
    - "Scan skills.dirs for SKILL.md files, read frontmatter (name, description)"
    - "Add skills field to MetaPromptParams and skills table section to build_meta_prompt()"
  debug_session: ""
