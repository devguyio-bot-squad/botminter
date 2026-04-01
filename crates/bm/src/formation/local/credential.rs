use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::formation::KeyValueCredentialStore;
use crate::keyring_backend;

// ── Key tracking ─────────────────────────────────────────────────────

/// Loads the set of known keys from a JSON tracking file.
/// Returns an empty set if the file doesn't exist.
fn load_tracked_keys(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(path)?;
    let keys: Vec<String> = serde_json::from_str(&data)?;
    Ok(keys)
}

/// Saves the set of known keys to a JSON tracking file.
fn save_tracked_keys(path: &Path, keys: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(keys)?;
    std::fs::write(path, data)?;
    Ok(())
}

// ── LocalKeyValueCredentialStore ─────────────────────────────────────

/// Key-value credential store backed by the system keyring.
///
/// Uses `keyring::Entry` for secret storage (or `dbus-secret-service` when
/// a custom collection is configured). Since keyring doesn't support
/// enumeration, a JSON file tracks known keys for `list_keys()`.
///
/// This store is formation-neutral — no bridge-state.json involvement.
/// Bridge-specific metadata is managed by the bridge module independently.
pub struct LocalKeyValueCredentialStore {
    service: String,
    keys_path: PathBuf,
    collection: Option<String>,
}

impl LocalKeyValueCredentialStore {
    pub fn new(service: String, keys_path: PathBuf) -> Self {
        Self {
            service,
            keys_path,
            collection: None,
        }
    }

    /// Set a custom Secret Service collection name.
    /// When set, bypasses `keyring::Entry` and uses `dbus-secret-service` directly.
    #[allow(dead_code)]
    pub fn with_collection(mut self, collection: Option<String>) -> Self {
        self.collection = collection;
        self
    }

    fn track_key(&self, key: &str) -> Result<()> {
        let mut keys = load_tracked_keys(&self.keys_path)?;
        if !keys.contains(&key.to_string()) {
            keys.push(key.to_string());
            keys.sort();
            save_tracked_keys(&self.keys_path, &keys)?;
        }
        Ok(())
    }

    fn untrack_key(&self, key: &str) -> Result<()> {
        let mut keys = load_tracked_keys(&self.keys_path)?;
        keys.retain(|k| k != key);
        save_tracked_keys(&self.keys_path, &keys)?;
        Ok(())
    }
}

impl KeyValueCredentialStore for LocalKeyValueCredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<()> {
        keyring_backend::with_keyring_dbus(|| {
            if let Some(ref collection) = self.collection {
                keyring_backend::dss_store(&self.service, key, value, collection)?;
            } else {
                keyring_backend::check_keyring_unlocked_for(None)?;
                let entry = keyring::Entry::new(&self.service, key).map_err(|e| {
                    anyhow::anyhow!("Failed to create keyring entry for key '{}'. ({})", key, e)
                })?;
                entry.set_password(value).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to store credential in system keyring for key '{}'. ({})",
                        key,
                        e
                    )
                })?;
            }

            self.track_key(key)?;
            Ok(())
        })
    }

    fn retrieve(&self, key: &str) -> Result<Option<String>> {
        keyring_backend::with_keyring_dbus(|| {
            if let Some(ref collection) = self.collection {
                return keyring_backend::dss_retrieve(&self.service, key, collection);
            }

            let entry = match keyring::Entry::new(&self.service, key) {
                Ok(entry) => entry,
                Err(_) => return Ok(None),
            };
            match entry.get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Ok(None),
            }
        })
    }

    fn remove(&self, key: &str) -> Result<()> {
        keyring_backend::with_keyring_dbus(|| {
            if let Some(ref collection) = self.collection {
                keyring_backend::dss_delete(&self.service, key, collection)?;
            } else if let Ok(entry) = keyring::Entry::new(&self.service, key) {
                match entry.delete_credential() {
                    Ok(()) => {}
                    Err(keyring::Error::NoEntry) => {}
                    Err(_) => {}
                }
            }

            self.untrack_key(key)?;
            Ok(())
        })
    }

    fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let keys = load_tracked_keys(&self.keys_path)?;
        let mut filtered: Vec<String> =
            keys.into_iter().filter(|k| k.starts_with(prefix)).collect();
        filtered.sort();
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_and_list_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("keys.json");

        save_tracked_keys(
            &keys_path,
            &["a/1".to_string(), "a/2".to_string(), "b/1".to_string()],
        )
        .unwrap();
        let keys = load_tracked_keys(&keys_path).unwrap();
        assert_eq!(keys, vec!["a/1", "a/2", "b/1"]);
    }

    #[test]
    fn load_tracked_keys_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("nonexistent.json");
        let keys = load_tracked_keys(&keys_path).unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn save_tracked_keys_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("nested").join("dir").join("keys.json");
        save_tracked_keys(&keys_path, &["key1".to_string()]).unwrap();
        let keys = load_tracked_keys(&keys_path).unwrap();
        assert_eq!(keys, vec!["key1"]);
    }

    #[test]
    fn track_key_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("keys.json");
        let store =
            LocalKeyValueCredentialStore::new("test-service".to_string(), keys_path.clone());

        store.track_key("my-key").unwrap();
        store.track_key("my-key").unwrap();
        let keys = load_tracked_keys(&keys_path).unwrap();
        assert_eq!(keys, vec!["my-key"]);
    }

    #[test]
    fn untrack_key_removes_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("keys.json");
        save_tracked_keys(
            &keys_path,
            &["a".to_string(), "b".to_string(), "c".to_string()],
        )
        .unwrap();

        let store =
            LocalKeyValueCredentialStore::new("test-service".to_string(), keys_path.clone());

        store.untrack_key("b").unwrap();
        let keys = load_tracked_keys(&keys_path).unwrap();
        assert_eq!(keys, vec!["a", "c"]);
    }

    #[test]
    fn untrack_key_nonexistent_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("keys.json");
        save_tracked_keys(&keys_path, &["a".to_string()]).unwrap();

        let store =
            LocalKeyValueCredentialStore::new("test-service".to_string(), keys_path.clone());

        store.untrack_key("nonexistent").unwrap();
        let keys = load_tracked_keys(&keys_path).unwrap();
        assert_eq!(keys, vec!["a"]);
    }

    #[test]
    fn list_keys_filters_by_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("keys.json");
        save_tracked_keys(
            &keys_path,
            &[
                "batman/app-id".to_string(),
                "superman/app-id".to_string(),
                "superman/private-key".to_string(),
            ],
        )
        .unwrap();

        let store = LocalKeyValueCredentialStore::new("test-service".to_string(), keys_path);

        let keys = store.list_keys("superman/").unwrap();
        assert_eq!(keys, vec!["superman/app-id", "superman/private-key"]);
    }

    #[test]
    fn list_keys_empty_prefix_returns_all() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("keys.json");
        save_tracked_keys(&keys_path, &["a".to_string(), "b".to_string()]).unwrap();

        let store = LocalKeyValueCredentialStore::new("test-service".to_string(), keys_path);

        let keys = store.list_keys("").unwrap();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn list_keys_no_match_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("keys.json");
        save_tracked_keys(&keys_path, &["superman/id".to_string()]).unwrap();

        let store = LocalKeyValueCredentialStore::new("test-service".to_string(), keys_path);

        let keys = store.list_keys("batman/").unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn list_keys_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_path = tmp.path().join("nonexistent.json");

        let store = LocalKeyValueCredentialStore::new("test-service".to_string(), keys_path);

        let keys = store.list_keys("").unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn store_constructor_and_with_collection() {
        let store = LocalKeyValueCredentialStore::new(
            "botminter.team.matrix".to_string(),
            PathBuf::from("/tmp/keys.json"),
        );
        assert_eq!(store.service, "botminter.team.matrix");
        assert!(store.collection.is_none());

        let store = store.with_collection(Some("my-collection".to_string()));
        assert_eq!(store.collection, Some("my-collection".to_string()));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn custom_collection_store_fails_clearly_off_linux() {
        let tmp = tempfile::tempdir().unwrap();
        let store = LocalKeyValueCredentialStore::new(
            "botminter.team.github-app".to_string(),
            tmp.path().join("keys.json"),
        )
        .with_collection(Some("custom".to_string()));

        let err = store.store("member/app-id", "123").unwrap_err();
        assert!(err
            .to_string()
            .contains("Custom keyring collections are only supported on Linux"));
    }
}
