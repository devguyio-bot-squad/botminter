---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: bm chat CLI Subcommand and Launch

## Description
Implement the `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt]` CLI subcommand. Wire it to `build_meta_prompt()`, handle `--render-system-prompt` for debugging, and implement coding agent launch via `--append-system-prompt-file`.

## Background
`bm chat` is the user-facing command for interactive sessions with hired members. It resolves the workspace, reads configuration files, builds a meta-prompt, and either prints it (for debugging) or launches the coding agent with the prompt injected as a system prompt.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "bm chat")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 17)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Add `chat` subcommand to `cli.rs`:
   - Required: `<member>` positional argument
   - Optional: `-t`/`--team`, `--hat <hat>`, `--render-system-prompt`
2. Implement `commands/chat.rs`:
   - Resolve workspace path for the member
   - Resolve coding agent from team config
   - Read `ralph.yml` from workspace root (guardrails, hat definitions)
   - Read Ralph prompts from disk profile's `ralph-prompts/`
   - Read `PROMPT.md` from workspace root
   - Call `build_meta_prompt()` with collected inputs
3. `--render-system-prompt`: print meta-prompt to stdout and exit
4. Normal mode: write meta-prompt to temp file, launch coding agent:
   ```
   Command::new(&coding_agent.binary)
       .current_dir(&ws_path)
       .arg("--append-system-prompt-file")
       .arg(&prompt_file)
       .exec();
   ```
5. Error handling: `bm chat nonexistent-member` produces helpful error

## Dependencies
- Task 1 of this step (build_meta_prompt function exists)
- Step 11 (workspace repos exist and are discoverable)

## Implementation Approach
1. Add CLI subcommand definition with clap
2. Implement command handler with file reading
3. Wire up to build_meta_prompt()
4. Implement render-system-prompt output mode
5. Implement agent launch mode
6. Add error handling for missing member, missing files, etc.
7. Write integration tests

## Acceptance Criteria

1. **bm chat resolves workspace**
   - Given a hired member "bob" with a workspace
   - When `bm chat bob` runs
   - Then it correctly resolves bob's workspace path

2. **--render-system-prompt prints to stdout**
   - Given `bm chat bob --render-system-prompt`
   - When the command runs
   - Then the meta-prompt is printed to stdout and the process exits (no agent launch)

3. **--render-system-prompt --hat filters**
   - Given `bm chat bob --render-system-prompt --hat executor`
   - When the command runs
   - Then only the executor hat's instructions are in the output

4. **Normal mode launches coding agent**
   - Given `bm chat bob`
   - When the command runs
   - Then the coding agent binary is exec'd with `--append-system-prompt-file`

5. **Nonexistent member error**
   - Given `bm chat nonexistent`
   - When the command runs
   - Then a helpful error message is shown (not a panic or cryptic error)

6. **Meta-prompt reads from correct sources**
   - Given a workspace with ralph.yml, PROMPT.md, and ralph-prompts/
   - When building the meta-prompt
   - Then content is sourced from all three locations

## Metadata
- **Complexity**: Medium
- **Labels**: bm-chat, cli, sprint-5
- **Required Skills**: Rust, clap CLI, process exec, file I/O
