use std::io::IsTerminal;

use anyhow::{bail, Result};

use crate::bridge;
use crate::config;
use crate::formation::{self, CredentialDomain};
use crate::git::{self, app_auth, manifest_flow};
use crate::team::Team;

/// Handles `bm fire <member> [-t team] [--keep-app] [--yes] [--delete-repo]`.
pub fn run(member: &str, team_flag: Option<&str>, keep_app: bool, yes: bool, delete_repo: bool) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let members_dir = team_repo.join("members");

    // Idempotency: if member directory is already gone, nothing to do.
    let member_dir = members_dir.join(member);
    if !member_dir.is_dir() {
        println!(
            "Member '{}' already removed from team '{}'. Nothing to do.",
            member, team.name
        );
        return Ok(());
    }

    // ── Interactive confirmation ───────────────────────────────────
    if !yes {
        if !std::io::stdin().is_terminal() {
            bail!(
                "Refusing to fire without confirmation in non-interactive mode.\n\
                 Use --yes to confirm: bm fire {} --yes",
                member
            );
        }
        let confirm: bool = cliclack::confirm(format!(
            "Fire '{}' from team '{}'? This will stop the member, remove credentials, and delete local files.",
            member, team.name
        ))
        .initial_value(false)
        .interact()?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    // ── Step 1: Stop the member ───────────────────────────────────
    let local_formation = formation::create_local_formation(&team.name)?;
    let team_api = Team::new(team, local_formation);
    match team_api.stop(&cfg, Some(member), false, false, false) {
        Ok(result) if result.no_members_running => {
            succeeded.push("Member was not running");
        }
        Ok(result) if !result.errors.is_empty() => {
            failed.push(format!("Stop member: {}", result.errors[0].error));
        }
        Ok(_) => {
            succeeded.push("Stopped member");
        }
        Err(e) => {
            failed.push(format!("Stop member: {e}"));
            eprintln!("Warning: Failed to stop member: {e}");
        }
    }

    // ── Step 2: Uninstall GitHub App (unless --keep-app) ──────────
    if !keep_app {
        let formation = formation::create_local_formation(&team.name)?;
        let cred_store = formation.credential_store(CredentialDomain::GitHubApp {
            team_name: team.name.clone(),
            member_name: member.to_string(),
        })?;

        let client_id = cred_store.retrieve(&manifest_flow::credential_keys::client_id(member))?;
        let private_key = cred_store.retrieve(&manifest_flow::credential_keys::private_key(member))?;
        let installation_id = cred_store.retrieve(&manifest_flow::credential_keys::installation_id(member))?;

        match (client_id, private_key, installation_id) {
            (Some(cid), Some(key), Some(iid)) => {
                match iid.parse::<u64>().ok().and_then(|inst_id| {
                    app_auth::generate_jwt(&cid, &key)
                        .and_then(|jwt| app_auth::uninstall_app(&jwt, inst_id))
                        .ok()
                }) {
                    Some(()) => succeeded.push("Uninstalled GitHub App"),
                    None => {
                        failed.push("Uninstall App: JWT or API call failed".to_string());
                        eprintln!("Warning: Failed to uninstall App");
                    }
                }
            }
            _ => succeeded.push("No App credentials found (skipped uninstall)"),
        }
    } else {
        succeeded.push("App installation preserved (--keep-app)");
    }

    // ── Step 3: Remove credentials from keyring ───────────────────
    let formation = formation::create_local_formation(&team.name)?;
    let cred_store = formation.credential_store(CredentialDomain::GitHubApp {
        team_name: team.name.clone(),
        member_name: member.to_string(),
    })?;
    match manifest_flow::remove_member_credentials(cred_store.as_ref(), member) {
        Ok(()) => succeeded.push("Removed App credentials"),
        Err(e) => {
            failed.push(format!("Remove credentials: {e}"));
            eprintln!("Warning: Failed to remove credentials: {e}");
        }
    }

    // ── Step 4: Remove bridge identity from state ─────────────────
    match bridge::discover(&team_repo, &team.name)? {
        Some(bridge_dir) => {
            let state_path = bridge::state_path(&cfg.workzone, &team.name);
            let mut b = bridge::Bridge::new(bridge_dir, state_path, team.name.clone())?;
            if b.identities().contains_key(member) {
                b.remove_identity(member);
                b.save()?;
                succeeded.push("Removed bridge identity");
            } else {
                succeeded.push("No bridge identity found (skipped)");
            }
        }
        None => succeeded.push("No bridge configured (skipped)"),
    }

    // ── Step 5: Remove member directory from team repo ────────────
    match std::fs::remove_dir_all(&member_dir) {
        Ok(()) => succeeded.push("Removed member directory"),
        Err(e) => {
            failed.push(format!("Remove member directory: {e}"));
            eprintln!("Warning: Failed to remove member directory: {e}");
        }
    }

    // ── Step 6: Remove member workspace ───────────────────────────
    let workspace_dir = cfg.workzone.join(&team.name).join(member);
    if workspace_dir.is_dir() {
        match std::fs::remove_dir_all(&workspace_dir) {
            Ok(()) => succeeded.push("Removed member workspace"),
            Err(e) => {
                failed.push(format!("Remove workspace: {e}"));
                eprintln!("Warning: Failed to remove workspace: {e}");
            }
        }
    } else {
        succeeded.push("No workspace found (skipped)");
    }

    // ── Step 7: Delete GitHub workspace repo ──────────────────────
    let org = team.github_repo.split('/').next().filter(|s| !s.is_empty());
    if let Some(org) = org {
        let ws_repo_name = format!("{}/{}-{}", org, team.name, member);
        let should_delete = if delete_repo {
            true
        } else if !yes && std::io::stdin().is_terminal() {
            cliclack::confirm(format!("Also delete GitHub repo '{}'?", ws_repo_name))
                .initial_value(false)
                .interact()?
        } else {
            false
        };

        if should_delete {
            match git::delete_repo(&ws_repo_name) {
                Ok(()) => succeeded.push("Deleted GitHub workspace repo"),
                Err(e) => {
                    failed.push(format!("Delete GitHub repo: {e}"));
                    eprintln!("Warning: Failed to delete GitHub repo: {e}");
                }
            }
        }
    }

    // ── Display summary ──────────────────────────────────────────
    println!("\nFired '{}' from team '{}'.", member, team.name);
    println!("  Succeeded: {}", succeeded.join(", "));
    if !failed.is_empty() {
        eprintln!("  Failed: {}", failed.join(", "));
    }

    if !keep_app {
        let org = team.github_repo.split('/').next().unwrap_or("YOUR_ORG");
        println!(
            "\nNote: The GitHub App itself cannot be deleted via API.\n\
             To delete it, visit: https://github.com/organizations/{}/settings/apps\n\
             Find the App associated with '{}' and click 'Delete'.",
            org, member
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

#[cfg(test)]
mod tests {
    #[test]
    fn extract_org_from_github_repo() {
        let repo = "devguyio-bot-squad/my-team";
        let org = repo.split('/').next().filter(|s| !s.is_empty());
        assert_eq!(org, Some("devguyio-bot-squad"));

        let empty = "";
        let org = empty.split('/').next().filter(|s| !s.is_empty());
        assert_eq!(org, None);
    }
}
