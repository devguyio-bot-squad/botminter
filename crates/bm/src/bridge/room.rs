use anyhow::Result;

use super::manifest::BridgeRoom;
use super::Bridge;

/// Result of creating a room.
#[derive(Debug)]
pub struct RoomCreateResult {
    /// The room name (may differ from requested if recipe overrides it).
    pub name: String,
    /// The room ID assigned by the bridge platform.
    pub room_id: Option<String>,
}

/// A room retrieved from the bridge's live list recipe.
#[derive(Debug)]
pub struct LiveRoom {
    /// The room name.
    pub name: String,
    /// The room ID on the bridge platform.
    pub room_id: Option<String>,
}

impl Bridge {
    /// Creates a room via the bridge's room-create recipe and adds it to state.
    ///
    /// Caller must call `save()` to persist state changes.
    pub fn create_room(&mut self, name: &str) -> Result<RoomCreateResult> {
        let room_spec = self.manifest.spec.room.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Bridge '{}' does not support room management.",
                self.bridge_name()
            )
        })?;
        let create_recipe = room_spec.create.clone();
        let result = self.invoke_recipe(&create_recipe, &[name])?;

        let now = chrono::Utc::now().to_rfc3339();

        let (room_name, room_id) = if let Some(val) = result {
            (
                val.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(name)
                    .to_string(),
                val.get("room_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            )
        } else {
            (name.to_string(), None)
        };

        self.add_room(BridgeRoom {
            name: room_name.clone(),
            room_id: room_id.clone(),
            created_at: now,
        });

        Ok(RoomCreateResult {
            name: room_name,
            room_id,
        })
    }

    /// Lists rooms from the bridge's live list recipe.
    ///
    /// Returns `Ok(Some(rooms))` if the recipe returns room data,
    /// `Ok(None)` if the recipe returns no usable data (caller falls back to state).
    pub fn list_rooms_live(&self) -> Result<Option<Vec<LiveRoom>>> {
        let room_spec = self.manifest.spec.room.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Bridge '{}' does not support room management.",
                self.bridge_name()
            )
        })?;
        let list_recipe = room_spec.list.clone();

        let result = self.invoke_recipe(&list_recipe, &[])?;

        if let Some(val) = result {
            if let Some(rooms) = val.get("rooms").and_then(|r| r.as_array()) {
                let live_rooms = rooms
                    .iter()
                    .map(|room| LiveRoom {
                        name: room
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("—")
                            .to_string(),
                        room_id: room
                            .get("room_id")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string()),
                    })
                    .collect();
                return Ok(Some(live_rooms));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn stub_bridge_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".planning")
            .join("specs")
            .join("bridge")
            .join("examples")
            .join("stub")
    }

    fn stub_bridge() -> (Bridge, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("bridge-state.json");
        let bridge =
            Bridge::new(stub_bridge_dir(), state_path, "test-team".to_string()).unwrap();
        (bridge, tmp)
    }

    #[test]
    fn create_room_returns_result_and_adds_to_state() {
        let (mut bridge, _tmp) = stub_bridge();

        let result = bridge.create_room("general").unwrap();

        assert_eq!(result.name, "general");
        assert_eq!(result.room_id.as_deref(), Some("stub-room-id"));
        assert_eq!(bridge.rooms().len(), 1);
        assert_eq!(bridge.rooms()[0].name, "general");
    }

    #[test]
    fn list_rooms_live_returns_rooms() {
        let (bridge, _tmp) = stub_bridge();

        let result = bridge.list_rooms_live().unwrap();

        assert!(result.is_some());
        let rooms = result.unwrap();
        assert!(!rooms.is_empty());
    }
}
