---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Agent Tag Insertion and Validation

## Description
Add inline `+agent:NAME` / `-agent` tags to profile content files that contain Claude Code-specific sections. Validate that all tags are balanced and that filtering for `claude-code` produces content identical to the original pre-rename files.

## Background
After renaming `CLAUDE.md` to `context.md`, the file becomes agent-agnostic by name but may still contain Claude Code-specific content (e.g., Claude Code slash commands, `.claude/` directory references). These sections need `<!-- +agent:claude-code -->` / `<!-- -agent -->` tags so the extraction pipeline can include/exclude them based on the resolved agent.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Inline agent tags")
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 3)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Audit all `context.md` files for Claude Code-specific content and wrap with HTML-style tags:
   - `<!-- +agent:claude-code -->` before Claude-specific sections
   - `<!-- -agent -->` after Claude-specific sections
   - Common (agent-agnostic) content remains untagged
2. Audit `ralph.yml` files for agent-specific config and wrap with hash-style tags:
   - `# +agent:claude-code` / `# -agent` around `cli.backend: claude` and similar
3. Audit shell scripts for agent-specific commands and tag similarly
4. Ensure all tags are balanced (every open has a close)
5. Verify: filtering each tagged file for `claude-code` produces output identical to the original content

## Dependencies
- Task 1 of this step (renames complete, files are now `context.md`)
- Step 1 (agent_tags filter module for validation)

## Implementation Approach
1. Read each profile's original `CLAUDE.md` (from git history) for comparison reference
2. Identify Claude Code-specific sections in each `context.md`
3. Insert tags around those sections
4. Write validation tests using the Step 1 filter to verify round-trip correctness
5. Check `ralph.yml` files for agent-specific config sections

## Acceptance Criteria

1. **context.md files have balanced tags**
   - Given each profile's `context.md`
   - When scanning for `+agent:` and `-agent` tags
   - Then every open tag has a matching close tag

2. **Filtering for claude-code reproduces original content**
   - Given a `context.md` with agent tags
   - When filtered with `filter_agent_tags(content, "claude-code", Html)`
   - Then the output matches the original `CLAUDE.md` content (minus tag lines)

3. **ralph.yml files with tags produce valid YAML**
   - Given a `ralph.yml` with hash-style agent tags
   - When filtered with `filter_agent_tags(content, "claude-code", Hash)`
   - Then the result is valid YAML with single-key entries (no duplicates)

4. **Common content is untagged**
   - Given each `context.md`
   - When inspecting content outside agent tag blocks
   - Then it contains only agent-agnostic content

5. **Non-matching agent filter excludes tagged sections**
   - Given a `context.md` with `claude-code` tagged sections
   - When filtered with `filter_agent_tags(content, "gemini-cli", Html)`
   - Then Claude Code-specific sections are excluded

6. **Shell scripts tagged correctly**
   - Given any shell scripts with agent-specific commands
   - When filtered for `claude-code`
   - Then agent-specific commands are included with tags stripped

## Metadata
- **Complexity**: Medium
- **Labels**: coding-agent-agnostic, migration, sprint-1
- **Required Skills**: Markdown, YAML, agent tag format knowledge
