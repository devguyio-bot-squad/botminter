use std::io::IsTerminal;

use anyhow::{bail, Result};
use comfy_table::{ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};

use crate::bridge::{self, BridgeIdentity, BridgeRoom, LocalCredentialStore, CredentialStore};
use crate::config;

/// Common setup: load config, resolve team, check `just` is installed, discover bridge.
/// Returns (team_name, bridge_dir, team_repo_path, workzone).
fn resolve_bridge(
    team_flag: Option<&str>,
) -> Result<Option<(String, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)>> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let team_name = team.name.clone();
    let workzone = cfg.workzone.clone();

    if which::which("just").is_err() {
        bail!(
            "Bridge commands require 'just'. Install it: https://just.systems/"
        );
    }

    match bridge::discover(&team_repo, &team_name)? {
        Some(bridge_dir) => Ok(Some((team_name, bridge_dir, team_repo, workzone))),
        None => {
            println!("No bridge configured for team '{}'.", team_name);
            Ok(None)
        }
    }
}

/// Handles `bm bridge start [-t team]`.
pub fn start(team_flag: Option<&str>) -> Result<()> {
    let (team_name, bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let manifest = bridge::load_manifest(&bridge_dir)?;
    let bridge_name = &manifest.metadata.name;

    if manifest.spec.bridge_type == "external" {
        println!(
            "Bridge '{}' is external -- lifecycle commands are not available. The service is managed externally.",
            bridge_name
        );
        return Ok(());
    }

    let lifecycle = manifest.spec.lifecycle.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Bridge '{}' is local but has no lifecycle section in bridge.yml",
            bridge_name
        )
    })?;

    let start_result =
        bridge::invoke_recipe(&bridge_dir, &lifecycle.start, &[], &team_name)?;
    bridge::invoke_recipe(&bridge_dir, &lifecycle.health, &[], &team_name)?;

    let state_path = bridge::state_path(&workzone, &team_name);
    let mut state = bridge::load_state(&state_path)?;

    let now = chrono::Utc::now().to_rfc3339();
    state.bridge_name = Some(bridge_name.clone());
    state.bridge_type = Some(manifest.spec.bridge_type.clone());
    state.status = "running".to_string();
    state.started_at = Some(now.clone());
    state.last_health_check = Some(now);

    // Extract service_url from the start recipe's config exchange
    if let Some(val) = start_result {
        if let Some(url) = val.get("url").and_then(|u| u.as_str()) {
            state.service_url = Some(url.to_string());
        }
    }

    bridge::save_state(&state_path, &state)?;
    println!("Bridge '{}' started.", bridge_name);
    Ok(())
}

/// Handles `bm bridge stop [-t team]`.
pub fn stop(team_flag: Option<&str>) -> Result<()> {
    let (team_name, bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let manifest = bridge::load_manifest(&bridge_dir)?;
    let bridge_name = &manifest.metadata.name;

    if manifest.spec.bridge_type == "external" {
        println!(
            "Bridge '{}' is external -- lifecycle commands are not available. The service is managed externally.",
            bridge_name
        );
        return Ok(());
    }

    let lifecycle = manifest.spec.lifecycle.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Bridge '{}' is local but has no lifecycle section in bridge.yml",
            bridge_name
        )
    })?;

    bridge::invoke_recipe(&bridge_dir, &lifecycle.stop, &[], &team_name)?;

    let state_path = bridge::state_path(&workzone, &team_name);
    let mut state = bridge::load_state(&state_path)?;
    state.status = "stopped".to_string();
    state.started_at = None;
    bridge::save_state(&state_path, &state)?;

    println!("Bridge '{}' stopped.", bridge_name);
    Ok(())
}

/// Handles `bm bridge status [-t team]`.
pub fn status(team_flag: Option<&str>) -> Result<()> {
    let (team_name, _bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let state_path = bridge::state_path(&workzone, &team_name);
    let state = bridge::load_state(&state_path)?;

    if state.bridge_name.is_none() {
        println!("No bridge active for team '{}'.", team_name);
        return Ok(());
    }

    let bridge_name = state.bridge_name.as_deref().unwrap_or("unknown");
    let bridge_type = state.bridge_type.as_deref().unwrap_or("unknown");

    println!("Bridge: {}", bridge_name);
    println!("Type: {}", bridge_type);
    println!("Status: {}", state.status);
    if let Some(ref url) = state.service_url {
        println!("URL: {}", url);
    }
    if let Some(ref started) = state.started_at {
        println!("Started: {}", started);
    }

    if !state.identities.is_empty() {
        println!();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Username", "User ID", "Created"]);

        let mut entries: Vec<_> = state.identities.iter().collect();
        entries.sort_by_key(|(k, _)| (*k).clone());
        for (_key, identity) in entries {
            table.add_row(vec![
                &identity.username,
                &identity.user_id,
                &identity.created_at,
            ]);
        }
        println!("{table}");
    }

    if !state.rooms.is_empty() {
        println!();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Room", "Room ID", "Created"]);

        for room in &state.rooms {
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
    let (team_name, bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let manifest = bridge::load_manifest(&bridge_dir)?;
    let bridge_name = &manifest.metadata.name;

    // For external bridges, prompt for token if interactive (mirrors hire.rs pattern)
    if manifest.spec.bridge_type == "external" && std::io::stdin().is_terminal() {
        let display_name = manifest
            .metadata
            .display_name
            .as_deref()
            .unwrap_or(&manifest.metadata.name);
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

    let result = bridge::invoke_recipe(
        &bridge_dir,
        &manifest.spec.identity.onboard,
        &[username],
        &team_name,
    )?;

    let state_path = bridge::state_path(&workzone, &team_name);
    let mut state = bridge::load_state(&state_path)?;

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
        }
    } else {
        BridgeIdentity {
            username: username.to_string(),
            user_id: String::new(),
            token: None,
            created_at: now,
        }
    };

    state
        .identities
        .insert(username.to_string(), identity);

    if state.bridge_name.is_none() {
        state.bridge_name = Some(bridge_name.clone());
        state.bridge_type = Some(manifest.spec.bridge_type.clone());
    }

    bridge::save_state(&state_path, &state)?;

    // Store token in system keyring via CredentialStore (best-effort)
    if !token_str.is_empty() {
        let credential_store = LocalCredentialStore::new(
            &team_name,
            bridge_name,
            state_path,
        );
        if let Err(e) = credential_store.store(username, &token_str) {
            eprintln!(
                "Warning: Could not store token in system keyring: {}\n\
                 Set BM_BRIDGE_TOKEN_{} environment variable instead.",
                e,
                crate::bridge::env_var_suffix_pub(username)
            );
        }
    }

    println!("Identity '{}' added to bridge '{}'.", username, bridge_name);
    Ok(())
}

/// Handles `bm bridge identity rotate <username> [-t team]`.
pub fn identity_rotate(username: &str, team_flag: Option<&str>) -> Result<()> {
    let (team_name, bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let state_path = bridge::state_path(&workzone, &team_name);
    let mut state = bridge::load_state(&state_path)?;

    if !state.identities.contains_key(username) {
        bail!(
            "Identity '{}' not found. Run 'bm bridge identity list' to see registered identities.",
            username
        );
    }

    let manifest = bridge::load_manifest(&bridge_dir)?;
    let bridge_name = &manifest.metadata.name;
    let result = bridge::invoke_recipe(
        &bridge_dir,
        &manifest.spec.identity.rotate_credentials,
        &[username],
        &team_name,
    )?;

    if let Some(val) = result {
        if let Some(identity) = state.identities.get_mut(username) {
            if let Some(user_id) = val.get("user_id").and_then(|v| v.as_str()) {
                identity.user_id = user_id.to_string();
            }
        }
        // Store rotated token in keyring (best-effort)
        if let Some(token) = val.get("token").and_then(|v| v.as_str()) {
            let credential_store = LocalCredentialStore::new(
                &team_name,
                bridge_name,
                state_path.clone(),
            );
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

    bridge::save_state(&state_path, &state)?;
    println!("Credentials rotated for '{}'.", username);
    Ok(())
}

/// Handles `bm bridge identity remove <username> [-t team]`.
pub fn identity_remove(username: &str, team_flag: Option<&str>) -> Result<()> {
    let (team_name, bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let state_path = bridge::state_path(&workzone, &team_name);
    let mut state = bridge::load_state(&state_path)?;

    if !state.identities.contains_key(username) {
        bail!(
            "Identity '{}' not found. Run 'bm bridge identity list' to see registered identities.",
            username
        );
    }

    let manifest = bridge::load_manifest(&bridge_dir)?;
    let bridge_name = &manifest.metadata.name;
    bridge::invoke_recipe(
        &bridge_dir,
        &manifest.spec.identity.remove,
        &[username],
        &team_name,
    )?;

    state.identities.remove(username);
    bridge::save_state(&state_path, &state)?;

    // Remove credential from keyring
    let credential_store = LocalCredentialStore::new(
        &team_name,
        bridge_name,
        state_path,
    );
    // Best-effort: don't fail if keyring is unavailable
    let _ = credential_store.remove(username);

    println!("Identity '{}' removed.", username);
    Ok(())
}

/// Handles `bm bridge identity list [-t team]`.
pub fn identity_list(team_flag: Option<&str>) -> Result<()> {
    let (team_name, _bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let state_path = bridge::state_path(&workzone, &team_name);
    let state = bridge::load_state(&state_path)?;

    if state.identities.is_empty() {
        println!("No identities registered.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Username", "User ID", "Created"]);

    let mut entries: Vec<_> = state.identities.iter().collect();
    entries.sort_by_key(|(k, _)| (*k).clone());
    for (_key, identity) in entries {
        table.add_row(vec![
            &identity.username,
            &identity.user_id,
            &identity.created_at,
        ]);
    }

    println!("{table}");
    Ok(())
}

/// Handles `bm bridge room create <name> [-t team]`.
pub fn room_create(name: &str, team_flag: Option<&str>) -> Result<()> {
    let (team_name, bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let manifest = bridge::load_manifest(&bridge_dir)?;
    let bridge_name = &manifest.metadata.name;

    let room_spec = manifest.spec.room.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Bridge '{}' does not support room management.",
            bridge_name
        )
    })?;

    let result =
        bridge::invoke_recipe(&bridge_dir, &room_spec.create, &[name], &team_name)?;

    let state_path = bridge::state_path(&workzone, &team_name);
    let mut state = bridge::load_state(&state_path)?;

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

    state.rooms.push(room);
    bridge::save_state(&state_path, &state)?;

    println!("Room '{}' created.", name);
    Ok(())
}

/// Handles `bm bridge room list [-t team]`.
pub fn room_list(team_flag: Option<&str>) -> Result<()> {
    let (team_name, bridge_dir, _team_repo, workzone) = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let manifest = bridge::load_manifest(&bridge_dir)?;
    let bridge_name = &manifest.metadata.name;

    let room_spec = manifest.spec.room.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Bridge '{}' does not support room management.",
            bridge_name
        )
    })?;

    let result =
        bridge::invoke_recipe(&bridge_dir, &room_spec.list, &[], &team_name)?;

    // Also load state for persisted rooms
    let state_path = bridge::state_path(&workzone, &team_name);
    let state = bridge::load_state(&state_path)?;

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
    if state.rooms.is_empty() {
        println!("No rooms found.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Room", "Room ID"]);

    for room in &state.rooms {
        table.add_row(vec![
            &room.name,
            room.room_id.as_deref().unwrap_or("—"),
        ]);
    }

    println!("{table}");
    Ok(())
}
