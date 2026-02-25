use std::fs;

use anyhow::{Context, Result};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};
use serde::Deserialize;

use crate::commands::init::run_git;
use crate::config;
use crate::profile;
use crate::workspace;

/// Minimal manifest for reading project count.
#[derive(Debug, Deserialize)]
struct TeamManifest {
    #[serde(default)]
    projects: Vec<profile::ProjectDef>,
}

/// Counts member directories under `team_repo/team/`.
fn count_members(team_repo: &std::path::Path) -> usize {
    let members_dir = team_repo.join("team");
    if !members_dir.is_dir() {
        return 0;
    }
    fs::read_dir(&members_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                        && !e
                            .file_name()
                            .to_string_lossy()
                            .starts_with('.')
                })
                .count()
        })
        .unwrap_or(0)
}

/// Reads project count from botminter.yml in the team repo.
fn read_projects(team_repo: &std::path::Path) -> Vec<profile::ProjectDef> {
    let manifest_path = team_repo.join("botminter.yml");
    if let Ok(contents) = fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_yml::from_str::<TeamManifest>(&contents) {
            return manifest.projects;
        }
    }
    Vec::new()
}

/// Handles `bm teams list` — displays a table of all registered teams.
pub fn list() -> Result<()> {
    let cfg = config::load()?;

    if cfg.teams.is_empty() {
        println!("No teams registered. Run `bm init` to create one.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Team", "Profile", "GitHub", "Members", "Projects", "Default"]);

    for team in &cfg.teams {
        let is_default = cfg.default_team.as_ref() == Some(&team.name);
        let default_marker = if is_default { "✔" } else { "" };
        let team_repo = team.path.join("team");
        let member_count = count_members(&team_repo);
        let project_count = read_projects(&team_repo).len();
        table.add_row(vec![
            team.name.as_str(),
            team.profile.as_str(),
            team.github_repo.as_str(),
            &member_count.to_string(),
            &project_count.to_string(),
            default_marker,
        ]);
    }

    println!("{table}");
    Ok(())
}

/// Handles `bm teams show [<name>] [-t team]` — displays detailed team info.
pub fn show(name: Option<&str>, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    // Resolve: positional name > -t flag > default
    let effective_flag = name.or(team_flag);
    let team = config::resolve_team(&cfg, effective_flag)?;
    let team_repo = team.path.join("team");
    let is_default = cfg.default_team.as_ref() == Some(&team.name);

    println!("Team: {}", team.name);
    println!("Profile: {}", team.profile);
    if !team.github_repo.is_empty() {
        println!("GitHub: {}", team.github_repo);
    }
    println!("Path: {}", team.path.display());
    println!("Default: {}", if is_default { "yes" } else { "no" });

    // Members section
    let members_dir = team_repo.join("team");
    let mut members: Vec<(String, String)> = Vec::new();
    if members_dir.is_dir() {
        for entry in fs::read_dir(&members_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let role = read_member_role(&members_dir, &name);
            members.push((name, role));
        }
    }
    members.sort_by(|a, b| a.0.cmp(&b.0));

    println!();
    if members.is_empty() {
        println!("Members: none");
    } else {
        println!("Members:");
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec!["Name", "Role"]);
        for (name, role) in &members {
            table.add_row(vec![name.as_str(), role.as_str()]);
        }
        println!("{table}");
    }

    // Projects section
    let projects = read_projects(&team_repo);
    println!();
    if projects.is_empty() {
        println!("Projects: none");
    } else {
        println!("Projects:");
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec!["Name", "Fork URL"]);
        for proj in &projects {
            table.add_row(vec![proj.name.as_str(), proj.fork_url.as_str()]);
        }
        println!("{table}");
    }

    Ok(())
}

/// Reads the role from a member's botminter.yml, falling back to dir-name inference.
fn read_member_role(members_dir: &std::path::Path, member_dir_name: &str) -> String {
    let manifest_path = members_dir.join(member_dir_name).join("botminter.yml");
    if let Ok(contents) = fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_yml::from_str::<MemberManifest>(&contents) {
            if let Some(role) = manifest.role {
                return role;
            }
        }
    }
    member_dir_name
        .split('-')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Minimal member manifest for reading role.
#[derive(Debug, Deserialize)]
struct MemberManifest {
    #[serde(default)]
    role: Option<String>,
}

/// Handles `bm teams sync [--push] [-t team]` — provisions and reconciles workspaces.
pub fn sync(push: bool, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Schema version guard
    let manifest_path = team_repo.join("botminter.yml");
    let manifest: profile::ProfileManifest = {
        let contents = fs::read_to_string(&manifest_path)
            .context("Failed to read team repo's botminter.yml")?;
        serde_yml::from_str(&contents).context("Failed to parse botminter.yml")?
    };
    profile::check_schema_version(&team.profile, &manifest.schema_version)?;

    // Optional push
    if push {
        run_git(&team_repo, &["push"])?;
    }

    // Discover hired members (scan team/team/ dir)
    let members_dir = team_repo.join("team");
    let mut members: Vec<String> = Vec::new();
    if members_dir.is_dir() {
        for entry in fs::read_dir(&members_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                members.push(name);
            }
        }
    }
    members.sort();

    if members.is_empty() {
        println!("No members hired. Run `bm hire <role>` first.");
        return Ok(());
    }

    let projects = &manifest.projects;
    let mut created = 0u32;
    let mut updated = 0u32;
    let mut failures: Vec<String> = Vec::new();

    for member_dir_name in &members {
        if projects.is_empty() {
            // No-project mode: workspace at {team.path}/{member_dir}/
            let ws = team.path.join(member_dir_name);
            let gh = Some(team.github_repo.as_str());
            if ws.join(".botminter").is_dir() {
                workspace::sync_workspace(&ws, member_dir_name, None, false, gh)?;
                updated += 1;
            } else {
                workspace::create_workspace(&team_repo, &team.path, member_dir_name, None, gh)?;
                created += 1;
            }
        } else {
            // Project mode: one workspace per member × project
            let gh = Some(team.github_repo.as_str());
            for proj in projects {
                let ws = team.path.join(member_dir_name).join(&proj.name);
                if ws.join(".botminter").is_dir() {
                    workspace::sync_workspace(
                        &ws,
                        member_dir_name,
                        Some(&proj.name),
                        true,
                        gh,
                    )?;
                    updated += 1;
                } else {
                    match workspace::create_workspace(
                        &team_repo,
                        &team.path,
                        member_dir_name,
                        Some((&proj.name, &proj.fork_url)),
                        gh,
                    ) {
                        Ok(()) => created += 1,
                        Err(e) => {
                            eprintln!(
                                "Error: {}/{}: {}",
                                member_dir_name, proj.name, e
                            );
                            failures.push(format!(
                                "{}/{} ({})",
                                member_dir_name, proj.name, proj.fork_url
                            ));
                        }
                    }
                }
            }
        }
    }

    let total = created + updated;
    println!(
        "Synced {} workspace{} ({} created, {} updated)",
        total,
        if total == 1 { "" } else { "s" },
        created,
        updated,
    );

    if !failures.is_empty() {
        anyhow::bail!(
            "{} workspace(s) failed to sync:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }

    Ok(())
}
