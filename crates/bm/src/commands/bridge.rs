use std::io::IsTerminal;

use anyhow::{bail, Result};
use comfy_table::{ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};

use crate::bridge::{self, Bridge, BridgeIdentity, BridgeRoom, LocalCredentialStore, CredentialStore};
use crate::config;

/// Common setup: load config, resolve team, check `just` is installed, discover bridge.
struct BridgeContext {
    team_name: String,
    bridge_dir: std::path::PathBuf,
    workzone: std::path::PathBuf,
    keyring_collection: Option<String>,
}

fn resolve_bridge(team_flag: Option<&str>) -> Result<Option<BridgeContext>> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let team_name = team.name.clone();
    let workzone = cfg.workzone.clone();
    let keyring_collection = cfg.keyring_collection.clone();

    if which::which("just").is_err() {
        bail!(
            "Bridge commands require 'just'. Install it: https://just.systems/"
        );
    }

    match bridge::discover(&team_repo, &team_name)? {
        Some(bridge_dir) => Ok(Some(BridgeContext {
            team_name, bridge_dir, workzone, keyring_collection,
        })),
        None => {
            println!("No bridge configured for team '{}'.", team_name);
            Ok(None)
        }
    }
}

/// Constructs a `Bridge` from the resolved context.
fn make_bridge(ctx: &BridgeContext) -> Result<Bridge> {
    let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
    Bridge::new(ctx.bridge_dir.clone(), state_path, ctx.team_name.clone())
}

/// Handles `bm bridge start [-t team]`.
pub fn start(team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;
    bridge.start()?;
    bridge.save()?;
    Ok(())
}

/// Handles `bm bridge stop [-t team]`.
pub fn stop(team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;
    bridge.stop()?;
    bridge.save()?;
    Ok(())
}

/// Handles `bm bridge status [--reveal] [-t team]`.
pub fn status(team_flag: Option<&str>, reveal: bool) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let bridge = make_bridge(&ctx)?;

    if !bridge.is_active() {
        println!("No bridge active for team '{}'.", ctx.team_name);
        return Ok(());
    }

    println!("Bridge: {}", bridge.bridge_name());
    println!("Type: {}", bridge.bridge_type());
    println!("Status: {}", bridge.status());
    if let Some(url) = bridge.service_url() {
        println!("URL: {}", url);
    }
    if let Some(started) = bridge.started_at() {
        println!("Started: {}", started);
    }

    if let Some(op_username) = bridge.operator_username() {
        let op_user_id = bridge.member_user_id(op_username);
        println!(
            "Operator: {} ({})",
            op_username,
            op_user_id.as_deref().unwrap_or("not provisioned")
        );

        if reveal {
            let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
            let credential_store = LocalCredentialStore::new(
                &ctx.team_name,
                bridge.bridge_name(),
                state_path,
            )
            .with_collection(ctx.keyring_collection.clone());
            match credential_store.retrieve(op_username) {
                Ok(Some(token)) => println!("Operator Token: {}", token),
                Ok(None) => println!("Operator Token: (not in keyring)"),
                Err(e) => println!("Operator Token: (keyring error: {})", e),
            }
        }
    }

    if !bridge.identities().is_empty() {
        println!();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Username", "User ID", "Created"]);

        let mut entries: Vec<_> = bridge.identities().iter().collect();
        entries.sort_by_key(|(k, _)| (*k).clone());
        for (_key, identity) in entries {
            let display_name = if identity.is_operator {
                format!("{} [operator]", identity.username)
            } else {
                identity.username.clone()
            };
            table.add_row(vec![
                &display_name,
                &identity.user_id,
                &identity.created_at,
            ]);
        }
        println!("{table}");
    }

    if !bridge.rooms().is_empty() {
        println!();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Room", "Room ID", "Created"]);

        for room in bridge.rooms() {
            table.add_row(vec![
                &room.name,
                room.room_id.as_deref().unwrap_or("—"),
                &room.created_at,
            ]);
        }
        println!("{table}");
    }

    Ok(())
}

/// Handles `bm bridge identity add <username> [-t team]`.
pub fn identity_add(username: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;

    // For external bridges, prompt for token if interactive (mirrors hire.rs pattern)
    if bridge.is_external() && std::io::stdin().is_terminal() {
        let display_name = bridge.display_name().to_string();
        let token: String = cliclack::input(format!(
            "{} bot token for {}",
            display_name, username
        ))
        .interact()?;

        if !token.is_empty() {
            let env_var = format!(
                "BM_BRIDGE_TOKEN_{}",
                crate::bridge::env_var_suffix_pub(username)
            );
            std::env::set_var(&env_var, &token);
        }
    }

    let onboard_recipe = bridge.manifest().spec.identity.onboard.clone();
    let result = bridge.invoke_recipe(&onboard_recipe, &[username])?;

    let now = chrono::Utc::now().to_rfc3339();

    // Extract token from recipe result and store in keyring
    let mut token_str = String::new();
    let identity = if let Some(val) = result {
        if let Some(tok) = val.get("token").and_then(|v| v.as_str()) {
            token_str = tok.to_string();
        }
        BridgeIdentity {
            username: val
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or(username)
                .to_string(),
            user_id: val
                .get("user_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            token: None, // No longer stored in bridge-state.json
            created_at: now,
            is_operator: false,
        }
    } else {
        BridgeIdentity {
            username: username.to_string(),
            user_id: String::new(),
            token: None,
            created_at: now,
            is_operator: false,
        }
    };

    let bridge_name = bridge.bridge_name().to_string();

    bridge.add_identity(username.to_string(), identity);
    bridge.save()?;

    // Store token in system keyring via CredentialStore (best-effort)
    if !token_str.is_empty() {
        let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
        let credential_store = LocalCredentialStore::new(
            &ctx.team_name,
            &bridge_name,
            state_path,
        ).with_collection(ctx.keyring_collection.clone());
        if let Err(e) = credential_store.store(username, &token_str) {
            eprintln!(
                "Warning: Could not store token in system keyring: {}\n\
                 Set BM_BRIDGE_TOKEN_{} environment variable instead.",
                e,
                crate::bridge::env_var_suffix_pub(username)
            );
        }
    }

    println!("Identity '{}' added to bridge '{}'.", username, &bridge_name);
    Ok(())
}

/// Handles `bm bridge identity rotate <username> [-t team]`.
pub fn identity_rotate(username: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;

    if !bridge.identities().contains_key(username) {
        bail!(
            "Identity '{}' not found. Run 'bm bridge identity list' to see registered identities.",
            username
        );
    }

    let rotate_recipe = bridge.manifest().spec.identity.rotate_credentials.clone();
    let bridge_name = bridge.bridge_name().to_string();
    let result = bridge.invoke_recipe(&rotate_recipe, &[username])?;

    if let Some(val) = result {
        if let Some(user_id) = val.get("user_id").and_then(|v| v.as_str()) {
            bridge.update_identity_user_id(username, user_id);
        }
        // Store rotated token in keyring (best-effort)
        if let Some(token) = val.get("token").and_then(|v| v.as_str()) {
            let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
            let credential_store = LocalCredentialStore::new(
                &ctx.team_name,
                &bridge_name,
                state_path,
            ).with_collection(ctx.keyring_collection.clone());
            if let Err(e) = credential_store.store(username, token) {
                eprintln!(
                    "Warning: Could not store rotated token in system keyring: {}\n\
                     Set BM_BRIDGE_TOKEN_{} environment variable instead.",
                    e,
                    crate::bridge::env_var_suffix_pub(username)
                );
            }
        }
    }

    bridge.save()?;
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

    if !bridge.identities().contains_key(username) {
        bail!(
            "Identity '{}' not found. Run 'bm bridge identity list' to see registered identities.",
            username
        );
    }

    let remove_recipe = bridge.manifest().spec.identity.remove.clone();
    let bridge_name = bridge.bridge_name().to_string();
    bridge.invoke_recipe(&remove_recipe, &[username])?;

    bridge.remove_identity(username);
    bridge.save()?;

    // Remove credential from keyring
    let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
    let credential_store = LocalCredentialStore::new(
        &ctx.team_name,
        &bridge_name,
        state_path,
    ).with_collection(ctx.keyring_collection.clone());
    // Best-effort: don't fail if keyring is unavailable
    let _ = credential_store.remove(username);

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
    let bridge_name = bridge.bridge_name().to_string();

    let identity = bridge
        .identities()
        .get(username)
        .ok_or_else(|| anyhow::anyhow!("Identity '{}' not found.", username))?;

    println!("Username:   {}", identity.username);
    println!("User ID:    {}", identity.user_id);
    println!("Created:    {}", identity.created_at);

    // Retrieve token from keyring
    let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
    let credential_store = LocalCredentialStore::new(
        &ctx.team_name,
        &bridge_name,
        state_path,
    ).with_collection(ctx.keyring_collection.clone());

    match credential_store.retrieve(username) {
        Ok(Some(token)) => {
            if reveal {
                println!("Token:      {}", token);
            } else {
                let masked = if token.len() > 8 {
                    format!("{}...{}", &token[..4], &token[token.len()-4..])
                } else {
                    "****".to_string()
                };
                println!("Token:      {} (use --reveal to show full token)", masked);
            }
        }
        Ok(None) => {
            let env_var = format!(
                "BM_BRIDGE_TOKEN_{}",
                crate::bridge::env_var_suffix_pub(username)
            );
            println!("Token:      (not in keyring — set {} env var)", env_var);
        }
        Err(e) => {
            println!("Token:      (keyring error: {})", e);
        }
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

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Username", "User ID", "Created"]);

    let mut entries: Vec<_> = bridge.identities().iter().collect();
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
    Ok(())
}

/// Handles `bm bridge room create <name> [-t team]`.
pub fn room_create(name: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;

    let room_spec = bridge.manifest().spec.room.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Bridge '{}' does not support room management.",
            bridge.bridge_name()
        )
    })?;
    let create_recipe = room_spec.create.clone();

    let result = bridge.invoke_recipe(&create_recipe, &[name])?;

    let now = chrono::Utc::now().to_rfc3339();

    let room = if let Some(val) = result {
        BridgeRoom {
            name: val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string(),
            room_id: val
                .get("room_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            created_at: now,
        }
    } else {
        BridgeRoom {
            name: name.to_string(),
            room_id: None,
            created_at: now,
        }
    };

    bridge.add_room(room);
    bridge.save()?;

    println!("Room '{}' created.", name);
    Ok(())
}

/// Handles `bm bridge room list [-t team]`.
pub fn room_list(team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let bridge = make_bridge(&ctx)?;

    let room_spec = bridge.manifest().spec.room.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Bridge '{}' does not support room management.",
            bridge.bridge_name()
        )
    })?;
    let list_recipe = room_spec.list.clone();

    let result = bridge.invoke_recipe(&list_recipe, &[])?;

    // Prefer live data from recipe if available, otherwise show state
    if let Some(val) = result {
        if let Some(rooms) = val.get("rooms").and_then(|r| r.as_array()) {
            if rooms.is_empty() {
                println!("No rooms found.");
                return Ok(());
            }

            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL_CONDENSED)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_content_arrangement(ContentArrangement::DynamicFullWidth)
                .set_header(vec!["Room", "Room ID"]);

            for room in rooms {
                let name = room.get("name").and_then(|n| n.as_str()).unwrap_or("—");
                let room_id = room.get("room_id").and_then(|n| n.as_str()).unwrap_or("—");
                table.add_row(vec![name, room_id]);
            }

            println!("{table}");
            return Ok(());
        }
    }

    // Fallback to state
    if bridge.rooms().is_empty() {
        println!("No rooms found.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Room", "Room ID"]);

    for room in bridge.rooms() {
        table.add_row(vec![
            &room.name,
            room.room_id.as_deref().unwrap_or("—"),
        ]);
    }

    println!("{table}");
    Ok(())
}
