use anyhow::{bail, Context, Result};

use crate::formation::KeyValueCredentialStore;

// ── Public types ────────────────────────────────────────────────────

/// Pre-generated credentials for `--reuse-app` mode.
pub struct PreGeneratedCredentials {
    pub app_id: String,
    pub client_id: String,
    pub private_key: String,
    pub installation_id: String,
}

// ── Manifest JSON construction ──────────────────────────────────────

/// Builds the manifest JSON for GitHub App creation.
///
/// The manifest includes:
/// - `redirect_url`: local callback for code exchange
/// - `setup_url`: local callback for post-installation redirect
/// - `organization_projects: admin` (NOT `projects: admin` — that's classic boards)
///
/// Currently used only by tests to verify the permission set. The browser-based
/// manifest flow that consumes this is deferred — Sprint 3 uses `--reuse-app`.
pub fn build_manifest_json(
    app_name: &str,
    team_repo_url: &str,
    port: u16,
) -> serde_json::Value {
    serde_json::json!({
        "name": app_name,
        "url": team_repo_url,
        "redirect_url": format!("http://127.0.0.1:{port}/callback"),
        "setup_url": format!("http://127.0.0.1:{port}/installed"),
        "default_permissions": {
            "issues": "write",
            "contents": "write",
            "pull_requests": "write",
            "administration": "write",
            "organization_projects": "admin"
        },
        "default_events": [],
        "public": false
    })
}

// ── Name collision check ────────────────────────────────────────────

/// Checks if a GitHub App with the given slug already exists.
///
/// The slug is derived from the app name by lowercasing and replacing
/// spaces with hyphens. GitHub App names share a namespace with org names.
pub fn check_name_collision(slug: &str) -> Result<bool> {
    let url = format!("https://github.com/apps/{slug}");
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("Failed to build HTTP client")?;
    let response = client
        .get(&url)
        .header("User-Agent", "botminter")
        .send()
        .context("Failed to check app name availability")?;

    // 200 means the app exists; 404 means available
    Ok(response.status().is_success())
}

/// Derives the slug from an app name (lowercased, spaces→hyphens).
pub fn app_name_to_slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

// ── Credential storage ──────────────────────────────────────────────

/// Key conventions for GitHub App credentials in the credential store.
pub mod credential_keys {
    pub fn app_id(member: &str) -> String {
        format!("{member}/github-app-id")
    }
    pub fn client_id(member: &str) -> String {
        format!("{member}/github-app-client-id")
    }
    pub fn private_key(member: &str) -> String {
        format!("{member}/github-app-private-key")
    }
    pub fn installation_id(member: &str) -> String {
        format!("{member}/github-installation-id")
    }
}

/// Stores pre-generated credentials in the credential store.
pub fn store_pregenerated_credentials(
    store: &dyn KeyValueCredentialStore,
    member: &str,
    creds: &PreGeneratedCredentials,
) -> Result<()> {
    store
        .store(&credential_keys::app_id(member), &creds.app_id)
        .context("Failed to store App ID")?;
    store
        .store(&credential_keys::client_id(member), &creds.client_id)
        .context("Failed to store Client ID")?;
    store
        .store(&credential_keys::private_key(member), &creds.private_key)
        .context("Failed to store private key")?;
    store
        .store(&credential_keys::installation_id(member), &creds.installation_id)
        .context("Failed to store installation ID")?;
    Ok(())
}

/// Removes all GitHub App credentials for a member from the credential store.
///
/// Removes the 4 known credential keys. Errors on individual keys are
/// collected and reported as a single error at the end.
pub fn remove_member_credentials(
    store: &dyn KeyValueCredentialStore,
    member: &str,
) -> Result<()> {
    let keys = [
        credential_keys::app_id(member),
        credential_keys::client_id(member),
        credential_keys::private_key(member),
        credential_keys::installation_id(member),
    ];

    let mut errors = Vec::new();
    for key in &keys {
        if let Err(e) = store.remove(key) {
            errors.push(format!("{key}: {e}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        bail!("Failed to remove some credentials:\n  {}", errors.join("\n  "))
    }
}

/// Saves credentials to a YAML file (for `--save-credentials`).
pub fn save_credentials_to_file(
    path: &str,
    member: &str,
    creds: &PreGeneratedCredentials,
) -> Result<()> {
    let content = serde_yml::to_string(&serde_json::json!({
        "member": member,
        "app_id": creds.app_id,
        "client_id": creds.client_id,
        "private_key": creds.private_key,
        "installation_id": creds.installation_id,
    }))
    .context("Failed to serialize credentials")?;

    std::fs::write(path, &content).context("Failed to write credentials file")?;

    // Set file permissions to 0600 on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .context("Failed to set credentials file permissions")?;
    }

    Ok(())
}

// ── Org resolution ──────────────────────────────────────────────────

/// Resolves the organization from a `github_repo` field (e.g., "org/repo").
/// Returns an error if the owner is a personal account (not an org).
pub fn resolve_org_from_repo(github_repo: &str) -> Result<String> {
    let owner = github_repo
        .split('/')
        .next()
        .context("Invalid github_repo format — expected 'owner/repo'")?;

    validate_is_org(owner)?;
    Ok(owner.to_string())
}

/// Validates that a GitHub account name is an Organization (not a personal account).
///
/// Calls `GET /users/{owner}` and checks `.type == "Organization"`.
/// Returns `Ok(())` if it is an org, or an error with a clear message if not.
pub fn validate_is_org(owner: &str) -> Result<()> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args(["api", &format!("users/{owner}"), "--jq", ".type"]);
    if let Some(token) = super::detect_token() {
        cmd.env("GH_TOKEN", token);
    }

    let output = cmd
        .output()
        .context("Failed to check repo owner type via `gh api`")?;

    if !output.status.success() {
        bail!(
            "Failed to verify owner type for '{}'. Is the GitHub CLI authenticated?",
            owner
        );
    }

    let owner_type = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if owner_type != "Organization" {
        bail!(
            "GitHub App identity requires an organization, but '{}' is a {}.\n\
             Personal accounts cannot use organization_projects permissions.\n\
             Create an organization and move your team repo there first.",
            owner,
            owner_type.to_lowercase()
        );
    }

    Ok(())
}

// ── Installation repo management ────────────────────────────────────

/// Ensures the App installation has access to the given repos.
///
/// For each repo, checks `GET /repos/{owner}/{repo}/installation` using an
/// App JWT (this endpoint requires App-level auth, not a user PAT).
///
/// This is idempotent: calling it multiple times is safe.
pub fn ensure_app_on_repos(
    installation_id: &str,
    client_id: &str,
    private_key: &str,
    repos: &[&str],
) -> Result<()> {
    let jwt = super::app_auth::generate_jwt(client_id, private_key)
        .context("Failed to generate JWT for repo installation check")?;
    for repo in repos {
        if repo.is_empty() {
            continue;
        }
        match check_repo_installation(repo, &jwt) {
            RepoInstallationStatus::Installed => {
                eprintln!("  App already has access to {repo}");
            }
            RepoInstallationStatus::NotInstalled => {
                // TODO: Programmatic repo addition requires the browser-based
                // GitHub App installation flow (interactive manifest flow).
                // Until that's implemented, the operator must install the App
                // on the repo manually via the GitHub UI.
                eprintln!(
                    "  Warning: App is not installed on {repo}.\n\
                     \x20 Install it manually: https://github.com/organizations/{org}/settings/installations/{installation_id}",
                    org = repo.split('/').next().unwrap_or("UNKNOWN"),
                );
            }
            RepoInstallationStatus::CheckFailed(e) => {
                eprintln!("  Warning: could not check installation status for {repo}: {e}");
            }
        }
    }
    Ok(())
}

enum RepoInstallationStatus {
    Installed,
    NotInstalled,
    CheckFailed(String),
}

/// Checks whether the App is installed on a specific repo.
///
/// Uses the App's JWT for authentication via `-H Authorization: Bearer` header,
/// which overrides any `GH_TOKEN` env var. The `GET /repos/{owner}/{repo}/installation`
/// endpoint requires App-level auth and cannot be called with a user PAT.
fn check_repo_installation(repo: &str, jwt: &str) -> RepoInstallationStatus {
    let auth_header = format!("Authorization: Bearer {jwt}");
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "api",
        &format!("repos/{repo}/installation"),
        "-H", &auth_header,
        "--silent",
    ]);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                RepoInstallationStatus::Installed
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("404") || stderr.contains("Not Found") {
                    RepoInstallationStatus::NotInstalled
                } else {
                    RepoInstallationStatus::CheckFailed(stderr.trim().to_string())
                }
            }
        }
        Err(e) => RepoInstallationStatus::CheckFailed(e.to_string()),
    }
}

/// Collects all repos that a member's App should have access to:
/// the team repo + any configured project repos from `botminter.yml`.
pub fn collect_team_repos(team: &crate::config::TeamEntry) -> Vec<String> {
    let mut repos = Vec::new();

    // Team repo
    if !team.github_repo.is_empty() {
        repos.push(team.github_repo.clone());
    }

    // Project repos — read from team repo's botminter.yml via read_team_projects()
    let team_repo = team.path.join("team");
    for project in crate::profile::read_team_projects(&team_repo) {
        let url = project.fork_url.trim().to_string();
        if let Some(repo) = fork_url_to_owner_repo(&url) {
            repos.push(repo);
        }
    }

    repos
}

/// Converts a GitHub fork URL to `owner/repo` format.
/// Returns `None` for non-GitHub or empty URLs.
pub(crate) fn fork_url_to_owner_repo(url: &str) -> Option<String> {
    let stripped = url
        .strip_prefix("https://github.com/")?;
    let repo = stripped
        .trim_end_matches('/')
        .trim_end_matches(".git");
    if repo.is_empty() || !repo.contains('/') {
        return None;
    }
    Some(repo.to_string())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_manifest_json_has_required_fields() {
        let manifest = build_manifest_json("my-team-superman", "https://github.com/org/my-team", 12345);

        assert_eq!(manifest["name"], "my-team-superman");
        assert_eq!(manifest["url"], "https://github.com/org/my-team");
        assert_eq!(
            manifest["redirect_url"],
            "http://127.0.0.1:12345/callback"
        );
        assert_eq!(
            manifest["setup_url"],
            "http://127.0.0.1:12345/installed"
        );
        assert_eq!(manifest["public"], false);
        assert_eq!(manifest["default_events"], serde_json::json!([]));
    }

    #[test]
    fn build_manifest_json_has_correct_permissions() {
        let manifest = build_manifest_json("app", "https://example.com", 8080);
        let perms = &manifest["default_permissions"];

        assert_eq!(perms["issues"], "write");
        assert_eq!(perms["contents"], "write");
        assert_eq!(perms["pull_requests"], "write");
        assert_eq!(perms["administration"], "write");
        assert_eq!(perms["organization_projects"], "admin");

        // MUST NOT have the deprecated "projects" permission
        assert!(
            perms.get("projects").is_none(),
            "Manifest must use organization_projects, not projects (which is for classic boards)"
        );
    }

    #[test]
    fn app_name_to_slug_lowercases_and_replaces_spaces() {
        assert_eq!(app_name_to_slug("My Team Superman"), "my-team-superman");
        assert_eq!(app_name_to_slug("already-lowercase"), "already-lowercase");
        assert_eq!(app_name_to_slug("MixedCase"), "mixedcase");
    }

    #[test]
    fn credential_keys_follow_convention() {
        assert_eq!(credential_keys::app_id("superman"), "superman/github-app-id");
        assert_eq!(
            credential_keys::client_id("superman"),
            "superman/github-app-client-id"
        );
        assert_eq!(
            credential_keys::private_key("superman"),
            "superman/github-app-private-key"
        );
        assert_eq!(
            credential_keys::installation_id("superman"),
            "superman/github-installation-id"
        );
    }

    #[test]
    fn store_pregenerated_credentials_writes_all_keys() {
        let store = crate::formation::InMemoryKeyValueCredentialStore::new();
        let creds = PreGeneratedCredentials {
            app_id: "789".to_string(),
            client_id: "Iv1.xyz".to_string(),
            private_key: "PEM_DATA".to_string(),
            installation_id: "1011".to_string(),
        };

        store_pregenerated_credentials(&store, "batman", &creds).unwrap();

        assert_eq!(
            store.retrieve("batman/github-app-id").unwrap(),
            Some("789".to_string())
        );
        assert_eq!(
            store.retrieve("batman/github-app-client-id").unwrap(),
            Some("Iv1.xyz".to_string())
        );
        assert_eq!(
            store.retrieve("batman/github-app-private-key").unwrap(),
            Some("PEM_DATA".to_string())
        );
        assert_eq!(
            store.retrieve("batman/github-installation-id").unwrap(),
            Some("1011".to_string())
        );
    }

    #[test]
    fn save_credentials_to_file_creates_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("creds.yml");
        let creds = PreGeneratedCredentials {
            app_id: "42".to_string(),
            client_id: "Iv1.test".to_string(),
            private_key: "PEM".to_string(),
            installation_id: "99".to_string(),
        };

        save_credentials_to_file(path.to_str().unwrap(), "test-member", &creds).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // Verify actual values, not just key presence
        let parsed: serde_json::Value = serde_yml::from_str(&content).unwrap();
        assert_eq!(parsed["member"], "test-member");
        assert_eq!(parsed["app_id"], "42");
        assert_eq!(parsed["client_id"], "Iv1.test");
        assert_eq!(parsed["private_key"], "PEM");
        assert_eq!(parsed["installation_id"], "99");

        // Check file permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o600, "File should be 0600");
        }
    }

    #[test]
    fn fork_url_to_owner_repo_extracts_correctly() {
        assert_eq!(
            fork_url_to_owner_repo("https://github.com/org/repo"),
            Some("org/repo".to_string())
        );
        assert_eq!(
            fork_url_to_owner_repo("https://github.com/org/repo.git"),
            Some("org/repo".to_string())
        );
        assert_eq!(
            fork_url_to_owner_repo("https://github.com/org/repo/"),
            Some("org/repo".to_string())
        );
        // Non-GitHub URLs return None
        assert_eq!(fork_url_to_owner_repo("https://gitlab.com/org/repo"), None);
        // Empty or malformed
        assert_eq!(fork_url_to_owner_repo(""), None);
        assert_eq!(fork_url_to_owner_repo("https://github.com/"), None);
        assert_eq!(fork_url_to_owner_repo("https://github.com/orgonly"), None);
    }

    #[test]
    fn collect_team_repos_includes_team_and_project_repos() {
        let tmp = tempfile::tempdir().unwrap();
        let team_dir = tmp.path().join("team");
        std::fs::create_dir_all(&team_dir).unwrap();

        // Write botminter.yml with projects
        std::fs::write(
            team_dir.join("botminter.yml"),
            "name: test\nschema_version: '1.0'\nversion: '1.0.0'\ndescription: t\ndisplay_name: t\nprojects:\n  - name: my-app\n    fork_url: https://github.com/org/my-app.git\n  - name: lib\n    fork_url: https://github.com/org/lib\n",
        ).unwrap();

        let team = crate::config::TeamEntry {
            name: "test-team".to_string(),
            path: tmp.path().to_path_buf(),
            profile: "scrum".to_string(),
            github_repo: "org/team-repo".to_string(),
            credentials: Default::default(),
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
            vm: None,
        };

        let repos = collect_team_repos(&team);
        assert_eq!(repos, vec!["org/team-repo", "org/my-app", "org/lib"]);
    }

    #[test]
    fn collect_team_repos_empty_github_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let team_dir = tmp.path().join("team");
        std::fs::create_dir_all(&team_dir).unwrap();
        std::fs::write(
            team_dir.join("botminter.yml"),
            "name: test\nschema_version: '1.0'\nversion: '1.0.0'\ndescription: t\ndisplay_name: t\n",
        ).unwrap();

        let team = crate::config::TeamEntry {
            name: "test-team".to_string(),
            path: tmp.path().to_path_buf(),
            profile: "scrum".to_string(),
            github_repo: "".to_string(),
            credentials: Default::default(),
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
            vm: None,
        };

        let repos = collect_team_repos(&team);
        assert!(repos.is_empty());
    }

    #[test]
    fn collect_team_repos_no_botminter_yml() {
        let tmp = tempfile::tempdir().unwrap();
        // No team/ dir at all — should gracefully return just the team repo
        let team = crate::config::TeamEntry {
            name: "test-team".to_string(),
            path: tmp.path().to_path_buf(),
            profile: "scrum".to_string(),
            github_repo: "org/repo".to_string(),
            credentials: Default::default(),
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
            vm: None,
        };

        let repos = collect_team_repos(&team);
        assert_eq!(repos, vec!["org/repo"]);
    }

    #[test]
    fn store_credentials_overwrites_existing() {
        let store = crate::formation::InMemoryKeyValueCredentialStore::new();

        // Store initial
        let creds1 = PreGeneratedCredentials {
            app_id: "100".to_string(),
            client_id: "Iv1.old".to_string(),
            private_key: "OLD_PEM".to_string(),
            installation_id: "200".to_string(),
        };
        store_pregenerated_credentials(&store, "hero", &creds1).unwrap();

        // Overwrite with new
        let creds2 = PreGeneratedCredentials {
            app_id: "300".to_string(),
            client_id: "Iv1.new".to_string(),
            private_key: "NEW_PEM".to_string(),
            installation_id: "400".to_string(),
        };
        store_pregenerated_credentials(&store, "hero", &creds2).unwrap();

        assert_eq!(
            store.retrieve("hero/github-app-id").unwrap(),
            Some("300".to_string())
        );
        assert_eq!(
            store.retrieve("hero/github-app-client-id").unwrap(),
            Some("Iv1.new".to_string())
        );
    }
}
