---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Parameterize Hardcoded Agent Strings in Workspace

## Description
Eliminate all hardcoded `"CLAUDE.md"` and `".claude"` strings in `workspace.rs`. Replace them with values read from the resolved `CodingAgentDef`. Thread `CodingAgentDef` through the call chain from `bm teams sync` and `bm hire` down to workspace functions.

## Background
After Steps 1-4, the profile and extraction pipeline are agent-agnostic. But `workspace.rs` still has hardcoded Claude Code assumptions (`"CLAUDE.md"`, `".claude"`). This task completes Sprint 1 by making the workspace layer fully parameterized.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Workspace parameterization")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 5)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `workspace.rs` functions to accept `CodingAgentDef` or its relevant fields:
   - `BM_GITIGNORE_ENTRIES`: use `coding_agent.agent_dir` instead of `".claude"`
   - `surface_files()`: use `coding_agent.context_file` instead of `"CLAUDE.md"`
   - `assemble_claude_dir()` -> rename to `assemble_agent_dir()`: use `coding_agent.agent_dir`
   - `sync_workspace()`: pass coding agent config through
2. Thread `CodingAgentDef` through the call chain:
   - `bm teams sync` -> resolve coding agent -> pass to workspace functions
   - `bm hire` -> resolve coding agent -> pass to member extraction
3. Search for any remaining hardcoded `"CLAUDE.md"`, `".claude"`, `"claude"` strings in non-test code and parameterize them
4. Test fixtures and assertions can still use concrete values (they test with `claude-code`)

## Dependencies
- Steps 1-4 complete (agent model, tags, extraction all working)

## Implementation Approach
1. Audit `workspace.rs` for all hardcoded agent strings
2. Update function signatures to accept agent config
3. Rename `assemble_claude_dir()` to `assemble_agent_dir()`
4. Update callers (`bm teams sync`, `bm hire`) to resolve and pass agent config
5. Run `grep` audit to confirm no hardcoded strings remain in production code
6. Run full test suite to verify backwards compatibility

## Acceptance Criteria

1. **No hardcoded agent strings in workspace.rs**
   - Given the `workspace.rs` source file
   - When searching for `"CLAUDE.md"` or `".claude"` outside test code
   - Then no matches are found

2. **assemble_claude_dir renamed to assemble_agent_dir**
   - Given the `workspace.rs` module
   - When searching for function names
   - Then `assemble_agent_dir` exists and `assemble_claude_dir` does not

3. **.gitignore uses parameterized agent dir**
   - Given a workspace created by `bm teams sync`
   - When inspecting the `.gitignore`
   - Then it references the agent dir from config (e.g., `.claude`) not a hardcoded value

4. **Symlinks use parameterized paths**
   - Given a workspace created by `bm teams sync`
   - When inspecting symlinks for the agent directory
   - Then paths are derived from `CodingAgentDef` fields

5. **bm teams sync resolves and passes coding agent**
   - Given `bm teams sync` command
   - When it executes
   - Then it resolves the coding agent from team/profile config and passes to workspace functions

6. **bm hire passes coding agent to extraction**
   - Given `bm hire <role>` command
   - When it executes
   - Then it resolves the coding agent and passes to member extraction

7. **All existing tests pass**
   - Given the parameterized workspace functions
   - When the full test suite runs
   - Then all tests pass (they resolve `claude-code` and get the same concrete values)

## Metadata
- **Complexity**: Medium
- **Labels**: coding-agent-agnostic, workspace, sprint-1
- **Required Skills**: Rust, workspace module knowledge
