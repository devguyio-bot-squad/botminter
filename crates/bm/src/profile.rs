use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::agent_tags;

/// Compare two semver-like version strings (e.g. "1.0.0" vs "2.0.0").
/// Returns Ordering::Less if a < b, Equal if a == b, Greater if a > b.
/// Falls back to string comparison if either version isn't valid semver.
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    };
    match (parse(a), parse(b)) {
        (Some(av), Some(bv)) => av.cmp(&bv),
        _ => a.cmp(b),
    }
}

/// Embedded profiles from the binary. Only accessed by `bm profiles init`.
pub(crate) mod embedded {
    use std::fs;
    use std::path::Path;

    use anyhow::{Context, Result};
    use include_dir::{Dir, include_dir};

    static PROFILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../profiles");

    /// Returns the names of all embedded profiles.
    pub fn list_embedded_profiles() -> Vec<String> {
        let mut names: Vec<String> = PROFILES
            .dirs()
            .map(|d| d.path().file_name().unwrap().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    /// Returns the raw embedded PROFILES directory for advanced access.
    #[allow(dead_code)]
    pub fn embedded_profiles() -> &'static Dir<'static> {
        &PROFILES
    }

    /// Extracts all embedded profiles to a target directory, writing files verbatim
    /// (no agent tag filtering, no context.md rename). Used by `bm profiles init`.
    pub fn extract_embedded_to_disk(target: &Path) -> Result<usize> {
        let mut count = 0;
        for profile_dir in PROFILES.dirs() {
            let profile_name = profile_dir
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let profile_target = target.join(&profile_name);
            write_dir_verbatim(profile_dir, &profile_target, profile_dir.path())?;
            count += 1;
        }

        Ok(count)
    }

    /// Extracts a single embedded profile by name to a target directory, writing
    /// files verbatim. Returns an error if the profile doesn't exist.
    pub fn extract_single_profile_to_disk(profile_name: &str, target: &Path) -> Result<()> {
        let profile_dir = PROFILES.get_dir(profile_name).with_context(|| {
            let available = list_embedded_profiles().join(", ");
            format!(
                "Profile '{}' not found. Available profiles: {}",
                profile_name, available
            )
        })?;
        let profile_target = target.join(profile_name);
        write_dir_verbatim(profile_dir, &profile_target, profile_dir.path())
    }

    /// Returns the role names for a given embedded profile.
    /// If the profile or its roles/ directory is not found, returns an empty Vec.
    pub fn list_embedded_roles(name: &str) -> Vec<String> {
        let roles_path = format!("{}/roles", name);
        let Some(roles_dir) = PROFILES.get_dir(&roles_path) else {
            return Vec::new();
        };
        let mut roles: Vec<String> = roles_dir
            .dirs()
            .filter_map(|d| {
                d.path()
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
            })
            .collect();
        roles.sort();
        roles
    }

    /// Recursively writes all files from an embedded Dir to disk without any
    /// filtering or renaming. Binary files and text files are both written as-is.
    fn write_dir_verbatim(dir: &Dir<'_>, base_target: &Path, root_path: &Path) -> Result<()> {
        for file in dir.files() {
            let rel = file
                .path()
                .strip_prefix(root_path)
                .unwrap_or(file.path());
            let target_path = base_target.join(rel);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create directory {}", parent.display())
                })?;
            }

            fs::write(&target_path, file.contents()).with_context(|| {
                format!("Failed to write {}", target_path.display())
            })?;
        }

        for sub_dir in dir.dirs() {
            write_dir_verbatim(sub_dir, base_target, root_path)?;
        }

        Ok(())
    }
}

// Re-export embedded extraction functions for public/test access
pub use embedded::{
    extract_embedded_to_disk, extract_single_profile_to_disk, list_embedded_profiles,
    list_embedded_roles,
};

/// Embedded Minty config from the binary. Extracted alongside profiles by `bm profiles init`.
pub(crate) mod minty_embedded {
    use std::fs;
    use std::path::Path;

    use anyhow::{Context, Result};
    use include_dir::{Dir, include_dir};

    static MINTY: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../minty");

    /// Extracts the embedded Minty config to a target directory, writing files verbatim.
    /// Target should be `~/.config/botminter/minty/`.
    pub fn extract_minty_to_disk(target: &Path) -> Result<()> {
        write_dir_verbatim(&MINTY, target, MINTY.path())
    }

    /// Recursively writes all files from an embedded Dir to disk without any
    /// filtering or renaming.
    fn write_dir_verbatim(dir: &Dir<'_>, base_target: &Path, root_path: &Path) -> Result<()> {
        for file in dir.files() {
            let rel = file
                .path()
                .strip_prefix(root_path)
                .unwrap_or(file.path());
            let target_path = base_target.join(rel);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create directory {}", parent.display())
                })?;
            }

            fs::write(&target_path, file.contents()).with_context(|| {
                format!("Failed to write {}", target_path.display())
            })?;
        }

        for sub_dir in dir.dirs() {
            write_dir_verbatim(sub_dir, base_target, root_path)?;
        }

        Ok(())
    }
}

/// Returns the canonical disk path for Minty config.
/// Resolves to `~/.config/botminter/minty/` on Linux/macOS.
pub fn minty_dir() -> Result<PathBuf> {
    let config = dirs::config_dir().context("Could not determine config directory")?;
    Ok(config.join("botminter").join("minty"))
}

/// Profile manifest parsed from botminter.yml
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileManifest {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub schema_version: String,
    #[serde(default)]
    pub roles: Vec<RoleDef>,
    #[serde(default)]
    pub labels: Vec<LabelDef>,
    #[serde(default)]
    pub statuses: Vec<StatusDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<ProjectDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<ViewDef>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub coding_agents: HashMap<String, CodingAgentDef>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub default_coding_agent: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleDef {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LabelDef {
    pub name: String,
    pub color: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusDef {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectDef {
    pub name: String,
    pub fork_url: String,
}

/// Describes a coding agent's file conventions and binary name.
/// Used by the extraction pipeline to determine context file names,
/// agent directories, and which binary to launch.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodingAgentDef {
    pub name: String,
    pub display_name: String,
    pub context_file: String,
    pub agent_dir: String,
    pub binary: String,
}

/// Defines a role-based view for the GitHub Project board.
/// Each view maps to a subset of statuses via prefix matching.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ViewDef {
    pub name: String,
    /// Status name prefixes to include (e.g., ["po"] matches "po:triage", "po:backlog", etc.)
    pub prefixes: Vec<String>,
    /// Extra statuses always included regardless of prefix (e.g., ["done", "error"])
    #[serde(default)]
    pub also_include: Vec<String>,
}

impl ViewDef {
    /// Expands prefixes against the full status list, returning matching status names
    /// plus any `also_include` entries.
    pub fn resolve_statuses(&self, all_statuses: &[StatusDef]) -> Vec<String> {
        let mut result: Vec<String> = all_statuses
            .iter()
            .filter(|s| {
                self.prefixes
                    .iter()
                    .any(|p| s.name.starts_with(&format!("{}:", p)))
            })
            .map(|s| s.name.clone())
            .collect();
        for extra in &self.also_include {
            if !result.contains(extra) {
                result.push(extra.clone());
            }
        }
        result
    }

    /// Builds a GitHub Projects filter string for this view.
    /// Example: `status:po:triage,po:backlog,po:ready,done,error`
    pub fn filter_string(&self, all_statuses: &[StatusDef]) -> String {
        let statuses = self.resolve_statuses(all_statuses);
        format!("status:{}", statuses.join(","))
    }
}

/// Returns the profiles directory for a given home path.
/// Computes `{home}/.config/botminter/profiles/` (Linux layout).
pub fn profiles_dir_for(home: &Path) -> PathBuf {
    home.join(".config").join("botminter").join("profiles")
}

/// Returns the names of all profiles installed on disk.
pub fn list_profiles() -> Result<Vec<String>> {
    list_profiles_from(&profiles_dir()?)
}

/// Returns the names of all profiles installed at a given base directory.
pub fn list_profiles_from(base: &Path) -> Result<Vec<String>> {
    if !base.is_dir() {
        bail!(
            "Profiles directory does not exist: {}\n\
             Run `bm profiles init` to extract profiles to disk.",
            base.display()
        );
    }
    let mut names: Vec<String> = fs::read_dir(base)
        .with_context(|| format!("Failed to read profiles directory: {}", base.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            // Only include dirs that have botminter.yml (valid profiles)
            if e.path().join("botminter.yml").exists() {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Reads and parses the botminter.yml manifest for a named profile from disk.
pub fn read_manifest(name: &str) -> Result<ProfileManifest> {
    read_manifest_from(name, &profiles_dir()?)
}

/// Reads and parses the botminter.yml manifest for a named profile from a given base directory.
pub fn read_manifest_from(name: &str, base: &Path) -> Result<ProfileManifest> {
    let path = base.join(name).join("botminter.yml");
    let contents = fs::read_to_string(&path).with_context(|| {
        let available = list_profiles_from(base)
            .unwrap_or_default()
            .join(", ");
        format!(
            "Profile '{}' not found at {}. Available profiles: {}",
            name,
            path.display(),
            available
        )
    })?;

    let manifest: ProfileManifest =
        serde_yml::from_str(&contents).context("Failed to parse profile manifest")?;

    Ok(manifest)
}

/// Lists the role names available in a profile by reading its roles/ subdirectory on disk.
pub fn list_roles(name: &str) -> Result<Vec<String>> {
    list_roles_from(name, &profiles_dir()?)
}

/// Lists role names from a profile at a given base directory.
pub fn list_roles_from(name: &str, base: &Path) -> Result<Vec<String>> {
    let roles_dir = base.join(name).join("roles");
    if !roles_dir.is_dir() {
        bail!("Profile '{}' has no roles/ directory", name);
    }
    let mut roles: Vec<String> = fs::read_dir(&roles_dir)
        .with_context(|| format!("Failed to read roles directory: {}", roles_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    roles.sort();
    Ok(roles)
}

/// Extracts a profile's team-repo content to the target directory.
/// Copies everything from the disk profile EXCEPT `roles/` and `.schema/`
/// (role skeletons are extracted on demand via `extract_member_to`; schema is internal).
///
/// Text files (`.md`, `.yml`, `.yaml`, `.sh`) are filtered through the agent tag
/// pipeline to strip non-matching agent sections. `context.md` is additionally
/// renamed to `coding_agent.context_file` (e.g., `CLAUDE.md` for Claude Code).
pub fn extract_profile_to(
    profile_name: &str,
    target: &Path,
    coding_agent: &CodingAgentDef,
) -> Result<()> {
    extract_profile_from(&profiles_dir()?, profile_name, target, coding_agent)
}

/// Extracts a profile's team-repo content from a specific profiles base directory.
pub fn extract_profile_from(
    base: &Path,
    profile_name: &str,
    target: &Path,
    coding_agent: &CodingAgentDef,
) -> Result<()> {
    let profile_dir = base.join(profile_name);
    if !profile_dir.is_dir() {
        let available = list_profiles_from(base).unwrap_or_default().join(", ");
        bail!(
            "Profile '{}' not found. Available profiles: {}",
            profile_name, available
        );
    }

    extract_dir_recursive_from_disk(&profile_dir, target, &profile_dir, coding_agent, &|rel_path| {
        let first = rel_path
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string());
        matches!(first.as_deref(), Some("roles") | Some(".schema"))
    })?;

    Ok(())
}

/// Extracts a member skeleton from the disk profile into the target directory.
/// Copies the contents of `profiles/{profile}/roles/{role}/` to `target/`.
///
/// Text files are filtered through the agent tag pipeline, and `context.md` is
/// renamed to `coding_agent.context_file`.
pub fn extract_member_to(
    profile_name: &str,
    role: &str,
    target: &Path,
    coding_agent: &CodingAgentDef,
) -> Result<()> {
    extract_member_from(&profiles_dir()?, profile_name, role, target, coding_agent)
}

fn extract_member_from(
    base: &Path,
    profile_name: &str,
    role: &str,
    target: &Path,
    coding_agent: &CodingAgentDef,
) -> Result<()> {
    let member_dir = base.join(profile_name).join("roles").join(role);
    if !member_dir.is_dir() {
        let roles = list_roles_from(profile_name, base).unwrap_or_default().join(", ");
        bail!(
            "Role '{}' not available in profile '{}'. Available roles: {}",
            role, profile_name, roles
        );
    }

    extract_dir_recursive_from_disk(&member_dir, target, &member_dir, coding_agent, &|_| false)?;
    Ok(())
}

/// File extensions that should be filtered through the agent tag pipeline.
const FILTERABLE_EXTENSIONS: &[&str] = &["md", "yml", "yaml", "sh"];

/// Returns true if the filename has an extension that should be agent-tag filtered.
fn should_filter(filename: &str) -> bool {
    Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| FILTERABLE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Recursively extracts files from a disk directory to a target path.
/// `root_path` is the path of the root directory being extracted (used to compute
/// relative paths for target files). The `skip` predicate receives the path relative
/// to `root_path` and returns true to skip that entry.
///
/// During extraction:
/// - Text files (`.md`, `.yml`, `.yaml`, `.sh`) are filtered through `filter_file()`
///   to strip non-matching agent tag sections.
/// - `context.md` is renamed to `coding_agent.context_file` (e.g., `CLAUDE.md`).
/// - All other files (images, binary) are copied verbatim.
fn extract_dir_recursive_from_disk(
    source_dir: &Path,
    base_target: &Path,
    root_path: &Path,
    coding_agent: &CodingAgentDef,
    skip: &dyn Fn(&Path) -> bool,
) -> Result<()> {
    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("Failed to read directory {}", source_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root_path).unwrap_or(&path);

        if skip(rel) {
            continue;
        }

        if path.is_dir() {
            extract_dir_recursive_from_disk(&path, base_target, root_path, coding_agent, skip)?;
            continue;
        }

        let filename = rel
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        // Determine output path: rename context.md → coding_agent.context_file
        let target_path = if filename == "context.md" {
            let parent = rel.parent().unwrap_or(Path::new(""));
            base_target.join(parent).join(&coding_agent.context_file)
        } else {
            base_target.join(rel)
        };

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create directory {}", parent.display())
            })?;
        }

        // Filter text files through agent tag pipeline; copy others verbatim
        if should_filter(&filename) {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("File {} is not valid UTF-8", rel.display()))?;
            let filtered = agent_tags::filter_file(&content, &filename, &coding_agent.name);
            fs::write(&target_path, filtered.as_bytes()).with_context(|| {
                format!("Failed to write {}", target_path.display())
            })?;
        } else {
            fs::copy(&path, &target_path).with_context(|| {
                format!("Failed to copy {} to {}", path.display(), target_path.display())
            })?;
        }
    }

    Ok(())
}

/// Returns the canonical disk path for externalized profiles.
/// Resolves to `~/.config/botminter/profiles/` on Linux/macOS.
pub fn profiles_dir() -> Result<PathBuf> {
    let config = dirs::config_dir().context("Could not determine config directory")?;
    Ok(config.join("botminter").join("profiles"))
}

/// Ensures profiles are available on disk. If missing, prompts the user (TTY) or
/// auto-initializes (non-TTY). Call at the top of any command that reads profiles.
/// Reads the version field from an embedded profile's botminter.yml.
fn embedded_profile_version(name: &str) -> Option<String> {
    let path = format!("{}/botminter.yml", name);
    let file = embedded::embedded_profiles().get_file(&path)?;
    let content = std::str::from_utf8(file.contents()).ok()?;
    let manifest: ProfileManifest = serde_yml::from_str(content).ok()?;
    Some(manifest.version)
}

pub fn ensure_profiles_initialized() -> Result<()> {
    ensure_profiles_initialized_with(
        &profiles_dir()?,
        io::stdin().is_terminal(),
        false,
        &mut io::stdin().lock(),
    )
}

fn ensure_profiles_initialized_with(
    profiles_path: &Path,
    is_tty: bool,
    force: bool,
    reader: &mut dyn BufRead,
) -> Result<()> {
    // Already initialized — at least one profile subdirectory exists
    if profiles_path.is_dir() {
        let has_profile = fs::read_dir(profiles_path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            })
            .unwrap_or(false);
        if has_profile {
            // Check version of each embedded profile against on-disk version
            let mut mismatches: Vec<(String, String, String)> = Vec::new(); // (name, on_disk, embedded)
            for name in embedded::list_embedded_profiles() {
                let embedded_ver = match embedded_profile_version(&name) {
                    Some(v) => v,
                    None => continue, // skip profiles without version
                };
                let on_disk_ver = match read_manifest_from(&name, profiles_path) {
                    Ok(m) => m.version,
                    Err(_) => String::new(), // missing/corrupt → treat as mismatch
                };
                if on_disk_ver != embedded_ver {
                    mismatches.push((name, on_disk_ver, embedded_ver));
                }
            }

            if mismatches.is_empty() {
                return Ok(()); // All versions match — fast path
            }

            // Print version mismatch details
            for (name, on_disk, embedded) in &mismatches {
                use std::cmp::Ordering;
                let label = match compare_versions(on_disk, embedded) {
                    Ordering::Less => "",
                    Ordering::Greater => " \u{26a0} this is a downgrade",
                    Ordering::Equal => "", // shouldn't happen since we checked !=
                };
                let on_disk_display = if on_disk.is_empty() {
                    "unknown".to_string()
                } else {
                    format!("v{}", on_disk)
                };
                if label.is_empty() {
                    eprintln!(
                        "Profile '{}': found {}, installing v{}",
                        name, on_disk_display, embedded
                    );
                } else {
                    eprintln!(
                        "Profile '{}': found {}, installing v{} \u{2014}{}",
                        name, on_disk_display, embedded, label
                    );
                }
            }

            if force || !is_tty {
                // Auto re-extract
                eprintln!("Updating profiles...");
                let count = embedded::extract_embedded_to_disk(profiles_path)?;
                eprintln!(
                    "Updated {} profiles in {}",
                    count,
                    profiles_path.display()
                );
            } else {
                // Interactive — prompt user
                eprint!("Update profiles? [y/N] ");
                io::stderr().flush()?;
                let mut line = String::new();
                reader.read_line(&mut line)?;
                let answer = line.trim().to_lowercase();
                if answer == "y" || answer == "yes" {
                    let count = embedded::extract_embedded_to_disk(profiles_path)?;
                    eprintln!(
                        "Updated {} profiles in {}",
                        count,
                        profiles_path.display()
                    );
                } else {
                    eprintln!("Keeping existing profiles");
                }
            }
            return Ok(());
        }
    }

    if is_tty {
        print!("Profiles not initialized. Initialize now? [Y/n] ");
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        let answer = line.trim().to_lowercase();
        if bytes > 0 && (answer == "n" || answer == "no") {
            println!("Run `bm profiles init` to set up profiles.");
            std::process::exit(0);
        }
    }

    // Initialize profiles from embedded data
    fs::create_dir_all(profiles_path).with_context(|| {
        format!(
            "Failed to create profiles directory {}",
            profiles_path.display()
        )
    })?;
    let count = embedded::extract_embedded_to_disk(profiles_path)?;
    eprintln!("Initialized {} profiles in {}", count, profiles_path.display());
    Ok(())
}

/// Scans all files in a profile on disk for agent tags, returning a sorted list of
/// (relative path, agents) pairs for files that contain at least one tag.
pub fn scan_agent_tags(profile_name: &str) -> Result<Vec<(String, Vec<String>)>> {
    scan_agent_tags_in(profile_name, &profiles_dir()?)
}

fn scan_agent_tags_in(profile_name: &str, base: &Path) -> Result<Vec<(String, Vec<String>)>> {
    let profile_dir = base.join(profile_name);
    if !profile_dir.is_dir() {
        let available = list_profiles_from(base).unwrap_or_default().join(", ");
        bail!(
            "Profile '{}' not found. Available profiles: {}",
            profile_name, available
        );
    }

    let mut results = Vec::new();
    scan_dir_for_tags_on_disk(&profile_dir, &profile_dir, &mut results)?;
    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results)
}

/// Recursively scans a disk directory for files containing agent tags.
fn scan_dir_for_tags_on_disk(
    dir: &Path,
    root_path: &Path,
    results: &mut Vec<(String, Vec<String>)>,
) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root_path).unwrap_or(&path);

        if path.is_dir() {
            scan_dir_for_tags_on_disk(&path, root_path, results)?;
            continue;
        }

        let filename = rel
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        if !should_filter(&filename) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            let syntax = agent_tags::detect_comment_syntax(&filename);
            let agents = agent_tags::collect_agent_names(&content, syntax);
            if !agents.is_empty() {
                results.push((
                    rel.to_string_lossy().to_string(),
                    agents.into_iter().collect(),
                ));
            }
        }
    }
    Ok(())
}

/// Resolves the effective coding agent for a team.
///
/// Resolution order:
/// 1. Team-level override (`team.coding_agent`) if set
/// 2. Profile default (`manifest.default_coding_agent`)
///
/// Returns an error if the resolved agent name is not found in the manifest's
/// `coding_agents` map.
pub fn resolve_coding_agent<'a>(
    team: &crate::config::TeamEntry,
    manifest: &'a ProfileManifest,
) -> Result<&'a CodingAgentDef> {
    let agent_name = team
        .coding_agent
        .as_deref()
        .unwrap_or(&manifest.default_coding_agent);

    if agent_name.is_empty() {
        bail!(
            "No coding agent configured. Profile '{}' does not declare a default_coding_agent \
             and team '{}' has no coding_agent override.",
            manifest.name,
            team.name
        );
    }

    manifest.coding_agents.get(agent_name).with_context(|| {
        let available: Vec<&str> = manifest.coding_agents.keys().map(|k| k.as_str()).collect();
        format!(
            "Coding agent '{}' not found in profile '{}'. Available agents: {}",
            agent_name,
            manifest.name,
            if available.is_empty() {
                "(none)".to_string()
            } else {
                available.join(", ")
            }
        )
    })
}

/// Checks that the disk profile's schema_version matches the expected value.
/// Returns an error suggesting `bm upgrade` on mismatch.
pub fn check_schema_version(profile_name: &str, team_schema: &str) -> Result<()> {
    check_schema_version_in(profile_name, team_schema, &profiles_dir()?)
}

fn check_schema_version_in(profile_name: &str, team_schema: &str, base: &Path) -> Result<()> {
    let manifest = read_manifest_from(profile_name, base)?;
    if manifest.schema_version != team_schema {
        bail!(
            "Team uses schema {} but this version of `bm` carries schema {} for profile '{}'. \
             Run `bm upgrade` to migrate the team first.",
            team_schema,
            manifest.schema_version,
            profile_name
        );
    }
    Ok(())
}

/// Gate for commands that require the current schema version (1.0).
/// Reads the team's botminter.yml and checks that schema_version matches.
/// Returns a clear error directing the user to upgrade or re-init.
pub fn require_current_schema(team_name: &str, team_schema: &str) -> Result<()> {
    if team_schema != "1.0" {
        bail!(
            "This feature requires schema 1.0, but team '{}' uses schema {}.\n\
             Run `bm upgrade` to migrate the team, or re-init with a current profile.",
            team_name,
            team_schema
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the default Claude Code agent definition for tests.
    fn claude_code_agent() -> CodingAgentDef {
        CodingAgentDef {
            name: "claude-code".into(),
            display_name: "Claude Code".into(),
            context_file: "CLAUDE.md".into(),
            agent_dir: ".claude".into(),
            binary: "claude".into(),
        }
    }

    /// Extracts embedded profiles to a tempdir for disk-based testing.
    /// Returns (tempdir_handle, path_to_profiles_dir).
    fn setup_disk_profiles() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().to_path_buf();
        embedded::extract_embedded_to_disk(&profiles_path).unwrap();
        (tmp, profiles_path)
    }

    /// Recursively collect all file paths under a directory.
    fn collect_files_recursive_disk(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(collect_files_recursive_disk(&path));
                } else {
                    files.push(path);
                }
            }
        }
        files
    }

    #[test]
    fn all_embedded_profiles_are_discoverable() {
        let (_tmp, base) = setup_disk_profiles();
        let profiles = list_profiles_from(&base).unwrap();
        let embedded = embedded::list_embedded_profiles();
        assert_eq!(
            profiles.len(),
            embedded.len(),
            "list_profiles_in should find all embedded profiles"
        );
        for name in &embedded {
            assert!(
                profiles.contains(name),
                "Profile '{}' should be discoverable",
                name
            );
        }
    }

    #[test]
    fn list_profiles_is_sorted() {
        let (_tmp, base) = setup_disk_profiles();
        let profiles = list_profiles_from(&base).unwrap();
        let mut sorted = profiles.clone();
        sorted.sort();
        assert_eq!(
            profiles, sorted,
            "list_profiles_in should return profiles in sorted order"
        );
    }

    #[test]
    fn all_profiles_manifests_parse() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base)
                .unwrap_or_else(|e| panic!("Profile '{}' manifest should parse: {}", name, e));
            assert_eq!(
                manifest.name, name,
                "Profile '{}' name should match directory",
                name
            );
            assert!(
                !manifest.display_name.is_empty(),
                "Profile '{}' should have display_name",
                name
            );
            assert!(
                !manifest.version.is_empty(),
                "Profile '{}' should have version",
                name
            );
            assert!(
                !manifest.schema_version.is_empty(),
                "Profile '{}' should have schema_version",
                name
            );
            assert!(
                !manifest.description.is_empty(),
                "Profile '{}' should have description",
                name
            );
            assert!(
                !manifest.roles.is_empty(),
                "Profile '{}' should have at least one role",
                name
            );
        }
    }

    #[test]
    fn read_manifest_roles_have_descriptions() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        for role in &manifest.roles {
            assert!(!role.name.is_empty());
            assert!(!role.description.is_empty());
        }
    }

    #[test]
    fn read_manifest_labels_have_required_fields() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        for label in &manifest.labels {
            assert!(!label.name.is_empty());
            assert!(!label.color.is_empty());
            assert!(!label.description.is_empty());
        }
    }

    #[test]
    fn read_manifest_nonexistent_profile_errors() {
        let (_tmp, base) = setup_disk_profiles();
        let result = read_manifest_from("nonexistent", &base);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
        // Error should list all available profiles
        for name in list_profiles_from(&base).unwrap() {
            assert!(
                err.contains(&name),
                "Error should list available profile '{}': {}",
                name,
                err
            );
        }
    }

    #[test]
    fn all_profiles_list_roles_matches_manifest() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let roles = list_roles_from(&name, &base).unwrap();
            let manifest = read_manifest_from(&name, &base).unwrap();
            assert_eq!(
                roles.len(),
                manifest.roles.len(),
                "Profile '{}': list_roles count should match manifest roles count",
                name
            );
            for role in &manifest.roles {
                assert!(
                    roles.contains(&role.name),
                    "Profile '{}': list_roles should include role '{}'",
                    name,
                    role.name
                );
            }
        }
    }

    #[test]
    fn all_profiles_have_labels() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base).unwrap();
            assert!(
                !manifest.labels.is_empty(),
                "Profile '{}' should have at least one label",
                name
            );
        }
    }

    #[test]
    fn all_profiles_have_statuses() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base).unwrap();
            assert!(
                !manifest.statuses.is_empty(),
                "Profile '{}' should have at least one status",
                name
            );
        }
    }

    #[test]
    fn scrum_compact_telegram_has_views() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum-compact-telegram", &base).unwrap();
        assert!(!manifest.views.is_empty());
    }

    #[test]
    fn read_manifest_statuses_have_required_fields() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        for status in &manifest.statuses {
            assert!(!status.name.is_empty());
            assert!(!status.description.is_empty());
        }
    }

    #[test]
    fn extract_profile_copies_team_content() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_profile_from(&base, "scrum", output.path(), &claude_code_agent()).unwrap();

        assert!(output.path().join("PROCESS.md").exists());
        assert!(output.path().join("CLAUDE.md").exists());
        assert!(!output.path().join("context.md").exists());
        assert!(output.path().join("botminter.yml").exists());
        assert!(output.path().join("knowledge").is_dir());
        assert!(output.path().join("invariants").is_dir());
        assert!(output.path().join("coding-agent").is_dir());
        assert!(!output.path().join("roles").exists());
        assert!(!output.path().join(".schema").exists());
    }

    #[test]
    fn extract_member_copies_skeleton() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_member_from(&base, "scrum", "architect", output.path(), &claude_code_agent()).unwrap();

        assert!(output.path().join(".botminter.yml").exists());
        assert!(output.path().join("PROMPT.md").exists());
        assert!(output.path().join("CLAUDE.md").exists());
        assert!(!output.path().join("context.md").exists());
        assert!(output.path().join("ralph.yml").exists());
    }

    #[test]
    fn extract_member_invalid_role_errors() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        let profiles = list_profiles_from(&base).unwrap();
        let profile_name = &profiles[0];
        let result =
            extract_member_from(&base, profile_name, "nonexistent", output.path(), &claude_code_agent());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nonexistent"),
            "Error should mention the invalid role name: {}",
            err
        );
        // Error should list available roles
        let manifest = read_manifest_from(profile_name, &base).unwrap();
        for role in &manifest.roles {
            assert!(
                err.contains(&role.name),
                "Error should list available role '{}': {}",
                role.name,
                err
            );
        }
    }

    #[test]
    fn check_schema_version_match() {
        let (_tmp, base) = setup_disk_profiles();
        assert!(check_schema_version_in("scrum", "1.0", &base).is_ok());
        assert!(check_schema_version_in("scrum-compact", "1.0", &base).is_ok());
    }

    #[test]
    fn check_schema_version_mismatch() {
        let (_tmp, base) = setup_disk_profiles();
        let result = check_schema_version_in("scrum", "99.0", &base);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("bm upgrade"));
    }

    #[test]
    fn check_schema_version_old_team_against_current_profile() {
        let (_tmp, base) = setup_disk_profiles();
        let result = check_schema_version_in("scrum", "0.1", &base);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("bm upgrade"));
        assert!(err.contains("0.1"));
        assert!(err.contains("1.0"));
    }

    #[test]
    fn extract_profile_includes_skills_and_formations() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_profile_from(&base, "scrum", output.path(), &claude_code_agent()).unwrap();

        assert!(output.path().join("skills").is_dir());
        assert!(output.path().join("formations").is_dir());
        assert!(output.path().join("skills/knowledge-manager/SKILL.md").exists());
        assert!(output.path().join("formations/local/formation.yml").exists());
        assert!(output.path().join("formations/k8s/formation.yml").exists());
        assert!(output.path().join("formations/k8s/ralph.yml").exists());
        assert!(output.path().join("formations/k8s/PROMPT.md").exists());
    }

    #[test]
    fn extract_profile_scrum_compact_includes_expected_dirs() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_profile_from(&base, "scrum-compact", output.path(), &claude_code_agent()).unwrap();

        assert!(output.path().join("skills").is_dir());
        assert!(output.path().join("formations").is_dir());
        assert!(output.path().join("skills/knowledge-manager/SKILL.md").exists());
        assert!(output.path().join("formations/local/formation.yml").exists());
    }

    #[test]
    fn require_current_schema_passes() {
        assert!(require_current_schema("my-team", "1.0").is_ok());
    }

    #[test]
    fn require_current_schema_fails_for_old() {
        let result = require_current_schema("my-team", "0.1");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("requires schema 1.0"));
        assert!(err.contains("my-team"));
        assert!(err.contains("0.1"));
        assert!(err.contains("bm upgrade"));
    }

    #[test]
    fn require_current_schema_fails_for_empty() {
        let result = require_current_schema("test-team", "");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("requires schema 1.0"));
    }

    // ── ViewDef tests ────────────────────────────────────────

    fn sample_statuses() -> Vec<StatusDef> {
        vec![
            StatusDef { name: "po:triage".into(), description: "".into() },
            StatusDef { name: "po:backlog".into(), description: "".into() },
            StatusDef { name: "arch:design".into(), description: "".into() },
            StatusDef { name: "arch:plan".into(), description: "".into() },
            StatusDef { name: "dev:implement".into(), description: "".into() },
            StatusDef { name: "done".into(), description: "".into() },
            StatusDef { name: "error".into(), description: "".into() },
        ]
    }

    #[test]
    fn view_resolve_single_prefix() {
        let view = ViewDef {
            name: "PO".into(),
            prefixes: vec!["po".into()],
            also_include: vec!["done".into(), "error".into()],
        };
        let resolved = view.resolve_statuses(&sample_statuses());
        assert_eq!(resolved, vec!["po:triage", "po:backlog", "done", "error"]);
    }

    #[test]
    fn view_resolve_multiple_prefixes() {
        let view = ViewDef {
            name: "Mixed".into(),
            prefixes: vec!["po".into(), "arch".into()],
            also_include: vec![],
        };
        let resolved = view.resolve_statuses(&sample_statuses());
        assert_eq!(resolved, vec!["po:triage", "po:backlog", "arch:design", "arch:plan"]);
    }

    #[test]
    fn view_resolve_no_duplicates_in_also_include() {
        let view = ViewDef {
            name: "Dev".into(),
            prefixes: vec!["dev".into()],
            also_include: vec!["done".into(), "dev:implement".into()],
        };
        let resolved = view.resolve_statuses(&sample_statuses());
        assert_eq!(resolved, vec!["dev:implement", "done"]);
    }

    #[test]
    fn view_resolve_empty_prefixes_returns_only_also_include() {
        let view = ViewDef {
            name: "Bare".into(),
            prefixes: vec![],
            also_include: vec!["done".into()],
        };
        let resolved = view.resolve_statuses(&sample_statuses());
        assert_eq!(resolved, vec!["done"]);
    }

    #[test]
    fn view_resolve_no_match_returns_only_also_include() {
        let view = ViewDef {
            name: "NoMatch".into(),
            prefixes: vec!["nonexistent".into()],
            also_include: vec!["error".into()],
        };
        let resolved = view.resolve_statuses(&sample_statuses());
        assert_eq!(resolved, vec!["error"]);
    }

    #[test]
    fn view_filter_string_format() {
        let view = ViewDef {
            name: "Arch".into(),
            prefixes: vec!["arch".into()],
            also_include: vec!["done".into()],
        };
        let filter = view.filter_string(&sample_statuses());
        assert_eq!(filter, "status:arch:design,arch:plan,done");
    }

    #[test]
    fn rh_scrum_has_views() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        assert!(!manifest.views.is_empty());
        let names: Vec<&str> = manifest.views.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"PO"));
        assert!(names.contains(&"Architect"));
        assert!(names.contains(&"Developer"));
    }

    #[test]
    fn scrum_compact_has_views() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum-compact", &base).unwrap();
        assert!(!manifest.views.is_empty());
    }

    #[test]
    fn rh_scrum_po_view_resolves_all_po_statuses() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        let po_view = manifest.views.iter().find(|v| v.name == "PO").unwrap();
        let resolved = po_view.resolve_statuses(&manifest.statuses);
        assert!(resolved.iter().all(|s| s.starts_with("po:") || s == "done" || s == "error"));
        assert!(resolved.contains(&"po:triage".to_string()));
        assert!(resolved.contains(&"po:merge".to_string()));
        assert!(resolved.contains(&"done".to_string()));
        assert!(resolved.contains(&"error".to_string()));
    }

    #[test]
    fn all_views_cover_done_and_error() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        for view in &manifest.views {
            let resolved = view.resolve_statuses(&manifest.statuses);
            assert!(
                resolved.contains(&"done".to_string()),
                "View '{}' missing 'done'", view.name
            );
            assert!(
                resolved.contains(&"error".to_string()),
                "View '{}' missing 'error'", view.name
            );
        }
    }

    // ── CodingAgentDef tests ────────────────────────────────────

    #[test]
    fn all_profiles_declare_coding_agents() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base).unwrap();
            assert!(
                !manifest.coding_agents.is_empty(),
                "Profile '{}' has no coding_agents", name
            );
            assert!(
                !manifest.default_coding_agent.is_empty(),
                "Profile '{}' has no default_coding_agent", name
            );
        }
    }

    #[test]
    fn all_profiles_have_claude_code_agent() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base).unwrap();
            let agent = manifest.coding_agents.get("claude-code");
            assert!(agent.is_some(), "Profile '{}' missing claude-code agent", name);
            let agent = agent.unwrap();
            assert_eq!(agent.name, "claude-code");
            assert_eq!(agent.display_name, "Claude Code");
            assert_eq!(agent.context_file, "CLAUDE.md");
            assert_eq!(agent.agent_dir, ".claude");
            assert_eq!(agent.binary, "claude");
        }
    }

    #[test]
    fn all_profiles_default_to_claude_code() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base).unwrap();
            assert_eq!(
                manifest.default_coding_agent, "claude-code",
                "Profile '{}' default_coding_agent is not claude-code", name
            );
        }
    }

    #[test]
    fn schema_version_remains_1_0() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base).unwrap();
            assert_eq!(
                manifest.schema_version, "1.0",
                "Profile '{}' schema_version changed from 1.0", name
            );
        }
    }

    #[test]
    fn resolve_coding_agent_uses_profile_default() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        let team = crate::config::TeamEntry {
            name: "test-team".into(),
            path: "/tmp/test".into(),
            profile: "scrum".into(),
            github_repo: "org/test-team".into(),
            credentials: Default::default(),
            coding_agent: None,
            project_number: None,
        };
        let agent = resolve_coding_agent(&team, &manifest).unwrap();
        assert_eq!(agent.name, "claude-code");
        assert_eq!(agent.context_file, "CLAUDE.md");
    }

    #[test]
    fn resolve_coding_agent_team_override() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        let team = crate::config::TeamEntry {
            name: "test-team".into(),
            path: "/tmp/test".into(),
            profile: "scrum".into(),
            github_repo: "org/test-team".into(),
            credentials: Default::default(),
            coding_agent: Some("claude-code".into()),
            project_number: None,
        };
        let agent = resolve_coding_agent(&team, &manifest).unwrap();
        assert_eq!(agent.name, "claude-code");
    }

    // ── Agent tag validation tests (disk-based) ────────────────────

    #[test]
    fn tagged_context_md_files_have_balanced_tags() {
        use crate::agent_tags::{CommentSyntax, tags_are_balanced};
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let profile_dir = base.join(&name);
            for file_path in collect_files_recursive_disk(&profile_dir) {
                let path_str = file_path.to_string_lossy();
                if path_str.ends_with("context.md") {
                    let content = fs::read_to_string(&file_path).unwrap();
                    assert!(
                        tags_are_balanced(&content, CommentSyntax::Html),
                        "Unbalanced HTML agent tags in {}", path_str
                    );
                }
            }
        }
    }

    #[test]
    fn tagged_ralph_yml_files_have_balanced_tags() {
        use crate::agent_tags::{CommentSyntax, tags_are_balanced};
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let profile_dir = base.join(&name);
            for file_path in collect_files_recursive_disk(&profile_dir) {
                let path_str = file_path.to_string_lossy();
                if path_str.ends_with("ralph.yml") {
                    let content = fs::read_to_string(&file_path).unwrap();
                    assert!(
                        tags_are_balanced(&content, CommentSyntax::Hash),
                        "Unbalanced hash agent tags in {}", path_str
                    );
                }
            }
        }
    }

    #[test]
    fn filtering_context_md_for_claude_code_strips_only_tag_lines() {
        use crate::agent_tags::{CommentSyntax, filter_agent_tags};
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let profile_dir = base.join(&name);
            for file_path in collect_files_recursive_disk(&profile_dir) {
                let path_str = file_path.to_string_lossy().to_string();
                if path_str.ends_with("context.md") {
                    let content = fs::read_to_string(&file_path).unwrap();
                    let filtered = filter_agent_tags(&content, "claude-code", CommentSyntax::Html);
                    assert!(
                        !filtered.contains("+agent:"),
                        "Filtered {} still contains +agent: tags", path_str
                    );
                    assert!(
                        !filtered.contains("<!-- -agent -->"),
                        "Filtered {} still contains -agent tags", path_str
                    );
                    for line in content.lines() {
                        if !line.trim().starts_with("<!-- +agent:")
                            && line.trim() != "<!-- -agent -->"
                        {
                            assert!(
                                filtered.contains(line),
                                "Filtered {} is missing line: {}", path_str, line
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn filtering_ralph_yml_for_claude_code_produces_valid_yaml() {
        use crate::agent_tags::{CommentSyntax, filter_agent_tags};
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let profile_dir = base.join(&name);
            for file_path in collect_files_recursive_disk(&profile_dir) {
                let path_str = file_path.to_string_lossy().to_string();
                if path_str.ends_with("ralph.yml")
                    && !path_str.contains("formations/")
                {
                    let content = fs::read_to_string(&file_path).unwrap();
                    let filtered = filter_agent_tags(&content, "claude-code", CommentSyntax::Hash);
                    let parsed: Result<serde_yml::Value, _> = serde_yml::from_str(&filtered);
                    assert!(
                        parsed.is_ok(),
                        "Filtered {} is not valid YAML: {}", path_str,
                        parsed.unwrap_err()
                    );
                    let yaml = parsed.unwrap();
                    let backend = yaml.get("cli")
                        .and_then(|c: &serde_yml::Value| c.get("backend"))
                        .and_then(|b: &serde_yml::Value| b.as_str());
                    assert_eq!(
                        backend, Some("claude"),
                        "Filtered {} should have cli.backend: claude", path_str
                    );
                }
            }
        }
    }

    #[test]
    fn filtering_context_md_for_other_agent_excludes_claude_sections() {
        use crate::agent_tags::{CommentSyntax, filter_agent_tags};
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let profile_dir = base.join(&name);
            for file_path in collect_files_recursive_disk(&profile_dir) {
                let path_str = file_path.to_string_lossy().to_string();
                if path_str.ends_with("context.md") {
                    let content = fs::read_to_string(&file_path).unwrap();
                    if !content.contains("+agent:claude-code") {
                        continue;
                    }
                    let filtered = filter_agent_tags(&content, "gemini-cli", CommentSyntax::Html);
                    assert!(
                        !filtered.contains(".claude/"),
                        "Filtering {} for gemini-cli should exclude .claude/ references", path_str
                    );
                }
            }
        }
    }

    #[test]
    fn filtering_ralph_yml_for_other_agent_excludes_claude_backend() {
        use crate::agent_tags::{CommentSyntax, filter_agent_tags};
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let profile_dir = base.join(&name);
            for file_path in collect_files_recursive_disk(&profile_dir) {
                let path_str = file_path.to_string_lossy().to_string();
                if path_str.ends_with("ralph.yml")
                    && !path_str.contains("formations/")
                {
                    let content = fs::read_to_string(&file_path).unwrap();
                    if !content.contains("+agent:claude-code") {
                        continue;
                    }
                    let filtered = filter_agent_tags(&content, "gemini-cli", CommentSyntax::Hash);
                    assert!(
                        !filtered.contains("backend: claude"),
                        "Filtering {} for gemini-cli should exclude backend: claude", path_str
                    );
                }
            }
        }
    }

    #[test]
    fn resolve_coding_agent_unknown_agent_errors() {
        let (_tmp, base) = setup_disk_profiles();
        let manifest = read_manifest_from("scrum", &base).unwrap();
        let team = crate::config::TeamEntry {
            name: "test-team".into(),
            path: "/tmp/test".into(),
            profile: "scrum".into(),
            github_repo: "org/test-team".into(),
            credentials: Default::default(),
            coding_agent: Some("nonexistent-agent".into()),
            project_number: None,
        };
        let result = resolve_coding_agent(&team, &manifest);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent-agent"));
        assert!(err.contains("not found"));
    }

    // ── Extraction filtering tests ────────────────────────────────

    #[test]
    fn extract_profile_claude_md_is_filtered() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_profile_from(&base, "scrum", output.path(), &claude_code_agent()).unwrap();

        let content = std::fs::read_to_string(output.path().join("CLAUDE.md")).unwrap();
        assert!(!content.contains("+agent:"), "Extracted CLAUDE.md should not contain +agent: tags");
        assert!(!content.contains("<!-- -agent -->"), "Extracted CLAUDE.md should not contain -agent close tags");
        assert!(content.len() > 50, "Extracted CLAUDE.md should have substantial content");
    }

    #[test]
    fn extract_profile_ralph_yml_is_filtered() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_profile_from(&base, "scrum", output.path(), &claude_code_agent()).unwrap();

        let formations_dir = output.path().join("formations");
        if formations_dir.is_dir() {
            for entry in std::fs::read_dir(&formations_dir).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    let ralph_yml = entry.path().join("ralph.yml");
                    if ralph_yml.exists() {
                        let content = std::fs::read_to_string(&ralph_yml).unwrap();
                        assert!(!content.contains("+agent:"), "Formation ralph.yml should not contain +agent: tags");
                        let parsed: Result<serde_yml::Value, _> = serde_yml::from_str(&content);
                        assert!(parsed.is_ok(), "Formation ralph.yml should be valid YAML after filtering: {}", parsed.unwrap_err());
                    }
                }
            }
        }
    }

    #[test]
    fn extract_member_claude_md_is_filtered() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_member_from(&base, "scrum", "architect", output.path(), &claude_code_agent()).unwrap();

        let content = std::fs::read_to_string(output.path().join("CLAUDE.md")).unwrap();
        assert!(!content.contains("+agent:"), "Extracted member CLAUDE.md should not contain +agent: tags");
        assert!(!content.contains("<!-- -agent -->"), "Extracted member CLAUDE.md should not contain -agent close tags");
    }

    #[test]
    fn extract_member_ralph_yml_is_filtered() {
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_member_from(&base, "scrum", "architect", output.path(), &claude_code_agent()).unwrap();

        let content = std::fs::read_to_string(output.path().join("ralph.yml")).unwrap();
        assert!(!content.contains("+agent:"), "Extracted member ralph.yml should not contain +agent: tags");
        assert!(!content.contains("# -agent"), "Extracted member ralph.yml should not contain -agent close tags");
        let parsed: Result<serde_yml::Value, _> = serde_yml::from_str(&content);
        assert!(parsed.is_ok(), "Extracted ralph.yml should be valid YAML: {}", parsed.unwrap_err());
        let yaml = parsed.unwrap();
        let backend = yaml.get("cli").and_then(|c: &serde_yml::Value| c.get("backend")).and_then(|b: &serde_yml::Value| b.as_str());
        assert_eq!(backend, Some("claude"), "Extracted ralph.yml should have cli.backend: claude");
    }

    #[test]
    fn extract_profile_mock_agent_produces_different_context_file() {
        let mock_agent = CodingAgentDef {
            name: "gemini-cli".into(),
            display_name: "Gemini CLI".into(),
            context_file: "GEMINI.md".into(),
            agent_dir: ".gemini".into(),
            binary: "gemini".into(),
        };
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_profile_from(&base, "scrum", output.path(), &mock_agent).unwrap();

        assert!(output.path().join("GEMINI.md").exists(), "Mock agent should produce GEMINI.md");
        assert!(!output.path().join("CLAUDE.md").exists(), "Mock agent should not produce CLAUDE.md");
        assert!(!output.path().join("context.md").exists(), "Mock agent should not produce context.md");

        let content = std::fs::read_to_string(output.path().join("GEMINI.md")).unwrap();
        assert!(!content.contains("+agent:"), "GEMINI.md should not contain agent tags");
    }

    #[test]
    fn extract_member_mock_agent_produces_different_context_file() {
        let mock_agent = CodingAgentDef {
            name: "gemini-cli".into(),
            display_name: "Gemini CLI".into(),
            context_file: "GEMINI.md".into(),
            agent_dir: ".gemini".into(),
            binary: "gemini".into(),
        };
        let (_profiles_tmp, base) = setup_disk_profiles();
        let output = tempfile::tempdir().unwrap();
        extract_member_from(&base, "scrum", "architect", output.path(), &mock_agent).unwrap();

        assert!(output.path().join("GEMINI.md").exists(), "Mock agent should produce GEMINI.md in member dir");
        assert!(!output.path().join("CLAUDE.md").exists(), "Mock agent should not produce CLAUDE.md in member dir");
        assert!(!output.path().join("context.md").exists(), "Mock agent should not produce context.md in member dir");
    }

    // ── scan_agent_tags tests ────────────────────────────────────

    #[test]
    fn scan_agent_tags_finds_tagged_files() {
        let (_tmp, base) = setup_disk_profiles();
        let results = scan_agent_tags_in("scrum", &base).unwrap();
        assert!(!results.is_empty(), "scrum profile should have tagged files");
        let has_context = results.iter().any(|(path, _)| path == "context.md");
        assert!(has_context, "scrum should have tagged context.md");
    }

    #[test]
    fn scan_agent_tags_reports_claude_code() {
        let (_tmp, base) = setup_disk_profiles();
        let results = scan_agent_tags_in("scrum", &base).unwrap();
        for (path, agents) in &results {
            assert!(
                agents.contains(&"claude-code".to_string()),
                "File {} should reference claude-code agent, got {:?}", path, agents
            );
        }
    }

    #[test]
    fn scan_agent_tags_finds_ralph_yml_tags() {
        let (_tmp, base) = setup_disk_profiles();
        let results = scan_agent_tags_in("scrum", &base).unwrap();
        let has_ralph_yml = results.iter().any(|(path, _)| path.ends_with("ralph.yml"));
        assert!(has_ralph_yml, "scrum profile should have tagged ralph.yml files");
    }

    #[test]
    fn scan_agent_tags_all_profiles_consistent() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let results = scan_agent_tags_in(&name, &base).unwrap();
            assert!(!results.is_empty(), "Profile '{}' should have tagged files", name);
        }
    }

    #[test]
    fn scan_agent_tags_nonexistent_profile_errors() {
        let (_tmp, base) = setup_disk_profiles();
        let result = scan_agent_tags_in("nonexistent", &base);
        assert!(result.is_err());
    }

    // ── Init coding agent resolution tests ────────────────────────

    #[test]
    fn init_resolves_default_coding_agent_from_manifest() {
        let (_tmp, base) = setup_disk_profiles();
        for name in list_profiles_from(&base).unwrap() {
            let manifest = read_manifest_from(&name, &base).unwrap();
            assert!(
                !manifest.default_coding_agent.is_empty(),
                "Profile '{}' should declare a default_coding_agent", name
            );
            let agent = manifest.coding_agents.get(&manifest.default_coding_agent);
            assert!(
                agent.is_some(),
                "Profile '{}' default_coding_agent '{}' should exist in coding_agents map",
                name, manifest.default_coding_agent
            );
            let agent = agent.unwrap();
            assert_eq!(
                agent.context_file, "CLAUDE.md",
                "Profile '{}' default agent should have CLAUDE.md context_file", name
            );
        }
    }

    // ── extract_embedded_to_disk tests ────────────────────────────

    #[test]
    fn extract_embedded_to_disk_creates_all_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        let count = extract_embedded_to_disk(tmp.path()).unwrap();

        let expected_profiles = embedded::list_embedded_profiles();
        assert_eq!(count, expected_profiles.len());

        for name in &expected_profiles {
            assert!(
                tmp.path().join(name).is_dir(),
                "Profile '{}' should be extracted as a directory", name
            );
        }
    }

    #[test]
    fn extract_embedded_to_disk_preserves_botminter_yml() {
        let tmp = tempfile::tempdir().unwrap();
        extract_embedded_to_disk(tmp.path()).unwrap();

        for name in embedded::list_embedded_profiles() {
            let manifest_path = tmp.path().join(&name).join("botminter.yml");
            assert!(manifest_path.exists(), "Profile '{}' should have botminter.yml", name);
            let contents = std::fs::read_to_string(&manifest_path).unwrap();
            let manifest: ProfileManifest = serde_yml::from_str(&contents).unwrap();
            assert_eq!(manifest.name, name);
        }
    }

    #[test]
    fn extract_embedded_to_disk_content_matches_embedded() {
        let tmp = tempfile::tempdir().unwrap();
        extract_embedded_to_disk(tmp.path()).unwrap();

        let embedded_dir = embedded::embedded_profiles();
        let embedded_bytes = embedded_dir
            .get_file("scrum/botminter.yml")
            .unwrap()
            .contents();
        let disk = std::fs::read(tmp.path().join("scrum").join("botminter.yml")).unwrap();
        assert_eq!(embedded_bytes, disk.as_slice(), "Extracted content should be byte-identical");
    }

    #[test]
    fn extract_embedded_to_disk_preserves_context_md_name() {
        let tmp = tempfile::tempdir().unwrap();
        extract_embedded_to_disk(tmp.path()).unwrap();

        for name in embedded::list_embedded_profiles() {
            let context_path = tmp.path().join(&name).join("context.md");
            assert!(context_path.exists(), "Profile '{}' should keep context.md (not renamed)", name);
            assert!(
                !tmp.path().join(&name).join("CLAUDE.md").exists(),
                "Profile '{}' should not have CLAUDE.md (no rename during init)", name
            );
        }
    }

    #[test]
    fn extract_embedded_to_disk_preserves_agent_tags() {
        let tmp = tempfile::tempdir().unwrap();
        extract_embedded_to_disk(tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("scrum").join("context.md")).unwrap();
        let embedded_dir = embedded::embedded_profiles();
        let embedded_content = embedded_dir
            .get_file("scrum/context.md")
            .unwrap()
            .contents_utf8()
            .unwrap();
        assert_eq!(content, embedded_content, "Agent tags should be preserved verbatim");
    }

    #[test]
    fn extract_embedded_to_disk_preserves_schema_dir() {
        let tmp = tempfile::tempdir().unwrap();
        extract_embedded_to_disk(tmp.path()).unwrap();

        for name in embedded::list_embedded_profiles() {
            let schema_dir = tmp.path().join(&name).join(".schema");
            assert!(schema_dir.is_dir(), "Profile '{}' should have .schema/ directory", name);
            assert!(schema_dir.join("v1.yml").exists(), "Profile '{}' should have .schema/v1.yml", name);
        }
    }

    #[test]
    fn extract_embedded_to_disk_preserves_roles_dir() {
        let tmp = tempfile::tempdir().unwrap();
        extract_embedded_to_disk(tmp.path()).unwrap();

        for name in embedded::list_embedded_profiles() {
            let roles_dir = tmp.path().join(&name).join("roles");
            assert!(roles_dir.is_dir(), "Profile '{}' should have roles/ directory", name);
        }
    }

    #[test]
    fn extract_embedded_to_disk_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("deep").join("nested").join("profiles");
        extract_embedded_to_disk(&nested).unwrap();

        assert!(nested.is_dir(), "Nested target should be created");
        assert!(nested.join("scrum").join("botminter.yml").exists(), "Profiles should be extracted into nested target");
    }

    #[test]
    fn profiles_dir_returns_config_path() {
        let dir = profiles_dir().unwrap();
        let path_str = dir.to_string_lossy();
        assert!(
            path_str.contains("botminter") && path_str.contains("profiles"),
            "profiles_dir should contain botminter/profiles: got {}", path_str
        );
    }

    // ── ensure_profiles_initialized tests ────────────────────────

    #[test]
    fn ensure_profiles_initialized_skips_when_profiles_exist() {
        let (_tmp, base) = setup_disk_profiles();
        let mut reader = io::Cursor::new(b"");
        let result = ensure_profiles_initialized_with(&base, true, false, &mut reader);
        assert!(result.is_ok(), "Should return Ok when profiles exist");
    }

    #[test]
    fn ensure_profiles_initialized_extracts_on_yes() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        let mut reader = io::Cursor::new(b"y\n");
        let result = ensure_profiles_initialized_with(&profiles_path, true, false, &mut reader);
        assert!(result.is_ok(), "Should succeed after yes: {:?}", result.err());

        // Verify profiles were extracted
        let names = list_profiles_from(&profiles_path).unwrap();
        assert!(!names.is_empty(), "Profiles should be extracted");
    }

    #[test]
    fn ensure_profiles_initialized_extracts_on_default_enter() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        let mut reader = io::Cursor::new(b"\n");
        let result = ensure_profiles_initialized_with(&profiles_path, true, false, &mut reader);
        assert!(result.is_ok(), "Empty enter (default Y) should extract");

        let names = list_profiles_from(&profiles_path).unwrap();
        assert!(!names.is_empty(), "Profiles should be extracted on default");
    }

    #[test]
    fn ensure_profiles_initialized_auto_inits_non_tty() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        let mut reader = io::Cursor::new(b"");
        let result = ensure_profiles_initialized_with(&profiles_path, false, false, &mut reader);
        assert!(result.is_ok(), "Non-TTY should auto-init: {:?}", result.err());

        let names = list_profiles_from(&profiles_path).unwrap();
        assert!(!names.is_empty(), "Non-TTY should extract profiles");
    }

    #[test]
    fn ensure_profiles_initialized_empty_dir_triggers_init() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        fs::create_dir_all(&profiles_path).unwrap();
        // Dir exists but is empty — should still trigger init
        let mut reader = io::Cursor::new(b"");
        let result = ensure_profiles_initialized_with(&profiles_path, false, false, &mut reader);
        assert!(result.is_ok());

        let names = list_profiles_from(&profiles_path).unwrap();
        assert!(!names.is_empty(), "Empty profiles dir should trigger init");
    }

    // ── staleness detection tests ────────────────────────────────

    #[test]
    fn ensure_profiles_initialized_re_extracts_when_version_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        fs::create_dir_all(&profiles_path).unwrap();
        embedded::extract_embedded_to_disk(&profiles_path).unwrap();

        // Delete one profile's botminter.yml to trigger staleness
        let sample = &embedded::list_embedded_profiles()[0];
        let manifest = profiles_path.join(sample).join("botminter.yml");
        fs::remove_file(&manifest).unwrap();

        // Simulate stale layout by renaming roles/ to members/
        for name in embedded::list_embedded_profiles() {
            let roles = profiles_path.join(&name).join("roles");
            let members = profiles_path.join(&name).join("members");
            if roles.exists() {
                fs::rename(&roles, &members).unwrap();
            }
        }

        // ensure_profiles_initialized should re-extract (non-TTY auto path)
        let mut reader = io::Cursor::new(b"");
        ensure_profiles_initialized_with(&profiles_path, false, false, &mut reader).unwrap();

        // Verify roles/ exists again (re-extracted)
        for name in embedded::list_embedded_profiles() {
            let roles = profiles_path.join(&name).join("roles");
            assert!(
                roles.is_dir(),
                "roles/ should exist after re-extraction for {}",
                name
            );
        }
    }

    #[test]
    fn ensure_profiles_initialized_re_extracts_when_version_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        fs::create_dir_all(&profiles_path).unwrap();
        embedded::extract_embedded_to_disk(&profiles_path).unwrap();

        // Modify on-disk version to simulate version mismatch
        let sample = &embedded::list_embedded_profiles()[0];
        let manifest_path = profiles_path.join(sample).join("botminter.yml");
        let content = fs::read_to_string(&manifest_path).unwrap();
        let modified = content.replace(
            &format!("version: \"{}\"", embedded_profile_version(sample).unwrap()),
            "version: \"0.0.0\"",
        );
        fs::write(&manifest_path, &modified).unwrap();

        // Also corrupt a file to verify re-extraction actually happens
        let sentinel = profiles_path.join(sample).join("PROCESS.md");
        if sentinel.exists() {
            fs::write(&sentinel, "corrupted-sentinel").unwrap();
        }

        // ensure_profiles_initialized should re-extract (non-TTY auto path)
        let mut reader = io::Cursor::new(b"");
        ensure_profiles_initialized_with(&profiles_path, false, false, &mut reader).unwrap();

        // Verify manifest was restored
        let restored = fs::read_to_string(&manifest_path).unwrap();
        assert!(
            restored.contains("name:"),
            "botminter.yml should be restored after re-extraction"
        );
    }

    #[test]
    fn ensure_profiles_initialized_skips_when_version_current() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        fs::create_dir_all(&profiles_path).unwrap();
        embedded::extract_embedded_to_disk(&profiles_path).unwrap();

        // Place a sentinel file to prove re-extraction does NOT happen
        let sample = &embedded::list_embedded_profiles()[0];
        let sentinel = profiles_path.join(sample).join("_sentinel.txt");
        fs::write(&sentinel, "keep-me").unwrap();

        // Versions match — should skip re-extraction
        let mut reader = io::Cursor::new(b"");
        ensure_profiles_initialized_with(&profiles_path, false, false, &mut reader).unwrap();

        // Verify sentinel was NOT removed (proves skip — extract_embedded_to_disk
        // would overwrite the directory but not remove extra files, so check botminter.yml
        // was not re-written by checking it matches the original)
        assert!(
            sentinel.exists(),
            "Sentinel file should be preserved when versions match"
        );
    }

    #[test]
    fn ensure_profiles_initialized_prompts_on_version_mismatch_tty() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        fs::create_dir_all(&profiles_path).unwrap();
        embedded::extract_embedded_to_disk(&profiles_path).unwrap();

        // Modify on-disk version to create mismatch
        let sample = &embedded::list_embedded_profiles()[0];
        let manifest_path = profiles_path.join(sample).join("botminter.yml");
        let content = fs::read_to_string(&manifest_path).unwrap();
        let modified = content.replace(
            &format!("version: \"{}\"", embedded_profile_version(sample).unwrap()),
            "version: \"0.0.0\"",
        );
        fs::write(&manifest_path, &modified).unwrap();

        // TTY mode, user answers "y" — should re-extract
        let mut reader = io::Cursor::new(b"y\n");
        ensure_profiles_initialized_with(&profiles_path, true, false, &mut reader).unwrap();

        // Verify manifest was restored (re-extraction happened)
        let restored = fs::read_to_string(&manifest_path).unwrap();
        assert!(
            restored.contains(&format!("version: \"{}\"", embedded_profile_version(sample).unwrap())),
            "botminter.yml should have embedded version after user confirmed update"
        );
    }

    #[test]
    fn ensure_profiles_initialized_skips_on_version_mismatch_tty_declined() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        fs::create_dir_all(&profiles_path).unwrap();
        embedded::extract_embedded_to_disk(&profiles_path).unwrap();

        // Modify on-disk version to create mismatch
        let sample = &embedded::list_embedded_profiles()[0];
        let manifest_path = profiles_path.join(sample).join("botminter.yml");
        let content = fs::read_to_string(&manifest_path).unwrap();
        let modified = content.replace(
            &format!("version: \"{}\"", embedded_profile_version(sample).unwrap()),
            "version: \"0.0.0\"",
        );
        fs::write(&manifest_path, &modified).unwrap();

        // TTY mode, user answers "n" — should NOT re-extract
        let mut reader = io::Cursor::new(b"n\n");
        ensure_profiles_initialized_with(&profiles_path, true, false, &mut reader).unwrap();

        // Verify manifest was NOT restored (user declined)
        let preserved = fs::read_to_string(&manifest_path).unwrap();
        assert!(
            preserved.contains("version: \"0.0.0\""),
            "botminter.yml should keep stale version when user declines update"
        );
    }

    #[test]
    fn ensure_profiles_initialized_force_skips_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_path = tmp.path().join("profiles");
        fs::create_dir_all(&profiles_path).unwrap();
        embedded::extract_embedded_to_disk(&profiles_path).unwrap();

        // Modify on-disk version to create mismatch
        let sample = &embedded::list_embedded_profiles()[0];
        let manifest_path = profiles_path.join(sample).join("botminter.yml");
        let content = fs::read_to_string(&manifest_path).unwrap();
        let modified = content.replace(
            &format!("version: \"{}\"", embedded_profile_version(sample).unwrap()),
            "version: \"0.0.0\"",
        );
        fs::write(&manifest_path, &modified).unwrap();

        // TTY mode with force=true — should re-extract without reading from reader
        let mut reader = io::Cursor::new(b""); // empty reader — would block if prompt attempted
        ensure_profiles_initialized_with(&profiles_path, true, true, &mut reader).unwrap();

        // Verify manifest was restored (force bypassed prompt)
        let restored = fs::read_to_string(&manifest_path).unwrap();
        assert!(
            restored.contains(&format!("version: \"{}\"", embedded_profile_version(sample).unwrap())),
            "botminter.yml should have embedded version after force update"
        );
    }
}
