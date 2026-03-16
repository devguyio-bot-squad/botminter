mod credential;
mod identity;
mod lifecycle;
mod manifest;
mod provisioning;
mod room;

pub use credential::{
    check_keyring_unlocked, check_keyring_unlocked_for, resolve_credential_from_store,
    CredentialStore, InMemoryCredentialStore, LocalCredentialStore,
};
pub use identity::{OnboardResult, RotateResult};
pub use lifecycle::invoke_recipe;
pub use provisioning::{ProvisionMemberResult, ProvisionResult};
pub use room::{LiveRoom, RoomCreateResult};
pub use manifest::{
    discover, load_manifest, load_state, save_state, state_path, BridgeIdentity, BridgeIdentitySpec,
    BridgeLifecycle, BridgeManifest, BridgeMetadata, BridgeRoom, BridgeRoomSpec, BridgeSpec,
    BridgeState,
};

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

/// Result of a bridge start operation.
#[derive(Debug, Clone)]
pub enum BridgeStartResult {
    /// Bridge is external — lifecycle commands are not available.
    External,
    /// Bridge was already running and healthy; health check timestamp updated.
    AlreadyRunning,
    /// Bridge health check failed; bridge was restarted.
    Restarted,
    /// Bridge was stopped; now started.
    Started,
}

/// Result of a bridge stop operation.
#[derive(Debug)]
pub enum BridgeStopResult {
    /// Bridge is external — lifecycle commands are not available.
    External,
    /// Bridge was stopped.
    Stopped,
}

// ── Bridge struct ────────────────────────────────────────────────────

/// A member to provision on the bridge.
pub struct BridgeMember {
    pub name: String,
    pub is_operator: bool,
}

/// Encapsulates bridge state: manifest, persisted state, and bridge directory.
///
/// Provides methods for lifecycle management (start/stop/health), provisioning,
/// and querying bridge state. Callers should prefer Bridge methods over direct
/// BridgeState field access.
pub struct Bridge {
    pub(crate) bridge_dir: PathBuf,
    pub(crate) state_path: PathBuf,
    pub(crate) team_name: String,
    pub(crate) manifest: BridgeManifest,
    pub(crate) state: BridgeState,
}

impl Bridge {
    /// Creates a new Bridge by loading the manifest and state from disk.
    pub fn new(bridge_dir: PathBuf, state_path: PathBuf, team_name: String) -> Result<Self> {
        let manifest = load_manifest(&bridge_dir)?;
        let state = load_state(&state_path)?;
        Ok(Self {
            bridge_dir,
            state_path,
            team_name,
            manifest,
            state,
        })
    }

    // ── Lifecycle ────────────────────────────────────────────────────

    /// Invokes a Just recipe from the bridge directory.
    pub fn invoke_recipe(&self, recipe: &str, args: &[&str]) -> Result<Option<serde_json::Value>> {
        invoke_recipe(&self.bridge_dir, recipe, args, &self.team_name, self.state_path.parent())
    }

    /// Starts the bridge: invokes start recipe, health check, and updates state.
    ///
    /// For external bridges, returns `BridgeStartResult::External`.
    /// For local bridges without a lifecycle section, returns an error.
    /// If the bridge is already running and healthy, updates the health check timestamp.
    pub fn start(&mut self) -> Result<BridgeStartResult> {
        if self.is_external() {
            return Ok(BridgeStartResult::External);
        }

        let lifecycle = self.manifest.spec.lifecycle.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Bridge '{}' is local but has no lifecycle section in bridge.yml",
                self.bridge_name()
            )
        })?;

        let start_recipe = lifecycle.start.clone();
        let health_recipe = lifecycle.health.clone();

        // Skip start if bridge is already running and healthy
        let was_restarted = if self.state.status == "running" {
            if self.invoke_recipe(&health_recipe, &[]).is_ok() {
                self.state.last_health_check = Some(chrono::Utc::now().to_rfc3339());
                return Ok(BridgeStartResult::AlreadyRunning);
            }
            true
        } else {
            false
        };

        let start_result = self.invoke_recipe(&start_recipe, &[])?;
        self.invoke_recipe(&health_recipe, &[])?;

        let now = chrono::Utc::now().to_rfc3339();
        self.state.bridge_name = Some(self.manifest.metadata.name.clone());
        self.state.bridge_type = Some(self.manifest.spec.bridge_type.clone());
        self.state.status = "running".to_string();
        self.state.started_at = Some(now.clone());
        self.state.last_health_check = Some(now);

        if let Some(val) = start_result {
            if let Some(url) = val.get("service_url").and_then(|u| u.as_str()) {
                self.state.service_url = Some(url.to_string());
            }
            if let Some(uid) = val.get("admin_user_id").and_then(|u| u.as_str()) {
                self.state.admin_user_id = Some(uid.to_string());
            }
        }

        Ok(if was_restarted {
            BridgeStartResult::Restarted
        } else {
            BridgeStartResult::Started
        })
    }

    /// Stops the bridge: invokes stop recipe and updates state.
    pub fn stop(&mut self) -> Result<BridgeStopResult> {
        if self.is_external() {
            return Ok(BridgeStopResult::External);
        }

        let lifecycle = self.manifest.spec.lifecycle.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Bridge '{}' is local but has no lifecycle section in bridge.yml",
                self.bridge_name()
            )
        })?;

        let stop_recipe = lifecycle.stop.clone();
        self.invoke_recipe(&stop_recipe, &[])?;

        self.state.status = "stopped".to_string();
        self.state.started_at = None;

        Ok(BridgeStopResult::Stopped)
    }

    /// Runs a health check and updates `last_health_check` in state.
    pub fn health(&mut self) -> Result<()> {
        let lifecycle = self.manifest.spec.lifecycle.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Bridge '{}' has no lifecycle section in bridge.yml",
                self.bridge_name()
            )
        })?;
        let health_recipe = lifecycle.health.clone();
        self.invoke_recipe(&health_recipe, &[])?;
        self.state.last_health_check = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    // ── Query methods ────────────────────────────────────────────────

    /// Returns the user_id for a member by their username key in identities.
    pub fn member_user_id(&self, username: &str) -> Option<String> {
        self.state
            .identities
            .get(username)
            .map(|id| id.user_id.clone())
    }

    /// Returns the admin user ID (captured from start recipe output).
    pub fn admin_user_id(&self) -> Option<&str> {
        self.state.admin_user_id.as_deref()
    }

    /// Returns the first room's ID, which is the team's default room.
    pub fn default_room_id(&self) -> Option<&str> {
        self.state
            .rooms
            .first()
            .and_then(|r| r.room_id.as_deref())
    }

    /// Returns the bridge service URL.
    pub fn service_url(&self) -> Option<&str> {
        self.state.service_url.as_deref()
    }

    /// Returns the bridge type (e.g., "local", "external").
    pub fn bridge_type(&self) -> &str {
        &self.manifest.spec.bridge_type
    }

    /// Returns the bridge name from the manifest metadata.
    pub fn bridge_name(&self) -> &str {
        &self.manifest.metadata.name
    }

    /// Returns the display name, falling back to the bridge name.
    pub fn display_name(&self) -> &str {
        self.manifest
            .metadata
            .display_name
            .as_deref()
            .unwrap_or(&self.manifest.metadata.name)
    }

    /// Returns the bridge status from persisted state.
    pub fn status(&self) -> &str {
        &self.state.status
    }

    /// Returns when the bridge was started.
    pub fn started_at(&self) -> Option<&str> {
        self.state.started_at.as_deref()
    }

    /// Returns true if the bridge status is "running".
    pub fn is_running(&self) -> bool {
        self.state.status == "running"
    }

    /// Returns true if the bridge type is "local".
    pub fn is_local(&self) -> bool {
        self.manifest.spec.bridge_type == "local"
    }

    /// Returns true if the bridge type is "external".
    pub fn is_external(&self) -> bool {
        self.manifest.spec.bridge_type == "external"
    }

    /// Returns the registered identities.
    pub fn identities(&self) -> &HashMap<String, BridgeIdentity> {
        &self.state.identities
    }

    /// Returns the registered rooms.
    pub fn rooms(&self) -> &[BridgeRoom] {
        &self.state.rooms
    }

    /// Returns true if bridge state has been initialized (has a bridge name).
    pub fn is_active(&self) -> bool {
        self.state.bridge_name.is_some()
    }

    /// Returns the operator's username, if one is marked in identities.
    pub fn operator_username(&self) -> Option<&str> {
        self.state
            .identities
            .values()
            .find(|id| id.is_operator)
            .map(|id| id.username.as_str())
    }

    /// Returns the operator's login password from the bridge state directory.
    /// For local bridges (e.g., Tuwunel), passwords are stored in a JSON file.
    /// Returns None if no passwords file exists or the operator isn't found.
    pub fn operator_password(&self) -> Option<String> {
        let op_username = self.operator_username()?;
        let state_dir = self.state_path.parent()?;
        let passwords_file = state_dir.join("tuwunel-passwords.json");
        let contents = std::fs::read_to_string(&passwords_file).ok()?;
        let passwords: serde_json::Value = serde_json::from_str(&contents).ok()?;
        passwords.get(op_username)?.as_str().map(String::from)
    }

    /// Returns true if an identity with the given name exists.
    pub fn has_identity(&self, username: &str) -> bool {
        self.state.identities.contains_key(username)
    }

    // ── State mutations ─────────────────────────────────────────────

    /// Adds an identity to the bridge state. Sets bridge metadata if not yet set.
    pub fn add_identity(&mut self, key: String, identity: BridgeIdentity) {
        self.ensure_bridge_metadata();
        self.state.identities.insert(key, identity);
    }

    /// Updates the user_id of an existing identity.
    pub fn update_identity_user_id(&mut self, username: &str, user_id: &str) {
        if let Some(identity) = self.state.identities.get_mut(username) {
            identity.user_id = user_id.to_string();
        }
    }

    /// Removes an identity from the bridge state.
    pub fn remove_identity(&mut self, username: &str) {
        self.state.identities.remove(username);
    }

    /// Adds a room to the bridge state.
    pub fn add_room(&mut self, room: BridgeRoom) {
        self.state.rooms.push(room);
    }

    /// Returns a reference to the manifest.
    pub fn manifest(&self) -> &BridgeManifest {
        &self.manifest
    }

    /// Ensures bridge name/type metadata is set on state.
    fn ensure_bridge_metadata(&mut self) {
        if self.state.bridge_name.is_none() {
            self.state.bridge_name = Some(self.manifest.metadata.name.clone());
            self.state.bridge_type = Some(self.manifest.spec.bridge_type.clone());
        }
    }

    // ── State persistence ────────────────────────────────────────────

    /// Saves the current bridge state to disk.
    pub fn save(&self) -> Result<()> {
        save_state(&self.state_path, &self.state)
    }
}

/// Normalizes a name into a valid env var suffix: uppercased, hyphens replaced with underscores.
pub fn env_var_suffix_pub(name: &str) -> String {
    env_var_suffix(name)
}

/// Normalizes a name into a valid env var suffix: uppercased, hyphens replaced with underscores.
pub(crate) fn env_var_suffix(name: &str) -> String {
    name.to_uppercase().replace('-', "_")
}

/// Resolves a credential for an identity (legacy path).
///
/// Priority: env var `BM_BRIDGE_TOKEN_{USERNAME}` (uppercased, hyphens to underscores) -> state file identity token.
///
/// Note: Prefer `resolve_credential_from_store()` for new code. This function
/// is retained for backward compatibility with code that still reads from bridge-state.json.
pub fn resolve_credential(identity_name: &str, state: &BridgeState) -> Option<String> {
    // Check env var first (hyphens replaced with underscores for valid env var names)
    let env_key = format!("BM_BRIDGE_TOKEN_{}", env_var_suffix(identity_name));
    if let Ok(val) = std::env::var(&env_key) {
        if !val.is_empty() {
            return Some(val);
        }
    }

    // Fall back to state file (token is now optional)
    state
        .identities
        .get(identity_name)
        .and_then(|id| id.token.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_priority_env_var() {
        // Set env var
        let key = "BM_BRIDGE_TOKEN_TESTUSER";
        std::env::set_var(key, "env-token");

        let mut identities = HashMap::new();
        identities.insert(
            "testuser".to_string(),
            BridgeIdentity {
                username: "testuser".to_string(),
                user_id: "u1".to_string(),
                token: Some("state-token".to_string()),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                is_operator: false,
            },
        );

        let state = BridgeState {
            identities,
            ..BridgeState::default()
        };

        let cred = resolve_credential("testuser", &state);
        assert_eq!(cred, Some("env-token".to_string()));

        // Clean up
        std::env::remove_var(key);
    }

    #[test]
    fn credential_priority_state_fallback() {
        // Make sure env var is NOT set
        let key = "BM_BRIDGE_TOKEN_FALLBACKUSER";
        std::env::remove_var(key);

        let mut identities = HashMap::new();
        identities.insert(
            "fallbackuser".to_string(),
            BridgeIdentity {
                username: "fallbackuser".to_string(),
                user_id: "u2".to_string(),
                token: Some("state-token-fb".to_string()),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                is_operator: false,
            },
        );

        let state = BridgeState {
            identities,
            ..BridgeState::default()
        };

        let cred = resolve_credential("fallbackuser", &state);
        assert_eq!(cred, Some("state-token-fb".to_string()));
    }

    #[test]
    fn credential_priority_none() {
        let key = "BM_BRIDGE_TOKEN_NOUSER";
        std::env::remove_var(key);

        let state = BridgeState::default();
        let cred = resolve_credential("nouser", &state);
        assert!(cred.is_none());
    }

    #[test]
    fn credential_env_var_hyphen_to_underscore() {
        // Member names with hyphens should produce valid env var names
        let key = "BM_BRIDGE_TOKEN_AGENT_ALICE";
        std::env::set_var(key, "hyphen-test-token");

        let state = BridgeState::default();
        let cred = resolve_credential("agent-alice", &state);
        assert_eq!(cred, Some("hyphen-test-token".to_string()));

        // Clean up
        std::env::remove_var(key);
    }

    #[test]
    fn env_var_suffix_normalization() {
        assert_eq!(env_var_suffix("alice"), "ALICE");
        assert_eq!(env_var_suffix("agent-alice"), "AGENT_ALICE");
        assert_eq!(env_var_suffix("my-agent-name"), "MY_AGENT_NAME");
    }
}
