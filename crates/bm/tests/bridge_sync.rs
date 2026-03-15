//! Tests for bridge provisioning during `bm teams sync --bridge`
//! and ralph.yml RObot.enabled injection.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Helper: path to the stub bridge fixture.
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

/// Helper: create a minimal team repo with a bridge configured.
fn setup_team_repo_with_bridge(tmp: &Path) -> PathBuf {
    let team_repo = tmp.join("team");
    fs::create_dir_all(&team_repo).unwrap();

    // Copy stub bridge into team repo
    let bridge_dir = team_repo.join("bridges").join("stub");
    fs::create_dir_all(&bridge_dir).unwrap();

    let src_bridge = stub_bridge_dir();
    for entry in fs::read_dir(&src_bridge).unwrap() {
        let entry = entry.unwrap();
        let dest = bridge_dir.join(entry.file_name());
        fs::copy(entry.path(), &dest).unwrap();
    }

    // Create botminter.yml with bridge key
    fs::write(
        team_repo.join("botminter.yml"),
        "profile: scrum-compact\nschema_version: \"0.7\"\nbridge: stub\nprojects: []\n",
    )
    .unwrap();

    // Create members dir
    fs::create_dir_all(team_repo.join("members").join("alice")).unwrap();
    fs::create_dir_all(team_repo.join("members").join("bob")).unwrap();

    team_repo
}

// ── provision_bridge tests ─────────────────────────────────────────────

#[cfg(test)]
mod provision_bridge {
    use super::*;
    use bm::bridge::{
        self, load_state, save_state, BridgeIdentity, BridgeState,
        CredentialStore, InMemoryCredentialStore,
    };

    #[test]
    fn sync_bridge_managed_invokes_onboard_and_stores_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_with_bridge(tmp.path());
        let state_path = bridge::state_path(tmp.path(), "test-team");

        let cred_store = InMemoryCredentialStore::new();

        bridge::provision_bridge(
            &team_repo,
            "test-team",
            tmp.path(),
            &["alice".to_string(), "bob".to_string()],
            &cred_store,
        )
        .unwrap();

        // Verify identities were created in state
        let state = load_state(&state_path).unwrap();
        assert!(
            state.identities.contains_key("alice"),
            "alice should be provisioned"
        );
        assert!(
            state.identities.contains_key("bob"),
            "bob should be provisioned"
        );

        // Verify credentials were stored
        assert!(
            cred_store.retrieve("alice").unwrap().is_some(),
            "alice should have a stored credential"
        );
        assert!(
            cred_store.retrieve("bob").unwrap().is_some(),
            "bob should have a stored credential"
        );
    }

    #[test]
    fn sync_bridge_skips_already_provisioned_members() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_with_bridge(tmp.path());
        let state_path = bridge::state_path(tmp.path(), "test-team");

        // Pre-populate state with alice already provisioned
        let mut identities = HashMap::new();
        identities.insert(
            "alice".to_string(),
            BridgeIdentity {
                username: "alice".to_string(),
                user_id: "existing-id".to_string(),
                token: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
        );
        let state = BridgeState {
            identities,
            ..BridgeState::default()
        };
        save_state(&state_path, &state).unwrap();

        let cred_store = InMemoryCredentialStore::new();

        bridge::provision_bridge(
            &team_repo,
            "test-team",
            tmp.path(),
            &["alice".to_string(), "bob".to_string()],
            &cred_store,
        )
        .unwrap();

        // alice should still have original user_id (not re-provisioned)
        let state = load_state(&state_path).unwrap();
        assert_eq!(
            state.identities.get("alice").unwrap().user_id,
            "existing-id",
            "alice should keep original identity (idempotent)"
        );

        // bob should be newly provisioned
        assert!(
            state.identities.contains_key("bob"),
            "bob should be provisioned"
        );
    }

    #[test]
    fn sync_bridge_creates_team_room_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_with_bridge(tmp.path());
        let state_path = bridge::state_path(tmp.path(), "test-team");

        let cred_store = InMemoryCredentialStore::new();

        bridge::provision_bridge(
            &team_repo,
            "test-team",
            tmp.path(),
            &["alice".to_string()],
            &cred_store,
        )
        .unwrap();

        let state = load_state(&state_path).unwrap();
        assert!(
            !state.rooms.is_empty(),
            "team room should be created when rooms list is empty"
        );
    }

    #[test]
    fn sync_bridge_skips_room_if_already_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_with_bridge(tmp.path());
        let state_path = bridge::state_path(tmp.path(), "test-team");

        // Pre-populate with existing room
        let state = BridgeState {
            rooms: vec![bridge::BridgeRoom {
                name: "general".to_string(),
                room_id: Some("existing-room-id".to_string()),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            }],
            ..BridgeState::default()
        };
        save_state(&state_path, &state).unwrap();

        let cred_store = InMemoryCredentialStore::new();

        bridge::provision_bridge(
            &team_repo,
            "test-team",
            tmp.path(),
            &["alice".to_string()],
            &cred_store,
        )
        .unwrap();

        let state = load_state(&state_path).unwrap();
        assert_eq!(state.rooms.len(), 1, "room count should not change");
        assert_eq!(
            state.rooms[0].room_id.as_deref(),
            Some("existing-room-id"),
            "existing room should be preserved"
        );
    }

    #[test]
    fn sync_bridge_external_skips_member_without_credential() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path().join("team");
        fs::create_dir_all(&team_repo).unwrap();

        // Create external bridge
        let bridge_dir = team_repo.join("bridges").join("telegram");
        fs::create_dir_all(&bridge_dir).unwrap();

        let bridge_yml = r#"apiVersion: botminter.dev/v1alpha1
kind: Bridge
metadata:
  name: telegram
  displayName: "Telegram"
spec:
  type: external
  configSchema: schema.json
  identity:
    onboard: onboard
    rotate-credentials: rotate
    remove: remove
  configDir: "$BRIDGE_CONFIG_DIR"
"#;
        fs::write(bridge_dir.join("bridge.yml"), bridge_yml).unwrap();
        fs::write(
            bridge_dir.join("Justfile"),
            "onboard member:\n  echo 'onboard {{member}}'\n",
        )
        .unwrap();
        fs::write(bridge_dir.join("schema.json"), "{}").unwrap();

        fs::write(
            team_repo.join("botminter.yml"),
            "profile: scrum-compact\nschema_version: \"0.7\"\nbridge: telegram\nprojects: []\n",
        )
        .unwrap();

        fs::create_dir_all(team_repo.join("members").join("alice")).unwrap();

        let cred_store = InMemoryCredentialStore::new();

        bridge::provision_bridge(
            &team_repo,
            "test-team",
            tmp.path(),
            &["alice".to_string()],
            &cred_store,
        )
        .unwrap();

        let state_path = bridge::state_path(tmp.path(), "test-team");
        let state = load_state(&state_path).unwrap();
        assert!(
            !state.identities.contains_key("alice"),
            "alice should be skipped (no credential for external bridge)"
        );
    }
}

// ── inject_robot_enabled tests ─────────────────────────────────────────

#[cfg(test)]
mod robot_enabled {
    use super::*;

    fn write_ralph_yml(path: &Path) {
        let content = "preset: feature-development\ntimeout_seconds: 3600\ncheckin_interval_seconds: 300\n";
        fs::write(path, content).unwrap();
    }

    fn read_ralph_yml_value(path: &Path) -> serde_yml::Value {
        let contents = fs::read_to_string(path).unwrap();
        serde_yml::from_str(&contents).unwrap()
    }

    #[test]
    fn inject_robot_enabled_true_when_credentials_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_path = tmp.path().join("ralph.yml");
        write_ralph_yml(&ralph_path);

        bm::workspace::inject_robot_enabled(&ralph_path, true).unwrap();

        let doc = read_ralph_yml_value(&ralph_path);
        assert_eq!(
            doc["RObot"]["enabled"].as_bool(),
            Some(true),
            "RObot.enabled should be true when member has credentials"
        );
    }

    #[test]
    fn inject_robot_enabled_false_when_no_credentials() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_path = tmp.path().join("ralph.yml");
        write_ralph_yml(&ralph_path);

        bm::workspace::inject_robot_enabled(&ralph_path, false).unwrap();

        let doc = read_ralph_yml_value(&ralph_path);
        assert_eq!(
            doc["RObot"]["enabled"].as_bool(),
            Some(false),
            "RObot.enabled should be false when member has no credentials"
        );
    }

    #[test]
    fn inject_robot_enabled_preserves_existing_content() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_path = tmp.path().join("ralph.yml");
        write_ralph_yml(&ralph_path);

        bm::workspace::inject_robot_enabled(&ralph_path, true).unwrap();

        let doc = read_ralph_yml_value(&ralph_path);
        assert_eq!(
            doc["preset"].as_str(),
            Some("feature-development"),
            "Other ralph.yml fields should be preserved"
        );
        assert_eq!(
            doc["timeout_seconds"].as_u64(),
            Some(3600),
            "timeout_seconds should be preserved"
        );
    }

    #[test]
    fn inject_robot_enabled_no_secrets_in_ralph_yml() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_path = tmp.path().join("ralph.yml");
        write_ralph_yml(&ralph_path);

        bm::workspace::inject_robot_enabled(&ralph_path, true).unwrap();

        let contents = fs::read_to_string(&ralph_path).unwrap();
        assert!(
            !contents.contains("token"),
            "ralph.yml should never contain token values"
        );
        assert!(
            !contents.contains("secret"),
            "ralph.yml should never contain secret values"
        );
    }

    #[test]
    fn inject_robot_enabled_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_path = tmp.path().join("ralph.yml");
        write_ralph_yml(&ralph_path);

        bm::workspace::inject_robot_enabled(&ralph_path, true).unwrap();
        bm::workspace::inject_robot_enabled(&ralph_path, true).unwrap();

        let doc = read_ralph_yml_value(&ralph_path);
        assert_eq!(doc["RObot"]["enabled"].as_bool(), Some(true));
        assert!(doc["RObot"].is_mapping());
    }
}
