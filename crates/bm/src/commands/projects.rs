use anyhow::{Context, Result};
use comfy_table::{
    ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table,
};

use crate::config;
use crate::formation::{self, CredentialDomain};
use crate::git;
use crate::git::manifest_flow::{self, credential_keys};
use crate::profile;
use crate::workspace;

/// Handles `bm projects list [-t team]`.
pub fn list(team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let manifest = profile::read_team_repo_manifest(&team.path.join("team"))?;

    if manifest.projects.is_empty() {
        println!("No projects configured. Run `bm projects add <url>` to add one.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Project", "Fork URL"]);
    for proj in &manifest.projects {
        table.add_row(vec![proj.name.as_str(), proj.fork_url.as_str()]);
    }
    println!("{table}");
    Ok(())
}

/// Handles `bm projects show <project> [-t team]`.
pub fn show(project: &str, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let manifest = profile::read_team_repo_manifest(&team_repo)?;

    let proj = manifest
        .projects
        .iter()
        .find(|p| p.name == project)
        .with_context(|| {
            let available: Vec<&str> = manifest.projects.iter().map(|p| p.name.as_str()).collect();
            if available.is_empty() {
                format!("Project '{}' not found. No projects configured — run `bm projects add <url>`.", project)
            } else {
                format!("Project '{}' not found. Available projects: {}", project, available.join(", "))
            }
        })?;

    println!("Project: {}", proj.name);
    println!("Fork URL: {}", proj.fork_url);

    let proj_dir = team_repo.join("projects").join(&proj.name);
    display_file_list("Knowledge", &profile::list_files_in_dir(&proj_dir.join("knowledge")));
    display_file_list("Invariants", &profile::list_files_in_dir(&proj_dir.join("invariants")));
    Ok(())
}

/// Handles `bm projects add <url> [-t team]`.
pub fn add(url: &str, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let name = git::add_project(
        &team_repo,
        url,
        &team.github_repo,
    )?;
    println!("Added project '{}' to team '{}'.", name, team.name);

    // Install all hired members' Apps on the new project repo (Req 17).
    if let Some(owner_repo) = manifest_flow::fork_url_to_owner_repo(url) {
        install_member_apps_on_repo(team, &team_repo, &owner_repo);
    }

    Ok(())
}

/// Installs each hired member's GitHub App on a project repo.
/// Skips members without credentials (warns and continues).
fn install_member_apps_on_repo(
    team: &config::TeamEntry,
    team_repo: &std::path::Path,
    owner_repo: &str,
) {
    let members_dir = team_repo.join("members");
    let members = match workspace::list_member_dirs(&members_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Warning: could not list members for App installation: {e}");
            return;
        }
    };

    if members.is_empty() {
        return;
    }

    let formation = match formation::create_local_formation(&team.name) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Warning: could not create formation for App installation: {e}");
            return;
        }
    };

    for member in &members {
        let cred_store = match formation.credential_store(CredentialDomain::GitHubApp {
            team_name: team.name.clone(),
            member_name: member.clone(),
        }) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: could not access credentials for {member}: {e}");
                continue;
            }
        };

        let installation_id = match cred_store.retrieve(&credential_keys::installation_id(member))
        {
            Ok(Some(id)) => id,
            Ok(None) => {
                eprintln!("Warning: no installation ID for member '{member}', skipping App installation on {owner_repo}");
                continue;
            }
            Err(e) => {
                eprintln!("Warning: could not read credentials for {member}: {e}");
                continue;
            }
        };

        let client_id = match cred_store.retrieve(&credential_keys::client_id(member)) {
            Ok(Some(id)) => id,
            Ok(None) => {
                eprintln!("Warning: no client ID for member '{member}', skipping App installation on {owner_repo}");
                continue;
            }
            Err(e) => {
                eprintln!("Warning: could not read client ID for {member}: {e}");
                continue;
            }
        };

        let private_key = match cred_store.retrieve(&credential_keys::private_key(member)) {
            Ok(Some(key)) => key,
            Ok(None) => {
                eprintln!("Warning: no private key for member '{member}', skipping App installation on {owner_repo}");
                continue;
            }
            Err(e) => {
                eprintln!("Warning: could not read private key for {member}: {e}");
                continue;
            }
        };

        eprintln!("Installing {member}'s App on {owner_repo}...");
        if let Err(e) = manifest_flow::ensure_app_on_repos(&installation_id, &client_id, &private_key, &[owner_repo]) {
            eprintln!("Warning: failed to install {member}'s App on {owner_repo}: {e}");
        }
    }
}

/// Handles `bm projects sync [-t team]`.
pub fn sync(team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let result = git::sync_project_board(&team.path.join("team"), team)?;

    println!("✓ Status field synced ({} options)", result.status_count);
    if result.views.is_empty() {
        println!("\nNo views defined in the profile.");
        return Ok(());
    }

    println!();
    println!("Your GitHub Project board needs role-based views so each role sees");
    println!("only its relevant statuses. Create one view per role listed below.");
    println!();
    println!("Open the board: {}", result.project_url);
    println!();
    println!("For each view:");
    println!("  1. Click \"+\" next to the existing view tabs");
    println!("  2. Choose \"Board\" layout");
    println!("  3. Rename the tab to the view name below");
    println!("  4. Click the filter bar and paste the filter string");
    println!("  5. Click save");
    println!("  6. To create the next view, click the tab dropdown → Duplicate view, then repeat from step 3");
    println!();

    let name_width = result.views.iter().map(|v| v.name.len()).max().unwrap_or(4).max(4);
    println!("  {:<width$}  Filter", "View", width = name_width);
    println!("  {:<width$}  ------", "----", width = name_width);
    for view in &result.views {
        println!("  {:<width$}  {}", view.name, view.filter, width = name_width);
    }
    println!();

    Ok(())
}

/// Displays a labeled list of files, or "none" if empty.
fn display_file_list(label: &str, files: &[String]) {
    println!();
    if files.is_empty() {
        println!("{}: none", label);
    } else {
        println!("{}:", label);
        for f in files {
            println!("  {}", f);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::git::derive_project_name;

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
}
