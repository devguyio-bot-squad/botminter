use std::collections::BTreeSet;
use std::io::{self, BufRead, Write};

use anyhow::{Context, Result};

use crate::profile;

/// Action taken for each profile during init.
#[derive(Debug, PartialEq)]
enum Action {
    New,
    Overwritten,
    Skipped,
}

/// Handles `bm profiles init [--force]` — extracts all embedded profiles and
/// Minty config to disk.
///
/// Profile target: `~/.config/botminter/profiles/` (via `dirs::config_dir()`).
/// Minty target: `~/.config/botminter/minty/` (via `dirs::config_dir()`).
/// Files are written verbatim — no agent tag filtering is applied.
///
/// Behavior:
/// - Fresh install (no existing profiles): extracts all without prompting.
/// - Existing profiles + `--force`: overwrites all without prompting.
/// - Existing profiles (no `--force`): prompts per-profile to overwrite or skip.
///   New profiles (not yet on disk) are extracted without prompting.
/// - Minty config is always extracted (overwritten on every init).
pub fn run(force: bool) -> Result<()> {
    run_with_reader(force, &mut io::stdin().lock())
}

/// Testable inner function that accepts a reader for stdin.
fn run_with_reader(force: bool, reader: &mut dyn BufRead) -> Result<()> {
    let target = profile::profiles_dir()?;
    let embedded = profile::embedded::list_embedded_profiles();

    // Discover which profiles already exist on disk
    let existing_on_disk: BTreeSet<String> = if target.exists() {
        std::fs::read_dir(&target)
            .context("Failed to read profiles directory")?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    } else {
        BTreeSet::new()
    };

    std::fs::create_dir_all(&target).with_context(|| {
        format!(
            "Failed to create profiles directory {}",
            target.display()
        )
    })?;

    // If nothing exists on disk, extract everything (fresh install)
    if existing_on_disk.is_empty() {
        let count = profile::extract_embedded_to_disk(&target)?;
        println!("Extracted {} profiles to {}", count, target.display());
        extract_minty(&target)?;
        return Ok(());
    }

    // Profiles exist — decide per-profile what to do
    let mut actions: Vec<(String, Action)> = Vec::new();

    for name in &embedded {
        if !existing_on_disk.contains(name) {
            // New profile — extract without prompting
            profile::extract_single_profile_to_disk(name, &target)?;
            actions.push((name.clone(), Action::New));
        } else if force {
            // Existing + --force — overwrite silently
            profile::extract_single_profile_to_disk(name, &target)?;
            actions.push((name.clone(), Action::Overwritten));
        } else {
            // Existing, no --force — prompt
            let overwrite = prompt_overwrite(name, reader)?;
            if overwrite {
                profile::extract_single_profile_to_disk(name, &target)?;
                actions.push((name.clone(), Action::Overwritten));
            } else {
                actions.push((name.clone(), Action::Skipped));
            }
        }
    }

    // Print summary
    print_summary(&actions, &target);

    // Always extract/update Minty config alongside profiles
    extract_minty(&target)?;

    Ok(())
}

/// Prompts the user whether to overwrite a profile. Returns `true` for yes.
/// Defaults to No (skip) on empty input or non-TTY.
fn prompt_overwrite(profile_name: &str, reader: &mut dyn BufRead) -> Result<bool> {
    print!("Overwrite {}? [y/N] ", profile_name);
    io::stdout().flush()?;

    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line)?;

    // EOF or empty input → default to No
    if bytes_read == 0 {
        println!();
        return Ok(false);
    }

    let answer = line.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

/// Extracts Minty config to `~/.config/botminter/minty/` (sibling of profiles dir).
/// The `profiles_target` is used to derive the parent `~/.config/botminter/` directory.
fn extract_minty(profiles_target: &std::path::Path) -> Result<()> {
    let botminter_config = profiles_target
        .parent()
        .context("Could not determine botminter config parent directory")?;
    let minty_target = botminter_config.join("minty");
    std::fs::create_dir_all(&minty_target).with_context(|| {
        format!(
            "Failed to create minty directory {}",
            minty_target.display()
        )
    })?;
    profile::minty_embedded::extract_minty_to_disk(&minty_target)?;
    println!("Extracted minty config to {}", minty_target.display());
    Ok(())
}

/// Prints a summary of actions taken during init.
fn print_summary(actions: &[(String, Action)], target: &std::path::Path) {
    let new_count = actions.iter().filter(|(_, a)| *a == Action::New).count();
    let overwritten_count = actions
        .iter()
        .filter(|(_, a)| *a == Action::Overwritten)
        .count();
    let skipped_count = actions
        .iter()
        .filter(|(_, a)| *a == Action::Skipped)
        .count();

    println!();
    println!("Profiles directory: {}", target.display());
    for (name, action) in actions {
        let label = match action {
            Action::New => "new",
            Action::Overwritten => "overwritten",
            Action::Skipped => "skipped",
        };
        println!("  {} ({})", name, label);
    }
    println!();
    println!(
        "Summary: {} new, {} overwritten, {} skipped",
        new_count, overwritten_count, skipped_count
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_existing_profiles(target: &std::path::Path) {
        // Extract all profiles first to simulate a previous init
        profile::extract_embedded_to_disk(target).unwrap();
    }

    #[test]
    fn fresh_install_extracts_all_without_prompting() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("botminter").join("profiles");

        // Override profiles_dir would be complex, so we test the inner logic directly
        // by calling extract_embedded_to_disk on a fresh dir
        std::fs::create_dir_all(&target).unwrap();
        let count = profile::extract_embedded_to_disk(&target).unwrap();
        assert!(count >= 2, "Should extract at least 2 profiles");

        for name in profile::embedded::list_embedded_profiles() {
            assert!(
                target.join(&name).join("botminter.yml").exists(),
                "Profile '{}' should be extracted",
                name
            );
        }
    }

    #[test]
    fn force_overwrites_all_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("profiles");
        std::fs::create_dir_all(&target).unwrap();
        setup_existing_profiles(&target);

        // Pick the first embedded profile as the sample to modify
        let sample_profile = &profile::embedded::list_embedded_profiles()[0];
        let marker = target.join(sample_profile).join("botminter.yml");
        std::fs::write(&marker, "modified").unwrap();

        // Extract each profile with force
        for name in profile::embedded::list_embedded_profiles() {
            profile::extract_single_profile_to_disk(&name, &target).unwrap();
        }

        // File should be restored (not "modified")
        let content = std::fs::read_to_string(&marker).unwrap();
        assert_ne!(content, "modified", "Force should overwrite modified file");
        assert!(
            content.contains("name:"),
            "Should contain valid YAML manifest"
        );
    }

    #[test]
    fn prompt_overwrite_yes_returns_true() {
        let mut input = io::Cursor::new(b"y\n");
        let result = prompt_overwrite("scrum", &mut input).unwrap();
        assert!(result, "'y' should return true");
    }

    #[test]
    fn prompt_overwrite_yes_full_returns_true() {
        let mut input = io::Cursor::new(b"yes\n");
        let result = prompt_overwrite("scrum", &mut input).unwrap();
        assert!(result, "'yes' should return true");
    }

    #[test]
    fn prompt_overwrite_no_returns_false() {
        let mut input = io::Cursor::new(b"n\n");
        let result = prompt_overwrite("scrum", &mut input).unwrap();
        assert!(!result, "'n' should return false");
    }

    #[test]
    fn prompt_overwrite_empty_returns_false() {
        let mut input = io::Cursor::new(b"\n");
        let result = prompt_overwrite("scrum", &mut input).unwrap();
        assert!(!result, "Empty input should default to No");
    }

    #[test]
    fn prompt_overwrite_eof_returns_false() {
        let mut input = io::Cursor::new(b"");
        let result = prompt_overwrite("scrum", &mut input).unwrap();
        assert!(!result, "EOF should default to No");
    }

    #[test]
    fn prompt_overwrite_case_insensitive() {
        let mut input = io::Cursor::new(b"Y\n");
        let result = prompt_overwrite("scrum", &mut input).unwrap();
        assert!(result, "'Y' (uppercase) should return true");
    }

    #[test]
    fn run_with_reader_force_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        // Use XDG_CONFIG_HOME-style layout to match profiles_dir()
        // But since we can't override profiles_dir() here, test via the
        // per-profile extraction functions directly
        let target = tmp.path().join("profiles");
        std::fs::create_dir_all(&target).unwrap();
        setup_existing_profiles(&target);

        // Pick the first embedded profile as the sample to modify
        let sample_profile = &profile::embedded::list_embedded_profiles()[0];
        let marker = target.join(sample_profile).join("botminter.yml");
        std::fs::write(&marker, "modified-by-test").unwrap();

        // Force-extract all
        for name in profile::embedded::list_embedded_profiles() {
            profile::extract_single_profile_to_disk(&name, &target).unwrap();
        }

        let content = std::fs::read_to_string(&marker).unwrap();
        assert!(
            content.contains("name:"),
            "Force should restore original content"
        );
    }

    #[test]
    fn skipped_profile_preserves_disk_content() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("profiles");
        std::fs::create_dir_all(&target).unwrap();
        setup_existing_profiles(&target);

        // Pick the first embedded profile as the sample to skip
        let skipped_profile = &profile::embedded::list_embedded_profiles()[0];
        let marker = target.join(skipped_profile).join("botminter.yml");
        std::fs::write(&marker, "custom-content").unwrap();

        // Don't extract the skipped profile (simulating "skip")
        // Only extract other profiles
        for name in profile::embedded::list_embedded_profiles() {
            if &name != skipped_profile {
                profile::extract_single_profile_to_disk(&name, &target).unwrap();
            }
        }

        let content = std::fs::read_to_string(&marker).unwrap();
        assert_eq!(
            content, "custom-content",
            "Skipped profile should preserve custom content"
        );
    }

    #[test]
    fn print_summary_shows_all_action_types() {
        let actions = vec![
            ("alpha".to_string(), Action::New),
            ("beta".to_string(), Action::Overwritten),
            ("gamma".to_string(), Action::Skipped),
        ];
        let target = std::path::PathBuf::from("/tmp/test-profiles");

        // Capture output by just verifying no panic — the function prints to stdout
        print_summary(&actions, &target);
    }

    #[test]
    fn extract_single_profile_creates_correct_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles = profile::embedded::list_embedded_profiles();
        let name = &profiles[0];

        profile::extract_single_profile_to_disk(name, tmp.path()).unwrap();

        assert!(
            tmp.path().join(name).is_dir(),
            "Should create profile directory"
        );
        assert!(
            tmp.path().join(name).join("botminter.yml").exists(),
            "Should contain botminter.yml"
        );
    }

    #[test]
    fn extract_single_profile_nonexistent_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let result = profile::extract_single_profile_to_disk("nonexistent", tmp.path());
        assert!(result.is_err(), "Should error for nonexistent profile");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "Error should mention not found: {}",
            err
        );
    }

    #[test]
    fn extract_minty_creates_config_files() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_target = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_target).unwrap();

        extract_minty(&profiles_target).unwrap();

        let minty_dir = tmp.path().join("botminter").join("minty");
        assert!(
            minty_dir.join("prompt.md").exists(),
            "Minty prompt.md should be extracted"
        );
        assert!(
            minty_dir.join("config.yml").exists(),
            "Minty config.yml should be extracted"
        );
        assert!(
            minty_dir.join(".claude").join("skills").is_dir(),
            "Minty .claude/skills/ directory should be extracted"
        );
    }

    #[test]
    fn extract_minty_skills_creates_all_skill_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_target = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_target).unwrap();

        extract_minty(&profiles_target).unwrap();

        let skills_dir = tmp
            .path()
            .join("botminter")
            .join("minty")
            .join(".claude")
            .join("skills");
        let expected_skills = [
            "team-overview",
            "profile-browser",
            "hire-guide",
            "workspace-doctor",
        ];
        for skill in &expected_skills {
            let skill_file = skills_dir.join(skill).join("SKILL.md");
            assert!(
                skill_file.exists(),
                "Minty skill {}/SKILL.md should be extracted",
                skill
            );
        }
    }

    #[test]
    fn extract_minty_skills_have_valid_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_target = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_target).unwrap();

        extract_minty(&profiles_target).unwrap();

        let skills_dir = tmp
            .path()
            .join("botminter")
            .join("minty")
            .join(".claude")
            .join("skills");
        let expected_skills = [
            "team-overview",
            "profile-browser",
            "hire-guide",
            "workspace-doctor",
        ];
        for skill in &expected_skills {
            let content =
                std::fs::read_to_string(skills_dir.join(skill).join("SKILL.md")).unwrap();
            assert!(
                content.starts_with("---\n"),
                "Skill {} should start with YAML frontmatter delimiter",
                skill
            );
            // Verify frontmatter contains required fields
            let frontmatter_end = content[4..]
                .find("\n---\n")
                .expect(&format!("Skill {} should have closing frontmatter delimiter", skill));
            let frontmatter = &content[4..4 + frontmatter_end];
            assert!(
                frontmatter.contains("name:"),
                "Skill {} frontmatter should contain 'name'",
                skill
            );
            assert!(
                frontmatter.contains("description:"),
                "Skill {} frontmatter should contain 'description'",
                skill
            );
            assert!(
                frontmatter.contains("version:"),
                "Skill {} frontmatter should contain 'version'",
                skill
            );
        }
    }

    #[test]
    fn extract_minty_prompt_contains_persona() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_target = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_target).unwrap();

        extract_minty(&profiles_target).unwrap();

        let prompt = std::fs::read_to_string(
            tmp.path().join("botminter").join("minty").join("prompt.md"),
        )
        .unwrap();
        assert!(
            prompt.contains("Minty"),
            "Prompt should contain Minty persona name"
        );
        assert!(
            prompt.contains("BotMinter"),
            "Prompt should reference BotMinter"
        );
        assert!(
            prompt.contains("profiles-only mode"),
            "Prompt should handle missing runtime data"
        );
    }

    #[test]
    fn extract_minty_config_is_valid_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_target = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_target).unwrap();

        extract_minty(&profiles_target).unwrap();

        let config_content = std::fs::read_to_string(
            tmp.path().join("botminter").join("minty").join("config.yml"),
        )
        .unwrap();
        let yaml: serde_yml::Value = serde_yml::from_str(&config_content)
            .expect("config.yml should be valid YAML");
        assert!(
            yaml.get("prompt").is_some(),
            "Config should have a 'prompt' key"
        );
        assert!(
            yaml.get("skills_dir").is_some(),
            "Config should have a 'skills_dir' key"
        );
    }

    #[test]
    fn extract_minty_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_target = tmp.path().join("botminter").join("profiles");
        std::fs::create_dir_all(&profiles_target).unwrap();

        extract_minty(&profiles_target).unwrap();
        // Second extraction should succeed without error
        extract_minty(&profiles_target).unwrap();

        let minty_dir = tmp.path().join("botminter").join("minty");
        assert!(
            minty_dir.join("prompt.md").exists(),
            "Minty files should still exist after re-extraction"
        );
    }
}
