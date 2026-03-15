use std::collections::HashMap;
use std::io::IsTerminal;

use anyhow::Result;
use comfy_table::{ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};

use crate::bridge::{self, BridgeIdentity, CredentialStore};

use super::{make_bridge, make_credential_store, resolve_bridge};

/// Handles `bm bridge identity add <username> [-t team]`.
pub fn identity_add(username: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;

    let token_override = if bridge.is_external() && std::io::stdin().is_terminal() {
        let display_name = bridge.display_name().to_string();
        let token: String = cliclack::input(format!(
            "{} bot token for {}", display_name, username
        )).interact()?;
        if token.is_empty() { None } else { Some(token) }
    } else {
        None
    };

    let bridge_name = bridge.bridge_name().to_string();
    let cred_store = make_credential_store(&ctx, &bridge_name);
    let result = bridge.onboard_identity(username, token_override.as_deref(), &cred_store)?;
    bridge.save()?;

    if let Some(warning) = result.keyring_warning {
        eprintln!("Warning: {}", warning);
    }
    println!("Identity '{}' added to bridge '{}'.", username, bridge_name);
    Ok(())
}

/// Handles `bm bridge identity rotate <username> [-t team]`.
pub fn identity_rotate(username: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;
    let bridge_name = bridge.bridge_name().to_string();
    let cred_store = make_credential_store(&ctx, &bridge_name);
    let result = bridge.rotate_identity(username, &cred_store)?;
    bridge.save()?;

    if let Some(warning) = result.keyring_warning {
        eprintln!("Warning: {}", warning);
    }
    println!("Credentials rotated for '{}'.", username);
    Ok(())
}

/// Handles `bm bridge identity remove <username> [-t team]`.
pub fn identity_remove(username: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;
    let bridge_name = bridge.bridge_name().to_string();
    let cred_store = make_credential_store(&ctx, &bridge_name);
    bridge.offboard_identity(username, &cred_store)?;
    bridge.save()?;

    println!("Identity '{}' removed.", username);
    Ok(())
}

/// Handles `bm bridge identity show <username> [--reveal] [-t team]`.
pub fn identity_show(username: &str, reveal: bool, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let bridge = make_bridge(&ctx)?;
    let identity = bridge
        .identities()
        .get(username)
        .ok_or_else(|| anyhow::anyhow!("Identity '{}' not found.", username))?;

    println!("Username:   {}", identity.username);
    println!("User ID:    {}", identity.user_id);
    println!("Created:    {}", identity.created_at);

    let cred_store = make_credential_store(&ctx, bridge.bridge_name());
    match cred_store.retrieve(username) {
        Ok(Some(token)) => {
            if reveal {
                println!("Token:      {}", token);
            } else {
                let masked = if token.len() > 8 {
                    format!("{}...{}", &token[..4], &token[token.len() - 4..])
                } else {
                    "****".to_string()
                };
                println!("Token:      {} (use --reveal to show full token)", masked);
            }
        }
        Ok(None) => {
            let env_var = format!(
                "BM_BRIDGE_TOKEN_{}",
                bridge::env_var_suffix_pub(username)
            );
            println!("Token:      (not in keyring — set {} env var)", env_var);
        }
        Err(e) => println!("Token:      (keyring error: {})", e),
    }

    Ok(())
}

/// Handles `bm bridge identity list [-t team]`.
pub fn identity_list(team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let bridge = make_bridge(&ctx)?;
    if bridge.identities().is_empty() {
        println!("No identities registered.");
        return Ok(());
    }

    print_identity_table(bridge.identities());
    Ok(())
}

pub(super) fn print_identity_table(identities: &HashMap<String, BridgeIdentity>) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Username", "User ID", "Created"]);

    let mut entries: Vec<_> = identities.iter().collect();
    entries.sort_by_key(|(k, _)| (*k).clone());
    for (_key, identity) in entries {
        let display_name = if identity.is_operator {
            format!("{} [operator]", identity.username)
        } else {
            identity.username.clone()
        };
        table.add_row(vec![&display_name, &identity.user_id, &identity.created_at]);
    }
    println!("{table}");
}
