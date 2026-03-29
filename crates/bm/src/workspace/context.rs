use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::brain;

const CONTEXT_START_MARKER: &str = "<!-- BM:WORKSPACE_CONTEXT -->";
const CONTEXT_END_MARKER: &str = "<!-- /BM:WORKSPACE_CONTEXT -->";

/// Generates the marker-delimited workspace context markdown section.
///
/// Missing optional fields are omitted gracefully — only available data
/// is included in the table rows.
fn generate_context_section(
    github_repo: Option<&str>,
    project_number: Option<u64>,
    project_names: &[&str],
    member_name: &str,
    role: &str,
) -> String {
    let mut rows = Vec::new();

    if let Some(repo) = github_repo {
        rows.push(format!("| Team repo | `{}` |", repo));
        if let Some((org, _)) = brain::parse_github_repo(repo) {
            rows.push(format!("| GitHub org | `{}` |", org));
        }
    }

    if let Some(num) = project_number {
        rows.push(format!("| Project number | `{}` |", num));
    }

    if !project_names.is_empty() {
        let projects = project_names
            .iter()
            .map(|p| format!("`{}`", p))
            .collect::<Vec<_>>()
            .join(", ");
        rows.push(format!("| Assigned projects | {} |", projects));
    }

    rows.push(format!("| Member | `{}` |", member_name));
    if !role.is_empty() {
        rows.push(format!("| Role | `{}` |", role));
    }

    format!(
        "{start}\n\
         ## Workspace Context\n\
         \n\
         | Key | Value |\n\
         |-----|-------|\n\
         {rows}\n\
         {end}\n",
        start = CONTEXT_START_MARKER,
        rows = rows.join("\n"),
        end = CONTEXT_END_MARKER,
    )
}

/// Injects workspace context into the coding agent's context file and extends
/// `.botminter.workspace` with key-value pairs.
///
/// Uses marker-delimited injection (`<!-- BM:WORKSPACE_CONTEXT -->`) for
/// idempotent updates. Runs unconditionally on every sync — decoupled from
/// the `copy_if_newer` timestamp check on the context file.
///
/// Reads member name and role from `team/members/<member>/botminter.yml`.
/// Gracefully handles missing optional fields and missing files.
pub fn inject_workspace_context(
    ws_root: &Path,
    member_dir_name: &str,
    context_file: &str,
    github_repo: Option<&str>,
    project_number: Option<u64>,
    project_names: &[&str],
) -> Result<()> {
    let team_dir = ws_root.join("team");

    // Read member name and role from team submodule's botminter.yml
    let member_name = brain::read_member_name(&team_dir, member_dir_name);
    let role = brain::read_member_role(&team_dir, member_dir_name).unwrap_or_default();

    // Generate context section
    let section = generate_context_section(
        github_repo,
        project_number,
        project_names,
        &member_name,
        &role,
    );

    // Inject into context file (e.g., CLAUDE.md)
    let context_path = ws_root.join(context_file);
    if context_path.exists() {
        let content = fs::read_to_string(&context_path)
            .with_context(|| format!("Failed to read {}", context_file))?;

        let new_content = inject_section(&content, &section);

        fs::write(&context_path, new_content)
            .with_context(|| format!("Failed to write {}", context_file))?;
    }

    // Extend .botminter.workspace with key-value pairs
    extend_workspace_marker(ws_root, github_repo, project_number)?;

    Ok(())
}

/// Injects or replaces a marker-delimited section in content.
///
/// If markers already exist, the section between them is replaced.
/// Otherwise, the section is appended to the end.
fn inject_section(content: &str, section: &str) -> String {
    if let (Some(start_idx), Some(end_idx)) = (
        content.find(CONTEXT_START_MARKER),
        content.find(CONTEXT_END_MARKER),
    ) {
        // Replace existing section (markers inclusive)
        let before = content[..start_idx].trim_end();
        let after_end = end_idx + CONTEXT_END_MARKER.len();
        let after = content[after_end..].trim_start_matches('\n');
        if after.is_empty() {
            format!("{}\n\n{}", before, section)
        } else {
            format!("{}\n\n{}\n{}", before, section, after)
        }
    } else {
        // Append to end
        let trimmed = content.trim_end();
        format!("{}\n\n{}", trimmed, section)
    }
}

/// Extends `.botminter.workspace` with workspace context key-value pairs.
///
/// Updates existing keys in place or appends new ones. Only writes keys
/// that have values (skips None fields gracefully).
fn extend_workspace_marker(
    ws_root: &Path,
    github_repo: Option<&str>,
    project_number: Option<u64>,
) -> Result<()> {
    let marker_path = ws_root.join(".botminter.workspace");
    if !marker_path.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&marker_path).context("Failed to read .botminter.workspace")?;

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    if let Some(repo) = github_repo {
        update_or_add_kv(&mut lines, "team_repo", repo);
        if let Some((org, _)) = brain::parse_github_repo(repo) {
            update_or_add_kv(&mut lines, "gh_org", org);
        }
    }
    if let Some(num) = project_number {
        update_or_add_kv(&mut lines, "project_number", &num.to_string());
    }

    let mut new_content = lines.join("\n");
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    fs::write(&marker_path, new_content).context("Failed to write .botminter.workspace")?;

    Ok(())
}

/// Updates an existing key-value pair or appends a new one.
fn update_or_add_kv(lines: &mut Vec<String>, key: &str, value: &str) {
    let prefix = format!("{}: ", key);
    let new_line = format!("{}{}", prefix, value);

    if let Some(pos) = lines.iter().position(|l| l.starts_with(&prefix)) {
        lines[pos] = new_line;
    } else {
        lines.push(new_line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_context_section_all_fields() {
        let section = generate_context_section(
            Some("myorg/my-team"),
            Some(643),
            &["botminter", "hypershift"],
            "superman-bob",
            "superman",
        );
        assert!(section.contains(CONTEXT_START_MARKER));
        assert!(section.contains(CONTEXT_END_MARKER));
        assert!(section.contains("| Team repo | `myorg/my-team` |"));
        assert!(section.contains("| GitHub org | `myorg` |"));
        assert!(section.contains("| Project number | `643` |"));
        assert!(section.contains("`botminter`, `hypershift`"));
        assert!(section.contains("| Member | `superman-bob` |"));
        assert!(section.contains("| Role | `superman` |"));
    }

    #[test]
    fn generate_context_section_missing_optional_fields() {
        let section = generate_context_section(None, None, &[], "arch-01", "");
        assert!(section.contains("| Member | `arch-01` |"));
        assert!(!section.contains("Team repo"));
        assert!(!section.contains("GitHub org"));
        assert!(!section.contains("Project number"));
        assert!(!section.contains("Assigned projects"));
        assert!(!section.contains("Role"));
    }

    #[test]
    fn inject_section_appends_when_no_markers() {
        let content = "# My File\n\nSome content.";
        let section = "<!-- BM:WORKSPACE_CONTEXT -->\n## Context\n<!-- /BM:WORKSPACE_CONTEXT -->\n";
        let result = inject_section(content, section);
        assert!(result.starts_with("# My File\n\nSome content."));
        assert!(result.contains("<!-- BM:WORKSPACE_CONTEXT -->"));
    }

    #[test]
    fn inject_section_replaces_existing_markers() {
        let content = "# Header\n\n<!-- BM:WORKSPACE_CONTEXT -->\nold content\n<!-- /BM:WORKSPACE_CONTEXT -->\n";
        let section = "<!-- BM:WORKSPACE_CONTEXT -->\nnew content\n<!-- /BM:WORKSPACE_CONTEXT -->\n";
        let result = inject_section(content, section);
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        // Should only have one set of markers
        assert_eq!(
            result.matches(CONTEXT_START_MARKER).count(),
            1,
            "Should have exactly one start marker"
        );
    }

    #[test]
    fn inject_section_idempotent() {
        let original = "# C";
        let section = generate_context_section(None, None, &[], "arch-01", "");

        let first = inject_section(original, &section);
        let second = inject_section(&first, &section);
        assert_eq!(first, second, "Re-injection should produce identical content");
    }

    #[test]
    fn inject_section_preserves_content_after_markers() {
        let content = "# Header\n\n<!-- BM:WORKSPACE_CONTEXT -->\nold\n<!-- /BM:WORKSPACE_CONTEXT -->\n\n# Footer\n";
        let section = "<!-- BM:WORKSPACE_CONTEXT -->\nnew\n<!-- /BM:WORKSPACE_CONTEXT -->\n";
        let result = inject_section(content, section);
        assert!(result.contains("# Header"));
        assert!(result.contains("# Footer"));
        assert!(result.contains("new"));
    }

    #[test]
    fn update_or_add_kv_adds_new() {
        let mut lines = vec!["# header".to_string(), "member: bob".to_string()];
        update_or_add_kv(&mut lines, "team_repo", "org/repo");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2], "team_repo: org/repo");
    }

    #[test]
    fn update_or_add_kv_updates_existing() {
        let mut lines = vec![
            "# header".to_string(),
            "member: bob".to_string(),
            "team_repo: old/repo".to_string(),
        ];
        update_or_add_kv(&mut lines, "team_repo", "new/repo");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2], "team_repo: new/repo");
    }
}
