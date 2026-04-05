use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Template variables for rendering the brain system prompt.
#[derive(Debug, Clone)]
pub struct BrainPromptVars {
    pub member_name: String,
    pub team_name: String,
    pub role: String,
    pub gh_org: String,
    pub gh_repo: String,
}

/// Renders a brain system prompt template by replacing `{{var}}` placeholders.
pub fn render_brain_prompt(template: &str, vars: &BrainPromptVars) -> String {
    template
        .replace("{{member_name}}", &vars.member_name)
        .replace("{{team_name}}", &vars.team_name)
        .replace("{{role}}", &vars.role)
        .replace("{{gh_org}}", &vars.gh_org)
        .replace("{{gh_repo}}", &vars.gh_repo)
}

/// Reads the brain system prompt template from a team repo, renders it with
/// member-specific variables, and writes the result to the workspace root.
///
/// Returns `Ok(true)` if the prompt was rendered, `Ok(false)` if no template exists.
/// This is a no-op when the profile doesn't include a brain template.
pub fn surface_brain_prompt(
    team_repo: &Path,
    ws_root: &Path,
    vars: &BrainPromptVars,
) -> Result<bool> {
    let template_path = team_repo.join("brain").join("system-prompt.md");
    if !template_path.exists() {
        return Ok(false);
    }

    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read brain template at {}", template_path.display()))?;

    let rendered = render_brain_prompt(&template, vars);

    let output_path = ws_root.join("brain-prompt.md");
    fs::write(&output_path, rendered)
        .with_context(|| format!("Failed to write brain prompt to {}", output_path.display()))?;

    // Copy envelope template if present
    let envelope_src = team_repo.join("brain").join("envelope.md");
    if envelope_src.exists() {
        let envelope_dst = ws_root.join("brain-envelope.md");
        fs::copy(&envelope_src, &envelope_dst)
            .with_context(|| format!("Failed to copy brain envelope to {}", envelope_dst.display()))?;
    }

    Ok(true)
}

/// Parses a GitHub repo string like "org/repo" into (org, repo).
/// Returns `None` if the format is invalid.
pub fn parse_github_repo(github_repo: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = github_repo.splitn(2, '/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}

/// Reads a member's role from their `botminter.yml` in the team repo.
/// Returns `None` if the file doesn't exist or the role field is missing.
pub fn read_member_role(team_repo: &Path, member_dir_name: &str) -> Option<String> {
    let manifest_path = team_repo
        .join("members")
        .join(member_dir_name)
        .join("botminter.yml");

    let contents = fs::read_to_string(&manifest_path).ok()?;
    let value: serde_yml::Value = serde_yml::from_str(&contents).ok()?;
    value
        .get("role")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Reads a member's display name from their `botminter.yml` in the team repo.
/// Falls back to the member directory name if not found.
pub fn read_member_name(team_repo: &Path, member_dir_name: &str) -> String {
    let manifest_path = team_repo
        .join("members")
        .join(member_dir_name)
        .join("botminter.yml");

    let name = fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|contents| {
            let value: serde_yml::Value = serde_yml::from_str(&contents).ok()?;
            value
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    name.unwrap_or_else(|| member_dir_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vars() -> BrainPromptVars {
        BrainPromptVars {
            member_name: "alice".into(),
            team_name: "alpha-team".into(),
            role: "superman".into(),
            gh_org: "myorg".into(),
            gh_repo: "team-repo".into(),
        }
    }

    #[test]
    fn render_replaces_all_variables() {
        let template = "You are {{member_name}} on {{team_name}}, role={{role}}, org={{gh_org}}, repo={{gh_repo}}.";
        let result = render_brain_prompt(template, &sample_vars());
        assert_eq!(
            result,
            "You are alice on alpha-team, role=superman, org=myorg, repo=team-repo."
        );
    }

    #[test]
    fn render_handles_multiple_occurrences() {
        let template = "{{member_name}} is {{member_name}}.";
        let result = render_brain_prompt(template, &sample_vars());
        assert_eq!(result, "alice is alice.");
    }

    #[test]
    fn render_preserves_text_without_variables() {
        let template = "No variables here.";
        let result = render_brain_prompt(template, &sample_vars());
        assert_eq!(result, "No variables here.");
    }

    #[test]
    fn render_handles_empty_template() {
        let result = render_brain_prompt("", &sample_vars());
        assert_eq!(result, "");
    }

    #[test]
    fn render_handles_adjacent_variables() {
        let template = "{{member_name}}{{team_name}}";
        let result = render_brain_prompt(template, &sample_vars());
        assert_eq!(result, "alicealpha-team");
    }

    #[test]
    fn parse_github_repo_valid() {
        let (org, repo) = parse_github_repo("myorg/team-repo").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(repo, "team-repo");
    }

    #[test]
    fn parse_github_repo_with_nested_slash() {
        let (org, repo) = parse_github_repo("myorg/sub/path").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(repo, "sub/path");
    }

    #[test]
    fn parse_github_repo_invalid_no_slash() {
        assert!(parse_github_repo("noslash").is_none());
    }

    #[test]
    fn parse_github_repo_invalid_empty_parts() {
        assert!(parse_github_repo("/repo").is_none());
        assert!(parse_github_repo("org/").is_none());
        assert!(parse_github_repo("").is_none());
    }

    #[test]
    fn surface_brain_prompt_writes_rendered_file() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path().join("team");
        let ws_root = tmp.path().join("workspace");
        fs::create_dir_all(team_repo.join("brain")).unwrap();
        fs::create_dir_all(&ws_root).unwrap();

        let template = "Hello {{member_name}} from {{team_name}}!";
        fs::write(team_repo.join("brain/system-prompt.md"), template).unwrap();

        let vars = sample_vars();
        let rendered = surface_brain_prompt(&team_repo, &ws_root, &vars).unwrap();
        assert!(rendered);

        let content = fs::read_to_string(ws_root.join("brain-prompt.md")).unwrap();
        assert_eq!(content, "Hello alice from alpha-team!");
    }

    #[test]
    fn surface_brain_prompt_returns_false_when_no_template() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path().join("team");
        let ws_root = tmp.path().join("workspace");
        fs::create_dir_all(&team_repo).unwrap();
        fs::create_dir_all(&ws_root).unwrap();

        let vars = sample_vars();
        let rendered = surface_brain_prompt(&team_repo, &ws_root, &vars).unwrap();
        assert!(!rendered);
        assert!(!ws_root.join("brain-prompt.md").exists());
    }

    #[test]
    fn surface_brain_prompt_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path().join("team");
        let ws_root = tmp.path().join("workspace");
        fs::create_dir_all(team_repo.join("brain")).unwrap();
        fs::create_dir_all(&ws_root).unwrap();

        fs::write(ws_root.join("brain-prompt.md"), "old content").unwrap();
        fs::write(
            team_repo.join("brain/system-prompt.md"),
            "New: {{member_name}}",
        )
        .unwrap();

        let vars = sample_vars();
        let rendered = surface_brain_prompt(&team_repo, &ws_root, &vars).unwrap();
        assert!(rendered);
        assert_eq!(
            fs::read_to_string(ws_root.join("brain-prompt.md")).unwrap(),
            "New: alice"
        );
    }

    #[test]
    fn read_member_role_from_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let member_dir = tmp.path().join("members/superman-alice");
        fs::create_dir_all(&member_dir).unwrap();
        fs::write(
            member_dir.join("botminter.yml"),
            "role: superman\nname: alice\n",
        )
        .unwrap();

        let role = read_member_role(tmp.path(), "superman-alice");
        assert_eq!(role.as_deref(), Some("superman"));
    }

    #[test]
    fn read_member_role_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let role = read_member_role(tmp.path(), "nonexistent");
        assert!(role.is_none());
    }

    #[test]
    fn read_member_name_from_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let member_dir = tmp.path().join("members/superman-alice");
        fs::create_dir_all(&member_dir).unwrap();
        fs::write(
            member_dir.join("botminter.yml"),
            "role: superman\nname: alice\n",
        )
        .unwrap();

        let name = read_member_name(tmp.path(), "superman-alice");
        assert_eq!(name, "alice");
    }

    #[test]
    fn read_member_name_fallback_to_dir_name() {
        let tmp = tempfile::tempdir().unwrap();
        let name = read_member_name(tmp.path(), "superman-bob");
        assert_eq!(name, "superman-bob");
    }
}
