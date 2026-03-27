# Implementation Plan: Loop Inbox

## Checklist

- [ ] Step 1: Core inbox domain logic + unit tests (`brain/inbox.rs`)
- [ ] Step 2: `bm-agent` CLI namespace + integration tests
- [ ] Step 3: Profile settings.json + workspace surfacing + e2e tests
- [ ] Step 4: Brain & agent context updates
- [ ] Step 5: Final validation (clippy, full test suite, exploratory tests)

## Steps

### Step 1: Core inbox domain logic + unit tests

**Objective:** Implement the `brain::inbox` submodule with all JSONL I/O, workspace discovery, and hook response formatting. All unit tests pass.

**Files changed:**
- **New:** `crates/bm/src/brain/inbox.rs`
- **Modified:** `crates/bm/src/brain/mod.rs` (add `pub mod inbox;`)
- **Modified:** `Cargo.toml` (add `fs2` dependency if chosen for file locking)

**Implementation guidance:**
- Create `crates/bm/src/brain/inbox.rs`
- Add `mod inbox;` to `crates/bm/src/brain/mod.rs` with selective `pub use` re-exports (following existing pattern)
- Implement types: `InboxMessage` (serde Serialize/Deserialize), `InboxReadResult { messages, consumed }`
- Implement functions:
  - `write_message(path, from, message) -> Result<()>`: validate non-empty, flock exclusive, append JSONL line with ISO 8601 timestamp, create parent dirs
  - `read_messages(path, consume) -> Result<InboxReadResult>`: flock exclusive, read lines, skip malformed (log to stderr), truncate if consume=true
  - `inbox_path(workspace_root) -> PathBuf`: returns `<root>/.ralph/loop-inbox.jsonl`
  - `discover_workspace_root(start) -> Option<PathBuf>`: walk up looking for `.botminter.workspace` marker
  - `format_hook_response(messages) -> Option<String>`: return `None` when empty, `Some(json)` with `additionalContext` when non-empty
- File locking: prefer `fs2` crate (`FileExt::lock_exclusive()`). Add `fs2` to `crates/bm/Cargo.toml`.

**Test requirements (all in `brain/inbox.rs`, using `tempfile::tempdir()`):**
- write + read roundtrip, multiple messages preserve chronological order
- consume=true truncates file, consume=false preserves
- empty/missing file returns empty vec (no error)
- malformed JSONL lines skipped gracefully
- hook format output is valid JSON with `additionalContext` key containing all messages
- empty hook format returns `None`
- empty message rejected with error
- workspace root discovery: marker in parent dir, marker in grandparent dir, no marker returns None
- `inbox_path` returns `<root>/.ralph/loop-inbox.jsonl`
- **Concurrency test (NFR-1):** spawn 8 threads, each writing a unique message. After all complete, `read_messages` returns all 8 with no corruption.

**Demo:** `cargo test -p bm inbox` passes all unit tests including concurrency.

---

### Step 2: `bm-agent` CLI namespace + integration tests

**Objective:** Wire up `bm-agent inbox write/read/peek` and `bm-agent claude hook post-tool-use` as a thin command layer. Integration tests verify the full CLI lifecycle.

**Files changed:**
- **New:** `crates/bm/src/agent_main.rs` (entry point for `bm-agent` binary)
- **New:** `crates/bm/src/agent_cli.rs` (Clap parser: `AgentCli`, `AgentCommand`, `InboxCommand`, `ClaudeCommand`, `ClaudeHookCommand`, `InboxFormat`)
- **Modified:** `crates/bm/Cargo.toml` (add `[[bin]]` target for `bm-agent`)
- Note: `cli.rs` and `main.rs` are NOT modified — `bm-agent` is a separate binary

**Implementation guidance:**
- `agent_cli.rs` defines the Clap parser, `agent_main.rs` parses and dispatches to thin command handlers (ADR-0007):
  - Inbox subcommands: discover workspace root -> construct path -> call domain function -> format output
  - `hook post-tool-use`: same flow but **always exits 0**, suppresses all errors and stderr internally
- `inbox write`: confirmation to stderr on success, error + exit 1 on failure
- `inbox read --format hook` (default): print to stdout if messages, nothing if empty
- `inbox read --format json`: JSON array to stdout
- `inbox peek`: human-readable table, "No pending messages." if empty
- `hook post-tool-use`: never fails. If workspace not found, no messages, or any error → exit 0 with no output

**Test requirements (integration tests using `TestEnv`/`TestCommand`, ADR-0005):**
- **Test setup:** Create `.botminter.workspace` marker and `.ralph/` dir in TestEnv temp directory.
- write + peek: `bm-agent inbox write "hello"` succeeds, `bm-agent inbox peek` output contains "hello"
- read + consume: `bm-agent inbox read --format json` returns valid JSON, then peek shows empty
- empty write rejected: `bm-agent inbox write ""` exits non-zero
- outside workspace: `bm-agent inbox write "test"` from non-workspace exits non-zero
- hook graceful: `bm-agent claude hook post-tool-use` from non-workspace exits 0 with no output
- hook empty: `bm-agent claude hook post-tool-use` with empty inbox exits 0 with no output
- hook delivery: write message, then `bm-agent claude hook post-tool-use` returns JSON with `additionalContext`

**Demo:** Full write-peek-consume-hook lifecycle via CLI.

---

### Step 3: Profile settings.json + workspace surfacing + e2e tests

**Objective:** Add settings.json to scrum-compact profile, implement workspace surfacing, verify with e2e tests.

**Files changed:**
- **New:** `profiles/scrum-compact/coding-agent/settings.json`
- **Modified:** `crates/bm/src/workspace/repo.rs` (surface settings.json during creation)
- **Modified:** `crates/bm/src/workspace/sync.rs` (surface settings.json during sync)
- **Modified:** E2E scenario test (add surfacing + inbox lifecycle cases)
- **Modified:** `crates/bm/tests/exploratory/PLAN.md` (add inbox test cases)

**Implementation guidance:**
- Create `profiles/scrum-compact/coding-agent/settings.json` with PostToolUse hook config: `"command": "bm-agent claude hook post-tool-use"`
- **`workspace/repo.rs`**: after `settings.local.json` copy, add settings.json copy from `team/coding-agent/settings.json` (team-level) to `<workspace>/.claude/settings.json`. **Note:** `settings.json` is sourced from `team/coding-agent/` (team-level), NOT from `team/members/<member>/coding-agent/` like `settings.local.json`. This is intentional — hooks are shared across all members.
- **`workspace/sync.rs`**: after `settings.local.json` re-copy, re-copy settings.json via `copy_if_newer_verbose()`

**Test requirements:**
- E2E (within existing scenario): after sync, `.claude/settings.json` exists with `bm-agent claude hook post-tool-use`
- E2E: `bm-agent inbox write "e2e test"` + peek + read + peek lifecycle
- E2E: re-sync preserves pending messages
- Exploratory test plan updated with D.10-D.13 from design

**Demo:** `just test` passes including new e2e cases.

---

### Step 4: Brain & agent context updates

**Objective:** Update profile docs so brain knows how to use inbox and coding agent knows how to respond.

**Files changed:**
- **Modified:** `profiles/scrum-compact/brain/system-prompt.md` (add Loop Feedback section)
- **Modified:** `profiles/scrum-compact/context.md` (add Brain Feedback section)

**Implementation guidance:**
- Brain prompt: add `## Loop Feedback (Inbox)` with `bm-agent inbox write` usage
- Context: add `## Brain Feedback` with priority/conflict guidance (FR-5)
- No `--loop` flag in docs (deferred per FR-2)

**Test requirements:**
- Profile tests pass. Content inspection for FR-5 and FR-9.

**Demo:** grep confirms both sections present.

---

### Step 5: Final validation

**Objective:** Full test suite green, all invariants satisfied.

**Implementation guidance:**
- `just clippy` — zero warnings
- `just test` — all pass
- `just exploratory-test` — green (workspace sync was modified)
- Invariant compliance audit (CLI idempotency, test isolation, profiles updated, e2e coverage, zero failures)

**Demo:** All three `just` commands pass.
