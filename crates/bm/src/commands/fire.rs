use std::io::IsTerminal;

use anyhow::{bail, Result};

use crate::config;
use crate::formation;
use crate::member_lifecycle::{self, FireParams};

/// Handles `bm fire <member> [-t team] [--keep-app] [--yes] [--delete-repo]`.
pub fn run(member: &str, team_flag: Option<&str>, keep_app: bool, yes: bool, delete_repo: bool) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Idempotency: if member directory is already gone, nothing to do.
    if !team_repo.join("members").join(member).is_dir() {
        println!("Member '{}' already removed from team '{}'. Nothing to do.", member, team.name);
        return Ok(());
    }

    // ── Interactive confirmations ──────────────────────────────────
    if !yes {
        if !std::io::stdin().is_terminal() {
            bail!("Refusing to fire without confirmation. Use --yes to confirm: bm fire {} --yes", member);
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

    let should_delete_repo = if delete_repo {
        true
    } else if !yes && std::io::stdin().is_terminal() {
        let org = team.github_repo.split('/').next().filter(|s| !s.is_empty());
        if let Some(org) = org {
            let ws_repo = format!("{}/{}-{}", org, team.name, member);
            cliclack::confirm(format!("Also delete GitHub repo '{}'?", ws_repo))
                .initial_value(false)
                .interact()?
        } else {
            false
        }
    } else {
        false
    };

    // ── Domain call ───────────────────────────────────────────────
    let local_formation = formation::create_local_formation(&team.name)?;
    let result = member_lifecycle::fire_member(
        &FireParams {
            team,
            config: &cfg,
            member,
            keep_app,
            delete_repo: should_delete_repo,
        },
        &*local_formation,
    )?;

    // ── Display ───────────────────────────────────────────────────
    let mut succeeded = Vec::new();
    if result.stopped { succeeded.push("Stopped member"); }
    if result.app_uninstalled { succeeded.push("Uninstalled GitHub App"); }
    if result.credentials_removed { succeeded.push("Removed App credentials"); }
    if result.bridge_identity_removed { succeeded.push("Removed bridge identity"); }
    if result.member_dir_removed { succeeded.push("Removed member directory"); }
    if result.workspace_removed { succeeded.push("Removed member workspace"); }
    if result.repo_deleted { succeeded.push("Deleted GitHub workspace repo"); }

    println!("\nFired '{}' from team '{}'.", member, team.name);
    if !succeeded.is_empty() {
        println!("  Succeeded: {}", succeeded.join(", "));
    }
    if !result.errors.is_empty() {
        for e in &result.errors {
            eprintln!("  Failed [{}]: {}", e.step, e.error);
        }
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

    if !result.errors.is_empty() {
        bail!("Some cleanup steps failed. Re-run `bm fire {}` or clean up manually.", member);
    }

    Ok(())
}
