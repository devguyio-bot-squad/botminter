use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use super::CodingAgentDef;
use crate::git::run_git;

/// Result of hiring a new member into a team.
#[derive(Debug)]
pub struct HireResult {
    /// The full member directory name (e.g., "architect-01").
    pub member_dir_name: String,
    /// The member's display name (e.g., "01").
    pub member_name: String,
}

/// Hires a new member into a team by extracting the role skeleton, finalizing
/// the manifest, and committing to the team repo.
///
/// Does NOT handle bridge token prompting — that's interactive I/O for the
/// command layer.
pub fn hire_member(
    team_repo: &Path,
    team_profile: &str,
    role: &str,
    name: Option<&str>,
    coding_agent: &CodingAgentDef,
) -> Result<HireResult> {
    // Verify role exists in profile
    let available_roles = super::list_roles(team_profile)?;
    if !available_roles.contains(&role.to_string()) {
        bail!(
            "Role '{}' not available in profile '{}'. Available roles: {}",
            role,
            team_profile,
            available_roles.join(", ")
        );
    }

    // Determine member name
    let member_name = match name {
        Some(n) => n.to_string(),
        None => auto_suffix(team_repo, role)?,
    };

    let member_dir_name = format!("{}-{}", role, member_name);
    let member_dir = team_repo.join("members").join(&member_dir_name);

    if member_dir.exists() {
        bail!(
            "Member directory '{}' already exists. Choose a different name.",
            member_dir_name
        );
    }

    // Extract member skeleton from embedded profile
    fs::create_dir_all(&member_dir)
        .with_context(|| format!("Failed to create member dir {}", member_dir.display()))?;

    super::extract_member_to(team_profile, role, &member_dir, coding_agent)?;

    // Finalize member manifest: .botminter.yml → botminter.yml with name added
    finalize_member_manifest(&member_dir, &member_name)?;

    // Git add + commit (no auto-push)
    run_git(
        team_repo,
        &["add", &format!("members/{}/", member_dir_name)],
    )?;
    let commit_msg = format!("feat: hire {} as {}", role, member_name);
    run_git(team_repo, &["commit", "-m", &commit_msg])?;

    Ok(HireResult {
        member_dir_name,
        member_name,
    })
}

/// Computes the next auto-suffix for a role by scanning existing member dirs.
/// Returns a 2-digit, zero-padded string (e.g., "01", "02").
/// Fills gaps: if 01 and 03 exist, returns "02".
pub fn auto_suffix(team_repo: &Path, role: &str) -> Result<String> {
    let team_members_dir = team_repo.join("members");
    let prefix = format!("{}-", role);

    let mut used: Vec<u32> = Vec::new();

    if team_members_dir.is_dir() {
        for entry in fs::read_dir(&team_members_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                let suffix = &name[prefix.len()..];
                if let Ok(n) = suffix.parse::<u32>() {
                    used.push(n);
                }
            }
        }
    }

    used.sort();

    let mut next = 1u32;
    for &n in &used {
        if n == next {
            next = n + 1;
        } else if n > next {
            break;
        }
    }

    Ok(format!("{:02}", next))
}

/// Reads the .botminter.yml template, augments with the member name, and
/// writes as botminter.yml (without the dot prefix).
pub fn finalize_member_manifest(member_dir: &Path, name: &str) -> Result<()> {
    let template_path = member_dir.join(".botminter.yml");
    if template_path.exists() {
        let contents = fs::read_to_string(&template_path)
            .context("Failed to read .botminter.yml template")?;

        let mut value: serde_yml::Value =
            serde_yml::from_str(&contents).context("Failed to parse .botminter.yml")?;

        if let serde_yml::Value::Mapping(ref mut map) = value {
            map.insert(
                serde_yml::Value::String("name".to_string()),
                serde_yml::Value::String(name.to_string()),
            );
        }

        let augmented =
            serde_yml::to_string(&value).context("Failed to serialize member manifest")?;

        let manifest_path = member_dir.join("botminter.yml");
        fs::write(&manifest_path, augmented)
            .context("Failed to write member botminter.yml")?;

        fs::remove_file(&template_path).ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_suffix_first_member() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members")).unwrap();

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "01");
    }

    #[test]
    fn auto_suffix_increments() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "02");
    }

    #[test]
    fn auto_suffix_fills_gaps() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();
        fs::create_dir_all(team_repo.join("members/architect-03")).unwrap();

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "02");
    }

    #[test]
    fn auto_suffix_skips_non_numeric() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-bob")).unwrap();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "02");
    }

    #[test]
    fn auto_suffix_different_roles_independent() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = tmp.path();
        fs::create_dir_all(team_repo.join("members/architect-01")).unwrap();
        fs::create_dir_all(team_repo.join("members/architect-02")).unwrap();
        fs::create_dir_all(team_repo.join("members/dev-01")).unwrap();

        let result = auto_suffix(team_repo, "dev").unwrap();
        assert_eq!(result, "02");

        let result = auto_suffix(team_repo, "architect").unwrap();
        assert_eq!(result, "03");
    }
}
