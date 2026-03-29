use std::fs;

use anyhow::{bail, Context, Result};

use crate::config;
use crate::profile;
use crate::profile::CodingAgentDef;

/// Handles `bm minty [-t team] [-a]`.
///
/// Launches the resolved coding agent in the current working directory with
/// Minty's persona prompt as the system prompt. Minty works without any teams
/// configured — in that case it operates in "profiles-only" mode.
pub fn run(team_flag: Option<&str>, autonomous: bool) -> Result<()> {
    let minty_dir = profile::ensure_minty_initialized()?;

    let prompt_path = minty_dir.join("prompt.md");
    if !prompt_path.exists() {
        bail!(
            "Minty prompt.md not found at {}. \
             Run `bm profiles init` to extract Minty config.",
            prompt_path.display()
        );
    }

    let agent = resolve_coding_agent(team_flag)?;

    if team_flag.is_none() && config::load().is_err() {
        eprintln!(
            "Note: ~/.botminter/ not found on this machine.\n\
             Minty is running in profiles-only mode — team commands are unavailable.\n\
             To connect to teams, run `bm init` or copy your config.yml.\n"
        );
    }

    let prompt_flag = agent.system_prompt_flag.as_deref().with_context(|| {
        format!(
            "Coding agent '{}' ({}) does not define a system_prompt_flag",
            agent.display_name, agent.binary
        )
    })?;

    // Launch coding agent via exec (replaces this process)
    use std::os::unix::process::CommandExt;
    let mut cmd = std::process::Command::new(&agent.binary);
    cmd.current_dir(&minty_dir)
        .arg(prompt_flag)
        .arg(&prompt_path);
    if autonomous {
        if let Some(flag) = agent.skip_permissions_flag.as_deref() {
            cmd.arg(flag);
        }
    }
    let err = cmd.exec();

    bail!("Failed to launch {}: {}", agent.binary, err);
}

/// Resolves the coding agent definition from team config or profile defaults.
fn resolve_coding_agent(team_flag: Option<&str>) -> Result<CodingAgentDef> {
    if let Some(team_name) = team_flag {
        let cfg = config::load()?;
        let team = config::resolve_team(&cfg, Some(team_name))?;
        let team_repo = team.path.join("team");
        let contents = fs::read_to_string(team_repo.join("botminter.yml"))
            .context("Failed to read team botminter.yml")?;
        let manifest: profile::ProfileManifest =
            serde_yml::from_str(&contents).context("Failed to parse team botminter.yml")?;
        let agent = profile::resolve_coding_agent(team, &manifest)?;
        Ok(agent.clone())
    } else {
        super::ensure_profiles(false)?;
        profile::resolve_agent_from_profiles()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_agent_from_profiles_finds_default() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        profile::extract_embedded_to_disk(&profiles_dir).unwrap();

        let profiles = profile::list_profiles_from(&profiles_dir).unwrap();
        assert!(!profiles.is_empty());

        let mut found_agent = false;
        for name in &profiles {
            if let Ok(manifest) = profile::read_manifest_from(name, &profiles_dir) {
                if !manifest.default_coding_agent.is_empty() {
                    let agent = manifest.coding_agents.get(&manifest.default_coding_agent);
                    assert!(agent.is_some());
                    found_agent = true;
                    break;
                }
            }
        }
        assert!(found_agent);
    }

    #[test]
    fn ensure_minty_initialized_creates_config() {
        let tmp = tempfile::tempdir().unwrap();
        let minty_dir = tmp.path().join("botminter").join("minty");
        std::fs::create_dir_all(&minty_dir).unwrap();
        profile::extract_minty_to_disk(&minty_dir).unwrap();

        assert!(minty_dir.join("prompt.md").exists());
        assert!(minty_dir.join("config.yml").exists());
        assert!(minty_dir.join(".claude/skills/hire-guide/SKILL.md").exists());
    }

    #[test]
    fn resolve_agent_team_not_found_errors() {
        let result = resolve_coding_agent(Some("nonexistent-team"));
        assert!(result.is_err());
    }
}
