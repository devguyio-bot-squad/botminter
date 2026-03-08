use std::fs;

use anyhow::{bail, Context, Result};

use crate::config;
use crate::profile;

/// Handles `bm minty [-t team]`.
///
/// Launches the resolved coding agent in the current working directory with
/// Minty's persona prompt as the system prompt. Minty works without any teams
/// configured — in that case it operates in "profiles-only" mode.
pub fn run(team_flag: Option<&str>) -> Result<()> {
    // Ensure Minty config is initialized on disk
    let minty_dir = ensure_minty_initialized()?;

    let prompt_path = minty_dir.join("prompt.md");
    if !prompt_path.exists() {
        bail!(
            "Minty prompt.md not found at {}. \
             Run `bm profiles init` to extract Minty config.",
            prompt_path.display()
        );
    }

    // Resolve coding agent
    let binary = resolve_coding_agent(team_flag)?;

    // Log profiles-only mode if no teams configured
    if team_flag.is_none() && config::load().is_err() {
        eprintln!(
            "Note: ~/.botminter/ not found on this machine.\n\
             Minty is running in profiles-only mode — team commands are unavailable.\n\
             To connect to teams, run `bm init` or copy your config.yml.\n"
        );
    }

    // Launch coding agent via exec (replaces this process)
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(&binary)
        .current_dir(&minty_dir)
        .arg("--append-system-prompt-file")
        .arg(&prompt_path)
        .exec();

    // exec() only returns on error
    bail!("Failed to launch {}: {}", binary, err);
}

/// Ensures Minty config is present on disk at `~/.config/botminter/minty/`.
/// Auto-extracts embedded config if the directory is missing.
fn ensure_minty_initialized() -> Result<std::path::PathBuf> {
    let minty_dir = profile::minty_dir()?;

    if !minty_dir.join("prompt.md").exists() {
        eprintln!("Initializing Minty config...");
        fs::create_dir_all(&minty_dir).with_context(|| {
            format!(
                "Failed to create minty directory {}",
                minty_dir.display()
            )
        })?;
        profile::minty_embedded::extract_minty_to_disk(&minty_dir)?;
        eprintln!("Extracted Minty config to {}", minty_dir.display());
    }

    Ok(minty_dir)
}

/// Resolves the coding agent binary name.
///
/// Resolution order:
/// 1. If `-t` specified: resolve from team config (same path as `bm chat`)
/// 2. If no `-t`: read the first available profile from disk and use its
///    default coding agent
fn resolve_coding_agent(team_flag: Option<&str>) -> Result<String> {
    if let Some(team_name) = team_flag {
        // Team-based resolution — same as bm chat
        let cfg = config::load()?;
        let team = config::resolve_team(&cfg, Some(team_name))?;
        let team_repo = team.path.join("team");
        let manifest_path = team_repo.join("botminter.yml");
        let contents = fs::read_to_string(&manifest_path)
            .context("Failed to read team botminter.yml")?;
        let manifest: profile::ProfileManifest = serde_yml::from_str(&contents)
            .context("Failed to parse team botminter.yml")?;
        let agent = profile::resolve_coding_agent(team, &manifest)?;
        Ok(agent.binary.clone())
    } else {
        // No team — resolve from first available profile on disk
        resolve_agent_from_profiles()
    }
}

/// Resolves the coding agent binary from the first available profile on disk.
fn resolve_agent_from_profiles() -> Result<String> {
    // Ensure profiles exist
    profile::ensure_profiles_initialized()?;

    let profiles = profile::list_profiles()?;
    if profiles.is_empty() {
        bail!(
            "No profiles found on disk. Run `bm profiles init` to extract profiles."
        );
    }

    // Try each profile until one has a default coding agent
    for name in &profiles {
        if let Ok(manifest) = profile::read_manifest(name) {
            if !manifest.default_coding_agent.is_empty() {
                if let Some(agent) = manifest.coding_agents.get(&manifest.default_coding_agent) {
                    return Ok(agent.binary.clone());
                }
            }
        }
    }

    bail!(
        "No profile defines a default coding agent. \
         Available profiles: {}",
        profiles.join(", ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_agent_from_profiles_finds_default() {
        // Uses the real embedded profiles (extracted to a temp dir)
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        profile::extract_embedded_to_disk(&profiles_dir).unwrap();

        // Read profiles from the temp dir and check at least one has a default agent
        let profiles = profile::list_profiles_from(&profiles_dir).unwrap();
        assert!(!profiles.is_empty(), "Should have at least one profile");

        let mut found_agent = false;
        for name in &profiles {
            if let Ok(manifest) = profile::read_manifest_from(name, &profiles_dir) {
                if !manifest.default_coding_agent.is_empty() {
                    let agent = manifest
                        .coding_agents
                        .get(&manifest.default_coding_agent);
                    assert!(
                        agent.is_some(),
                        "Profile '{}' declares default_coding_agent '{}' but it's not in coding_agents map",
                        name,
                        manifest.default_coding_agent
                    );
                    found_agent = true;
                    break;
                }
            }
        }
        assert!(
            found_agent,
            "At least one profile should define a default coding agent"
        );
    }

    #[test]
    fn ensure_minty_initialized_creates_config() {
        let tmp = tempfile::tempdir().unwrap();
        let minty_dir = tmp.path().join("botminter").join("minty");

        // Simulate ensure_minty_initialized logic with explicit path
        std::fs::create_dir_all(&minty_dir).unwrap();
        profile::minty_embedded::extract_minty_to_disk(&minty_dir).unwrap();

        assert!(
            minty_dir.join("prompt.md").exists(),
            "prompt.md should exist after initialization"
        );
        assert!(
            minty_dir.join("config.yml").exists(),
            "config.yml should exist after initialization"
        );
        assert!(
            minty_dir.join(".claude/skills/hire-guide/SKILL.md").exists(),
            "skills should be at .claude/skills/ path after initialization"
        );
    }

    #[test]
    fn resolve_agent_team_not_found_errors() {
        // Resolving with a nonexistent team name should fail
        // (config::load() will fail because ~/.botminter/ doesn't exist in test env)
        let result = resolve_coding_agent(Some("nonexistent-team"));
        assert!(result.is_err(), "Should fail for nonexistent team");
    }
}
