---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: bm init Integration and --show-tags Flag

## Description
Update `bm init` to resolve the coding agent before extraction, passing `CodingAgentDef` to the extraction pipeline. Add a `--show-tags` flag to `bm profiles describe` that shows a summary of agent tags in profile files.

## Background
The extraction functions now accept a `CodingAgentDef` parameter, but `bm init` doesn't yet resolve or pass it. This task connects the dots so the full `bm init` flow is agent-aware. The `--show-tags` flag provides visibility into which files have agent-specific content.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 4)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `bm init` command handler:
   - After selecting a profile, read `default_coding_agent` from the manifest
   - Resolve `CodingAgentDef` using `resolve_coding_agent()`
   - Pass the resolved agent to `extract_profile_to()` and `extract_member_to()`
2. Add `--show-tags` flag to `bm profiles describe`:
   - Scan profile files for agent tags
   - Show summary: which files have tags, which agents are referenced
   - Example output: `Tagged files: context.md (claude-code), ralph.yml (claude-code)`

## Dependencies
- Task 1 of this step (extraction functions updated)
- Step 2 (resolve_coding_agent function)

## Implementation Approach
1. Update the init command to resolve coding agent early in the flow
2. Thread the resolved agent through to extraction calls
3. Add --show-tags flag to the profiles describe CLI definition
4. Implement tag scanning logic (reuse agent_tags module for detection)
5. Write integration tests for the full init flow

## Acceptance Criteria

1. **bm init resolves coding agent**
   - Given a profile with `default_coding_agent: claude-code`
   - When `bm init` runs
   - Then it resolves the `claude-code` agent and passes it to extraction

2. **bm init produces correctly named files**
   - Given `bm init` with a profile that has `context.md` + tags
   - When the team repo is created
   - Then it contains `CLAUDE.md` (not `context.md`) with filtered content

3. **profiles describe --show-tags lists tagged files**
   - Given `bm profiles describe scrum --show-tags`
   - When the command runs
   - Then output shows which files contain agent tags and which agents they reference

4. **profiles describe without --show-tags unchanged**
   - Given `bm profiles describe scrum` (no flag)
   - When the command runs
   - Then output is unchanged from previous behavior (plus the Coding Agents section from Step 2)

5. **Integration test: full bm init flow**
   - Given the full `bm init` wizard flow
   - When run end-to-end
   - Then the team repo has filtered, renamed files with no tag markers

## Metadata
- **Complexity**: Medium
- **Labels**: coding-agent-agnostic, cli, sprint-1
- **Required Skills**: Rust, CLI (clap), integration testing
