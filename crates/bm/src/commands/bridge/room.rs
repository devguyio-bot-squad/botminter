use anyhow::Result;
use comfy_table::{ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};

use crate::bridge::BridgeRoom;

use super::{make_bridge, resolve_bridge};

/// Handles `bm bridge room create <name> [-t team]`.
pub fn room_create(name: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;
    bridge.create_room(name)?;
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

    // Prefer live data from recipe if available
    if let Some(live_rooms) = bridge.list_rooms_live()? {
        if live_rooms.is_empty() {
            println!("No rooms found.");
            return Ok(());
        }
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Room", "Room ID"]);
        for room in &live_rooms {
            table.add_row(vec![room.name.as_str(), room.room_id.as_deref().unwrap_or("—")]);
        }
        println!("{table}");
        return Ok(());
    }

    // Fallback to state
    if bridge.rooms().is_empty() {
        println!("No rooms found.");
        return Ok(());
    }

    print_room_table(bridge.rooms());
    Ok(())
}

pub(super) fn print_room_table(rooms: &[BridgeRoom]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Room", "Room ID", "Created"]);

    for room in rooms {
        table.add_row(vec![
            room.name.as_str(),
            room.room_id.as_deref().unwrap_or("—"),
            room.created_at.as_str(),
        ]);
    }
    println!("{table}");
}
