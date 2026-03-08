use anyhow::Result;
use comfy_table::{Table, presets::UTF8_FULL_CONDENSED, modifiers::UTF8_ROUND_CORNERS};

use crate::profile;

/// Handles `bm profiles list` — displays a table of all embedded profiles.
pub fn list() -> Result<()> {
    profile::ensure_profiles_initialized()?;
    let names = profile::list_profiles()?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Profile", "Version", "Schema", "Description"]);

    for name in &names {
        let manifest = profile::read_manifest(name)?;
        table.add_row(vec![
            &manifest.name,
            &manifest.version,
            &manifest.schema_version,
            &manifest.description,
        ]);
    }

    println!("{table}");
    Ok(())
}

/// Handles `bm profiles describe <profile>` — shows full profile details.
/// When `show_tags` is true, appends a summary of agent-tagged files.
pub fn describe(name: &str, show_tags: bool) -> Result<()> {
    profile::ensure_profiles_initialized()?;
    let manifest = profile::read_manifest(name)?;

    println!("Profile: {}", manifest.name);
    println!("Display Name: {}", manifest.display_name);
    println!("Version: {}", manifest.version);
    println!("Schema: {}", manifest.schema_version);
    println!("Description: {}", manifest.description);

    println!();
    println!("Available Roles:");
    let roles = profile::list_roles(name)?;
    // Build a lookup from manifest roles for descriptions
    let role_descriptions: std::collections::HashMap<&str, &str> = manifest
        .roles
        .iter()
        .map(|r| (r.name.as_str(), r.description.as_str()))
        .collect();

    for role in &roles {
        let desc = role_descriptions
            .get(role.as_str())
            .unwrap_or(&"");
        println!("  {:<20} {}", role, desc);
    }

    println!();
    println!("Labels ({}):", manifest.labels.len());
    for label in &manifest.labels {
        println!("  {:<30} {}", label.name, label.description);
    }

    if !manifest.coding_agents.is_empty() {
        println!();
        println!("Coding Agents ({}):", manifest.coding_agents.len());
        let mut agent_keys: Vec<&String> = manifest.coding_agents.keys().collect();
        agent_keys.sort();
        for key in agent_keys {
            let agent = &manifest.coding_agents[key];
            let default_marker = if key == &manifest.default_coding_agent {
                " (default)"
            } else {
                ""
            };
            println!(
                "  {}{:<14} {} — context: {}, dir: {}, binary: {}",
                key, default_marker, agent.display_name, agent.context_file, agent.agent_dir, agent.binary
            );
        }
    }

    if show_tags {
        let tagged_files = profile::scan_agent_tags(name)?;
        println!();
        if tagged_files.is_empty() {
            println!("Coding-Agent Dependent Files: none");
        } else {
            println!("Coding-Agent Dependent Files ({} files):", tagged_files.len());
            for (path, agents) in &tagged_files {
                println!("  {} ({})", path, agents.join(", "));
            }
        }
    }

    Ok(())
}
