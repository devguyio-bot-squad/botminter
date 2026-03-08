---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Extract Ralph Prompts to Profiles

## Description
Extract Ralph Orchestrator's hardcoded system prompts into `ralph-prompts/` within each profile. These are reference copies — Ralph still uses its compiled-in versions. The profile copies enable `bm chat` (Step 17) to reconstruct similar context without Ralph at runtime. Content-only step, no CLI code changes.

## Background
Ralph Orchestrator injects system prompts (guardrails, orientation, hat templates, workflows, etc.) into each coding agent session. For `bm chat` to create interactive sessions that feel like talking to a Ralph-managed member, it needs access to these prompts. Shipping them in profiles makes them available at the profile level.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Skills Extraction", Sprint 4 table)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 14)

**Additional References:**
- Research: specs/milestones/minty-and-friends-rapid/research/ralph-injected-prompts.md

**Note:** Read the design document and research before beginning implementation.

## Technical Requirements
1. Create `ralph-prompts/` directory in each profile
2. Extract and write the following files (content from Ralph's Rust codebase):
   - `ralph-prompts/guardrails.md` — from `hatless_ralph.rs` guardrails
   - `ralph-prompts/orientation.md` — from `hatless_ralph.rs` orientation
   - `ralph-prompts/hat-template.md` — from `instructions.rs` hat template
   - `ralph-prompts/reference/workflows.md` — workflow variants
   - `ralph-prompts/reference/event-writing.md` — event mechanics
   - `ralph-prompts/reference/completion.md` — completion mechanics
   - `ralph-prompts/reference/ralph-tools.md` — from `data/ralph-tools.md`
   - `ralph-prompts/reference/robot-interaction.md` — from `data/robot-interaction-skill.md`
3. Content must accurately represent what Ralph injects (compare against Ralph source)
4. These are static files — no code changes to the `bm` CLI

## Dependencies
- Steps 1-9 complete (profiles on disk, extraction working)

## Implementation Approach
1. Read the research document on Ralph's injected prompts
2. Read Ralph Orchestrator source files to extract prompt content
3. Create `ralph-prompts/` and `ralph-prompts/reference/` in each profile
4. Write extracted content, preserving markdown formatting
5. Verify completeness against the design's extraction table
6. Test that `bm profiles init` extracts the new files

## Acceptance Criteria

1. **ralph-prompts/ exists in each profile**
   - Given each profile directory
   - When listing contents
   - Then `ralph-prompts/` directory exists

2. **Core prompt files present**
   - Given `ralph-prompts/`
   - When listing files
   - Then `guardrails.md`, `orientation.md`, `hat-template.md` exist

3. **Reference subdirectory populated**
   - Given `ralph-prompts/reference/`
   - When listing files
   - Then `workflows.md`, `event-writing.md`, `completion.md`, `ralph-tools.md`, `robot-interaction.md` exist

4. **Content matches Ralph source**
   - Given each extracted prompt file
   - When compared to the corresponding Ralph source
   - Then the content accurately represents what Ralph injects

5. **bm profiles init extracts ralph-prompts**
   - Given `bm profiles init` running
   - When profiles are extracted to disk
   - Then `ralph-prompts/` is included in the extraction

## Metadata
- **Complexity**: Medium
- **Labels**: skills-extraction, content, sprint-4
- **Required Skills**: Ralph Orchestrator knowledge, Markdown, source reading
