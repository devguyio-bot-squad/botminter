use std::fmt::Write;

use anyhow::Result;
use comfy_table::{ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};

use crate::bridge;
use crate::config;
use crate::profile;

/// Handles `bm teams show [<name>] [-t team]` — displays detailed team info.
pub fn show(name: Option<&str>, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let effective_flag = name.or(team_flag);
    let team = config::resolve_team(&cfg, effective_flag)?;
    let team_repo = team.path.join("team");
    let is_default = cfg.default_team.as_ref() == Some(&team.name);

    println!("Team: {}", team.name);
    println!("Profile: {}", team.profile);

    if let Ok(profiles_path) = profile::profiles_dir() {
        let profile_source = profiles_path.join(&team.profile);
        if profile_source.is_dir() {
            println!("Profile Source: {}", profile_source.display());
        }
    }

    let manifest_path = team_repo.join("botminter.yml");
    if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_yml::from_str::<profile::ProfileManifest>(&contents) {
            if let Ok(agent) = profile::resolve_coding_agent(team, &manifest) {
                println!("Coding Agent: {}", agent.display_name);
            }
        }
    }

    if let Ok(Some(bridge_dir)) = bridge::discover(&team_repo, &team.name) {
        let state_path = bridge::state_path(&cfg.workzone, &team.name);
        if let Ok(b) = bridge::Bridge::new(bridge_dir, state_path, team.name.clone()) {
            println!("Bridge: {} [{}]", b.display_name(), b.bridge_type());
        }
    }

    if !team.github_repo.is_empty() {
        println!("GitHub: {}", team.github_repo);
    }
    if let Some(number) = team.project_number {
        let owner = team.github_repo.split('/').next().unwrap_or(&team.github_repo);
        println!("Board: https://github.com/orgs/{}/projects/{}", owner, number);
    }
    println!("Path: {}", team.path.display());
    println!("Default: {}", if is_default { "yes" } else { "no" });

    let summary = profile::gather_team_summary(&team_repo);
    print!("{}", format_team_summary(&summary));

    Ok(())
}

/// Formats a TeamSummary into a display string with tables.
pub fn format_team_summary(summary: &profile::TeamSummary) -> String {
    let mut out = String::new();

    writeln!(out).unwrap();
    if summary.members.is_empty() {
        writeln!(out, "Members: none").unwrap();
    } else {
        writeln!(out, "Members:").unwrap();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Name", "Role"]);
        for (name, role) in &summary.members {
            table.add_row(vec![name.as_str(), role.as_str()]);
        }
        writeln!(out, "{table}").unwrap();
    }

    writeln!(out).unwrap();
    if summary.projects.is_empty() {
        writeln!(out, "Projects: none").unwrap();
    } else {
        writeln!(out, "Projects:").unwrap();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Name", "Fork URL"]);
        for proj in &summary.projects {
            table.add_row(vec![proj.name.as_str(), proj.fork_url.as_str()]);
        }
        writeln!(out, "{table}").unwrap();
    }

    out
}
