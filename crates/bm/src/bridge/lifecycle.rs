use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Invokes a Justfile recipe from the bridge directory.
///
/// Sets `BRIDGE_CONFIG_DIR` (temp dir), `BM_TEAM_NAME`, and optionally
/// `BM_BRIDGE_STATE_DIR` environment variables. After the recipe completes,
/// reads `config.json` from the temp dir if it exists.
/// Returns Ok(None) if no config.json was written.
pub fn invoke_recipe(
    bridge_dir: &Path,
    recipe: &str,
    args: &[&str],
    team_name: &str,
    state_dir: Option<&Path>,
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
        .env("BM_BRIDGE_DIR", bridge_dir)
        .env("BM_TEAM_NAME", team_name);

    // Persistent state directory for bridge-specific data (e.g., passwords).
    // Survives team re-initialization unlike BM_BRIDGE_DIR which points to
    // the profile template directory.
    if let Some(sd) = state_dir {
        cmd.env("BM_BRIDGE_STATE_DIR", sd);
    }

    // If BM_BRIDGE_HOME is set, override HOME for bridge recipes.
    // This allows test environments to use a different HOME for bm config
    // while bridge recipes (which spawn podman) use the real HOME for
    // container storage. Absent = no-op (production behavior unchanged).
    if let Ok(bridge_home) = std::env::var("BM_BRIDGE_HOME") {
        cmd.env("HOME", bridge_home);
    }

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

    #[test]
    fn invoke_recipe_start() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "start", &[], "test-team", None).unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["service_url"], "http://localhost:0");
        assert_eq!(val["status"], "stub");
    }

    #[test]
    fn invoke_recipe_onboard() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "onboard", &["alice"], "test-team", None).unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["username"], "alice");
        assert_eq!(val["user_id"], "stub-id");
        assert_eq!(val["token"], "stub-token");
    }

    #[test]
    fn invoke_recipe_stop_no_config() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "stop", &[], "test-team", None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn invoke_recipe_room_create() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "room-create", &["general"], "test-team", None).unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["name"], "general");
        assert_eq!(val["room_id"], "stub-room-id");
    }

    #[test]
    fn invoke_recipe_room_list() {
        let bridge_dir = stub_bridge_dir();
        let result = invoke_recipe(&bridge_dir, "room-list", &[], "test-team", None).unwrap();
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val["rooms"].is_array());
    }
}
