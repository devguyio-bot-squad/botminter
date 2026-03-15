use anyhow::{bail, Result};

use super::credential::CredentialStore;
use super::env_var_suffix;
use super::manifest::BridgeIdentity;
use super::Bridge;

/// Result of onboarding a new identity.
#[derive(Debug)]
pub struct OnboardResult {
    /// The username as returned by the onboard recipe.
    pub username: String,
    /// The user ID as returned by the onboard recipe.
    pub user_id: String,
    /// Warning if token could not be stored in the keyring.
    pub keyring_warning: Option<String>,
}

/// Result of rotating an identity's credentials.
#[derive(Debug)]
pub struct RotateResult {
    /// Warning if rotated token could not be stored in the keyring.
    pub keyring_warning: Option<String>,
}

impl Bridge {
    /// Onboards a new identity via the bridge's onboard recipe.
    ///
    /// If `token_override` is provided (e.g., from interactive prompt for external bridges),
    /// it is set as `BM_BRIDGE_TOKEN_{USERNAME}` before invoking the recipe.
    ///
    /// Stores the returned token in the credential store (best-effort).
    /// Caller must call `save()` to persist state changes.
    pub fn onboard_identity(
        &mut self,
        username: &str,
        token_override: Option<&str>,
        cred_store: &dyn CredentialStore,
    ) -> Result<OnboardResult> {
        if let Some(token) = token_override {
            if !token.is_empty() {
                let env_var = format!("BM_BRIDGE_TOKEN_{}", env_var_suffix(username));
                std::env::set_var(&env_var, token);
            }
        }

        let onboard_recipe = self.manifest.spec.identity.onboard.clone();
        let result = self.invoke_recipe(&onboard_recipe, &[username])?;

        let now = chrono::Utc::now().to_rfc3339();
        let mut token_str = String::new();

        let identity = if let Some(ref val) = result {
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
                token: None,
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

        let result_username = identity.username.clone();
        let result_user_id = identity.user_id.clone();
        self.add_identity(username.to_string(), identity);

        let keyring_warning = if !token_str.is_empty() {
            match cred_store.store(username, &token_str) {
                Err(e) => Some(format!(
                    "Could not store token in system keyring: {}\n\
                     Set BM_BRIDGE_TOKEN_{} environment variable instead.",
                    e,
                    env_var_suffix(username)
                )),
                Ok(()) => None,
            }
        } else {
            None
        };

        Ok(OnboardResult {
            username: result_username,
            user_id: result_user_id,
            keyring_warning,
        })
    }

    /// Rotates credentials for an existing identity.
    ///
    /// Invokes the rotate-credentials recipe, updates user_id if returned,
    /// and stores the new token in the credential store (best-effort).
    /// Caller must call `save()` to persist state changes.
    pub fn rotate_identity(
        &mut self,
        username: &str,
        cred_store: &dyn CredentialStore,
    ) -> Result<RotateResult> {
        if !self.has_identity(username) {
            bail!(
                "Identity '{}' not found. \
                 Run 'bm bridge identity list' to see registered identities.",
                username
            );
        }

        let rotate_recipe = self.manifest.spec.identity.rotate_credentials.clone();
        let result = self.invoke_recipe(&rotate_recipe, &[username])?;

        let mut keyring_warning = None;
        if let Some(val) = result {
            if let Some(user_id) = val.get("user_id").and_then(|v| v.as_str()) {
                self.update_identity_user_id(username, user_id);
            }
            if let Some(token) = val.get("token").and_then(|v| v.as_str()) {
                if let Err(e) = cred_store.store(username, token) {
                    keyring_warning = Some(format!(
                        "Could not store rotated token in system keyring: {}\n\
                         Set BM_BRIDGE_TOKEN_{} environment variable instead.",
                        e,
                        env_var_suffix(username)
                    ));
                }
            }
        }

        Ok(RotateResult { keyring_warning })
    }

    /// Removes an identity: invokes the remove recipe, removes from state and keyring.
    ///
    /// Caller must call `save()` to persist state changes.
    pub fn offboard_identity(
        &mut self,
        username: &str,
        cred_store: &dyn CredentialStore,
    ) -> Result<()> {
        if !self.has_identity(username) {
            bail!(
                "Identity '{}' not found. \
                 Run 'bm bridge identity list' to see registered identities.",
                username
            );
        }

        let remove_recipe = self.manifest.spec.identity.remove.clone();
        self.invoke_recipe(&remove_recipe, &[username])?;
        self.remove_identity(username);

        // Best-effort keyring removal
        let _ = cred_store.remove(username);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::InMemoryCredentialStore;
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
    fn onboard_identity_creates_identity_and_stores_token() {
        let (mut bridge, _tmp) = stub_bridge();
        let cred_store = InMemoryCredentialStore::new();

        let result = bridge
            .onboard_identity("alice", None, &cred_store)
            .unwrap();

        assert_eq!(result.username, "alice");
        assert_eq!(result.user_id, "stub-id");
        assert!(result.keyring_warning.is_none());
        assert!(bridge.has_identity("alice"));
        assert_eq!(
            cred_store.retrieve("alice").unwrap(),
            Some("stub-token".to_string())
        );
    }

    #[test]
    fn onboard_identity_second_call_overwrites() {
        let (mut bridge, _tmp) = stub_bridge();
        let cred_store = InMemoryCredentialStore::new();

        bridge
            .onboard_identity("alice", None, &cred_store)
            .unwrap();
        let result = bridge
            .onboard_identity("alice", None, &cred_store)
            .unwrap();

        assert_eq!(result.username, "alice");
        assert!(bridge.has_identity("alice"));
    }

    #[test]
    fn rotate_identity_nonexistent_fails() {
        let (mut bridge, _tmp) = stub_bridge();
        let cred_store = InMemoryCredentialStore::new();

        let err = bridge
            .rotate_identity("nonexistent", &cred_store)
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn rotate_identity_existing_succeeds() {
        let (mut bridge, _tmp) = stub_bridge();
        let cred_store = InMemoryCredentialStore::new();

        bridge
            .onboard_identity("alice", None, &cred_store)
            .unwrap();
        let result = bridge.rotate_identity("alice", &cred_store).unwrap();

        assert!(result.keyring_warning.is_none());
    }

    #[test]
    fn offboard_identity_nonexistent_fails() {
        let (mut bridge, _tmp) = stub_bridge();
        let cred_store = InMemoryCredentialStore::new();

        let err = bridge
            .offboard_identity("nonexistent", &cred_store)
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn offboard_identity_removes_from_state_and_store() {
        let (mut bridge, _tmp) = stub_bridge();
        let cred_store = InMemoryCredentialStore::new();

        bridge
            .onboard_identity("alice", None, &cred_store)
            .unwrap();
        assert!(bridge.has_identity("alice"));
        assert!(cred_store.retrieve("alice").unwrap().is_some());

        bridge.offboard_identity("alice", &cred_store).unwrap();

        assert!(!bridge.has_identity("alice"));
        assert!(cred_store.retrieve("alice").unwrap().is_none());
    }
}
