use std::fs;
use std::io::IsTerminal;

use anyhow::{Context, Result};

use crate::bridge::{self, CredentialStore};
use crate::config;
use crate::profile;

/// Handles `bm hire <role> [--name <name>] [-t team]`.
pub fn run(role: &str, name: Option<&str>, team_flag: Option<&str>) -> Result<()> {
    super::ensure_profiles(false)?;
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Read and validate manifest
    let manifest: profile::ProfileManifest = {
        let contents = fs::read_to_string(team_repo.join("botminter.yml"))
            .context("Failed to read team repo's botminter.yml")?;
        serde_yml::from_str(&contents).context("Failed to parse botminter.yml")?
    };
    profile::check_schema_version(&team.profile, &manifest.schema_version)?;
    let coding_agent = profile::resolve_coding_agent(team, &manifest)?;

    // Hire the member (domain operation)
    let result = profile::hire_member(&team_repo, &team.profile, role, name, coding_agent)?;

    println!(
        "Hired {} as {} in team '{}'.",
        role, result.member_name, team.name
    );

    // Prompt for bridge token if team has an external bridge configured
    prompt_bridge_token(&team_repo, team, &cfg, &result.member_name)?;

    Ok(())
}

/// Prompts for bridge token if the team has an external bridge and stdin is a TTY.
fn prompt_bridge_token(
    team_repo: &std::path::Path,
    team: &config::TeamEntry,
    cfg: &config::BotminterConfig,
    member_name: &str,
) -> Result<()> {
    let bridge_dir = match bridge::discover(team_repo, &team.name)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let bridge_manifest = match bridge::load_manifest(&bridge_dir) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };
    if bridge_manifest.spec.bridge_type != "external" || !std::io::stdin().is_terminal() {
        return Ok(());
    }

    let display_name = bridge_manifest
        .metadata
        .display_name
        .as_deref()
        .unwrap_or(&bridge_manifest.metadata.name);

    let token: String = cliclack::input(format!(
        "{} bot token for {} (optional, press Enter to skip)",
        display_name, member_name
    ))
    .default_input("")
    .interact()?;

    if token.is_empty() {
        println!(
            "No bridge token provided. Add later with: bm bridge identity add {}",
            member_name
        );
        return Ok(());
    }

    let workzone = team.path.parent().unwrap_or(&team.path);
    let state_path = bridge::state_path(workzone, &team.name);
    let cred_store = bridge::LocalCredentialStore::new(
        &team.name,
        &bridge_manifest.metadata.name,
        state_path,
    )
    .with_collection(cfg.keyring_collection.clone());

    match cred_store.store(member_name, &token) {
        Ok(()) => println!("Bridge token stored for {}.", member_name),
        Err(e) => eprintln!(
            "Warning: Could not store token in keyring: {}. \
             Set BM_BRIDGE_TOKEN_{} env var instead.",
            e,
            bridge::env_var_suffix_pub(member_name)
        ),
    }

    Ok(())
}
