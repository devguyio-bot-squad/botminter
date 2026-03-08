---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Schema Updates and Profiles Describe Output

## Description
Update the `.schema/` JSON schema for `botminter.yml` to include the new `coding_agents` and `default_coding_agent` fields. Update existing profile `botminter.yml` files to declare `claude-code` as the sole agent. Update `bm profiles describe` to show a "Coding Agents" section.

## Background
Step 2's data model additions need corresponding schema validation updates and actual profile content. The `bm profiles describe` command should surface coding agent information so operators can see which agents a profile supports.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 2)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Update `.schema/botminter.yml` (or equivalent schema file) with:
   - `coding_agents` object property (map of agent name to agent definition)
   - `default_coding_agent` string property
   - `CodingAgentDef` object schema (name, display_name, context_file, agent_dir, binary)
2. Update all profile `botminter.yml` files (`profiles/scrum/`, `profiles/scrum-compact/`, etc.):
   - Add `coding_agents` section with `claude-code` entry
   - Add `default_coding_agent: claude-code`
3. Update `bm profiles describe` command output to include a "Coding Agents" section

## Dependencies
- Task 1 of this step (CodingAgentDef struct exists)

## Implementation Approach
1. Locate and update the JSON schema files
2. Add claude-code agent definition to each profile's botminter.yml
3. Update the profiles describe command handler to format coding agent info
4. Run schema validation and integration tests

## Acceptance Criteria

1. **Schema validates new fields**
   - Given a `botminter.yml` with `coding_agents` and `default_coding_agent`
   - When validated against the updated schema
   - Then validation passes

2. **Schema rejects invalid agent definitions**
   - Given a `botminter.yml` with missing required fields in a coding agent entry
   - When validated against the schema
   - Then validation fails with a clear error

3. **All profiles declare claude-code**
   - Given each profile's `botminter.yml`
   - When parsed
   - Then `coding_agents` contains `claude-code` with correct fields (context_file: "CLAUDE.md", agent_dir: ".claude", binary: "claude")

4. **Profiles describe shows coding agents**
   - Given `bm profiles describe scrum`
   - When the command runs
   - Then output includes a "Coding Agents" section showing `claude-code (default)`

5. **Existing tests pass with updated profiles**
   - Given the updated profile YAML files
   - When the full test suite runs
   - Then all existing tests pass

6. **Schema version remains 1.0**
   - Given the updated `botminter.yml` files with new `coding_agents` fields
   - When inspecting `schema_version`
   - Then it remains `"1.0"` (no version bump for this milestone)

## Metadata
- **Complexity**: Low
- **Labels**: coding-agent-agnostic, schema, sprint-1
- **Required Skills**: Rust, JSON Schema, YAML
