//! Shared keyring backend for credential storage via the system keyring.
//!
//! Provides platform-abstracted access to the D-Bus Secret Service on Linux
//! and the `keyring` crate's platform backend on macOS. Both
//! `bridge::credential::LocalCredentialStore` and
//! `formation::local::credential::LocalKeyValueCredentialStore` delegate
//! keyring operations to this module.

use anyhow::Result;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

// ── D-Bus Secret Service helpers (Linux only) ────────────────────────

/// Connects to the D-Bus Secret Service.
#[cfg(target_os = "linux")]
pub fn connect_secret_service() -> Result<dbus_secret_service::SecretService> {
    dbus_secret_service::SecretService::connect(dbus_secret_service::EncryptionType::Plain).map_err(
        |e| {
            anyhow::anyhow!(
                "Cannot connect to Secret Service (D-Bus). \
                 Install a Secret Service provider (e.g., gnome-keyring) \
                 or use environment variable overrides instead. ({})",
                e
            )
        },
    )
}

/// Finds or creates a collection by label.
#[cfg(target_os = "linux")]
pub fn get_or_create_collection<'a>(
    ss: &'a dbus_secret_service::SecretService,
    name: &str,
) -> Result<dbus_secret_service::Collection<'a>> {
    if let Ok(collections) = ss.get_all_collections() {
        for c in collections {
            if let Ok(label) = c.get_label() {
                if label == name {
                    return Ok(c);
                }
            }
        }
    }
    ss.create_collection(name, "")
        .map_err(|e| anyhow::anyhow!("Failed to create keyring collection '{}': {}", name, e))
}

/// Stores a secret in a named collection via dbus-secret-service.
#[cfg(target_os = "linux")]
pub fn dss_store(service: &str, key: &str, value: &str, collection_name: &str) -> Result<()> {
    let ss = connect_secret_service()?;
    let collection = get_or_create_collection(&ss, collection_name)?;
    collection
        .ensure_unlocked()
        .map_err(|e| anyhow::anyhow!("Failed to unlock collection '{}': {}", collection_name, e))?;

    let mut attrs = HashMap::new();
    attrs.insert("service", service);
    attrs.insert("username", key);

    collection
        .create_item(
            &format!("{} — {}", service, key),
            attrs,
            value.as_bytes(),
            true, // replace existing
            "text/plain",
        )
        .map_err(|e| anyhow::anyhow!("Failed to store credential: {}", e))?;

    Ok(())
}

/// Retrieves a secret from a named collection via dbus-secret-service.
#[cfg(target_os = "linux")]
pub fn dss_retrieve(service: &str, key: &str, collection_name: &str) -> Result<Option<String>> {
    let ss = connect_secret_service()?;
    let collection = match get_or_create_collection(&ss, collection_name) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    if collection.is_locked().unwrap_or(true) {
        return Ok(None);
    }

    let mut attrs = HashMap::new();
    attrs.insert("service", service);
    attrs.insert("username", key);

    let items = collection
        .search_items(attrs)
        .map_err(|e| anyhow::anyhow!("Failed to search keyring: {}", e))?;

    if let Some(item) = items.first() {
        let secret = item
            .get_secret()
            .map_err(|e| anyhow::anyhow!("Failed to read secret: {}", e))?;
        Ok(Some(String::from_utf8_lossy(&secret).to_string()))
    } else {
        Ok(None)
    }
}

/// Deletes a secret from a named collection via dbus-secret-service.
#[cfg(target_os = "linux")]
pub fn dss_delete(service: &str, key: &str, collection_name: &str) -> Result<()> {
    let ss = connect_secret_service()?;
    let collection = match get_or_create_collection(&ss, collection_name) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let mut attrs = HashMap::new();
    attrs.insert("service", service);
    attrs.insert("username", key);

    if let Ok(items) = collection.search_items(attrs) {
        for item in items {
            let _ = item.delete();
        }
    }
    Ok(())
}

// ── Non-Linux stubs ──────────────────────────────────────────────────

#[cfg(not(target_os = "linux"))]
pub fn dss_store(_service: &str, _key: &str, _value: &str, collection_name: &str) -> Result<()> {
    anyhow::bail!(
        "Custom keyring collections are only supported on Linux (requested '{}')",
        collection_name
    )
}

#[cfg(not(target_os = "linux"))]
pub fn dss_retrieve(
    _service: &str,
    _key: &str,
    _collection_name: &str,
) -> Result<Option<String>> {
    Ok(None)
}

#[cfg(not(target_os = "linux"))]
pub fn dss_delete(_service: &str, _key: &str, _collection_name: &str) -> Result<()> {
    Ok(())
}

// ── Keyring unlock checks ────────────────────────────────────────────

/// Checks if a keyring collection is unlocked.
///
/// When `collection_name` is `Some`, checks that specific collection.
/// Otherwise checks the default collection.
#[cfg(target_os = "linux")]
pub fn check_keyring_unlocked_for(collection_name: Option<&str>) -> Result<()> {
    let ss = connect_secret_service()?;

    let collection = if let Some(name) = collection_name {
        get_or_create_collection(&ss, name)?
    } else {
        ss.get_default_collection().map_err(|e| {
            anyhow::anyhow!(
                "No default keyring collection found. \
                 Run `seahorse` or `gnome-keyring-daemon` to create one. ({})",
                e
            )
        })?
    };

    let locked = collection
        .is_locked()
        .map_err(|e| anyhow::anyhow!("Cannot check keyring lock state: {}", e))?;

    if locked {
        anyhow::bail!(
            "System keyring is locked. Unlock it before storing credentials.\n\
             On GNOME: the keyring unlocks automatically on login.\n\
             On headless systems: run `gnome-keyring-daemon --unlock` or \
             use environment variable overrides instead."
        );
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn check_keyring_unlocked_for(collection_name: Option<&str>) -> Result<()> {
    if collection_name.is_some() {
        anyhow::bail!("Custom keyring collections are only supported on Linux")
    }
    Ok(())
}

/// Checks if the default keyring is unlocked (convenience wrapper).
pub fn check_keyring_unlocked() -> Result<()> {
    check_keyring_unlocked_for(None)
}

// ── D-Bus session override ───────────────────────────────────────────

/// Runs a closure with `BM_KEYRING_DBUS` as `DBUS_SESSION_BUS_ADDRESS` if set.
///
/// This allows keyring operations to use an isolated D-Bus session while
/// the process-wide `DBUS_SESSION_BUS_ADDRESS` points to the real system bus
/// (needed by podman). Since `bm` is single-threaded, this is safe.
pub fn with_keyring_dbus<T, F: FnOnce() -> T>(f: F) -> T {
    if let Ok(dbus) = std::env::var("BM_KEYRING_DBUS") {
        let original = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &dbus);
        let result = f();
        match original {
            Some(v) => std::env::set_var("DBUS_SESSION_BUS_ADDRESS", v),
            None => std::env::remove_var("DBUS_SESSION_BUS_ADDRESS"),
        }
        result
    } else {
        f()
    }
}
