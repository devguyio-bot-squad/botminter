use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

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
        Ok(self
            .tokens
            .lock()
            .unwrap()
            .get(member_name)
            .cloned())
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

/// Connects to Secret Service and finds a collection by label.
/// Returns the collection, or creates it if it doesn't exist.
fn get_or_create_collection<'a>(
    ss: &'a dbus_secret_service::SecretService,
    name: &str,
) -> Result<dbus_secret_service::Collection<'a>> {
    // Search existing collections by label
    if let Ok(collections) = ss.get_all_collections() {
        for c in collections {
            if let Ok(label) = c.get_label() {
                if label == name {
                    return Ok(c);
                }
            }
        }
    }

    // Create the collection (empty alias = no alias)
    ss.create_collection(name, "")
        .map_err(|e| anyhow::anyhow!("Failed to create keyring collection '{}': {}", name, e))
}


fn connect_secret_service() -> Result<dbus_secret_service::SecretService> {
    dbus_secret_service::SecretService::connect(
        dbus_secret_service::EncryptionType::Plain,
    )
    .map_err(|e| {
        anyhow::anyhow!(
            "Cannot connect to Secret Service (D-Bus). \
             Install a Secret Service provider (e.g., gnome-keyring) \
             or set BM_BRIDGE_TOKEN_* environment variables instead. ({})",
            e
        )
    })
}

/// Checks if the keyring collection is unlocked.
/// When `collection_name` is Some, checks that specific collection.
/// Otherwise checks the default collection.
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

    let locked = collection.is_locked().map_err(|e| {
        anyhow::anyhow!("Cannot check keyring lock state: {}", e)
    })?;

    if locked {
        anyhow::bail!(
            "System keyring is locked. Unlock it before storing credentials.\n\
             On GNOME: the keyring unlocks automatically on login.\n\
             On headless systems: run `gnome-keyring-daemon --unlock` or \
             set BM_BRIDGE_TOKEN_* environment variables instead."
        );
    }

    Ok(())
}

/// Checks if the default keyring is unlocked (backward compat).
pub fn check_keyring_unlocked() -> Result<()> {
    check_keyring_unlocked_for(None)
}

/// Store a secret in a named collection using dbus-secret-service directly.
fn dss_store(service: &str, member_name: &str, token: &str, collection_name: &str) -> Result<()> {
    let ss = connect_secret_service()?;
    let collection = get_or_create_collection(&ss, collection_name)?;
    collection.ensure_unlocked().map_err(|e| {
        anyhow::anyhow!("Failed to unlock collection '{}': {}", collection_name, e)
    })?;

    let mut attrs = std::collections::HashMap::new();
    attrs.insert("service", service);
    attrs.insert("username", member_name);

    collection
        .create_item(
            &format!("{} — {}", service, member_name),
            attrs,
            token.as_bytes(),
            true, // replace existing
            "text/plain",
        )
        .map_err(|e| anyhow::anyhow!("Failed to store credential: {}", e))?;

    Ok(())
}

/// Retrieve a secret from a named collection using dbus-secret-service directly.
fn dss_retrieve(service: &str, member_name: &str, collection_name: &str) -> Result<Option<String>> {
    let ss = connect_secret_service()?;
    let collection = match get_or_create_collection(&ss, collection_name) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    if collection.is_locked().unwrap_or(true) {
        return Ok(None);
    }

    let mut attrs = std::collections::HashMap::new();
    attrs.insert("service", service);
    attrs.insert("username", member_name);

    let items = collection.search_items(attrs).map_err(|e| {
        anyhow::anyhow!("Failed to search keyring: {}", e)
    })?;

    if let Some(item) = items.first() {
        let secret = item.get_secret().map_err(|e| {
            anyhow::anyhow!("Failed to read secret: {}", e)
        })?;
        Ok(Some(String::from_utf8_lossy(&secret).to_string()))
    } else {
        Ok(None)
    }
}

/// Delete a secret from a named collection using dbus-secret-service directly.
fn dss_delete(service: &str, member_name: &str, collection_name: &str) -> Result<()> {
    let ss = connect_secret_service()?;
    let collection = match get_or_create_collection(&ss, collection_name) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let mut attrs = std::collections::HashMap::new();
    attrs.insert("service", service);
    attrs.insert("username", member_name);

    if let Ok(items) = collection.search_items(attrs) {
        for item in items {
            let _ = item.delete();
        }
    }
    Ok(())
}

impl CredentialStore for LocalCredentialStore {
    fn store(&self, member_name: &str, token: &str) -> Result<()> {
        if let Some(ref coll) = self.collection {
            // Custom collection via dbus-secret-service
            dss_store(&self.service, member_name, token, coll)?;
        } else {
            // Default: keyring::Entry → login collection
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
                },
            );
            save_state(&self.state_path, &state)?;
        }

        Ok(())
    }

    fn retrieve(&self, member_name: &str) -> Result<Option<String>> {
        if let Some(ref coll) = self.collection {
            return dss_retrieve(&self.service, member_name, coll);
        }

        let entry = match keyring::Entry::new(&self.service, member_name) {
            Ok(e) => e,
            Err(_) => {
                // Keyring not available — fall back to env var resolution
                return Ok(None);
            }
        };
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => {
                eprintln!(
                    "Warning: System keyring error ({}). \
                     Falling back to BM_BRIDGE_TOKEN_{} env var.",
                    e,
                    env_var_suffix(member_name)
                );
                Ok(None)
            }
        }
    }

    fn remove(&self, member_name: &str) -> Result<()> {
        if let Some(ref coll) = self.collection {
            dss_delete(&self.service, member_name, coll)?;
        } else if let Ok(entry) = keyring::Entry::new(&self.service, member_name) {
            match entry.delete_credential() {
                Ok(()) => {}
                Err(keyring::Error::NoEntry) => {} // Already gone
                Err(e) => {
                    eprintln!("Warning: Could not remove credential from keyring: {}", e);
                }
            }
        }

        // Remove from bridge-state.json identities
        let mut state = load_state(&self.state_path)?;
        state.identities.remove(member_name);
        save_state(&self.state_path, &state)?;

        Ok(())
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
}

/// A room on the bridge.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeRoom {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    pub created_at: String,
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

/// Invokes a Justfile recipe from the bridge directory.
///
/// Sets `BRIDGE_CONFIG_DIR` (temp dir) and `BM_TEAM_NAME` environment variables.
/// After the recipe completes, reads `config.json` from the temp dir if it exists.
/// Returns Ok(None) if no config.json was written.
pub fn invoke_recipe(
    bridge_dir: &Path,
    recipe: &str,
    args: &[&str],
    team_name: &str,
) -> Result<Option<serde_json::Value>> {
    let config_dir = tempfile::tempdir().context("Failed to create temp dir for bridge config")?;
    let config_dir_path = config_dir.path().to_path_buf();

    let justfile = bridge_dir.join("Justfile");

    let mut cmd = Command::new("just");
    cmd.arg("--justfile")
        .arg(&justfile)
        .arg(recipe)
        .args(args)
        .current_dir(bridge_dir)
        .env("BRIDGE_CONFIG_DIR", &config_dir_path)
        .env("BM_TEAM_NAME", team_name);

    let output = cmd
        .output()
        .with_context(|| format!("Failed to invoke bridge recipe '{}'", recipe))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Bridge recipe '{}' failed (exit {:?}):\n{}",
            recipe,
            output.status.code(),
            stderr
        );
    }

    let config_file = config_dir_path.join("config.json");
    if config_file.exists() {
        let contents = fs::read_to_string(&config_file)
            .context("Failed to read bridge config exchange output")?;
        let value: serde_json::Value =
            serde_json::from_str(&contents).context("Failed to parse bridge config exchange JSON")?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// Provisions bridge identities for team members during `bm teams sync --bridge`.
///
/// For each member NOT already in `bridge-state.json`:
/// - **Local (managed) bridges:** invoke the onboard recipe directly (creates user + returns token).
/// - **External bridges:** check if a credential exists in `credential_store`; skip with warning if not.
///
/// After provisioning identities, creates a team room if `state.rooms` is empty and
/// the manifest has a room spec. Saves bridge state at the end.
pub fn provision_bridge(
    team_repo: &Path,
    team_name: &str,
    workzone: &Path,
    members: &[String],
    credential_store: &dyn CredentialStore,
) -> Result<()> {
    // Discover bridge
    let bridge_dir = match discover(team_repo, team_name)? {
        Some(dir) => dir,
        None => {
            println!("No bridge configured -- skipping");
            return Ok(());
        }
    };

    let manifest = load_manifest(&bridge_dir)?;
    let state_path = state_path(workzone, team_name);
    let mut state = load_state(&state_path)?;

    // Provision identities for members not yet in state
    for member in members {
        if state.identities.contains_key(member) {
            println!("  {}: already provisioned -- skipping", member);
            continue;
        }

        if manifest.spec.bridge_type == "external" {
            // External bridge: operator must have pre-supplied a token
            let has_cred = resolve_credential_from_store(member, credential_store)?;
            if has_cred.is_none() {
                eprintln!(
                    "  {}: no bridge credentials -- skipping. Use `bm bridge identity add` to add later.",
                    member
                );
                continue;
            }
            // Set env var for recipe to use
            let env_key = format!("BM_BRIDGE_TOKEN_{}", env_var_suffix(member));
            std::env::set_var(&env_key, has_cred.as_ref().unwrap());
        }

        // Invoke onboard recipe
        let recipe_result = invoke_recipe(
            &bridge_dir,
            &manifest.spec.identity.onboard,
            &[member.as_str()],
            team_name,
        )?;

        // Process recipe output
        if let Some(config) = recipe_result {
            let username = config["username"]
                .as_str()
                .unwrap_or(member.as_str())
                .to_string();
            let user_id = config["user_id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let token = config["token"].as_str().map(|s| s.to_string());

            // Store identity metadata in state (no token)
            state.identities.insert(
                member.clone(),
                BridgeIdentity {
                    username,
                    user_id,
                    token: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                },
            );

            // Store token in credential store (system keyring)
            if let Some(ref tok) = token {
                if let Err(e) = credential_store.store(member, tok) {
                    eprintln!(
                        "Warning: Could not store credential for {} in keyring: {}. \
                         Set BM_BRIDGE_TOKEN_{} env var instead.",
                        member,
                        e,
                        env_var_suffix(member)
                    );
                }
            }

            println!("  {}: provisioned", member);
        } else {
            println!("  {}: onboard recipe returned no config", member);
        }

        // Clean up env var if we set it for external bridge
        if manifest.spec.bridge_type == "external" {
            let env_key = format!("BM_BRIDGE_TOKEN_{}", env_var_suffix(member));
            std::env::remove_var(&env_key);
        }
    }

    // Create team room if rooms are empty and manifest has room spec
    if state.rooms.is_empty() {
        if let Some(ref room_spec) = manifest.spec.room {
            let room_name = format!("{}-general", team_name);
            let room_result = invoke_recipe(
                &bridge_dir,
                &room_spec.create,
                &[&room_name],
                team_name,
            )?;

            let room_id = room_result
                .as_ref()
                .and_then(|v| v["room_id"].as_str())
                .map(|s| s.to_string());

            state.rooms.push(BridgeRoom {
                name: room_name.clone(),
                room_id,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
            println!("  Created team room: {}", room_name);
        }
    }

    // Save bridge state
    state.bridge_name = Some(manifest.metadata.name.clone());
    state.bridge_type = Some(manifest.spec.bridge_type.clone());
    save_state(&state_path, &state)?;

    Ok(())
}

/// Normalizes a name into a valid env var suffix: uppercased, hyphens replaced with underscores.
pub fn env_var_suffix_pub(name: &str) -> String {
    env_var_suffix(name)
}

/// Normalizes a name into a valid env var suffix: uppercased, hyphens replaced with underscores.
fn env_var_suffix(name: &str) -> String {
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

    // Path to the stub bridge fixture relative to workspace root
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
            identities,
            rooms: vec![BridgeRoom {
                name: "general".to_string(),
                room_id: Some("r-123".to_string()),
                created_at: "2026-03-08T00:00:00Z".to_string(),
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

    // ── CredentialStore (InMemory) tests ─────────────────────────────

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

    #[test]
    fn invoke_recipe_start() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "start", &[], "test-team").unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["url"], "http://localhost:0");
        assert_eq!(val["status"], "stub");
    }

    #[test]
    fn invoke_recipe_onboard() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "onboard", &["alice"], "test-team").unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["username"], "alice");
        assert_eq!(val["user_id"], "stub-id");
        assert_eq!(val["token"], "stub-token");
    }

    #[test]
    fn invoke_recipe_stop_no_config() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "stop", &[], "test-team").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn invoke_recipe_room_create() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "room-create", &["general"], "test-team").unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["name"], "general");
        assert_eq!(val["room_id"], "stub-room-id");
    }

    #[test]
    fn invoke_recipe_room_list() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "room-list", &[], "test-team").unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val["rooms"].is_array());
    }
}
