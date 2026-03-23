use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Parsed bridge manifest from bridge.yml.
#[derive(Debug, Deserialize)]
pub struct BridgeManifest {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: BridgeMetadata,
    pub spec: BridgeSpec,
}

/// Metadata section of bridge manifest.
#[derive(Debug, Deserialize)]
pub struct BridgeMetadata {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<String>,
}

/// Spec section of bridge manifest.
#[derive(Debug, Deserialize)]
pub struct BridgeSpec {
    #[serde(rename = "type")]
    pub bridge_type: String,
    #[serde(rename = "configSchema")]
    pub config_schema: String,
    pub lifecycle: Option<BridgeLifecycle>,
    pub identity: BridgeIdentitySpec,
    #[serde(rename = "configDir")]
    pub config_dir: String,
    pub room: Option<BridgeRoomSpec>,
}

/// Lifecycle section for local bridges.
#[derive(Debug, Deserialize)]
pub struct BridgeLifecycle {
    pub start: String,
    pub stop: String,
    pub health: String,
}

/// Identity command spec.
#[derive(Debug, Deserialize)]
pub struct BridgeIdentitySpec {
    pub onboard: String,
    #[serde(rename = "rotate-credentials")]
    pub rotate_credentials: String,
    pub remove: String,
    /// Optional verify recipe for local bridges. When present, provisioning
    /// calls this recipe to confirm credentials are valid against the actual
    /// bridge backend before deciding a member is already provisioned.
    #[serde(default)]
    pub verify: Option<String>,
}

/// Room command spec (optional).
#[derive(Debug, Deserialize)]
pub struct BridgeRoomSpec {
    pub create: String,
    pub list: String,
}

/// Persisted bridge state at {workzone}/{team}/bridge-state.json.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_url: Option<String>,
    #[serde(default)]
    pub container_ids: Vec<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_health_check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_user_id: Option<String>,
    #[serde(default)]
    pub identities: HashMap<String, BridgeIdentity>,
    #[serde(default)]
    pub rooms: Vec<BridgeRoom>,
}

/// A registered identity on the bridge.
///
/// The `token` field is kept for backward compatibility with old bridge-state.json files.
/// New serializations never include it (tokens are stored in the system keyring).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeIdentity {
    pub username: String,
    pub user_id: String,
    /// Legacy field: old bridge-state.json files may contain tokens. Read but never re-serialized.
    /// New code stores tokens in the system keyring via CredentialStore.
    #[serde(default, skip_serializing)]
    pub token: Option<String>,
    pub created_at: String,
    /// Whether this identity is the operator (human) rather than a bot member.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_operator: bool,
}

fn is_false(v: &bool) -> bool {
    !v
}

/// A room on the bridge.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeRoom {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    pub created_at: String,
    /// Member name this room belongs to (None = shared/team room).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
}

impl Default for BridgeState {
    fn default() -> Self {
        Self {
            bridge_name: None,
            bridge_type: None,
            service_url: None,
            container_ids: Vec::new(),
            status: "unknown".to_string(),
            started_at: None,
            last_health_check: None,
            admin_user_id: None,
            identities: HashMap::new(),
            rooms: Vec::new(),
        }
    }
}

/// Returns the bridge state file path for a team.
pub fn state_path(workzone: &Path, team_name: &str) -> PathBuf {
    workzone.join(team_name).join("bridge-state.json")
}

/// Loads and parses bridge.yml from a bridge directory.
pub fn load_manifest(bridge_dir: &Path) -> Result<BridgeManifest> {
    let manifest_path = bridge_dir.join("bridge.yml");
    let contents = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read bridge manifest at {}", manifest_path.display()))?;
    let manifest: BridgeManifest = serde_yml::from_str(&contents)
        .with_context(|| format!("Failed to parse bridge manifest at {}", manifest_path.display()))?;
    Ok(manifest)
}

/// Loads bridge state from a JSON file. Returns default state if the file doesn't exist.
pub fn load_state(path: &Path) -> Result<BridgeState> {
    if !path.exists() {
        return Ok(BridgeState::default());
    }
    let contents = fs::read_to_string(path).context("Failed to read bridge state file")?;
    let state: BridgeState =
        serde_json::from_str(&contents).context("Failed to parse bridge state file")?;
    Ok(state)
}

/// Saves bridge state to a JSON file atomically with 0600 permissions.
pub fn save_state(path: &Path, state: &BridgeState) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create bridge state dir {}", dir.display()))?;
    }

    let contents =
        serde_json::to_string_pretty(state).context("Failed to serialize bridge state")?;

    // Atomic write: temp file -> rename
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, contents).context("Failed to write temp bridge state file")?;

    // Set permissions before rename (0600 -- contains credentials)
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(&tmp_path, perms)
        .context("Failed to set bridge state file permissions")?;

    fs::rename(&tmp_path, path).context("Failed to rename temp bridge state file")?;

    Ok(())
}

/// Discovers the active bridge for a team by reading the team repo's botminter.yml.
///
/// Returns Ok(None) if no `bridge` key is present.
/// Returns Ok(Some(path)) if a bridge is configured and its directory exists.
/// Returns Err if the bridge key points to a non-existent directory.
pub fn discover(team_repo: &Path, _team_name: &str) -> Result<Option<PathBuf>> {
    let manifest_path = team_repo.join("botminter.yml");
    if !manifest_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&manifest_path)
        .context("Failed to read team botminter.yml")?;
    let value: serde_yml::Value =
        serde_yml::from_str(&contents).context("Failed to parse team botminter.yml")?;

    let bridge_name = match value.get("bridge") {
        Some(serde_yml::Value::String(name)) => name.clone(),
        Some(_) => bail!("Invalid `bridge` value in botminter.yml — expected a string"),
        None => return Ok(None),
    };

    let bridge_dir = team_repo.join("bridges").join(&bridge_name);
    if !bridge_dir.exists() {
        bail!(
            "Bridge '{}' configured in botminter.yml but directory {} does not exist. \
             Create the bridge directory or remove the `bridge` key from botminter.yml.",
            bridge_name,
            bridge_dir.display()
        );
    }

    Ok(Some(bridge_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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

    #[test]
    fn parse_manifest() {
        let manifest = load_manifest(&stub_bridge_dir()).unwrap();
        assert_eq!(manifest.api_version, "botminter.dev/v1alpha1");
        assert_eq!(manifest.kind, "Bridge");
        assert_eq!(manifest.metadata.name, "stub");
        assert_eq!(
            manifest.metadata.display_name.as_deref(),
            Some("Stub Bridge")
        );
        assert_eq!(manifest.spec.bridge_type, "local");
        assert_eq!(manifest.spec.config_schema, "schema.json");

        // Lifecycle present for local bridge
        let lifecycle = manifest.spec.lifecycle.as_ref().unwrap();
        assert_eq!(lifecycle.start, "start");
        assert_eq!(lifecycle.stop, "stop");
        assert_eq!(lifecycle.health, "health");

        // Identity
        assert_eq!(manifest.spec.identity.onboard, "onboard");
        assert_eq!(manifest.spec.identity.rotate_credentials, "rotate");
        assert_eq!(manifest.spec.identity.remove, "remove");

        // Room (added in Task 1)
        let room = manifest.spec.room.as_ref().unwrap();
        assert_eq!(room.create, "room-create");
        assert_eq!(room.list, "room-list");
    }

    #[test]
    fn parse_manifest_external() {
        // Create a temp external bridge.yml
        let tmp = tempfile::tempdir().unwrap();
        let bridge_yml = r#"
apiVersion: botminter.dev/v1alpha1
kind: Bridge
metadata:
  name: telegram
  displayName: "Telegram"
  description: "Telegram bot integration"
spec:
  type: external
  configSchema: schema.json
  identity:
    onboard: onboard
    rotate-credentials: rotate
    remove: remove
  configDir: "$BRIDGE_CONFIG_DIR"
"#;
        fs::write(tmp.path().join("bridge.yml"), bridge_yml).unwrap();

        let manifest = load_manifest(tmp.path()).unwrap();
        assert_eq!(manifest.spec.bridge_type, "external");
        assert!(manifest.spec.lifecycle.is_none());
        assert!(manifest.spec.room.is_none());
        assert_eq!(manifest.spec.identity.onboard, "onboard");
    }

    #[test]
    fn state_round_trip() {
        let mut identities = HashMap::new();
        identities.insert(
            "alice".to_string(),
            BridgeIdentity {
                username: "alice".to_string(),
                user_id: "u123".to_string(),
                token: None,
                created_at: "2026-03-08T00:00:00Z".to_string(),
                is_operator: false,
            },
        );

        let state = BridgeState {
            bridge_name: Some("stub".to_string()),
            bridge_type: Some("local".to_string()),
            service_url: Some("http://localhost:3000".to_string()),
            container_ids: vec!["abc123".to_string()],
            status: "running".to_string(),
            started_at: Some("2026-03-08T00:00:00Z".to_string()),
            last_health_check: Some("2026-03-08T00:01:00Z".to_string()),
            admin_user_id: None,
            identities,
            rooms: vec![BridgeRoom {
                name: "general".to_string(),
                room_id: Some("r-123".to_string()),
                created_at: "2026-03-08T00:00:00Z".to_string(),
                member: None,
            }],
        };

        let json = serde_json::to_string(&state).unwrap();
        let loaded: BridgeState = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.bridge_name.as_deref(), Some("stub"));
        assert_eq!(loaded.bridge_type.as_deref(), Some("local"));
        assert_eq!(loaded.service_url.as_deref(), Some("http://localhost:3000"));
        assert_eq!(loaded.container_ids, vec!["abc123"]);
        assert_eq!(loaded.status, "running");
        assert_eq!(
            loaded.started_at.as_deref(),
            Some("2026-03-08T00:00:00Z")
        );
        assert_eq!(
            loaded.last_health_check.as_deref(),
            Some("2026-03-08T00:01:00Z")
        );
        assert_eq!(loaded.identities.len(), 1);
        let alice = loaded.identities.get("alice").unwrap();
        // Token is skip_serializing so it won't round-trip
        assert!(alice.token.is_none());
        assert_eq!(loaded.rooms.len(), 1);
        assert_eq!(loaded.rooms[0].name, "general");
        assert_eq!(loaded.rooms[0].room_id.as_deref(), Some("r-123"));
    }

    #[test]
    fn state_default() {
        let state = BridgeState::default();
        assert_eq!(state.status, "unknown");
        assert!(state.identities.is_empty());
        assert!(state.rooms.is_empty());
        assert!(state.bridge_name.is_none());
        assert!(state.container_ids.is_empty());
    }

    #[test]
    fn load_state_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let state = load_state(&path).unwrap();
        assert_eq!(state.status, "unknown");
        assert!(state.identities.is_empty());
    }

    #[test]
    fn save_state_permissions() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bridge-state.json");

        save_state(&path, &BridgeState::default()).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Bridge state file should have 0600 permissions");
    }

    #[test]
    fn save_and_load_state_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bridge-state.json");

        let state = BridgeState {
            status: "running".to_string(),
            bridge_name: Some("test".to_string()),
            ..BridgeState::default()
        };

        save_state(&path, &state).unwrap();
        let loaded = load_state(&path).unwrap();
        assert_eq!(loaded.status, "running");
        assert_eq!(loaded.bridge_name.as_deref(), Some("test"));
    }

    #[test]
    fn discover_no_bridge() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a botminter.yml without a bridge key
        fs::write(
            tmp.path().join("botminter.yml"),
            "profile: scrum\n",
        )
        .unwrap();

        let result = discover(tmp.path(), "test-team").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn discover_no_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        // No botminter.yml at all
        let result = discover(tmp.path(), "test-team").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn discover_bridge() {
        let tmp = tempfile::tempdir().unwrap();
        // Create botminter.yml with bridge key
        fs::write(
            tmp.path().join("botminter.yml"),
            "bridge: stub\n",
        )
        .unwrap();
        // Create the bridge directory
        let bridge_dir = tmp.path().join("bridges").join("stub");
        fs::create_dir_all(&bridge_dir).unwrap();

        let result = discover(tmp.path(), "test-team").unwrap();
        assert_eq!(result, Some(bridge_dir));
    }

    #[test]
    fn discover_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("botminter.yml"),
            "bridge: nonexistent\n",
        )
        .unwrap();

        let result = discover(tmp.path(), "test-team");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"));
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn bridge_room_member_field_round_trip() {
        let room = BridgeRoom {
            name: "dm-alice".to_string(),
            room_id: Some("!abc:localhost".to_string()),
            created_at: "2026-03-23T00:00:00Z".to_string(),
            member: Some("superman-alice".to_string()),
        };

        let json = serde_json::to_string(&room).unwrap();
        assert!(json.contains("\"member\":\"superman-alice\""));

        let loaded: BridgeRoom = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.member.as_deref(), Some("superman-alice"));
    }

    #[test]
    fn bridge_room_member_none_omitted_in_json() {
        let room = BridgeRoom {
            name: "general".to_string(),
            room_id: Some("!xyz:localhost".to_string()),
            created_at: "2026-03-23T00:00:00Z".to_string(),
            member: None,
        };

        let json = serde_json::to_string(&room).unwrap();
        assert!(!json.contains("member"), "member: None should be omitted from JSON");
    }

    #[test]
    fn bridge_room_member_defaults_to_none() {
        // Old bridge-state.json files won't have the member field
        let json = r#"{"name": "general", "room_id": "!xyz:localhost", "created_at": "2026-03-23T00:00:00Z"}"#;
        let room: BridgeRoom = serde_json::from_str(json).unwrap();
        assert!(room.member.is_none());
    }

    #[test]
    fn old_bridge_state_with_token_deserializes() {
        // Simulate old bridge-state.json format where token was a required string field
        let old_json = r#"{
            "status": "running",
            "identities": {
                "alice": {
                    "username": "alice",
                    "user_id": "u123",
                    "token": "old-secret-token",
                    "created_at": "2026-01-01T00:00:00Z"
                }
            },
            "rooms": []
        }"#;

        let state: BridgeState = serde_json::from_str(old_json).unwrap();
        let alice = state.identities.get("alice").unwrap();
        // Old token field is deserialized but won't be re-serialized
        assert_eq!(alice.token, Some("old-secret-token".to_string()));
        assert_eq!(alice.username, "alice");

        // Re-serialize and verify token is NOT included
        let re_serialized = serde_json::to_string_pretty(&state).unwrap();
        assert!(
            !re_serialized.contains("old-secret-token"),
            "Token should not appear in re-serialized output"
        );
    }

    #[test]
    fn state_path_construction() {
        let workzone = Path::new("/home/user/workzone");
        let result = state_path(workzone, "my-team");
        assert_eq!(
            result,
            PathBuf::from("/home/user/workzone/my-team/bridge-state.json")
        );
    }
}
