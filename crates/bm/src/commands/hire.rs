use std::fs;
use std::io::IsTerminal;

use anyhow::{bail, Context, Result};

use crate::bridge::{self, CredentialStore};
use crate::config;
use crate::member_lifecycle::{self, AppCredentials};
use crate::profile;

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
    let team_repo = team.path.join("team");

    // Read and validate manifest
    let manifest: profile::ProfileManifest = {
        let contents = fs::read_to_string(team_repo.join("botminter.yml"))
            .context("Failed to read team repo's botminter.yml")?;
        serde_yml::from_str(&contents).context("Failed to parse botminter.yml")?
    };
    profile::check_schema_version(&team.profile, &manifest.schema_version)?;
    let coding_agent = profile::resolve_coding_agent(team, &manifest)?;

    // Hire the member (domain operation — creates dir, renders placeholders)
    let result = profile::hire_member(&team_repo, &team.profile, role, name, coding_agent)?;

    if result.already_existed {
        if app_flags.reuse_app {
            println!(
                "Member {} already exists in team '{}'. Storing App credentials.",
                result.member_dir_name, team.name
            );
            store_app_credentials(team, &result.member_dir_name, &app_flags)?;
            return Ok(());
        }
        bail!(
            "Member '{}' already exists. Use --reuse-app to attach App credentials \
             to an existing member, or choose a different --name.",
            result.member_dir_name
        );
    }

    println!("Hired {} as {} in team '{}'.", role, result.member_name, team.name);

    // GitHub App credential storage
    if app_flags.reuse_app {
        store_app_credentials(team, &result.member_dir_name, &app_flags)?;
    } else if !team.github_repo.is_empty() {
        eprintln!(
            "Note: GitHub App credentials not configured for {}.\n\
             Use --reuse-app with --app-id, --client-id, --private-key-file, \
             and --installation-id to provide App credentials.",
            result.member_name
        );
    }

    // Prompt for bridge token if team has an external bridge configured
    prompt_bridge_token(&team_repo, team, &cfg, &result.member_dir_name)?;

    Ok(())
}

/// Resolves CLI flags into domain call for App credential setup.
fn store_app_credentials(
    team: &config::TeamEntry,
    member_name: &str,
    flags: &AppCredentialFlags<'_>,
) -> Result<()> {
    let app_id = flags.app_id.context("--reuse-app requires --app-id")?;
    let client_id = flags.client_id.context("--reuse-app requires --client-id")?;
    let key_file = flags.private_key_file.context("--reuse-app requires --private-key-file")?;
    let installation_id = flags.installation_id.context("--reuse-app requires --installation-id")?;

    let private_key = fs::read_to_string(key_file)
        .with_context(|| format!("Failed to read private key file: {key_file}"))?;

    let creds = AppCredentials {
        app_id: app_id.to_string(),
        client_id: client_id.to_string(),
        private_key,
        installation_id: installation_id.to_string(),
    };

    let result = member_lifecycle::setup_app_credentials(
        team,
        member_name,
        &creds,
        flags.save_credentials,
    )?;

    println!("GitHub App credentials stored for {}.", member_name);
    if !result.repos_checked.is_empty() {
        eprintln!("Ensuring App installation has access to repos...");
    }
    if let Some(path) = &result.credentials_saved_to {
        println!("Credentials saved to {path}");
    }

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
