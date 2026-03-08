---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Syntax Detection + Convenience Wrapper

## Description
Add filename-based comment syntax detection and a convenience wrapper that combines detection with filtering. This completes the `agent_tags` module's public API, making it ready for use by the extraction pipeline in Step 4.

## Background
Different file types use different comment syntaxes for agent tags: HTML-style comments (`<!-- -->`) for Markdown and HTML files, hash comments (`#`) for YAML, shell scripts, and others. The detection function maps file extensions to the correct syntax, and the convenience wrapper combines detection + filtering into a single call for callers who just want to filter a file by name.

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Coding-Agent-Agnostic" section)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 1)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Implement `detect_comment_syntax(filename: &str) -> CommentSyntax` in `agent_tags.rs`:
   - `.md`, `.html` -> `CommentSyntax::Html`
   - `.yml`, `.yaml`, `.sh` -> `CommentSyntax::Hash`
   - Default (unknown extensions) -> `CommentSyntax::Hash`
2. Implement `filter_file(content: &str, filename: &str, agent: &str) -> String`:
   - Calls `detect_comment_syntax(filename)` then `filter_agent_tags(content, agent, syntax)`
   - This is a thin convenience wrapper — no additional logic
3. Both functions should be `pub` (part of the module's public API)
4. Extension matching should be case-insensitive and handle filenames with paths (extract extension from the final component)

## Dependencies
- Task 1 (core tag filter engine) — depends on `CommentSyntax` enum and `filter_agent_tags()` function

## Implementation Approach
1. Add `detect_comment_syntax()` using `Path::extension()` or manual string splitting
2. Add `filter_file()` as a two-line wrapper
3. Add unit tests for detection across all specified extensions
4. Add integration-style tests that exercise `filter_file()` end-to-end with realistic file content

## Acceptance Criteria

1. **Markdown files detected as HTML syntax**
   - Given filename `"context.md"` or `"README.md"`
   - When `detect_comment_syntax()` is called
   - Then it returns `CommentSyntax::Html`

2. **HTML files detected as HTML syntax**
   - Given filename `"page.html"`
   - When `detect_comment_syntax()` is called
   - Then it returns `CommentSyntax::Html`

3. **YAML files detected as Hash syntax**
   - Given filename `"ralph.yml"` or `"config.yaml"`
   - When `detect_comment_syntax()` is called
   - Then it returns `CommentSyntax::Hash`

4. **Shell scripts detected as Hash syntax**
   - Given filename `"setup.sh"`
   - When `detect_comment_syntax()` is called
   - Then it returns `CommentSyntax::Hash`

5. **Unknown extensions default to Hash**
   - Given filename `"Makefile"` or `"script.py"`
   - When `detect_comment_syntax()` is called
   - Then it returns `CommentSyntax::Hash`

6. **Paths with directories handled correctly**
   - Given filename `"profiles/scrum/context.md"` (path, not just filename)
   - When `detect_comment_syntax()` is called
   - Then it returns `CommentSyntax::Html` (extracts extension from final component)

7. **filter_file convenience wrapper works end-to-end**
   - Given markdown content with HTML-style agent tags and filename `"context.md"`
   - When `filter_file(content, "context.md", "claude-code")` is called
   - Then matching sections are included, non-matching excluded, tags stripped (same as calling detect + filter manually)

8. **filter_file with YAML content**
   - Given YAML content with hash-style agent tags and filename `"ralph.yml"`
   - When `filter_file(content, "ralph.yml", "claude-code")` is called
   - Then the output is correctly filtered using Hash syntax

9. **Unit tests pass**
   - Given the full test suite in `agent_tags.rs`
   - When `cargo test -p bm agent_tags` is run
   - Then all tests pass (including tests from Task 1)

## Metadata
- **Complexity**: Low
- **Labels**: coding-agent-agnostic, library, sprint-1
- **Required Skills**: Rust, file path handling
