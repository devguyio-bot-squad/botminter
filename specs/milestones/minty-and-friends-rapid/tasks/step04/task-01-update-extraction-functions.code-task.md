---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Update Extraction Functions with Agent Filtering

## Description
Update `extract_profile_to()` and `extract_member_to()` to run the agent tag filter during extraction and rename `context.md` to the resolved agent's `context_file` (e.g., `CLAUDE.md`). After this task, profile extraction produces cleanly filtered, properly named files.

## Background
The extraction pipeline copies files from the profile source into a team repo. Previously it was a simple copy. Now it must: (1) run `filter_agent_tags()` on `.md`, `.yml`, `.yaml`, `.sh` files to strip non-matching agent sections, and (2) rename `context.md` to the agent-specific name (e.g., `CLAUDE.md` for Claude Code).

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Extraction pipeline")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 4)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `extract_profile_to(name, target, coding_agent: &CodingAgentDef)`:
   - Copy all files (skip `members/`, `.schema/`)
   - For `context.md`: run `filter_agent_tags()`, write result as `coding_agent.context_file`
   - For other `.md`, `.yml`, `.yaml`, `.sh` files: run `filter_agent_tags()` to strip non-matching sections
   - Copy remaining files (images, binary) verbatim
2. Update `extract_member_to(name, role, target, coding_agent: &CodingAgentDef)`:
   - Same pattern: filter + rename `context.md` -> agent's context_file
   - All text files get filtered
3. Update function signatures to accept `CodingAgentDef`
4. `coding-agent/` directory stays as-is in output (it's a BotMinter convention, not agent-specific)

## Dependencies
- Step 1 (agent_tags filter module)
- Step 2 (CodingAgentDef data model)
- Step 3 (profiles restructured with tags)

## Implementation Approach
1. Read existing extraction functions in `profile.rs`
2. Add `CodingAgentDef` parameter to both functions
3. Add file-type detection logic (which files to filter vs. copy verbatim)
4. Integrate `filter_file()` from the agent_tags module
5. Add context.md -> context_file rename logic
6. Write unit tests with fixture profiles

## Acceptance Criteria

1. **Profile extraction produces agent-named context file**
   - Given `extract_profile_to()` called with `claude-code` agent
   - When extracting a profile with `context.md`
   - Then the output contains `CLAUDE.md` (not `context.md`) at team repo root

2. **Extracted CLAUDE.md is filtered and clean**
   - Given a profile `context.md` with agent tags
   - When extracted for `claude-code`
   - Then `CLAUDE.md` contains common + claude-code sections, no tag markers

3. **Member extraction produces filtered context file**
   - Given `extract_member_to()` called with `claude-code` agent
   - When extracting a member skeleton
   - Then the member dir contains `CLAUDE.md`, filtered and clean

4. **ralph.yml filtered during extraction**
   - Given a `ralph.yml` with agent tags
   - When extracted for `claude-code`
   - Then the output has `cli.backend: claude` with no tag markers

5. **Mock agent produces different context file name**
   - Given a hypothetical agent with `context_file: "GEMINI.md"`
   - When `extract_profile_to()` is called with that agent
   - Then `context.md` -> `GEMINI.md` in the output

6. **Binary files copied verbatim**
   - Given a profile with image files
   - When extracted
   - Then images are byte-identical (no filtering applied)

7. **Existing integration tests pass**
   - Given updated extraction functions
   - When the full test suite runs
   - Then all existing `bm init` tests pass

## Metadata
- **Complexity**: Medium
- **Labels**: coding-agent-agnostic, extraction, sprint-1
- **Required Skills**: Rust, file I/O, agent_tags module
