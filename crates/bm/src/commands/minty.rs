use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use anyhow::{bail, Context, Result};

use crate::config;

/// Handles `bm minty [--discover] [-t team]`.
///
/// Migration tool for transitioning from permanent workspaces to ephemeral sessions.
/// - `bm minty --discover`: Discover existing permanent workspaces
/// - `bm minty -t <team>`: Initialize shared clones from permanent workspace repos
pub fn run(team_flag: Option<&str>, discover: bool, _autonomous: bool) -> Result<()> {
    if discover {
        discover_permanent_workspaces(team_flag)?;
    } else if let Some(team_name) = team_flag {
        migrate_team(team_name)?;
    } else {
        bail!(
            "bm minty requires either --discover or -t <team>.\n\
             \n\
             Usage:\n\
               bm minty --discover              Discover permanent workspaces\n\
               bm minty -t <team>               Migrate team to ephemeral sessions"
        );
    }

    Ok(())
}

/// Discover existing permanent workspaces for a team.
fn discover_permanent_workspaces(team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;

    println!("Discovering permanent workspaces for team '{}'...", team.name);

    // Scan team directory for permanent workspace directories
    let team_path = &team.path;
    let mut found_workspaces = Vec::new();

    if let Ok(entries) = fs::read_dir(team_path) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                // Check for workspace marker
                let marker = path.join(".botminter.workspace");
                if marker.exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        // Skip the team repo itself
                        if name != "team" {
                            found_workspaces.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    if found_workspaces.is_empty() {
        println!("No permanent workspaces found.");
    } else {
        println!("Found {} permanent workspace(s):", found_workspaces.len());
        for name in &found_workspaces {
            println!("  - {}", name);
        }
    }

    Ok(())
}

/// Migrate a team by initializing shared clones from permanent workspace repos.
fn migrate_team(team_name: &str) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, Some(team_name))?;

    println!("Migrating team '{}' to ephemeral session model...", team.name);

    // Find permanent workspaces
    let team_path = &team.path;
    let mut workspace_dirs = Vec::new();

    if let Ok(entries) = fs::read_dir(team_path) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                let marker = path.join(".botminter.workspace");
                if marker.exists() {
                    if let Some(name) = path.file_name() {
                        // Skip the team repo itself
                        if name != "team" {
                            workspace_dirs.push(path.clone());
                        }
                    }
                }
            }
        }
    }

    if workspace_dirs.is_empty() {
        println!("No permanent workspaces found for team '{}'.", team.name);
        return Ok(());
    }

    // Create shared-clones directory
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let shared_clones_dir = home.join(".botminter").join("shared-clones");
    fs::create_dir_all(&shared_clones_dir)
        .context("Failed to create shared-clones directory")?;

    println!("Created shared-clones directory: {}", shared_clones_dir.display());

    // Initialize shared clones from permanent workspace repos
    for workspace_dir in &workspace_dirs {
        migrate_workspace_repos(workspace_dir, &shared_clones_dir, &team)?;
    }

    println!("Migration complete. Use 'bm start' to create ephemeral sessions.");
    Ok(())
}

/// Migrate repos from a permanent workspace to shared clones.
fn migrate_workspace_repos(
    workspace_dir: &Path,
    shared_clones_dir: &Path,
    team: &config::TeamEntry,
) -> Result<()> {
    // Clone team repo if it exists in workspace
    let workspace_team_repo = workspace_dir.join("team");
    if workspace_team_repo.is_dir() && is_git_repo(&workspace_team_repo) {
        let team_repo_name = format!("{}-team", team.name);
        clone_to_shared(&workspace_team_repo, shared_clones_dir, &team_repo_name)?;
    }

    // Clone project repos if they exist in workspace
    let workspace_projects = workspace_dir.join("projects");
    if workspace_projects.is_dir() {
        if let Ok(entries) = fs::read_dir(&workspace_projects) {
            for entry in entries.filter_map(Result::ok) {
                let project_dir = entry.path();
                if project_dir.is_dir() && is_git_repo(&project_dir) {
                    if let Some(project_name) = project_dir.file_name().and_then(|n| n.to_str()) {
                        clone_to_shared(&project_dir, shared_clones_dir, project_name)?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if a directory is a git repository.
fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Clone a local repo to shared-clones as a bare repository.
fn clone_to_shared(source: &Path, shared_clones_dir: &Path, repo_name: &str) -> Result<()> {
    let target = shared_clones_dir.join(repo_name);

    // Skip if already exists
    if target.exists() {
        println!("  Skipping {} (already exists)", repo_name);
        return Ok(());
    }

    println!("  Cloning {} to shared-clones...", repo_name);

    // Use git clone --bare to create a bare repository
    let output = ProcessCommand::new("git")
        .args(["clone", "--bare"])
        .arg(source)
        .arg(&target)
        .output()
        .context("Failed to run git clone --bare")?;

    if !output.status.success() {
        bail!(
            "Failed to clone {} to shared-clones:\n{}",
            repo_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("  ✓ Cloned {}", repo_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_git_repo_detects_git_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).unwrap();

        // Not a repo yet
        assert!(!is_git_repo(&repo_dir));

        // Create .git directory
        fs::create_dir(repo_dir.join(".git")).unwrap();
        assert!(is_git_repo(&repo_dir));
    }
}
