---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Create profile hat reviewer skill

## Description
Create a skill that validates generated profile hats against botminter's conventions and catches the class of bugs found during compact profile testing. This reviewer can be used manually (as a Claude Code skill) or integrated as a Ralph hat within botminter's own development workflow to validate profiles before they ship.

## Background
The compact profile shipped with several bugs that weren't caught by tests:
- `LOOP_COMPLETE` used as `default_publishes` and `completion_promise`
- `just -f .botminter/Justfile sync` referencing a non-existent file
- `gh repo view` without fallback for local remotes
- Team repo initialized on `master` instead of `main`

These are all violations of botminter's design decisions that could have been caught by automated review. The existing test suite validates profile extraction mechanics (files copied, schemas match) but has zero coverage for profile content — what's actually written inside hat instructions, YAML config values, and shell commands. A botminter-specific reviewer skill fills this gap.

## Reference Documentation
**Required:**
- `profiles/compact/members/superman/ralph.yml` — the file that had bugs (reference for what to check)
- `profiles/compact/PROCESS.md` — process conventions to validate against

**Additional References:**
- Task 5's output (profile hat generator skill) — the reviewer should validate against the same conventions the generator encodes
- `profiles/compact/agent/skills/gh/SKILL.md` — gh CLI patterns to validate

## Technical Requirements
1. Create `skills/review-bm-profile-hats/SKILL.md` (or similar location)
2. Define a checklist of validations organized by category:
   - **YAML structure**: valid YAML, required hat fields present, no disallowed config values
   - **Event flow**: triggers map to exactly one hat, no dangling events, no LOOP_COMPLETE
   - **Instructions content**: no references to non-existent files, valid shell commands, proper `gh` CLI patterns
   - **Conventions**: correct comment format, knowledge path references, status label names match PROCESS.md
   - **Git operations**: no bare `git init`, proper branch assumptions
3. Provide a structured output format (pass/fail per check, with file:line references for failures)
4. Include remediation guidance for each check (what to fix and how)
5. Design for integration as a Ralph hat — the reviewer can be triggered during profile development

## Dependencies
- Task 5 (profile hat generator) — the reviewer validates the same conventions the generator encodes

## Implementation Approach
1. Catalog all bugs found during compact profile testing as validation rules
2. Generalize each bug into a category of check (e.g., "non-existent file reference" → "instruction text references valid paths")
3. Organize checks into a structured review checklist
4. Define the output format (similar to a linting report)
5. Include examples of passing and failing profiles
6. Document how to integrate as a Ralph hat for automated validation

## Acceptance Criteria

1. **Skill document exists and is well-structured**
   - Given the skill directory
   - When reading SKILL.md
   - Then it follows the standard skill format with clear validation categories

2. **All known bugs are covered by checks**
   - Given the validation checklist
   - When reviewing against the bugs found in compact profile testing
   - Then each bug maps to at least one check:
     - LOOP_COMPLETE in config values → "no disallowed config values" check
     - Justfile reference → "instruction references valid paths" check
     - gh repo view without fallback → "gh CLI patterns" check
     - master vs main → "git branch assumptions" check
     - Local file path remote → "git remote URL" check

3. **Output format is actionable**
   - Given a validation run output
   - When reading the report
   - Then each failure includes: check name, severity, file:line location, what's wrong, how to fix

4. **Checklist is comprehensive beyond known bugs**
   - Given the validation checklist
   - When reviewing for completeness
   - Then it also checks: comment format consistency, knowledge path validity, status label names match PROCESS.md, event flow graph connectivity

5. **Integration path documented**
   - Given the skill document
   - When looking for Ralph integration instructions
   - Then it describes how to use the reviewer as a Ralph hat in botminter's development workflow

## Metadata
- **Complexity**: High
- **Labels**: feature, skill, profile, tooling, quality
- **Required Skills**: Markdown, YAML, botminter architecture, linting patterns
