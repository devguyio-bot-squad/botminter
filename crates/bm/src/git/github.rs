use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::profile;

/// Derives the project name from a git URL (basename minus .git suffix).
pub fn derive_project_name(url: &str) -> String {
    let url = url.trim_end_matches('/');
    let basename = url.rsplit('/').next().unwrap_or(url);
    basename.trim_end_matches(".git").to_string()
}

/// Verifies that a fork URL is a remote URL and is reachable.
///
/// Rejects local paths — workspace repos need remote URLs so they can be cloned
/// on any machine. Runs `gh repo view` to verify the repo exists and is accessible.
pub fn verify_fork_url(url: &str, gh_token: Option<&str>) -> Result<()> {
    if url.starts_with("file://") {
        // file:// URI — check the local path exists and is a git repo
        let path_str = url.strip_prefix("file://").unwrap();
        let path = Path::new(path_str);
        if !path.join(".git").is_dir() {
            bail!(
                "Repository '{}' not found or is not a git repository.",
                url
            );
        }
        return Ok(());
    }

    if !url.starts_with("https://") && !url.starts_with("git@") {
        bail!(
            "Project URL must use a URI scheme (https://, git@, or file://), got: {}\n\
             Bare local paths are not supported — use file:// for local repos.",
            url
        );
    }

    let mut cmd = Command::new("gh");
    cmd.args(["repo", "view", url, "--json", "name"]);
    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }
    let output = cmd.output().context("Failed to run `gh repo view`")?;
    if !output.status.success() {
        bail!(
            "Repository '{}' not found or not accessible.\n\
             Check the URL and ensure your token has access.\n\
             To verify manually:  gh repo view {}",
            url, url
        );
    }
    Ok(())
}

/// Creates a single label on a GitHub repo. Idempotent (uses --force).
pub fn create_github_label(
    repo: &str,
    name: &str,
    color: &str,
    description: &str,
    gh_token: Option<&str>,
) -> Result<()> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "label",
        "create",
        name,
        "--color",
        color,
        "--description",
        description,
        "--force",
        "--repo",
        repo,
    ]);

    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }

    let output = cmd.output().with_context(|| {
        format!("Failed to create label '{}'", name)
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to create label '{}': {}",
            name,
            stderr.trim(),
        );
    }

    Ok(())
}

/// Finds a GitHub Project by title for the given owner. Returns the project number.
pub fn find_project_number(
    owner: &str,
    team_name: &str,
    gh_token: Option<&str>,
) -> Result<u64> {
    let board_title = format!("{} Board", team_name);
    let mut cmd = Command::new("gh");
    cmd.args([
        "project",
        "list",
        "--owner",
        owner,
        "--format",
        "json",
    ]);
    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }
    let output = cmd
        .output()
        .context("Failed to run `gh project list`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh project list failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(stdout.trim()).context("Could not parse project list JSON")?;

    json["projects"]
        .as_array()
        .and_then(|projects| {
            projects
                .iter()
                .find(|p| p["title"].as_str() == Some(&board_title))
                .and_then(|p| p["number"].as_u64())
        })
        .with_context(|| format!("No project named '{}' found for owner '{}'", board_title, owner))
}

/// Finds the built-in Status field ID and updates its options via GraphQL.
/// This replaces the default (Todo/In Progress/Done) with profile-defined statuses.
pub fn sync_project_status_field(
    owner: &str,
    project_number: u64,
    statuses: &[profile::StatusDef],
    gh_token: Option<&str>,
) -> Result<()> {
    let num_str = project_number.to_string();

    // 1. Find the Status field ID
    let mut cmd = Command::new("gh");
    cmd.args([
        "project",
        "field-list",
        &num_str,
        "--owner",
        owner,
        "--format",
        "json",
    ]);
    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }
    let output = cmd
        .output()
        .context("Failed to run `gh project field-list`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh project field-list failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let fields_json: serde_json::Value = serde_json::from_str(stdout.trim())
        .context("Could not parse field-list JSON")?;

    let field_id = fields_json["fields"]
        .as_array()
        .and_then(|fields| {
            fields
                .iter()
                .find(|f| f["name"].as_str() == Some("Status"))
                .and_then(|f| f["id"].as_str())
        })
        .context("Could not find Status field in project")?
        .to_string();

    // 2. Build the GraphQL mutation to update Status field options
    //    Assign colors by role prefix for visual grouping.
    let options_json: Vec<String> = statuses
        .iter()
        .map(|s| {
            let color = color_for_status(&s.name);
            format!(
                "{{name:\"{}\",color:{},description:\"\"}}",
                s.name, color
            )
        })
        .collect();

    let mutation = format!(
        "mutation {{ updateProjectV2Field(input: {{ fieldId: \"{}\", \
         singleSelectOptions: [{}] }}) {{ projectV2Field {{ \
         ... on ProjectV2SingleSelectField {{ name options {{ name id }} }} }} }} }}",
        field_id,
        options_json.join(",")
    );

    let mut cmd = Command::new("gh");
    cmd.args(["api", "graphql", "-f", &format!("query={}", mutation)]);
    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }
    let output = cmd
        .output()
        .context("Failed to run GraphQL updateProjectV2Field")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to sync Status field: {}", stderr.trim());
    }

    Ok(())
}

/// Maps a status name prefix to a GitHub Project color for visual grouping.
fn color_for_status(name: &str) -> &'static str {
    match name.split(':').next().unwrap_or("") {
        "po" => "BLUE",
        "arch" => "PURPLE",
        "dev" => "YELLOW",
        "qe" => "PINK",
        "lead" => "ORANGE",
        "sre" => "GRAY",
        "cw" => "ORANGE",
        "mgr" => "PURPLE",
        "error" => "RED",
        "done" => "GREEN",
        _ => "GRAY",
    }
}

// ── GitHub API operations (extracted from init wizard) ─────────────

/// Result of validating a GitHub token via `gh api user`.
pub struct TokenInfo {
    pub login: String,
}

/// Checks if a GitHub repository exists and is accessible.
pub fn repo_exists(repo_name: &str, gh_token: Option<&str>) -> Result<bool> {
    let mut cmd = Command::new("gh");
    cmd.args(["repo", "view", repo_name, "--json", "name"]);
    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }
    let output = cmd.output().context("Failed to run `gh repo view`")?;
    Ok(output.status.success())
}

/// Creates a GitHub repo from a local git directory and pushes.
pub fn create_repo_and_push(
    local_repo: &Path,
    repo_name: &str,
    gh_token: Option<&str>,
) -> Result<()> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "repo", "create", repo_name, "--private", "--source", ".", "--push",
    ])
    .current_dir(local_repo);

    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }

    let output = cmd.output().context("Failed to run `gh repo create`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "gh repo create failed: {}\n\n\
             To fix, run manually:\n  \
             gh repo create {} --private --source . --push",
            stderr.trim(),
            repo_name,
        );
    }

    Ok(())
}

/// Clones an existing GitHub repo into `{parent_dir}/team/`.
pub fn clone_repo(parent_dir: &Path, repo_name: &str, gh_token: Option<&str>) -> Result<()> {
    let target = parent_dir.join("team");
    let mut cmd = Command::new("gh");
    cmd.args(["repo", "clone", repo_name, &target.to_string_lossy()]);

    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }

    let output = cmd.output().context("Failed to run `gh repo clone`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to clone repo '{}': {}\n\n\
             To fix, run manually:\n  \
             gh repo clone {} {}",
            repo_name,
            stderr.trim(),
            repo_name,
            target.display(),
        );
    }

    Ok(())
}

/// Lists repository names for a given GitHub owner (org or user).
pub fn list_repos(gh_token: &str, owner: &str) -> Result<Vec<String>> {
    let output = Command::new("gh")
        .args([
            "repo", "list", owner,
            "--limit", "50",
            "--json", "name",
            "--jq", ".[].name",
        ])
        .env("GH_TOKEN", gh_token)
        .output()
        .context("Failed to list repos")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to list repos for '{}': {}", owner, stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter(|l| !l.is_empty()).map(String::from).collect())
}

/// Lists GitHub Project boards for a given owner. Returns `(number, title)` pairs.
pub fn list_projects(gh_token: &str, owner: &str) -> Result<Vec<(u64, String)>> {
    let output = Command::new("gh")
        .args([
            "project", "list", "--owner", owner, "--format", "json",
        ])
        .env("GH_TOKEN", gh_token)
        .output()
        .context("Failed to run `gh project list`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to list projects for '{}': {}",
            owner,
            stderr.trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(stdout.trim()).context("Could not parse project list JSON")?;

    Ok(json["projects"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|p| {
            let number = p["number"].as_u64()?;
            let title = p["title"].as_str()?.to_string();
            Some((number, title))
        })
        .collect())
}

/// Creates a GitHub Project (v2), syncs the Status field options, and returns the project number.
pub fn create_project(
    owner: &str,
    team_name: &str,
    statuses: &[profile::StatusDef],
    gh_token: Option<&str>,
) -> Result<u64> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "project", "create", "--owner", owner,
        "--title", &format!("{} Board", team_name),
        "--format", "json",
    ]);
    if let Some(token) = gh_token {
        cmd.env("GH_TOKEN", token);
    }
    let output = cmd.output().context("Failed to run `gh project create`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh project create failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let project_json: serde_json::Value = serde_json::from_str(stdout.trim())
        .context("Could not parse JSON from `gh project create` output")?;
    let project_number = project_json["number"]
        .as_u64()
        .context("Could not find 'number' field in gh project create output")?;

    sync_project_status_field(owner, project_number, statuses, gh_token)?;

    Ok(project_number)
}

/// Bootstraps labels on a GitHub repo from profile label definitions.
/// Idempotent — uses `--force` per label.
pub fn bootstrap_labels(
    repo: &str,
    labels: &[profile::LabelDef],
    gh_token: Option<&str>,
) -> Result<()> {
    for label in labels {
        create_github_label(repo, &label.name, &label.color, &label.description, gh_token)?;
    }
    Ok(())
}

/// Detects GH_TOKEN from environment or `gh auth token` (no interactive prompt).
pub fn detect_token_non_interactive() -> Result<String> {
    if let Ok(token) = std::env::var("GH_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|t| !t.is_empty());

    match output {
        Some(token) => Ok(token),
        None => bail!(
            "No GitHub token found. Set GH_TOKEN or run `gh auth login` before using --non-interactive."
        ),
    }
}

/// Detects an existing GH_TOKEN from environment or `gh auth token`.
/// Returns `Some(token)` if found, `None` if no token is available.
pub fn detect_token() -> Option<String> {
    std::env::var("GH_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
        .or_else(|| {
            Command::new("gh")
                .args(["auth", "token"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|t| !t.is_empty())
        })
}

/// Validates that a GitHub token works by calling `gh api user`.
/// Returns the authenticated user's login on success.
pub fn validate_token(token: &str) -> Result<TokenInfo> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .env("GH_TOKEN", token)
        .output()
        .context("Failed to run `gh api user` for token validation")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "GitHub token validation failed: {}\n\n\
             Make sure your token is valid and not expired.\n\
             To create a new token, visit: https://github.com/settings/tokens\n\
             Required permissions: Contents (Write), Issues (Write), Projects (Admin)",
            stderr.trim(),
        );
    }

    let login = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(TokenInfo { login })
}

/// Masks a token for display: shows first 4 and last 4 characters.
pub fn mask_token(token: &str) -> String {
    if token.len() <= 12 {
        return "****".to_string();
    }
    format!("{}...{}", &token[..4], &token[token.len() - 4..])
}

/// Returns the authenticated GitHub user's login.
pub fn get_user_login(gh_token: &str) -> Result<String> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .env("GH_TOKEN", gh_token)
        .output()
        .context("Failed to get GitHub user")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Lists the authenticated user's GitHub organizations.
/// May return empty if the token lacks the Organization:Read scope.
pub fn list_user_orgs(gh_token: &str) -> Result<Vec<String>> {
    let output = Command::new("gh")
        .args(["api", "user/orgs", "--jq", ".[].login"])
        .env("GH_TOKEN", gh_token)
        .output()
        .context("Failed to list GitHub orgs")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter(|l| !l.is_empty()).map(String::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn derive_project_name_git_url() {
        assert_eq!(
            derive_project_name("git@github.com:org/my-repo.git"),
            "my-repo"
        );
    }

    #[test]
    fn derive_project_name_https() {
        assert_eq!(
            derive_project_name("https://github.com/org/my-repo.git"),
            "my-repo"
        );
    }

    #[test]
    fn derive_project_name_trailing_slash() {
        assert_eq!(
            derive_project_name("https://github.com/org/my-repo/"),
            "my-repo"
        );
    }

    #[test]
    fn derive_project_name_no_git_suffix() {
        assert_eq!(
            derive_project_name("https://github.com/org/my-repo"),
            "my-repo"
        );
    }

    // ── verify_fork_url rejects bare local paths ──────────────

    #[test]
    fn verify_fork_url_rejects_bare_local_path() {
        let result = verify_fork_url("/tmp/some-repo", None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must use a URI scheme"), "{}", err);
    }

    #[test]
    fn verify_fork_url_rejects_relative_path() {
        let result = verify_fork_url("../my-project", None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must use a URI scheme"), "{}", err);
    }

    #[test]
    fn verify_fork_url_rejects_dot_path() {
        let result = verify_fork_url("./local-repo", None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must use a URI scheme"), "{}", err);
    }

    #[test]
    fn verify_fork_url_accepts_file_uri() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::fs::create_dir_all(&repo).unwrap();
        Command::new("git").args(["init", "-b", "main"]).current_dir(&repo).output().unwrap();
        let url = format!("file://{}", repo.to_string_lossy());
        assert!(verify_fork_url(&url, None).is_ok());
    }

    #[test]
    fn verify_fork_url_rejects_nonexistent_file_uri() {
        let result = verify_fork_url("file:///tmp/does-not-exist-repo-xyz", None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found") || err.contains("not a git repository"), "{}", err);
    }

    // ── mask_token ──────────────────────────────────────────────

    #[test]
    fn mask_token_normal() {
        assert_eq!(mask_token("github_pat_abc123xyz789"), "gith...z789");
    }

    #[test]
    fn mask_token_short_token_returns_stars() {
        assert_eq!(mask_token("abc"), "****");
    }

    #[test]
    fn mask_token_exactly_12_returns_stars() {
        assert_eq!(mask_token("123456789012"), "****");
    }

    #[test]
    fn mask_token_13_chars_shows_ends() {
        assert_eq!(mask_token("1234567890123"), "1234...0123");
    }

    // ── project number JSON parsing ─────────────────────────────

    #[test]
    fn parse_project_number_from_json() {
        let json_output = r#"{"number":42,"title":"test Board","url":"https://github.com/orgs/test/projects/42"}"#;
        let parsed: serde_json::Value = serde_json::from_str(json_output).unwrap();
        let number = parsed["number"].as_u64().unwrap();
        assert_eq!(number, 42);
    }

    #[test]
    fn parse_project_number_missing_field() {
        let json_output = r#"{"title":"test Board"}"#;
        let parsed: serde_json::Value = serde_json::from_str(json_output).unwrap();
        assert!(parsed["number"].as_u64().is_none());
    }

    // ── list_repos parsing ───────────────────────────────────────

    #[test]
    fn parse_repo_names_from_gh_output() {
        let output = "repo-one\nrepo-two\nrepo-three\n";
        let repos: Vec<String> = output
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
        assert_eq!(repos, vec!["repo-one", "repo-two", "repo-three"]);
    }

    #[test]
    fn parse_repo_names_empty_output() {
        let output = "";
        let repos: Vec<String> = output
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
        assert!(repos.is_empty());
    }
}
