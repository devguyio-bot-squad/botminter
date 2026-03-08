//! Line-based filter for `+agent:NAME` / `-agent` inline tags.
//!
//! Profile files contain sections that are agent-specific (Claude Code, Gemini CLI, etc.)
//! or shared across all agents. This module filters content based on the resolved coding
//! agent, stripping tag lines and including/excluding agent-specific blocks.

/// Comment syntax used to detect agent tag lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentSyntax {
    /// HTML-style: `<!-- +agent:NAME -->` / `<!-- -agent -->`
    Html,
    /// Hash-style: `# +agent:NAME` / `# -agent`
    Hash,
}

use std::path::Path;

/// Detect the comment syntax for a file based on its extension.
///
/// Returns `Html` for `.md` and `.html` files, `Hash` for everything else
/// (`.yml`, `.yaml`, `.sh`, and unknown extensions).
pub fn detect_comment_syntax(filename: &str) -> CommentSyntax {
    match Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("md" | "html") => CommentSyntax::Html,
        _ => CommentSyntax::Hash,
    }
}

/// Convenience wrapper: detect comment syntax from filename, then filter.
///
/// Equivalent to calling `detect_comment_syntax` + `filter_agent_tags`.
pub fn filter_file(content: &str, filename: &str, agent: &str) -> String {
    filter_agent_tags(content, agent, detect_comment_syntax(filename))
}

/// Parse an open tag line (`+agent:NAME`), returning the agent name if matched.
fn parse_open_tag(line: &str, syntax: CommentSyntax) -> Option<&str> {
    let trimmed = line.trim();
    match syntax {
        CommentSyntax::Html => {
            let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
            inner.strip_prefix("+agent:")
        }
        CommentSyntax::Hash => {
            let inner = trimmed.strip_prefix('#')?.trim();
            inner.strip_prefix("+agent:")
        }
    }
}

/// Check if a line is a close tag (`-agent`).
fn is_close_tag(line: &str, syntax: CommentSyntax) -> bool {
    let trimmed = line.trim();
    match syntax {
        CommentSyntax::Html => {
            trimmed
                .strip_prefix("<!--")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim() == "-agent")
                .unwrap_or(false)
        }
        CommentSyntax::Hash => {
            trimmed
                .strip_prefix('#')
                .map(|s| s.trim() == "-agent")
                .unwrap_or(false)
        }
    }
}

/// Filter content by agent tag, including only sections matching the target agent.
///
/// Content outside any tag pair is always included ("common" content). Content inside
/// a `+agent:NAME` / `-agent` pair is included only when `NAME` matches `agent`.
/// Tag lines themselves are always stripped from the output.
///
/// # Arguments
/// * `content` — the full file content to filter
/// * `agent` — the target coding agent name (e.g., `"claude-code"`)
/// * `comment_syntax` — the comment syntax to use for tag detection
///
/// # Returns
/// The filtered content with tag lines removed.
pub fn filter_agent_tags(content: &str, agent: &str, comment_syntax: CommentSyntax) -> String {
    if content.is_empty() {
        return String::new();
    }

    let mut output = String::with_capacity(content.len());
    let mut included = true;

    for line in content.lines() {
        if let Some(tag_agent) = parse_open_tag(line, comment_syntax) {
            included = tag_agent == agent;
            continue; // strip tag line
        }
        if is_close_tag(line, comment_syntax) {
            included = true;
            continue; // strip tag line
        }
        if included {
            output.push_str(line);
            output.push('\n');
        }
    }

    // Preserve lack of trailing newline if original didn't have one
    if !content.ends_with('\n') && output.ends_with('\n') {
        output.pop();
    }

    output
}

/// Collects all distinct agent names referenced by `+agent:NAME` tags in the content.
///
/// Returns an empty set if the content has no agent tags.
pub fn collect_agent_names(content: &str, syntax: CommentSyntax) -> std::collections::BTreeSet<String> {
    let mut agents = std::collections::BTreeSet::new();
    for line in content.lines() {
        if let Some(name) = parse_open_tag(line, syntax) {
            agents.insert(name.to_string());
        }
    }
    agents
}

/// Returns `true` if all agent tags in the content are balanced
/// (every `+agent:NAME` has a matching `-agent`).
pub fn tags_are_balanced(content: &str, syntax: CommentSyntax) -> bool {
    let mut depth = 0i32;
    for line in content.lines() {
        if parse_open_tag(line, syntax).is_some() {
            depth += 1;
        }
        if is_close_tag(line, syntax) {
            depth -= 1;
        }
        if depth < 0 {
            return false; // close without open
        }
    }
    depth == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(filter_agent_tags("", "claude-code", CommentSyntax::Html), "");
        assert_eq!(filter_agent_tags("", "claude-code", CommentSyntax::Hash), "");
    }

    #[test]
    fn no_tags_passes_through_unchanged() {
        let input = "Line one\nLine two\nLine three\n";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            input
        );
        assert_eq!(
            filter_agent_tags(input, "gemini-cli", CommentSyntax::Hash),
            input
        );
    }

    #[test]
    fn no_tags_preserves_no_trailing_newline() {
        let input = "Line one\nLine two";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            input
        );
    }

    #[test]
    fn matching_agent_section_included_html() {
        let input = "\
Common line
<!-- +agent:claude-code -->
Claude-specific line
<!-- -agent -->
Another common line
";
        let expected = "\
Common line
Claude-specific line
Another common line
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            expected
        );
    }

    #[test]
    fn non_matching_agent_section_excluded_html() {
        let input = "\
Common line
<!-- +agent:gemini-cli -->
Gemini-specific line
<!-- -agent -->
Another common line
";
        let expected = "\
Common line
Another common line
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            expected
        );
    }

    #[test]
    fn matching_agent_section_included_hash() {
        let input = "\
common: true
# +agent:claude-code
backend: claude
# -agent
other: value
";
        let expected = "\
common: true
backend: claude
other: value
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Hash),
            expected
        );
    }

    #[test]
    fn non_matching_agent_section_excluded_hash() {
        let input = "\
common: true
# +agent:gemini-cli
backend: gemini
# -agent
other: value
";
        let expected = "\
common: true
other: value
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Hash),
            expected
        );
    }

    #[test]
    fn multiple_agent_blocks_interleaved() {
        let input = "\
Common header
<!-- +agent:claude-code -->
Claude section 1
<!-- -agent -->
Common middle
<!-- +agent:gemini-cli -->
Gemini section
<!-- -agent -->
Common footer
<!-- +agent:claude-code -->
Claude section 2
<!-- -agent -->
";
        let expected = "\
Common header
Claude section 1
Common middle
Common footer
Claude section 2
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            expected
        );
    }

    #[test]
    fn yaml_duplicate_keys_resolved() {
        let input = "\
cli:
  # +agent:claude-code
  backend: claude
  # -agent
  # +agent:gemini-cli
  backend: gemini
  # -agent
  timeout: 30
";
        let claude_expected = "\
cli:
  backend: claude
  timeout: 30
";
        let gemini_expected = "\
cli:
  backend: gemini
  timeout: 30
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Hash),
            claude_expected
        );
        assert_eq!(
            filter_agent_tags(input, "gemini-cli", CommentSyntax::Hash),
            gemini_expected
        );
    }

    #[test]
    fn tag_lines_always_stripped() {
        let input = "\
Before
<!-- +agent:claude-code -->
Inside
<!-- -agent -->
After
";
        let output = filter_agent_tags(input, "claude-code", CommentSyntax::Html);
        assert!(!output.contains("+agent:"));
        assert!(!output.contains("-agent"));
    }

    #[test]
    fn tag_lines_stripped_even_when_not_matching() {
        let input = "\
Before
<!-- +agent:gemini-cli -->
Inside
<!-- -agent -->
After
";
        let output = filter_agent_tags(input, "claude-code", CommentSyntax::Html);
        assert!(!output.contains("+agent:"));
        assert!(!output.contains("-agent"));
    }

    #[test]
    fn whitespace_around_tags_tolerated() {
        let input = "\
Before
  <!-- +agent:claude-code -->
Claude content
  <!-- -agent -->
After
";
        let expected = "\
Before
Claude content
After
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            expected
        );
    }

    #[test]
    fn hash_tag_whitespace_around_tags_tolerated() {
        let input = "\
before: true
  # +agent:claude-code
backend: claude
  # -agent
after: true
";
        let expected = "\
before: true
backend: claude
after: true
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Hash),
            expected
        );
    }

    #[test]
    fn only_common_content_when_no_agent_matches() {
        let input = "\
Common
<!-- +agent:gemini-cli -->
Gemini only
<!-- -agent -->
<!-- +agent:aider -->
Aider only
<!-- -agent -->
More common
";
        let expected = "\
Common
More common
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            expected
        );
    }

    #[test]
    fn file_ending_inside_tag_block() {
        // Edge case: file ends without a close tag — the unclosed block
        // is still filtered by the current state (included or not)
        let input = "\
Common
<!-- +agent:gemini-cli -->
Gemini content";
        let expected = "Common";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            expected
        );
    }

    #[test]
    fn multiline_agent_block_preserves_content() {
        let input = "\
# Header
<!-- +agent:claude-code -->
Line 1
Line 2
Line 3
<!-- -agent -->
# Footer
";
        let expected = "\
# Header
Line 1
Line 2
Line 3
# Footer
";
        assert_eq!(
            filter_agent_tags(input, "claude-code", CommentSyntax::Html),
            expected
        );
    }

    // --- detect_comment_syntax tests ---

    #[test]
    fn detect_markdown_as_html() {
        assert_eq!(detect_comment_syntax("context.md"), CommentSyntax::Html);
        assert_eq!(detect_comment_syntax("README.md"), CommentSyntax::Html);
    }

    #[test]
    fn detect_html_as_html() {
        assert_eq!(detect_comment_syntax("page.html"), CommentSyntax::Html);
    }

    #[test]
    fn detect_yaml_as_hash() {
        assert_eq!(detect_comment_syntax("ralph.yml"), CommentSyntax::Hash);
        assert_eq!(detect_comment_syntax("config.yaml"), CommentSyntax::Hash);
    }

    #[test]
    fn detect_shell_as_hash() {
        assert_eq!(detect_comment_syntax("setup.sh"), CommentSyntax::Hash);
    }

    #[test]
    fn detect_unknown_defaults_to_hash() {
        assert_eq!(detect_comment_syntax("Makefile"), CommentSyntax::Hash);
        assert_eq!(detect_comment_syntax("script.py"), CommentSyntax::Hash);
    }

    #[test]
    fn detect_with_directory_path() {
        assert_eq!(
            detect_comment_syntax("profiles/scrum/context.md"),
            CommentSyntax::Html
        );
        assert_eq!(
            detect_comment_syntax("team/member/ralph.yml"),
            CommentSyntax::Hash
        );
    }

    #[test]
    fn detect_case_insensitive() {
        assert_eq!(detect_comment_syntax("FILE.MD"), CommentSyntax::Html);
        assert_eq!(detect_comment_syntax("FILE.HTML"), CommentSyntax::Html);
        assert_eq!(detect_comment_syntax("FILE.YML"), CommentSyntax::Hash);
    }

    // --- tags_are_balanced tests ---

    #[test]
    fn balanced_html_tags() {
        let input = "\
Common
<!-- +agent:claude-code -->
Claude only
<!-- -agent -->
Footer
";
        assert!(tags_are_balanced(input, CommentSyntax::Html));
    }

    #[test]
    fn unbalanced_missing_close_html() {
        let input = "\
Common
<!-- +agent:claude-code -->
Claude only
Footer
";
        assert!(!tags_are_balanced(input, CommentSyntax::Html));
    }

    #[test]
    fn unbalanced_extra_close_html() {
        let input = "\
Common
<!-- -agent -->
Footer
";
        assert!(!tags_are_balanced(input, CommentSyntax::Html));
    }

    #[test]
    fn balanced_hash_tags() {
        let input = "\
common: true
# +agent:claude-code
backend: claude
# -agent
";
        assert!(tags_are_balanced(input, CommentSyntax::Hash));
    }

    #[test]
    fn no_tags_is_balanced() {
        assert!(tags_are_balanced("Just plain text\n", CommentSyntax::Html));
        assert!(tags_are_balanced("key: value\n", CommentSyntax::Hash));
    }

    #[test]
    fn multiple_balanced_blocks() {
        let input = "\
<!-- +agent:claude-code -->
A
<!-- -agent -->
<!-- +agent:gemini-cli -->
B
<!-- -agent -->
";
        assert!(tags_are_balanced(input, CommentSyntax::Html));
    }

    // --- collect_agent_names tests ---

    #[test]
    fn collect_no_tags_returns_empty() {
        let agents = collect_agent_names("Just plain text\n", CommentSyntax::Html);
        assert!(agents.is_empty());
    }

    #[test]
    fn collect_single_agent_html() {
        let input = "\
Common
<!-- +agent:claude-code -->
Claude
<!-- -agent -->
";
        let agents = collect_agent_names(input, CommentSyntax::Html);
        assert_eq!(agents.len(), 1);
        assert!(agents.contains("claude-code"));
    }

    #[test]
    fn collect_multiple_agents_html() {
        let input = "\
<!-- +agent:claude-code -->
A
<!-- -agent -->
<!-- +agent:gemini-cli -->
B
<!-- -agent -->
";
        let agents = collect_agent_names(input, CommentSyntax::Html);
        assert_eq!(agents.len(), 2);
        assert!(agents.contains("claude-code"));
        assert!(agents.contains("gemini-cli"));
    }

    #[test]
    fn collect_deduplicates_repeated_agent() {
        let input = "\
<!-- +agent:claude-code -->
A
<!-- -agent -->
<!-- +agent:claude-code -->
B
<!-- -agent -->
";
        let agents = collect_agent_names(input, CommentSyntax::Html);
        assert_eq!(agents.len(), 1);
        assert!(agents.contains("claude-code"));
    }

    #[test]
    fn collect_agents_hash_syntax() {
        let input = "\
# +agent:claude-code
backend: claude
# -agent
";
        let agents = collect_agent_names(input, CommentSyntax::Hash);
        assert_eq!(agents.len(), 1);
        assert!(agents.contains("claude-code"));
    }

    // --- filter_file convenience wrapper tests ---

    #[test]
    fn filter_file_markdown_end_to_end() {
        let input = "\
Common line
<!-- +agent:claude-code -->
Claude-specific
<!-- -agent -->
<!-- +agent:gemini-cli -->
Gemini-specific
<!-- -agent -->
Footer
";
        let expected = "\
Common line
Claude-specific
Footer
";
        assert_eq!(filter_file(input, "context.md", "claude-code"), expected);
    }

    #[test]
    fn filter_file_yaml_end_to_end() {
        let input = "\
cli:
  # +agent:claude-code
  backend: claude
  # -agent
  # +agent:gemini-cli
  backend: gemini
  # -agent
  timeout: 30
";
        let expected = "\
cli:
  backend: claude
  timeout: 30
";
        assert_eq!(filter_file(input, "ralph.yml", "claude-code"), expected);
    }
}
