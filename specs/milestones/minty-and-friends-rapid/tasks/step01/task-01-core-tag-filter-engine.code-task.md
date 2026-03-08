---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Core Tag Filter Engine

## Description
Build the core line-based filter that processes `+agent:NAME` / `-agent` inline tags. This is the foundational building block for the entire coding-agent-agnostic architecture — every subsequent extraction and workspace operation depends on it.

## Background
BotMinter profiles contain files with content that may be agent-specific (Claude Code, Gemini CLI, etc.) or shared across all agents. Inline tags like `<!-- +agent:claude-code -->` (HTML) or `# +agent:claude-code` (Hash) delimit agent-specific sections. A filter function strips the tags and includes/excludes sections based on the resolved coding agent.

The filter is a simple state machine: content outside any tag pair is always included; content inside a `+agent:NAME` / `-agent` pair is included only when `NAME` matches the target agent. Tags are flat (no nesting) and the tag lines themselves are always stripped from output.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Coding-Agent-Agnostic" section)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 1)

**Additional References:**
- Research: specs/milestones/minty-and-friends-rapid/research/claude-code-coupling-audit.md

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Create a new module `agent_tags.rs` in `crates/bm/src/` and register it in `lib.rs`
2. Define `CommentSyntax` enum with variants `Html` (`<!-- -->`) and `Hash` (`#`)
3. Implement `filter_agent_tags(content: &str, agent: &str, comment_syntax: CommentSyntax) -> String`:
   - Line-by-line processing with a boolean state tracking inclusion
   - Default state: included (content outside any tag pair)
   - On encountering `+agent:NAME` open tag: set included = (NAME == agent), strip tag line
   - On encountering `-agent` close tag: reset included = true, strip tag line
   - Tag detection must account for comment syntax: `<!-- +agent:NAME -->` for Html, `# +agent:NAME` for Hash
   - No nesting support — tags are flat open/close pairs
4. Tag line patterns to match:
   - Html open: line trimmed matches `<!-- +agent:<name> -->`
   - Html close: line trimmed matches `<!-- -agent -->`
   - Hash open: line trimmed matches `# +agent:<name>`
   - Hash close: line trimmed matches `# -agent`
5. Preserve original line endings and whitespace for non-tag lines
6. Empty input returns empty string

## Dependencies
- No external crate dependencies — this is pure string processing
- Existing crate structure in `crates/bm/src/lib.rs` for module registration

## Implementation Approach
1. Define `CommentSyntax` enum (derive `Debug`, `Clone`, `Copy`, `PartialEq`)
2. Implement tag-line detection helpers (private functions that check if a line is an open/close tag for a given syntax)
3. Implement `filter_agent_tags()` as a line iterator with fold or manual loop tracking `included` state
4. Add `#[cfg(test)] mod tests` block with comprehensive unit tests

## Acceptance Criteria

1. **Common-only content passes through unchanged**
   - Given a file with no agent tags
   - When `filter_agent_tags()` is called with any agent name
   - Then the output equals the input exactly

2. **Matching agent sections are included**
   - Given a file with `+agent:claude-code` / `-agent` wrapping some lines
   - When filtered for agent `"claude-code"`
   - Then the wrapped content is included and tag lines are stripped

3. **Non-matching agent sections are excluded**
   - Given a file with `+agent:gemini-cli` / `-agent` wrapping some lines
   - When filtered for agent `"claude-code"`
   - Then the wrapped content is excluded and tag lines are stripped

4. **Multiple agent blocks in one file**
   - Given a file with interleaved `claude-code` and `gemini-cli` sections plus common content
   - When filtered for `"claude-code"`
   - Then only common content and `claude-code` sections remain, all tag lines stripped

5. **Empty file returns empty string**
   - Given an empty string input
   - When `filter_agent_tags()` is called
   - Then the output is an empty string

6. **YAML files with tagged duplicate keys produce valid single-key output**
   - Given YAML content with two `backend:` lines each inside different agent tags
   - When filtered for one agent using `Hash` syntax
   - Then the output contains exactly one `backend:` line

7. **HTML comment syntax for .md files**
   - Given markdown content with `<!-- +agent:claude-code -->` / `<!-- -agent -->` tags
   - When filtered with `CommentSyntax::Html`
   - Then agent-specific sections are correctly included/excluded

8. **Hash comment syntax for .yml and .sh files**
   - Given YAML or shell content with `# +agent:claude-code` / `# -agent` tags
   - When filtered with `CommentSyntax::Hash`
   - Then agent-specific sections are correctly included/excluded

9. **Tag lines are always stripped**
   - Given any input with agent tags
   - When filtered for any agent
   - Then no `+agent:` or `-agent` lines appear in the output

10. **Unit tests pass**
    - Given the test suite in `agent_tags.rs`
    - When `cargo test -p bm agent_tags` is run
    - Then all tests pass

## Metadata
- **Complexity**: Medium
- **Labels**: coding-agent-agnostic, library, sprint-1
- **Required Skills**: Rust, state machines, string processing
