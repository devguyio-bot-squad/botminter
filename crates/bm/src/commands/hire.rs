use std::fs;
use std::io::IsTerminal;

use anyhow::{Context, Result};

use crate::bridge::{self, CredentialStore};
use crate::config;
use crate::member_lifecycle::{self, AppCredentials, HireParams};

/// GitHub App credential flags from the CLI.
pub struct AppCredentialFlags<'a> {
    pub reuse_app: bool,
    pub app_id: Option<&'a str>,
    pub client_id: Option<&'a str>,
    pub private_key_file: Option<&'a str>,
    pub installation_id: Option<&'a str>,
    pub save_credentials: Option<&'a str>,
}

/// Handles `bm hire <role> [--name <name>] [-t team] [--reuse-app ...]`.
pub fn run(
    role: &str,
    name: Option<&str>,
    team_flag: Option<&str>,
    app_flags: AppCredentialFlags<'_>,
) -> Result<()> {
    super::ensure_profiles(false)?;
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;

    // Resolve App credentials from CLI flags (if --reuse-app)
    let app_credentials = if app_flags.reuse_app {
        let app_id = app_flags.app_id.context("--reuse-app requires --app-id")?;
        let client_id = app_flags.client_id.context("--reuse-app requires --client-id")?;
        let key_file = app_flags.private_key_file.context("--reuse-app requires --private-key-file")?;
        let installation_id = app_flags.installation_id.context("--reuse-app requires --installation-id")?;
        let private_key = fs::read_to_string(key_file)
            .with_context(|| format!("Failed to read private key file: {key_file}"))?;
        Some(AppCredentials {
            app_id: app_id.to_string(),
            client_id: client_id.to_string(),
            private_key,
            installation_id: installation_id.to_string(),
        })
    } else {
        None
    };

    // ── Domain call ───────────────────────────────────────────────
    let result = member_lifecycle::hire_member(&HireParams {
        team,
        role,
        name,
        app_credentials,
        save_credentials_path: app_flags.save_credentials,
    })?;

    // ── Display ───────────────────────────────────────────────────
    if result.already_existed {
        println!(
            "Member {} already exists in team '{}'. Storing App credentials.",
            result.member_dir_name, team.name
        );
    } else {
        println!("Hired {} as {} in team '{}'.", role, result.member_name, team.name);
    }

    if result.app_credentials_stored {
        println!("GitHub App credentials stored for {}.", result.member_dir_name);
        if !result.repos_checked.is_empty() {
            eprintln!("Ensuring App installation has access to repos...");
        }
    } else if !team.github_repo.is_empty() && !app_flags.reuse_app {
        eprintln!(
            "Note: GitHub App credentials not configured for {}.\n\
             Use --reuse-app with --app-id, --client-id, --private-key-file, \
             and --installation-id to provide App credentials.",
            result.member_name
        );
    }

    if let Some(path) = &result.credentials_saved_to {
        println!("Credentials saved to {path}");
    }

    // Prompt for bridge token if team has an external bridge configured
    prompt_bridge_token(&team.path.join("team"), team, &cfg, &result.member_dir_name)?;

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
