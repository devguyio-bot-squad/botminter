use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

use super::env_var_suffix;
use super::manifest::{load_state, save_state, BridgeIdentity};
use crate::keyring_backend;

// ── CredentialStore trait + implementations ──────────────────────────

/// Trait for storing and retrieving bridge credentials (tokens).
///
/// Different formation backends implement this trait:
/// - `LocalCredentialStore` uses the system keyring (local formation)
/// - `InMemoryCredentialStore` for testing
/// - Future: K8s Secrets backend for K8s formation
pub trait CredentialStore {
    fn store(&self, member_name: &str, token: &str) -> Result<()>;
    fn retrieve(&self, member_name: &str) -> Result<Option<String>>;
    fn remove(&self, member_name: &str) -> Result<()>;
    fn list(&self) -> Result<Vec<String>>;
}

/// In-memory credential store for testing. Avoids system keyring dependency.
pub struct InMemoryCredentialStore {
    tokens: std::sync::Mutex<HashMap<String, String>>,
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self {
            tokens: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn store(&self, member_name: &str, token: &str) -> Result<()> {
        self.tokens
            .lock()
            .unwrap()
            .insert(member_name.to_string(), token.to_string());
        Ok(())
    }

    fn retrieve(&self, member_name: &str) -> Result<Option<String>> {
        Ok(self.tokens.lock().unwrap().get(member_name).cloned())
    }

    fn remove(&self, member_name: &str) -> Result<()> {
        self.tokens.lock().unwrap().remove(member_name);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>> {
        let mut names: Vec<String> = self.tokens.lock().unwrap().keys().cloned().collect();
        names.sort();
        Ok(names)
    }
}

/// Local credential store backed by the system keyring (via `keyring` crate).
///
/// Uses `keyring::Entry` for credential storage. The keyring service name
/// is `botminter.{team}.{bridge}`. Member names from bridge-state.json
/// serve as the index (keyring doesn't support enumeration).
///
/// When `collection` is set, uses `dbus-secret-service` directly to target
/// a named collection instead of the default `login` collection.
pub struct LocalCredentialStore {
    service: String,
    state_path: PathBuf,
    collection: Option<String>,
}

impl LocalCredentialStore {
    pub fn new(team_name: &str, bridge_name: &str, state_path: PathBuf) -> Self {
        Self {
            service: format!("botminter.{}.{}", team_name, bridge_name),
            state_path,
            collection: None,
        }
    }

    /// Set a custom Secret Service collection name.
    /// When set, bypasses `keyring::Entry` and uses `dbus-secret-service` directly.
    pub fn with_collection(mut self, collection: Option<String>) -> Self {
        self.collection = collection;
        self
    }
}

/// Checks if the default keyring is unlocked (backward compat).
pub fn check_keyring_unlocked() -> Result<()> {
    keyring_backend::check_keyring_unlocked()
}

/// Checks if a keyring collection is unlocked.
/// When `collection_name` is Some, checks that specific collection.
/// Otherwise checks the default collection.
pub fn check_keyring_unlocked_for(collection_name: Option<&str>) -> Result<()> {
    keyring_backend::check_keyring_unlocked_for(collection_name)
}

impl CredentialStore for LocalCredentialStore {
    fn store(&self, member_name: &str, token: &str) -> Result<()> {
        keyring_backend::with_keyring_dbus(|| {
            if let Some(ref coll) = self.collection {
                // Custom collection via dbus-secret-service
                keyring_backend::dss_store(&self.service, member_name, token, coll)?;
            } else {
                // Default: keyring::Entry -> login collection
                check_keyring_unlocked()?;

                match keyring::Entry::new(&self.service, member_name) {
                    Ok(entry) => {
                        entry.set_password(token).map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to store credential in system keyring. \
                                 Set BM_BRIDGE_TOKEN_{} environment variable instead. ({})",
                                env_var_suffix(member_name),
                                e
                            )
                        })?;
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to create keyring entry. \
                             Set BM_BRIDGE_TOKEN_{} environment variable instead. ({})",
                            env_var_suffix(member_name),
                            e
                        ));
                    }
                }
            }

            // Record member in bridge-state.json identities (metadata only, no token)
            let mut state = load_state(&self.state_path)?;
            if !state.identities.contains_key(member_name) {
                state.identities.insert(
                    member_name.to_string(),
                    BridgeIdentity {
                        username: member_name.to_string(),
                        user_id: String::new(),
                        token: None,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        is_operator: false,
                    },
                );
                save_state(&self.state_path, &state)?;
            }

            Ok(())
        })
    }

    fn retrieve(&self, member_name: &str) -> Result<Option<String>> {
        keyring_backend::with_keyring_dbus(|| {
            if let Some(ref coll) = self.collection {
                return keyring_backend::dss_retrieve(&self.service, member_name, coll);
            }

            let entry = match keyring::Entry::new(&self.service, member_name) {
                Ok(e) => e,
                Err(_) => {
                    // Keyring not available -- fall back to env var resolution
                    return Ok(None);
                }
            };
            match entry.get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => {
                    // Keyring error -- fall back to env var resolution.
                    // Caller can check BM_BRIDGE_TOKEN_{name} env var.
                    Ok(None)
                }
            }
        })
    }

    fn remove(&self, member_name: &str) -> Result<()> {
        keyring_backend::with_keyring_dbus(|| {
            if let Some(ref coll) = self.collection {
                keyring_backend::dss_delete(&self.service, member_name, coll)?;
            } else if let Ok(entry) = keyring::Entry::new(&self.service, member_name) {
                match entry.delete_credential() {
                    Ok(()) => {}
                    Err(keyring::Error::NoEntry) => {} // Already gone
                    Err(_) => {}                       // Best-effort removal; credential may remain
                }
            }

            // Remove from bridge-state.json identities
            let mut state = load_state(&self.state_path)?;
            state.identities.remove(member_name);
            save_state(&self.state_path, &state)?;

            Ok(())
        })
    }

    fn list(&self) -> Result<Vec<String>> {
        let state = load_state(&self.state_path)?;
        let mut names: Vec<String> = state.identities.keys().cloned().collect();
        names.sort();
        Ok(names)
    }
}

/// Resolves a credential for a member using the CredentialStore abstraction.
///
/// Priority: env var `BM_BRIDGE_TOKEN_{USERNAME}` (uppercased, hyphens to underscores) first,
/// then `credential_store.retrieve(member)` second.
pub fn resolve_credential_from_store(
    member_name: &str,
    credential_store: &dyn CredentialStore,
) -> Result<Option<String>> {
    // Check env var first
    let env_key = format!("BM_BRIDGE_TOKEN_{}", env_var_suffix(member_name));
    if let Ok(val) = std::env::var(&env_key) {
        if !val.is_empty() {
            return Ok(Some(val));
        }
    }

    // Fall back to credential store
    credential_store.retrieve(member_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_store_store_and_retrieve() {
        let store = InMemoryCredentialStore::new();
        store.store("alice", "tok123").unwrap();
        assert_eq!(store.retrieve("alice").unwrap(), Some("tok123".to_string()));
    }

    #[test]
    fn credential_store_retrieve_unknown() {
        let store = InMemoryCredentialStore::new();
        assert_eq!(store.retrieve("unknown").unwrap(), None);
    }

    #[test]
    fn credential_store_remove() {
        let store = InMemoryCredentialStore::new();
        store.store("alice", "tok123").unwrap();
        store.remove("alice").unwrap();
        assert_eq!(store.retrieve("alice").unwrap(), None);
    }

    #[test]
    fn credential_store_list() {
        let store = InMemoryCredentialStore::new();
        store.store("bob", "tok-b").unwrap();
        store.store("alice", "tok-a").unwrap();
        let names = store.list().unwrap();
        assert_eq!(names, vec!["alice", "bob"]); // sorted
    }

    #[test]
    fn credential_store_overwrite() {
        let store = InMemoryCredentialStore::new();
        store.store("alice", "old_token").unwrap();
        store.store("alice", "new_token").unwrap();
        assert_eq!(
            store.retrieve("alice").unwrap(),
            Some("new_token".to_string())
        );
    }

    #[test]
    fn credential_store_list_empty() {
        let store = InMemoryCredentialStore::new();
        let names = store.list().unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn resolve_credential_from_store_env_var_priority() {
        let key = "BM_BRIDGE_TOKEN_STORETESTUSER";
        std::env::set_var(key, "env-token-store");

        let store = InMemoryCredentialStore::new();
        store.store("storetestuser", "store-token").unwrap();

        let cred = resolve_credential_from_store("storetestuser", &store).unwrap();
        assert_eq!(cred, Some("env-token-store".to_string()));

        std::env::remove_var(key);
    }

    #[test]
    fn resolve_credential_from_store_fallback() {
        let key = "BM_BRIDGE_TOKEN_STOREFALLBACK";
        std::env::remove_var(key);

        let store = InMemoryCredentialStore::new();
        store.store("storefallback", "store-token-fb").unwrap();

        let cred = resolve_credential_from_store("storefallback", &store).unwrap();
        assert_eq!(cred, Some("store-token-fb".to_string()));
    }
}
