use std::fs;

use anyhow::{bail, Context, Result};

use crate::config;
use crate::formation::{self, CredentialDomain};
use crate::git::{app_auth, manifest_flow};
use crate::team::Team;

/// Handles `bm fire <member> [-t team] [--keep-app]`.
///
/// Executes a 6-step teardown sequence for a member:
/// 1. Stop the member (if running)
/// 2. Uninstall the GitHub App installation (unless --keep-app)
/// 3. Remove App credentials from keyring
/// 4. Remove member directory from team repo
/// 5. Remove member workspace
/// 6. Print manual App deletion instructions
///
/// Steps execute sequentially. On failure, the command reports what
/// succeeded and what failed — no rollback.
pub fn run(member: &str, team_flag: Option<&str>, keep_app: bool) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let members_dir = team_repo.join("members");

    // If the member directory is already gone, the desired end state is reached.
    // This makes `bm fire` idempotent per invariants/cli-idempotency.md.
    let member_dir = members_dir.join(member);
    if !member_dir.is_dir() {
        println!(
            "Member '{}' already removed from team '{}'. Nothing to do.",
            member, team.name
        );
        return Ok(());
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    // ── Step 1: Stop the member ────────────────────────────────────
    match stop_member(team, member) {
        Ok(stopped) => {
            if stopped {
                succeeded.push("Stopped member".to_string());
            } else {
                succeeded.push("Member was not running".to_string());
            }
        }
        Err(e) => {
            failed.push(format!("Stop member: {e}"));
            eprintln!("Warning: Failed to stop member: {e}");
        }
    }

    // ── Step 2: Uninstall GitHub App (unless --keep-app) ───────────
    if !keep_app {
        match uninstall_member_app(team, member) {
            Ok(uninstalled) => {
                if uninstalled {
                    succeeded.push("Uninstalled GitHub App".to_string());
                } else {
                    succeeded.push("No App credentials found (skipped uninstall)".to_string());
                }
            }
            Err(e) => {
                failed.push(format!("Uninstall App: {e}"));
                eprintln!("Warning: Failed to uninstall App: {e}");
            }
        }
    } else {
        succeeded.push("App installation preserved (--keep-app)".to_string());
    }

    // ── Step 3: Remove credentials from keyring ────────────────────
    match remove_credentials(team, member) {
        Ok(()) => {
            succeeded.push("Removed App credentials".to_string());
        }
        Err(e) => {
            failed.push(format!("Remove credentials: {e}"));
            eprintln!("Warning: Failed to remove credentials: {e}");
        }
    }

    // ── Step 4: Remove member directory from team repo ─────────────
    match fs::remove_dir_all(&member_dir) {
        Ok(()) => {
            succeeded.push("Removed member directory".to_string());
        }
        Err(e) => {
            failed.push(format!("Remove member directory: {e}"));
            eprintln!("Warning: Failed to remove member directory: {e}");
        }
    }

    // ── Step 5: Remove member workspace ────────────────────────────
    let workspace_dir = cfg.workzone.join(&team.name).join(member);
    if workspace_dir.is_dir() {
        match fs::remove_dir_all(&workspace_dir) {
            Ok(()) => {
                succeeded.push("Removed member workspace".to_string());
            }
            Err(e) => {
                failed.push(format!("Remove workspace: {e}"));
                eprintln!("Warning: Failed to remove workspace: {e}");
            }
        }
    } else {
        succeeded.push("No workspace found (skipped)".to_string());
    }

    // ── Step 6: Print summary and manual cleanup instructions ──────
    println!("\nFired '{}' from team '{}'.", member, team.name);
    println!("  Succeeded: {}", succeeded.join(", "));
    if !failed.is_empty() {
        eprintln!("  Failed: {}", failed.join(", "));
    }

    if !keep_app {
        println!(
            "\nNote: The GitHub App itself cannot be deleted via API.\n\
             To delete it, visit: https://github.com/organizations/{org}/settings/apps\n\
             Find the App associated with '{}' and click 'Delete'.",
            member,
            org = extract_org(&team.github_repo).unwrap_or("YOUR_ORG"),
        );
    }

    if !failed.is_empty() {
        bail!(
            "Some cleanup steps failed. Re-run `bm fire {}` or clean up manually.",
            member
        );
    }

    Ok(())
}

/// Stops a member via the Team → Formation → Daemon pipeline.
/// Returns Ok(true) if stopped, Ok(false) if not running.
fn stop_member(team: &config::TeamEntry, member: &str) -> Result<bool> {
    let local_formation = formation::create_local_formation(&team.name)?;
    let cfg = config::load()?;
    let team_api = Team::new(team, local_formation);
    let result = team_api.stop(&cfg, Some(member), false, false, false)?;

    if result.no_members_running {
        return Ok(false);
    }

    if !result.errors.is_empty() {
        bail!(
            "Failed to stop member '{}': {}",
            member,
            result.errors[0].error
        );
    }

    Ok(true)
}

/// Signs a JWT from the member's stored credentials and uninstalls the App.
/// Returns Ok(true) if uninstalled, Ok(false) if no credentials found.
fn uninstall_member_app(team: &config::TeamEntry, member: &str) -> Result<bool> {
    let formation = formation::create_local_formation(&team.name)?;
    let cred_store = formation.credential_store(CredentialDomain::GitHubApp {
        team_name: team.name.clone(),
        member_name: member.to_string(),
    })?;

    // Read credentials needed for JWT signing
    let client_id = match cred_store.retrieve(&manifest_flow::credential_keys::client_id(member))? {
        Some(id) => id,
        None => return Ok(false),
    };
    let private_key =
        match cred_store.retrieve(&manifest_flow::credential_keys::private_key(member))? {
            Some(key) => key,
            None => return Ok(false),
        };
    let installation_id =
        match cred_store.retrieve(&manifest_flow::credential_keys::installation_id(member))? {
            Some(id) => id,
            None => return Ok(false),
        };

    let inst_id: u64 = installation_id
        .parse()
        .context("Invalid installation ID — expected a number")?;

    // Sign JWT and uninstall
    let jwt = app_auth::generate_jwt(&client_id, &private_key)?;
    app_auth::uninstall_app(&jwt, inst_id)?;

    Ok(true)
}

/// Removes all GitHub App credentials for a member from the keyring.
fn remove_credentials(team: &config::TeamEntry, member: &str) -> Result<()> {
    let formation = formation::create_local_formation(&team.name)?;
    let cred_store = formation.credential_store(CredentialDomain::GitHubApp {
        team_name: team.name.clone(),
        member_name: member.to_string(),
    })?;

    manifest_flow::remove_member_credentials(cred_store.as_ref(), member)
}

/// Extracts the org from a "org/repo" github_repo string.
fn extract_org(github_repo: &str) -> Option<&str> {
    github_repo.split('/').next().filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_org_from_github_repo() {
        assert_eq!(extract_org("devguyio-bot-squad/my-team"), Some("devguyio-bot-squad"));
        assert_eq!(extract_org(""), None);
        assert_eq!(extract_org("org-only"), Some("org-only"));
    }
}
