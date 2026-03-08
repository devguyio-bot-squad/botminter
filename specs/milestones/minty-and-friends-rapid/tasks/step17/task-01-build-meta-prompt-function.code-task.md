---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: build_meta_prompt() Function

## Description
Implement `build_meta_prompt()` as a standalone, testable function that assembles a meta-prompt from Ralph prompts, guardrails, hat instructions, and PROMPT.md content. Supports both hatless mode (all hats) and hat-specific mode (single hat).

## Background
`bm chat` needs to recreate a Ralph-like experience without running Ralph itself. The meta-prompt combines the member's identity, guardrails, hat instructions, role context, and reference materials into a single system prompt that primes the coding agent for an interactive session.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "bm chat" and meta-prompt template)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 17)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Implement `build_meta_prompt()` as a pure function (string inputs -> string output):
   - Inputs: member name, role name, team name, guardrails content, orientation content, hat instructions (map of hat name -> instructions), PROMPT.md content, reference dir path
   - Output: assembled markdown meta-prompt
2. Meta-prompt template follows the design:
   ```
   # Interactive Session — [Role Name]
   You are [member name], a [role] on the [team name] team.
   ...
   ## Your Capabilities
   [Hat instructions]
   ## Guardrails
   [From ralph.yml]
   ## Role Context
   [PROMPT.md content]
   ## Reference: Operation Mode
   [Path to ralph-prompts/reference/]
   ```
3. Hatless mode: include ALL hats' instructions under "Your Capabilities"
4. Hat-specific mode: include only the specified hat's instructions
5. Reference materials are paths (not inlined) — they're informational, not directives

## Dependencies
- Step 14 (Ralph prompts shipped in profiles)
- Step 16 (Team Manager exists as a concrete member to test with)

## Implementation Approach
1. Create a new module or add to an existing one (e.g., `chat.rs`)
2. Define the function with clear input types
3. Implement template assembly with string formatting
4. Write comprehensive unit tests for both modes
5. Test with realistic content (Team Manager hat instructions)

## Acceptance Criteria

1. **Meta-prompt contains role identity**
   - Given member "bob", role "team-manager", team "my-team"
   - When `build_meta_prompt()` is called
   - Then the output contains "You are bob, a team-manager on the my-team team"

2. **Guardrails included**
   - Given guardrails content
   - When `build_meta_prompt()` is called
   - Then the output contains a "## Guardrails" section with the guardrails content

3. **PROMPT.md content included**
   - Given PROMPT.md content
   - When `build_meta_prompt()` is called
   - Then the output contains a "## Role Context" section with the PROMPT content

4. **Hatless mode includes all hats**
   - Given multiple hats (executor, reviewer)
   - When `build_meta_prompt()` is called without a specific hat
   - Then all hats' instructions appear under "## Your Capabilities"

5. **Hat-specific mode includes only one hat**
   - Given hat "executor" specified
   - When `build_meta_prompt()` is called with hat="executor"
   - Then only the executor hat's instructions appear

6. **Reference materials are paths, not inlined**
   - Given a reference dir path
   - When `build_meta_prompt()` is called
   - Then reference materials appear as paths/pointers, not full content

7. **Output is well-formed markdown**
   - Given valid inputs
   - When `build_meta_prompt()` is called
   - Then the output is valid markdown with proper heading hierarchy

## Metadata
- **Complexity**: Medium
- **Labels**: bm-chat, core, sprint-5
- **Required Skills**: Rust, string formatting, markdown generation
