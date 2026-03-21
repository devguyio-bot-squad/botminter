# Plan: Loop Inbox — Brain-to-Loop Feedback Channel

## Context

The brain (ACP-wrapped Claude Code) can **observe** Ralph loop events via the EventWatcher polling `.ralph/events-*.jsonl`, but it has **no way to send feedback back** to running loops. This creates a one-way observation channel. When a human says "stop refactoring, fix CI" on the bridge, the brain can acknowledge it but cannot steer its own working loops.

**Solution:** A per-loop inbox file (`.ralph/loop-inbox.jsonl`) + a PostToolUse Claude Code hook inside the Ralph-spawned coding agent. The brain writes via `bm inbox write`, the hook reads and injects as `additionalContext` after every tool call. The inbox is automatically per-loop scoped because each worktree loop has its own `.ralph/` directory.

```
Brain (Claude Code #1)                    Ralph's coding agent (Claude Code #2)
  │                                              │
  │── bm inbox write "fix CI" ──────────────────►│ .ralph/loop-inbox.jsonl
  │                                              │── PostToolUse hook reads it
  │                                              │── additionalContext injected
  │◄── .ralph/events-*.jsonl (already exists) ───│
```

## Implementation Steps

### Step 1: `inbox` module — core JSONL read/write logic

**New file: `crates/bm/src/inbox.rs`**

Types and functions:
- `InboxMessage { ts: String, from: String, message: String }` — serde Serialize/Deserialize
- `write_message(inbox_path, from, message) -> Result<()>` — append one JSONL line with ISO8601 timestamp, file-locked (`flock`)
- `read_messages(inbox_path, consume: bool) -> Result<Vec<InboxMessage>>` — read all lines, truncate if `consume=true`, file-locked
- `inbox_path_for_loop(workspace_root, loop_id: Option<&str>) -> PathBuf` — returns `<root>/.ralph/loop-inbox.jsonl` or `<root>/.worktrees/<id>/.ralph/loop-inbox.jsonl`
- `discover_workspace_root(start) -> Option<PathBuf>` — walk up looking for `.botminter.workspace` marker
- `format_hook_output(messages: &[InboxMessage]) -> String` — produce complete PostToolUse hook JSON response with `additionalContext`

Register: add `pub mod inbox;` in `crates/bm/src/lib.rs`.

**Unit tests** (in-file):
- write + read roundtrip, multiple messages preserve order
- consume=true truncates, consume=false preserves
- empty/missing file returns empty vec (no error)
- malformed lines skipped gracefully
- hook format output is valid JSON with `additionalContext` key
- workspace root discovery (marker in parent)
- path construction for primary vs worktree loop

### Step 2: `bm inbox` CLI subcommand

**New file: `crates/bm/src/commands/inbox.rs`**

Three subcommands:
```
bm inbox write <message> [--loop <id>] [--from <name>]
bm inbox read [--loop <id>] [--format json|hook]
bm inbox peek [--loop <id>]
```

- `write`: discover workspace root from CWD, construct inbox path, call `inbox::write_message`. Default `--from brain`.
- `read --format json`: output JSON array to stdout, consume messages.
- `read --format hook`: output complete PostToolUse hook JSON (with `additionalContext` + framing prompt), consume messages. This is what the hook script calls.
- `peek`: read without consuming, human-readable output.

**Modify files:**
- `crates/bm/src/cli.rs` — add `Inbox { command: InboxCommand }` variant + `InboxCommand` enum
- `crates/bm/src/commands/mod.rs` — add `pub mod inbox;`
- `crates/bm/src/main.rs` — add `Command::Inbox` match arm

### Step 3: PostToolUse hook script in profile

**New file: `profiles/scrum-compact/coding-agent/hooks/check-loop-inbox.sh`**

Minimal shell shim — all logic lives in `bm inbox read --format hook`:

```bash
#!/bin/bash
BM=$(command -v bm 2>/dev/null)
[ -z "$BM" ] && exit 0
exec "$BM" inbox read --format hook 2>/dev/null
```

The `--format hook` output includes the framing prompt that tells the coding agent:
- Brain feedback takes priority over current subtask
- Acknowledge and adjust approach
- If feedback conflicts with current task, comply with feedback

### Step 4: Hook configuration in profile

**New file: `profiles/scrum-compact/coding-agent/settings.json`**

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/check-loop-inbox.sh"
          }
        ]
      }
    ]
  }
}
```

This is project-level `settings.json` (not `settings.local.json`), so it applies to all Claude Code sessions in the workspace and is shareable via git.

### Step 5: Surface hooks + settings.json during workspace sync

**Modify: `crates/bm/src/workspace/repo.rs`** (in `assemble_agent_dir_submodule`)

After step 4 (copy settings.local.json), add:

5a. Copy `settings.json` from team-level `coding-agent/settings.json` if present:
  - Source: `team/coding-agent/settings.json`
  - Dest: `<workspace>/.claude/settings.json`

5b. Copy hook scripts from `team/coding-agent/hooks/` into `<workspace>/.claude/hooks/`:
  - Create `.claude/hooks/` directory
  - Copy all files, preserve executable permission (`chmod +x`)

**Modify: `crates/bm/src/workspace/sync.rs`** (in `sync_workspace`)

After re-copying settings.local.json:
- Re-copy `settings.json` using `copy_if_newer_verbose` (same pattern as settings.local.json)
- Re-sync hook scripts from `team/coding-agent/hooks/` (copy_if_newer + chmod +x)

### Step 6: Update brain system prompt

**Modify: `profiles/scrum-compact/brain/system-prompt.md`**

Add after "## Loop Management":

```markdown
## Loop Feedback (Inbox)

You can send feedback to your running loops. Messages are delivered to the
coding agent inside the loop via a PostToolUse hook — the agent sees your
message after its next tool call.

**Send feedback to the primary loop:**
\`\`\`bash
bm inbox write "Stop working on the CSS. Focus on the API endpoint instead."
\`\`\`

**Send feedback to a specific worktree loop:**
\`\`\`bash
bm inbox write --loop <loop-id> "The human approved the design. Proceed."
\`\`\`

**When to use:** human sends a redirect, you observe a loop going wrong,
you need to pass context from another loop or the board.

**When NOT to use:** routine status checks (just observe events),
stopping a loop (`ralph loops stop`), starting new work (start a new loop).
```

### Step 7: Update coding agent context

**Modify: `profiles/scrum-compact/context.md`**

Add a section:

```markdown
## Brain Feedback

You may receive messages marked "Brain feedback" injected after tool calls.
These come from your team member's brain — the consciousness that monitors
your work, receives human messages, and manages the board.

When you receive brain feedback:
1. It takes priority over your current subtask
2. Acknowledge by adjusting your approach
3. If feedback conflicts with your current task, comply with the feedback
```

## Files Summary

### New files (4)
| File | Purpose |
|------|---------|
| `crates/bm/src/inbox.rs` | Core inbox JSONL read/write/format logic |
| `crates/bm/src/commands/inbox.rs` | CLI command handlers for write/read/peek |
| `profiles/scrum-compact/coding-agent/hooks/check-loop-inbox.sh` | PostToolUse hook shell shim |
| `profiles/scrum-compact/coding-agent/settings.json` | Claude Code project settings with hook config |

### Modified files (8)
| File | Change |
|------|--------|
| `crates/bm/src/lib.rs` | Add `pub mod inbox;` |
| `crates/bm/src/cli.rs` | Add `Inbox` variant + `InboxCommand` enum |
| `crates/bm/src/main.rs` | Add `Command::Inbox` match arm |
| `crates/bm/src/commands/mod.rs` | Add `pub mod inbox;` |
| `crates/bm/src/workspace/repo.rs` | Surface hooks dir + settings.json during creation |
| `crates/bm/src/workspace/sync.rs` | Surface hooks dir + settings.json during sync |
| `profiles/scrum-compact/brain/system-prompt.md` | Add Loop Feedback section |
| `profiles/scrum-compact/context.md` | Add Brain Feedback section |

## Verification

1. **Unit tests:** `just unit` — inbox module tests (write/read/consume/peek/format/discovery)
2. **Integration test:** write via `bm inbox write`, read via `bm inbox read --format hook`, verify JSON output
3. **Sync test:** existing workspace sync tests should cover the new settings.json + hooks surfacing
4. **Full test suite:** `just test` must pass
5. **Manual verification:** in a synced workspace, confirm `.claude/hooks/check-loop-inbox.sh` exists and is executable, `.claude/settings.json` has the PostToolUse hook config
