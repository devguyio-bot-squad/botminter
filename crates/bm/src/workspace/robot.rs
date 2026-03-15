use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

// ── RObot injection ─────────────────────────────────────────────────

/// Bridge-specific configuration to inject into ralph.yml's RObot section.
///
/// Per ADR-0003: NO secrets (auth tokens) go in ralph.yml.
/// Only non-secret config (bot_user_id, room_id, server_url, operator_id).
pub struct RobotBridgeConfig {
    pub bot_user_id: String,
    pub room_id: String,
    pub server_url: String,
    pub operator_id: Option<String>,
}

/// Sets `RObot.enabled` in a ralph.yml file based on credential availability.
///
/// Thin wrapper around `inject_robot_config` for backward compatibility.
pub fn inject_robot_enabled(
    ralph_yml_path: &Path,
    member_has_credentials: bool,
) -> Result<()> {
    inject_robot_config(ralph_yml_path, member_has_credentials, None, None)
}

/// Injects bridge-type-aware RObot configuration into ralph.yml.
///
/// This function:
/// - Loads ralph.yml as a YAML value
/// - Sets `doc["RObot"]["enabled"]` based on credentials
/// - For `bridge_type == Some("rocketchat")` with credentials, also sets:
///   - `RObot.rocketchat.bot_user_id`
///   - `RObot.rocketchat.room_id`
///   - `RObot.rocketchat.server_url`
///   - `RObot.operator_id` (if present in config)
/// - Does NOT write any token, secret, or credential to ralph.yml
/// - Preserves all other ralph.yml content
/// - Writes back to disk
///
/// Per ADR-0003: ralph.yml only gets RObot config. NO secrets.
/// Secrets are injected as env vars by `bm start`.
pub fn inject_robot_config(
    ralph_yml_path: &Path,
    member_has_credentials: bool,
    bridge_type: Option<&str>,
    bridge_config: Option<&RobotBridgeConfig>,
) -> Result<()> {
    let contents = fs::read_to_string(ralph_yml_path)
        .with_context(|| format!("Failed to read ralph.yml at {}", ralph_yml_path.display()))?;
    let mut doc: serde_yml::Value =
        serde_yml::from_str(&contents).context("Failed to parse ralph.yml")?;

    // Ensure RObot section exists as a mapping
    if !doc.get("RObot").is_some_and(|v| v.is_mapping()) {
        doc["RObot"] = serde_yml::Value::Mapping(serde_yml::Mapping::new());
    }

    // Set RObot.enabled and timeout_seconds
    doc["RObot"]["enabled"] = serde_yml::Value::Bool(member_has_credentials);
    if member_has_credentials && !doc["RObot"].get("timeout_seconds").is_some_and(|v| v.is_number()) {
        doc["RObot"]["timeout_seconds"] = serde_yml::Value::Number(serde_yml::Number::from(600u64));
    }

    // For rocketchat bridge with credentials, inject bridge-specific config
    if bridge_type == Some("rocketchat") && member_has_credentials {
        if let Some(config) = bridge_config {
            // Ensure RObot.rocketchat section exists
            if !doc["RObot"].get("rocketchat").is_some_and(|v| v.is_mapping()) {
                doc["RObot"]["rocketchat"] = serde_yml::Value::Mapping(serde_yml::Mapping::new());
            }

            doc["RObot"]["rocketchat"]["bot_user_id"] =
                serde_yml::Value::String(config.bot_user_id.clone());
            doc["RObot"]["rocketchat"]["room_id"] =
                serde_yml::Value::String(config.room_id.clone());
            doc["RObot"]["rocketchat"]["server_url"] =
                serde_yml::Value::String(config.server_url.clone());

            if let Some(ref op_id) = config.operator_id {
                doc["RObot"]["operator_id"] = serde_yml::Value::String(op_id.clone());
            }
        }
    }

    // For tuwunel bridge with credentials, inject Matrix-specific config
    if bridge_type == Some("tuwunel") && member_has_credentials {
        if let Some(config) = bridge_config {
            if !doc["RObot"].get("matrix").is_some_and(|v| v.is_mapping()) {
                doc["RObot"]["matrix"] = serde_yml::Value::Mapping(serde_yml::Mapping::new());
            }

            doc["RObot"]["matrix"]["bot_user_id"] =
                serde_yml::Value::String(config.bot_user_id.clone());
            doc["RObot"]["matrix"]["room_id"] =
                serde_yml::Value::String(config.room_id.clone());
            doc["RObot"]["matrix"]["homeserver_url"] =
                serde_yml::Value::String(config.server_url.clone());

            if let Some(ref op_id) = config.operator_id {
                doc["RObot"]["operator_id"] = serde_yml::Value::String(op_id.clone());
            }
        }
    }

    let output = serde_yml::to_string(&doc).context("Failed to serialize ralph.yml")?;
    fs::write(ralph_yml_path, output)
        .with_context(|| format!("Failed to write ralph.yml at {}", ralph_yml_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_robot_config_rocketchat_writes_bridge_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(&ralph_yml, "preset: feature-development\n").unwrap();

        let config = RobotBridgeConfig {
            bot_user_id: "user123".to_string(),
            room_id: "room456".to_string(),
            server_url: "http://127.0.0.1:3000".to_string(),
            operator_id: Some("op789".to_string()),
        };

        inject_robot_config(&ralph_yml, true, Some("rocketchat"), Some(&config)).unwrap();

        let contents = fs::read_to_string(&ralph_yml).unwrap();
        let doc: serde_yml::Value = serde_yml::from_str(&contents).unwrap();

        assert_eq!(doc["RObot"]["enabled"].as_bool(), Some(true));
        assert_eq!(
            doc["RObot"]["rocketchat"]["bot_user_id"].as_str(),
            Some("user123")
        );
        assert_eq!(
            doc["RObot"]["rocketchat"]["room_id"].as_str(),
            Some("room456")
        );
        assert_eq!(
            doc["RObot"]["rocketchat"]["server_url"].as_str(),
            Some("http://127.0.0.1:3000")
        );
        assert_eq!(
            doc["RObot"]["operator_id"].as_str(),
            Some("op789")
        );
        assert_eq!(
            doc["RObot"]["timeout_seconds"].as_u64(),
            Some(600),
            "timeout_seconds should be set to 600 when enabling RObot"
        );

        // Verify NO auth_token in YAML
        assert!(
            !contents.contains("auth_token"),
            "auth_token must NOT appear in ralph.yml"
        );
    }

    #[test]
    fn inject_robot_config_tuwunel_writes_matrix_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(&ralph_yml, "preset: feature-development\n").unwrap();

        let config = RobotBridgeConfig {
            bot_user_id: "@bot:localhost".to_string(),
            room_id: "!room123:localhost".to_string(),
            server_url: "http://127.0.0.1:8008".to_string(),
            operator_id: None,
        };

        inject_robot_config(&ralph_yml, true, Some("tuwunel"), Some(&config)).unwrap();

        let contents = fs::read_to_string(&ralph_yml).unwrap();
        let doc: serde_yml::Value = serde_yml::from_str(&contents).unwrap();

        assert_eq!(doc["RObot"]["enabled"].as_bool(), Some(true));
        assert_eq!(
            doc["RObot"]["matrix"]["bot_user_id"].as_str(),
            Some("@bot:localhost")
        );
        assert_eq!(
            doc["RObot"]["matrix"]["room_id"].as_str(),
            Some("!room123:localhost")
        );
        assert_eq!(
            doc["RObot"]["matrix"]["homeserver_url"].as_str(),
            Some("http://127.0.0.1:8008")
        );
        assert_eq!(
            doc["RObot"]["timeout_seconds"].as_u64(),
            Some(600),
            "timeout_seconds should be set to 600 when enabling RObot"
        );

        // Verify NO token in YAML
        assert!(
            !contents.contains("access_token"),
            "access_token must NOT appear in ralph.yml"
        );
    }

    #[test]
    fn inject_robot_config_telegram_only_sets_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(&ralph_yml, "preset: feature-development\n").unwrap();

        // Telegram bridge: no bridge_config, just enabled
        inject_robot_config(&ralph_yml, true, Some("telegram"), None).unwrap();

        let contents = fs::read_to_string(&ralph_yml).unwrap();
        let doc: serde_yml::Value = serde_yml::from_str(&contents).unwrap();

        assert_eq!(doc["RObot"]["enabled"].as_bool(), Some(true));
        // No rocketchat section
        assert!(doc["RObot"].get("rocketchat").is_none());
    }

    #[test]
    fn inject_robot_config_no_bridge_sets_enabled_false() {
        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(&ralph_yml, "preset: feature-development\n").unwrap();

        inject_robot_config(&ralph_yml, false, None, None).unwrap();

        let contents = fs::read_to_string(&ralph_yml).unwrap();
        let doc: serde_yml::Value = serde_yml::from_str(&contents).unwrap();

        assert_eq!(doc["RObot"]["enabled"].as_bool(), Some(false));
    }

    #[test]
    fn inject_robot_enabled_backward_compat() {
        // inject_robot_enabled should still work as before
        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(&ralph_yml, "preset: feature-development\n").unwrap();

        inject_robot_enabled(&ralph_yml, true).unwrap();

        let contents = fs::read_to_string(&ralph_yml).unwrap();
        let doc: serde_yml::Value = serde_yml::from_str(&contents).unwrap();
        assert_eq!(doc["RObot"]["enabled"].as_bool(), Some(true));
    }
}
