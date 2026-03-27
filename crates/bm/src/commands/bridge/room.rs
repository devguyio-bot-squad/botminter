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

/// Handles `bm bridge room create-dm <member> [-t team]`.
pub fn room_create_dm(member: &str, team_flag: Option<&str>) -> Result<()> {
    let ctx = match resolve_bridge(team_flag)? {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut bridge = make_bridge(&ctx)?;

    // Check if DM room already exists for this member
    if let Some(room_id) = bridge.room_for_member(member) {
        println!("DM room for '{}' already exists ({})", member, room_id);
        return Ok(());
    }

    bridge.create_dm_room(member)?;
    bridge.save()?;

    println!("DM room for '{}' created.", member);
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
    let has_members = rooms.iter().any(|r| r.member.is_some());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    if has_members {
        table.set_header(vec!["Room", "Room ID", "Member", "Created"]);
        for room in rooms {
            table.add_row(vec![
                room.name.as_str(),
                room.room_id.as_deref().unwrap_or("—"),
                room.member.as_deref().unwrap_or("—"),
                room.created_at.as_str(),
            ]);
        }
    } else {
        table.set_header(vec!["Room", "Room ID", "Created"]);
        for room in rooms {
            table.add_row(vec![
                room.name.as_str(),
                room.room_id.as_deref().unwrap_or("—"),
                room.created_at.as_str(),
            ]);
        }
    }
    println!("{table}");
}
