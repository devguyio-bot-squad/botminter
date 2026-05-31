//! Documentation verification tests for AC-30
//!
//! These tests verify that session model documentation is complete and
//! that obsolete references (bm teams sync) have been removed.

use std::fs;
use std::path::{Path, PathBuf};

/// Root directory for the project (assumes tests run from crates/bm/)
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Recursively find all markdown files in a directory
fn find_markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if !dir.exists() {
        return files;
    }

    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            files.push(path.to_path_buf());
        }
    }

    files
}

/// Check if a file contains a pattern
fn file_contains_pattern(path: &Path, pattern: &str) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        content.contains(pattern)
    } else {
        false
    }
}

/// Find all occurrences of a pattern in documentation files
fn find_pattern_in_docs(pattern: &str, exclude_paths: &[&str]) -> Vec<(PathBuf, usize)> {
    let root = project_root();
    let mut occurrences = Vec::new();

    // Check docs/
    let docs_dir = root.join("docs");
    for file in find_markdown_files(&docs_dir) {
        if exclude_paths.iter().any(|excluded| file.to_string_lossy().contains(excluded)) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&file) {
            let count = content.matches(pattern).count();
            if count > 0 {
                occurrences.push((file, count));
            }
        }
    }

    // Check team/ directory (if it exists in test context)
    let team_dir = root.join("team");
    for file in find_markdown_files(&team_dir) {
        if exclude_paths.iter().any(|excluded| file.to_string_lossy().contains(excluded)) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&file) {
            let count = content.matches(pattern).count();
            if count > 0 {
                occurrences.push((file, count));
            }
        }
    }

    occurrences
}

#[test]
fn test_session_model_concepts_documented() {
    let root = project_root();

    // Key session model concepts that should be documented
    let required_concepts = vec![
        "ephemeral session",
        "session lifecycle",
        "session daemon",
        "session retention",
        "session garbage collection",
    ];

    let docs_dir = root.join("docs/content");
    let all_docs = find_markdown_files(&docs_dir);

    assert!(!all_docs.is_empty(), "No documentation files found in docs/content/");

    let mut missing_concepts = Vec::new();

    for concept in &required_concepts {
        let mut found = false;
        for doc in &all_docs {
            if file_contains_pattern(doc, concept) {
                found = true;
                break;
            }
        }
        if !found {
            missing_concepts.push(concept.to_string());
        }
    }

    assert!(
        missing_concepts.is_empty(),
        "Missing session model concepts in documentation: {:?}",
        missing_concepts
    );
}

#[test]
fn test_new_cli_behavior_documented() {
    let root = project_root();
    let docs_dir = root.join("docs/content");

    // New CLI commands and behaviors that should be documented
    let required_cli_docs = vec![
        ("bm start", "creates ephemeral"),
        ("bm stop --preserve", "preserve work"),
        ("bm status", "session detail"),
    ];

    let all_docs = find_markdown_files(&docs_dir);
    assert!(!all_docs.is_empty(), "No documentation files found");

    for (command, behavior_hint) in &required_cli_docs {
        let mut found_command = false;
        let mut found_behavior = false;

        for doc in &all_docs {
            if let Ok(content) = fs::read_to_string(doc) {
                if content.contains(command) {
                    found_command = true;
                    if content.contains(behavior_hint) {
                        found_behavior = true;
                        break;
                    }
                }
            }
        }

        assert!(
            found_command,
            "Command '{}' not documented in docs/content/",
            command
        );
        assert!(
            found_behavior,
            "Command '{}' documented but behavior '{}' not explained",
            command, behavior_hint
        );
    }
}

#[test]
fn test_session_management_operations_documented() {
    let root = project_root();
    let docs_dir = root.join("docs/content");

    // Session management operations that should be documented
    let operations = vec![
        "session inspection",
        "session cleanup",
        "session finalization",
    ];

    let all_docs = find_markdown_files(&docs_dir);
    assert!(!all_docs.is_empty(), "No documentation files found");

    for operation in &operations {
        let mut found = false;
        for doc in &all_docs {
            if file_contains_pattern(doc, operation) {
                found = true;
                break;
            }
        }

        assert!(
            found,
            "Session management operation '{}' not documented",
            operation
        );
    }
}

#[test]
fn test_bm_teams_sync_removed_from_docs() {
    // Excluded paths: changelog, migration notes, or ADR docs that may reference historical commands
    let excluded_paths = vec![
        "CHANGELOG",
        "changelog",
        "migration",
        "MIGRATION",
        "adrs/",
        "ADR",
    ];

    let occurrences = find_pattern_in_docs("bm teams sync", &excluded_paths);

    assert!(
        occurrences.is_empty(),
        "Found 'bm teams sync' in documentation (excluding migration notes):\n{}",
        occurrences
            .iter()
            .map(|(path, count)| format!("  - {} ({} occurrence(s))", path.display(), count))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_team_knowledge_workspace_layout_updated() {
    let root = project_root();
    let team_knowledge = root.join("team/knowledge");

    if !team_knowledge.exists() {
        // If team/ doesn't exist in test context, skip
        eprintln!("Skipping team/knowledge test — team/ directory not found in test context");
        return;
    }

    let knowledge_files = find_markdown_files(&team_knowledge);

    // Look for workspace layout documentation
    let mut found_session_aware_paths = false;

    for file in &knowledge_files {
        if let Ok(content) = fs::read_to_string(file) {
            // Check for session-aware path patterns
            if content.contains("session") && (content.contains("workspace") || content.contains("layout"))
                && (content.contains(".sessions/") || content.contains("ephemeral")) {
                found_session_aware_paths = true;
                break;
            }
        }
    }

    assert!(
        found_session_aware_paths,
        "Team knowledge should document session-aware workspace paths"
    );
}

#[test]
fn test_claude_md_workspace_context_updated() {
    let root = project_root();

    // Check both the project CLAUDE.md and team member CLAUDE.md files
    let claude_files = vec![
        root.join("team/CLAUDE.md"),
        root.join("team/members/engineer-bob/CLAUDE.md"),
    ];

    for claude_file in &claude_files {
        if !claude_file.exists() {
            eprintln!("Skipping {} — file not found in test context", claude_file.display());
            continue;
        }

        let content = fs::read_to_string(claude_file)
            .unwrap_or_else(|_| panic!("Should be able to read {}", claude_file.display()));

        // Should mention session paths or session model
        assert!(
            content.contains("session") && (content.contains("workspace") || content.contains("path")),
            "CLAUDE.md at {} should document session workspace context",
            claude_file.display()
        );
    }
}
