use std::fs;
use std::io::IsTerminal;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::bridge::{self, CredentialStore};
use crate::config;
use crate::profile;

use super::init::{finalize_member_manifest, run_git};

/// Handles `bm hire <role> [--name <name>] [-t team]`.
pub fn run(role: &str, name: Option<&str>, team_flag: Option<&str>) -> Result<()> {
    profile::ensure_profiles_initialized()?;
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Read team repo's botminter.yml for profile name + schema_version
    let manifest_path = team_repo.join("botminter.yml");
    let manifest: profile::ProfileManifest = {
        let contents = fs::read_to_string(&manifest_path)
            .context("Failed to read team repo's botminter.yml")?;
        serde_yml::from_str(&contents).context("Failed to parse botminter.yml")?
    };

    // Schema version guard
    profile::check_schema_version(&team.profile, &manifest.schema_version)?;

    // Verify role exists
    let available_roles = profile::list_roles(&team.profile)?;
    if !available_roles.contains(&role.to_string()) {
        bail!(
            "Role '{}' not available in profile '{}'. Available roles: {}",
            role,
            team.profile,
            available_roles.join(", ")
        );
    }

    // Determine member name: use --name flag or auto-generate suffix
    let member_name = match name {
        Some(n) => n.to_string(),
        None => auto_suffix(&team_repo, role)?,
    };

    let member_dir_name = format!("{}-{}", role, member_name);
    let member_dir = team_repo.join("members").join(&member_dir_name);

    if member_dir.exists() {
        bail!(
            "Member directory '{}' already exists. Choose a different name.",
            member_dir_name
        );
    }

    // Resolve coding agent for this team
    let coding_agent = profile::resolve_coding_agent(team, &manifest)?;

    // Extract member skeleton from embedded profile
    fs::create_dir_all(&member_dir)
        .with_context(|| format!("Failed to create member dir {}", member_dir.display()))?;

    profile::extract_member_to(&team.profile, role, &member_dir, coding_agent)?;

    // Finalize member manifest: .botminter.yml → botminter.yml with name added
    finalize_member_manifest(&member_dir, &member_name)?;

    // Git add + commit (no auto-push)
    run_git(
        &team_repo,
        &["add", &format!("members/{}/", member_dir_name)],
    )?;
    let commit_msg = format!("feat: hire {} as {}", role, member_name);
    run_git(&team_repo, &["commit", "-m", &commit_msg])?;

    println!(
        "Hired {} as {} in team '{}'.",
        role, member_name, team.name
    );

    // Prompt for bridge token if team has an external bridge configured
    if let Ok(Some(bridge_dir)) = bridge::discover(&team_repo, &team.name) {
        if let Ok(bridge_manifest) = bridge::load_manifest(&bridge_dir) {
            if bridge_manifest.spec.bridge_type == "external"
                && std::io::stdin().is_terminal()
            {
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

                if !token.is_empty() {
                    let workzone = team.path.parent().unwrap_or(&team.path);
                    let state_path = bridge::state_path(
                        workzone,
                        &team.name,
                    );
                    let cred_store = bridge::LocalCredentialStore::new(
                        &team.name,
                        &bridge_manifest.metadata.name,
                        state_path,
                    ).with_collection(cfg.keyring_collection.clone());
                    match cred_store.store(&member_name, &token) {
                        Ok(()) => {
                            println!(
                                "Bridge token stored for {}.",
                                member_name
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Could not store token in keyring: {}. \
                                 Set BM_BRIDGE_TOKEN_{} env var instead.",
                                e,
                                bridge::env_var_suffix_pub(&member_name)
                            );
                        }
                    }
                } else {
                    println!(
                        "No bridge token provided. Add later with: bm bridge identity add {}",
                        member_name
                    );
                }
            }
        }
    }

    Ok(())
}

/// Computes the next auto-suffix for a role by scanning existing member dirs.
/// Returns a 2-digit, zero-padded string (e.g., "01", "02").
/// Fills gaps: if 01 and 03 exist, returns "02".
fn auto_suffix(team_repo: &Path, role: &str) -> Result<String> {
    let team_members_dir = team_repo.join("members");
    let prefix = format!("{}-", role);

    let mut used: Vec<u32> = Vec::new();

    if team_members_dir.is_dir() {
        for entry in fs::read_dir(&team_members_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                let suffix = &name[prefix.len()..];
                if let Ok(n) = suffix.parse::<u32>() {
                    used.push(n);
                }
            }
        }
    }

    used.sort();

    // Fill gaps or increment from 1
    let mut next = 1u32;
    for &n in &used {
        if n == next {
            next = n + 1;
        } else if n > next {
            break; // gap found, use `next`
        }
    }

    Ok(format!("{:02}", next))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_suffix_first_member() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members")).unwrap();

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "01");
    }

    #[test]
    fn auto_suffix_increments() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "02");
    }

    #[test]
    fn auto_suffix_fills_gaps() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();
        fs::create_dir_all(team_repo.join("members/architect-03")).unwrap();

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "02");
    }

    #[test]
    fn auto_suffix_skips_non_numeric() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-bob")).unwrap();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();

        // "bob" is not numeric, so ignored. Next after 01 is 02.
        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "02");
    }

    #[test]
    fn auto_suffix_different_roles_independent() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();
        fs::create_dir_all(team_repo.join("members/architect-02")).unwrap();
        fs::create_dir_all(team_repo.join("members/dev-01")).unwrap();

        // dev suffix is independent of architect
        let result = auto_suffix(team_repo, "dev").unwrap();
        assert_eq!(result, "02");

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "03");
    }
}
