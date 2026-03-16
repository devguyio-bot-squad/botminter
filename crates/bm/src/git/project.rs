use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use super::{
    create_github_label, derive_project_name, run_git,
    sync_project_status_field, verify_fork_url,
};
use crate::config::TeamEntry;
use crate::profile;

/// Adds a project to the team repo: verifies URL, creates GitHub label,
/// updates manifest, creates project directories, and commits.
/// Returns the derived project name.
pub fn add_project(
    team_repo: &Path,
    url: &str,
    github_repo: &str,
    gh_token: Option<&str>,
) -> Result<String> {
    let manifest_path = team_repo.join("botminter.yml");
    let mut manifest = profile::read_team_repo_manifest(team_repo)?;

    let name = derive_project_name(url);

    if manifest.projects.iter().any(|p| p.name == name) {
        bail!("Project '{}' already exists in this team.", name);
    }

    verify_fork_url(url, gh_token)?;

    if !github_repo.is_empty() {
        let label_name = format!("project/{}", name);
        create_github_label(
            github_repo,
            &label_name,
            "BFD4F2",
            &format!("Issues for the {} project", name),
            gh_token,
        )?;
    }

    manifest.projects.push(profile::ProjectDef {
        name: name.clone(),
        fork_url: url.to_string(),
    });

    let contents =
        serde_yml::to_string(&manifest).context("Failed to serialize botminter.yml")?;
    fs::write(&manifest_path, contents).context("Failed to write botminter.yml")?;

    let proj_dir = team_repo.join("projects").join(&name);
    fs::create_dir_all(proj_dir.join("knowledge"))
        .with_context(|| format!("Failed to create projects/{}/knowledge/", name))?;
    fs::create_dir_all(proj_dir.join("invariants"))
        .with_context(|| format!("Failed to create projects/{}/invariants/", name))?;
    fs::write(proj_dir.join("knowledge/.gitkeep"), "").ok();
    fs::write(proj_dir.join("invariants/.gitkeep"), "").ok();

    run_git(
        team_repo,
        &["add", "botminter.yml", &format!("projects/{}/", name)],
    )?;
    let commit_msg = format!("feat: add project {}", name);
    run_git(team_repo, &["commit", "-m", &commit_msg])?;

    Ok(name)
}

/// Result of syncing the project board.
pub struct ProjectSyncResult {
    pub status_count: usize,
    pub project_url: String,
    pub views: Vec<ViewDisplay>,
}

/// A view with its computed filter string for display.
pub struct ViewDisplay {
    pub name: String,
    pub filter: String,
}

/// Syncs the GitHub Project board's Status field options with the profile,
/// then returns view data for display.
pub fn sync_project_board(
    team_repo: &Path,
    team: &TeamEntry,
) -> Result<ProjectSyncResult> {
    let manifest = profile::read_team_repo_manifest(team_repo)?;

    let owner = team
        .github_repo
        .split('/')
        .next()
        .unwrap_or(&team.github_repo);
    let gh_token = team.credentials.gh_token.as_deref();

    let project_number = team.project_number.with_context(|| {
        format!(
            "No project board number stored for team '{}'. \
             Re-run `bm init` to select or create a project board.",
            team.name
        )
    })?;
    sync_project_status_field(owner, project_number, &manifest.statuses, gh_token)?;

    let views: Vec<ViewDisplay> = manifest
        .views
        .iter()
        .map(|v| ViewDisplay {
            name: v.name.clone(),
            filter: v.filter_string(&manifest.statuses),
        })
        .collect();

    let project_url = format!(
        "https://github.com/orgs/{}/projects/{}",
        owner, project_number
    );

    Ok(ProjectSyncResult {
        status_count: manifest.statuses.len(),
        project_url,
        views,
    })
}
