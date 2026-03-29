use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

// ── YAML deserialization types for ralph.yml ──────────────────────────

/// Minimal deserialization of ralph.yml — only the fields needed for chat.
#[derive(Deserialize)]
pub struct RalphConfig {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub hats: BTreeMap<String, HatConfig>,
    #[serde(default)]
    pub skills: SkillsConfig,
}

#[derive(Deserialize, Default)]
pub struct SkillsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub dirs: Vec<String>,
}

#[derive(Deserialize, Default)]
pub struct CoreConfig {
    #[serde(default)]
    pub guardrails: Vec<String>,
}

#[derive(Deserialize)]
pub struct HatConfig {
    #[serde(default)]
    pub instructions: Option<String>,
}

/// Minimal member manifest — role and name fields.
#[derive(Deserialize)]
pub struct MemberManifest {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

// ── Member info reading ──────────────────────────────────────────────

/// Reads the role and display name from a member's botminter.yml manifest.
///
/// Returns `(role, display_name)`. The display name is the human-friendly name
/// given at hire time (e.g., "testbot"), distinct from the directory slug
/// (e.g., "superman-testbot").
pub fn read_member_info(member_dir: &Path, member_name: &str) -> Result<(String, String)> {
    let manifest_path = member_dir.join("botminter.yml");
    if manifest_path.exists() {
        let contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
        let manifest: MemberManifest = serde_yml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;
        let role = manifest
            .role
            .unwrap_or_else(|| infer_role(member_name));
        let display_name = manifest
            .name
            .unwrap_or_else(|| member_name.to_string());
        Ok((role, display_name))
    } else {
        Ok((infer_role(member_name), member_name.to_string()))
    }
}

/// Infers a role name from a member directory name (e.g., "architect-01" -> "architect").
pub fn infer_role(member_name: &str) -> String {
    member_name
        .split('-')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_role_from_member_name() {
        assert_eq!(infer_role("architect-01"), "architect");
        assert_eq!(infer_role("chief-of-staff-bob"), "chief");
        assert_eq!(infer_role("superman"), "superman");
    }

    #[test]
    fn parse_ralph_yml_guardrails() {
        let yaml = r#"
core:
  guardrails:
    - "Rule one"
    - "Rule two"
hats: {}
"#;
        let config: RalphConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.core.guardrails.len(), 2);
        assert_eq!(config.core.guardrails[0], "Rule one");
    }

    #[test]
    fn parse_ralph_yml_hats() {
        let yaml = r#"
core:
  guardrails: []
hats:
  executor:
    name: Executor
    description: Executes tasks
    instructions: |
      You are the executor.
      Do the work.
  reviewer:
    name: Reviewer
    description: Reviews code
    instructions: |
      Review carefully.
"#;
        let config: RalphConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.hats.len(), 2);
        assert!(config.hats.contains_key("executor"));
        let executor = &config.hats["executor"];
        assert!(executor.instructions.as_ref().unwrap().contains("executor"));
    }

    #[test]
    fn parse_ralph_yml_with_extra_fields() {
        // ralph.yml has many fields we don't need — verify we can parse it
        let yaml = r#"
event_loop:
  prompt_file: PROMPT.md
  max_iterations: 10000
cli:
  backend: claude
core:
  guardrails:
    - "Be careful"
hats:
  builder:
    name: Builder
    triggers:
      - build.task
    publishes:
      - build.completed
    instructions: |
      Build things.
tasks:
  enabled: true
memories:
  enabled: true
skills:
  enabled: true
"#;
        let config: RalphConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.core.guardrails.len(), 1);
        assert_eq!(config.hats.len(), 1);
        assert!(config.hats["builder"]
            .instructions
            .as_ref()
            .unwrap()
            .contains("Build things"));
    }

    #[test]
    fn parse_ralph_yml_missing_core_defaults() {
        let yaml = "hats: {}\n";
        let config: RalphConfig = serde_yml::from_str(yaml).unwrap();
        assert!(config.core.guardrails.is_empty());
        assert!(config.hats.is_empty());
    }

    #[test]
    fn parse_ralph_yml_skills_section() {
        let yaml = r#"
core:
  guardrails: []
hats: {}
skills:
  enabled: true
  dirs:
    - team/coding-agent/skills
    - team/projects/myproject/coding-agent/skills
"#;
        let config: RalphConfig = serde_yml::from_str(yaml).unwrap();
        assert!(config.skills.enabled);
        assert_eq!(config.skills.dirs.len(), 2);
        assert_eq!(config.skills.dirs[0], "team/coding-agent/skills");
        assert_eq!(
            config.skills.dirs[1],
            "team/projects/myproject/coding-agent/skills"
        );
    }

    #[test]
    fn parse_ralph_yml_skills_defaults() {
        let yaml = r#"
core:
  guardrails: []
hats: {}
"#;
        let config: RalphConfig = serde_yml::from_str(yaml).unwrap();
        assert!(!config.skills.enabled);
        assert!(config.skills.dirs.is_empty());
    }
}
