use std::collections::BTreeMap;
use std::fs;
use std::io::Write;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use std::path::Path;

use crate::chat::{build_meta_prompt, MetaPromptParams, SkillInfo};
use crate::config;
use crate::profile;

/// Handles `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt]`.
pub fn run(
    member: &str,
    team_flag: Option<&str>,
    hat: Option<&str>,
    render_system_prompt: bool,
) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Verify member exists in team repo
    let member_dir = team_repo.join("members").join(member);
    if !member_dir.is_dir() {
        bail!(
            "Member '{}' not found in team '{}'. \
             Run `bm members list` to see hired members.",
            member,
            team.name
        );
    }

    // Find workspace
    let ws_path = team.path.join(member);
    if !ws_path.join(".botminter.workspace").exists() {
        bail!(
            "No workspace found for member '{}'. \
             Run `bm teams sync` first.",
            member
        );
    }

    // Read ralph.yml from workspace root
    let ralph_yml_path = ws_path.join("ralph.yml");
    let ralph_contents = fs::read_to_string(&ralph_yml_path)
        .with_context(|| format!("Failed to read {}", ralph_yml_path.display()))?;
    let ralph_config: RalphConfig = serde_yml::from_str(&ralph_contents)
        .with_context(|| format!("Failed to parse {}", ralph_yml_path.display()))?;

    // Read PROMPT.md from workspace root
    let prompt_md_path = ws_path.join("PROMPT.md");
    let prompt_md_content = fs::read_to_string(&prompt_md_path)
        .with_context(|| format!("Failed to read {}", prompt_md_path.display()))?;

    // Read member role and display name from member manifest
    let (role_name, display_name) = read_member_info(&member_dir, member)?;

    // Extract hat instructions (only hats that have instructions)
    let hat_instructions: BTreeMap<String, String> = ralph_config
        .hats
        .into_iter()
        .filter_map(|(name, hat)| hat.instructions.map(|instr| (name, instr)))
        .collect();

    // Validate --hat flag against available hats
    if let Some(hat_name) = hat {
        if !hat_instructions.contains_key(hat_name) {
            if hat_instructions.is_empty() {
                bail!(
                    "Hat '{}' not found for member '{}'. \
                     No hats with instructions found in ralph.yml",
                    hat_name,
                    member
                );
            } else {
                let mut available: Vec<&str> =
                    hat_instructions.keys().map(|k| k.as_str()).collect();
                available.sort();
                bail!(
                    "Hat '{}' not found for member '{}'. Available hats: {}",
                    hat_name,
                    member,
                    available.join(", ")
                );
            }
        }
    }

    // Load profile manifest for role description lookup
    let manifest = {
        let manifest_path = team_repo.join("botminter.yml");
        let contents = fs::read_to_string(&manifest_path)
            .context("Failed to read team botminter.yml")?;
        serde_yml::from_str::<profile::ProfileManifest>(&contents)
            .context("Failed to parse team botminter.yml")?
    };

    // Look up role description from manifest
    let role_description = manifest
        .roles
        .iter()
        .find(|r| r.name == role_name)
        .map(|r| r.description.as_str())
        .unwrap_or("");

    // Scan for available skills
    let skills = if ralph_config.skills.enabled {
        scan_skills(&ws_path, &ralph_config.skills.dirs)
    } else {
        Vec::new()
    };

    // Reference dir path (relative to workspace root, via team/ submodule)
    let reference_dir = "team/ralph-prompts/reference/";

    // Build meta-prompt
    let params = MetaPromptParams {
        member_name: &display_name,
        role_name: &role_name,
        role_description,
        team_name: &team.name,
        guardrails: &ralph_config.core.guardrails,
        hat_instructions: &hat_instructions,
        prompt_md_content: &prompt_md_content,
        reference_dir,
        hat,
        skills: &skills,
    };
    let meta_prompt = build_meta_prompt(&params);

    if render_system_prompt {
        println!("{}", meta_prompt);
        return Ok(());
    }

    // Resolve coding agent for launch
    let coding_agent = profile::resolve_coding_agent(team, &manifest)?;

    // Write meta-prompt to temp file
    let mut tmp_file = tempfile::Builder::new()
        .prefix("bm-chat-")
        .suffix(".md")
        .tempfile()
        .context("Failed to create temp file for meta-prompt")?;
    tmp_file
        .write_all(meta_prompt.as_bytes())
        .context("Failed to write meta-prompt to temp file")?;
    // Keep the path but disown the file — exec() replaces the process,
    // so Drop never runs and the file persists for the coding agent.
    let tmp_path = tmp_file.into_temp_path();

    // Launch coding agent via exec (replaces this process)
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(&coding_agent.binary)
        .current_dir(&ws_path)
        .arg("--append-system-prompt-file")
        .arg(&tmp_path)
        .exec();

    // exec() only returns on error
    bail!("Failed to launch {}: {}", coding_agent.binary, err);
}

/// Reads the role and display name from a member's botminter.yml manifest.
///
/// Returns `(role, display_name)`. The display name is the human-friendly name
/// given at hire time (e.g., "testbot"), distinct from the directory slug
/// (e.g., "superman-testbot").
fn read_member_info(member_dir: &std::path::Path, member_name: &str) -> Result<(String, String)> {
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

/// Infers a role name from a member directory name (e.g., "architect-01" → "architect").
fn infer_role(member_name: &str) -> String {
    member_name
        .split('-')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

// ── YAML deserialization types for ralph.yml ──────────────────────────

/// Minimal deserialization of ralph.yml — only the fields needed for chat.
#[derive(Deserialize)]
struct RalphConfig {
    #[serde(default)]
    core: CoreConfig,
    #[serde(default)]
    hats: BTreeMap<String, HatConfig>,
    #[serde(default)]
    skills: SkillsConfig,
}

#[derive(Deserialize, Default)]
struct SkillsConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    dirs: Vec<String>,
}

#[derive(Deserialize, Default)]
struct CoreConfig {
    #[serde(default)]
    guardrails: Vec<String>,
}

#[derive(Deserialize)]
struct HatConfig {
    #[serde(default)]
    instructions: Option<String>,
}

/// Minimal member manifest — role and name fields.
#[derive(Deserialize)]
struct MemberManifest {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

/// Frontmatter fields extracted from SKILL.md files.
#[derive(Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
}

/// Scans skill directories for SKILL.md files and extracts skill metadata.
///
/// For each dir in `dirs`, resolves it relative to `ws_path`, lists subdirectories,
/// reads `SKILL.md` frontmatter, and collects skill entries. Skips template paths
/// containing `<project>` placeholders. Results are sorted by name and deduplicated.
fn scan_skills(ws_path: &Path, dirs: &[String]) -> Vec<SkillInfo> {
    let mut skills: Vec<SkillInfo> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for dir in dirs {
        // Skip template paths with <project> placeholder
        if dir.contains("<project>") {
            continue;
        }

        let skill_dir = ws_path.join(dir);
        let entries = match fs::read_dir(&skill_dir) {
            Ok(entries) => entries,
            Err(_) => continue, // Skip non-existent dirs silently
        };

        let mut subdirs: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();
        subdirs.sort_by_key(|e| e.file_name());

        for entry in subdirs {
            let skill_md_path = entry.path().join("SKILL.md");
            if !skill_md_path.exists() {
                continue;
            }

            let content = match fs::read_to_string(&skill_md_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Extract YAML frontmatter between --- markers
            if let Some(frontmatter) = extract_frontmatter(&content) {
                if let Ok(meta) = serde_yml::from_str::<SkillFrontmatter>(&frontmatter) {
                    if seen_names.contains(&meta.name) {
                        continue; // Deduplicate by name
                    }

                    let description = truncate_description(&meta.description);
                    let relative_path = format!(
                        "{}/{}/SKILL.md",
                        dir,
                        entry.file_name().to_string_lossy()
                    );

                    seen_names.insert(meta.name.clone());
                    skills.push(SkillInfo {
                        name: meta.name,
                        description,
                        load_command: relative_path,
                    });
                }
            }
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Extracts YAML frontmatter from between `---` markers.
fn extract_frontmatter(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    Some(after_first[..end].to_string())
}

/// Truncates a description to the first sentence or 120 chars, whichever is shorter.
fn truncate_description(desc: &str) -> String {
    // Normalize whitespace (multiline YAML descriptions may have newlines)
    let normalized: String = desc.split_whitespace().collect::<Vec<_>>().join(" ");

    // Find first sentence boundary
    let sentence_end = normalized
        .find(". ")
        .or_else(|| normalized.find(".\n"))
        .map(|pos| pos + 1); // Include the period

    let truncated = match sentence_end {
        Some(end) if end <= 120 => &normalized[..end],
        _ => {
            if normalized.len() <= 120 {
                &normalized
            } else {
                // Find last space before 120 chars to avoid cutting words
                let cut = normalized[..120]
                    .rfind(' ')
                    .unwrap_or(120);
                &normalized[..cut]
            }
        }
    };

    truncated.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_role_from_member_name() {
        assert_eq!(infer_role("architect-01"), "architect");
        assert_eq!(infer_role("team-manager-bob"), "team");
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

    #[test]
    fn scan_skills_from_filesystem() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("team/coding-agent/skills");

        // Create gh skill
        let gh_dir = skills_dir.join("gh");
        fs::create_dir_all(&gh_dir).unwrap();
        fs::write(
            gh_dir.join("SKILL.md"),
            "---\nname: gh\ndescription: Manages GitHub Projects v2 workflows and issue operations.\n---\n\n# GH Skill\n",
        )
        .unwrap();

        // Create status-workflow skill
        let sw_dir = skills_dir.join("status-workflow");
        fs::create_dir_all(&sw_dir).unwrap();
        fs::write(
            sw_dir.join("SKILL.md"),
            "---\nname: status-workflow\ndescription: Performs status transitions on issues.\n---\n\n# Status Workflow\n",
        )
        .unwrap();

        let dirs = vec!["team/coding-agent/skills".to_string()];
        let result = scan_skills(tmp.path(), &dirs);

        assert_eq!(result.len(), 2);
        // Sorted by name
        assert_eq!(result[0].name, "gh");
        assert!(result[0].description.starts_with("Manages GitHub"));
        assert_eq!(
            result[0].load_command,
            "team/coding-agent/skills/gh/SKILL.md"
        );
        assert_eq!(result[1].name, "status-workflow");
    }

    #[test]
    fn scan_skills_skips_project_placeholder() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = vec![
            "team/projects/<project>/coding-agent/skills".to_string(),
        ];
        let result = scan_skills(tmp.path(), &dirs);
        assert!(result.is_empty(), "Should skip dirs with <project> placeholder");
    }

    #[test]
    fn scan_skills_deduplicates_by_name() {
        let tmp = tempfile::tempdir().unwrap();

        // Create same skill in two dirs
        for dir_name in &["team/skills", "member/skills"] {
            let skill_dir = tmp.path().join(dir_name).join("gh");
            fs::create_dir_all(&skill_dir).unwrap();
            fs::write(
                skill_dir.join("SKILL.md"),
                "---\nname: gh\ndescription: GitHub skill\n---\n",
            )
            .unwrap();
        }

        let dirs = vec![
            "team/skills".to_string(),
            "member/skills".to_string(),
        ];
        let result = scan_skills(tmp.path(), &dirs);
        assert_eq!(result.len(), 1, "Should deduplicate skills with same name");
        assert_eq!(result[0].name, "gh");
    }

    #[test]
    fn scan_skills_skips_missing_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = vec!["nonexistent/skills".to_string()];
        let result = scan_skills(tmp.path(), &dirs);
        assert!(result.is_empty());
    }

    #[test]
    fn scan_skills_skips_dirs_without_skill_md() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("skills/empty-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        // No SKILL.md file

        let dirs = vec!["skills".to_string()];
        let result = scan_skills(tmp.path(), &dirs);
        assert!(result.is_empty());
    }

    #[test]
    fn truncate_description_first_sentence() {
        assert_eq!(
            truncate_description("Manages GitHub workflows. Also does other things."),
            "Manages GitHub workflows."
        );
    }

    #[test]
    fn truncate_description_short_no_truncation() {
        assert_eq!(
            truncate_description("Short description"),
            "Short description"
        );
    }

    #[test]
    fn truncate_description_long_text() {
        let long = "A".repeat(200);
        let result = truncate_description(&long);
        assert!(result.len() <= 120);
    }

    #[test]
    fn extract_frontmatter_basic() {
        let content = "---\nname: gh\ndescription: test\n---\n\n# Content\n";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("name: gh"));
        assert!(fm.contains("description: test"));
    }

    #[test]
    fn extract_frontmatter_no_markers() {
        assert!(extract_frontmatter("No frontmatter here").is_none());
    }

    #[test]
    fn truncate_description_multiline_yaml() {
        let desc = "Manages GitHub\nProjects v2 workflows. Also does other things.";
        let result = truncate_description(desc);
        assert_eq!(result, "Manages GitHub Projects v2 workflows.");
    }
}
