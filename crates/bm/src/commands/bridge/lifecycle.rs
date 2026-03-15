use anyhow::Result;

use crate::bridge::{self, CredentialStore};

use super::{make_bridge, make_credential_store, resolve_bridge};

/// Handles `bm bridge start [-t team]`.
pub fn start(team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;
    let name = bridge.bridge_name().to_string();
    match bridge.start()? {
        bridge::BridgeStartResult::External => println!(
            "Bridge '{}' is external -- lifecycle commands are not available. \
             The service is managed externally.",
            name
        ),
        bridge::BridgeStartResult::AlreadyRunning => println!("Bridge '{}' already running.", name),
        bridge::BridgeStartResult::Restarted => println!("Bridge '{}' restarted.", name),
        bridge::BridgeStartResult::Started => println!("Bridge '{}' started.", name),
    }
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
    let name = bridge.bridge_name().to_string();
    match bridge.stop()? {
        bridge::BridgeStopResult::External => println!(
            "Bridge '{}' is external -- lifecycle commands are not available. \
             The service is managed externally.",
            name
        ),
        bridge::BridgeStopResult::Stopped => println!("Bridge '{}' stopped.", name),
    }
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
            let cred_store = make_credential_store(&ctx, bridge.bridge_name());
            match cred_store.retrieve(op_username) {
                Ok(Some(token)) => println!("Operator Token: {}", token),
                Ok(None) => println!("Operator Token: (not in keyring)"),
                Err(e) => println!("Operator Token: (keyring error: {})", e),
            }
        }
    }

    if !bridge.identities().is_empty() {
        println!();
        super::identity::print_identity_table(bridge.identities());
    }
    if !bridge.rooms().is_empty() {
        println!();
        super::room::print_room_table(bridge.rooms());
    }

    Ok(())
}
