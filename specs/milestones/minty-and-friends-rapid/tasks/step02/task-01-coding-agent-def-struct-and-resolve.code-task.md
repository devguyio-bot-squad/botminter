---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: CodingAgentDef Struct and Resolve Function

## Description
Create the `CodingAgentDef` data model, update `ProfileManifest` with coding agent fields, add a `coding_agent` override to `TeamEntry`, and implement the `resolve_coding_agent()` resolution function. This establishes the config-driven mapping that all subsequent coding-agent-agnostic work depends on.

## Background
The coding-agent-agnostic architecture requires a data model to describe each supported coding agent's file conventions (context file name, agent directory, binary). Profiles declare supported agents and a default; teams can override the default. The resolve function determines the effective agent for a given team.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Coding-Agent-Agnostic" and "Config-driven mapping")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 2)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Add `CodingAgentDef` struct (in `profile.rs` or a new `coding_agent.rs` module):
   - Fields: `name`, `display_name`, `context_file`, `agent_dir`, `binary` (all `String`)
   - Derive `Deserialize`, `Serialize`, `Debug`, `Clone`
2. Update `ProfileManifest` with:
   - `coding_agents: HashMap<String, CodingAgentDef>`
   - `default_coding_agent: String`
3. Update `TeamEntry` with `coding_agent: Option<String>` override field
4. Implement `resolve_coding_agent(team: &TeamEntry, manifest: &ProfileManifest) -> Result<&CodingAgentDef>`:
   - Team override present -> look up in manifest's `coding_agents`
   - No override -> use manifest's `default_coding_agent`
   - Error if resolved agent not found in map

## Dependencies
- Step 1 complete (agent_tags module exists, but no runtime dependency)

## Implementation Approach
1. Study existing `ProfileManifest` and `TeamEntry` structs in `profile.rs` / `config.rs`
2. Add `CodingAgentDef` struct with serde derives
3. Add new fields to `ProfileManifest` (ensure backwards-compatible deserialization)
4. Add optional field to `TeamEntry`
5. Implement resolve function with clear error messages
6. Write unit tests for all resolution paths

## Acceptance Criteria

1. **CodingAgentDef deserializes from YAML**
   - Given a YAML snippet with name, display_name, context_file, agent_dir, binary
   - When deserialized into `CodingAgentDef`
   - Then all fields are populated correctly

2. **ProfileManifest parses with new fields**
   - Given a profile `botminter.yml` with `coding_agents` map and `default_coding_agent`
   - When parsed into `ProfileManifest`
   - Then the coding_agents map contains the expected entries

3. **Resolve returns profile default when no team override**
   - Given a `TeamEntry` with `coding_agent: None` and a manifest with `default_coding_agent: "claude-code"`
   - When `resolve_coding_agent()` is called
   - Then it returns the `claude-code` `CodingAgentDef`

4. **Resolve returns team override when present**
   - Given a `TeamEntry` with `coding_agent: Some("gemini-cli")` and a manifest with both agents
   - When `resolve_coding_agent()` is called
   - Then it returns the `gemini-cli` `CodingAgentDef`

5. **Resolve errors on unknown agent**
   - Given a `TeamEntry` with `coding_agent: Some("unknown-agent")`
   - When `resolve_coding_agent()` is called
   - Then it returns an error with a helpful message

6. **Existing integration tests still pass**
   - Given the updated `ProfileManifest` with new optional fields
   - When the existing test suite is run
   - Then all tests pass (new fields are additive / have defaults)

## Metadata
- **Complexity**: Medium
- **Labels**: coding-agent-agnostic, data-model, sprint-1
- **Required Skills**: Rust, serde, YAML
