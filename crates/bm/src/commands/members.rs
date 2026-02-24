use std::fs;

use anyhow::{bail, Context, Result};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};
use serde::Deserialize;

use crate::commands::start::{resolve_member_status, MemberStatus};
use crate::config;
use crate::state;

/// Minimal member manifest — only the fields we need for listing.
#[derive(Debug, Deserialize)]
struct MemberManifest {
    #[serde(default)]
    role: Option<String>,
}

/// Handles `bm members list [-t team]`.
pub fn list(team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let team_members_dir = team_repo.join("team");

    if !team_members_dir.is_dir() {
        println!("No members hired yet. Run `bm hire <role>` to hire a member.");
        return Ok(());
    }

    let mut entries: Vec<(String, String, String)> = Vec::new();

    for entry in fs::read_dir(&team_members_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden dirs (e.g., .gitkeep wouldn't be a dir, but be safe)
        if dir_name.starts_with('.') {
            continue;
        }

        let manifest_path = entry.path().join("botminter.yml");
        let role = if manifest_path.exists() {
            let contents = fs::read_to_string(&manifest_path)
                .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
            let manifest: MemberManifest = serde_yml::from_str(&contents)
                .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;
            manifest
                .role
                .unwrap_or_else(|| infer_role_from_dir(&dir_name))
        } else {
            infer_role_from_dir(&dir_name)
        };

        entries.push((dir_name, role, String::new()));
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    if entries.is_empty() {
        println!("No members hired yet. Run `bm hire <role>` to hire a member.");
        return Ok(());
    }

    let runtime_state = state::load().unwrap_or_default();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Member", "Role", "Status"]);

    for (member, role, _) in &entries {
        let status = resolve_member_status(&runtime_state, &team.name, member);
        table.add_row(vec![member.as_str(), role.as_str(), status.label()]);
    }

    println!("{table}");
    Ok(())
}

/// Handles `bm members show <member> [-t team]`.
pub fn show(member: &str, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let team_members_dir = team_repo.join("team");
    let member_dir = team_members_dir.join(member);

    if !member_dir.is_dir() {
        bail!(
            "Member '{}' not found in team '{}'. Run `bm members list` to see hired members.",
            member,
            team.name
        );
    }

    // Read role
    let manifest_path = member_dir.join("botminter.yml");
    let role = if manifest_path.exists() {
        let contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
        let manifest: MemberManifest = serde_yml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;
        manifest
            .role
            .unwrap_or_else(|| infer_role_from_dir(member))
    } else {
        infer_role_from_dir(member)
    };

    println!("Member: {}", member);
    println!("Role: {}", role);

    // Status from runtime state
    let runtime_state = state::load().unwrap_or_default();
    let status = resolve_member_status(&runtime_state, &team.name, member);
    match &status {
        MemberStatus::Running { pid, started_at } => {
            println!("Status: running");
            println!("PID: {}", pid);
            println!("Started: {}", started_at);
        }
        MemberStatus::Crashed { pid, started_at } => {
            println!("Status: crashed");
            println!("PID: {}", pid);
            println!("Started: {}", started_at);
        }
        MemberStatus::Stopped => {
            println!("Status: stopped");
        }
    }

    // Workspace path from runtime state
    let state_key = format!("{}/{}", team.name, member);
    if let Some(rt) = runtime_state.members.get(&state_key) {
        println!("Workspace: {}", rt.workspace.display());
    }

    // Knowledge files
    let knowledge_dir = member_dir.join("knowledge");
    let knowledge_files = list_files_in_dir(&knowledge_dir);
    println!();
    if knowledge_files.is_empty() {
        println!("Knowledge: none");
    } else {
        println!("Knowledge:");
        for f in &knowledge_files {
            println!("  {}", f);
        }
    }

    // Invariant files
    let invariants_dir = member_dir.join("invariants");
    let invariant_files = list_files_in_dir(&invariants_dir);
    if invariant_files.is_empty() {
        println!("Invariants: none");
    } else {
        println!("Invariants:");
        for f in &invariant_files {
            println!("  {}", f);
        }
    }

    Ok(())
}

/// Lists non-hidden files in a directory, returning their names sorted.
fn list_files_in_dir(dir: &std::path::Path) -> Vec<String> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && !e.file_name().to_string_lossy().starts_with('.')
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    files.sort();
    files
}

/// Infers the role from a member dir name by taking everything before the first '-'.
fn infer_role_from_dir(dir_name: &str) -> String {
    dir_name
        .split('-')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_role_standard_pattern() {
        assert_eq!(infer_role_from_dir("architect-alice"), "architect");
    }

    #[test]
    fn infer_role_multiple_hyphens() {
        assert_eq!(infer_role_from_dir("po-bob-senior"), "po");
    }

    #[test]
    fn infer_role_no_hyphen() {
        assert_eq!(infer_role_from_dir("superman"), "superman");
    }

    #[test]
    fn infer_role_empty_string() {
        assert_eq!(infer_role_from_dir(""), "");
    }
}
