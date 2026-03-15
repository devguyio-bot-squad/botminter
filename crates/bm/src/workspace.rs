use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::profile::CodingAgentDef;

/// Parameters for creating a workspace repo with submodules.
pub struct WorkspaceRepoParams<'a> {
    pub team_repo_path: &'a Path,
    pub workspace_base: &'a Path,
    pub member_dir_name: &'a str,
    pub team_name: &'a str,
    pub projects: &'a [(&'a str, &'a str)], // [(project_name, fork_url)]
    pub github_repo: Option<&'a str>,
    pub push: bool,
    pub gh_token: Option<&'a str>,
    pub coding_agent: &'a CodingAgentDef,
}

/// Creates a workspace repo for a member using the submodule model.
///
/// This replaces the old `.botminter/` clone model. The workspace is a git repo
/// containing submodules: `team/` points to the team repo, and `projects/<name>/`
/// points to project forks. Member branches are checked out in all submodules.
///
/// When `push` is true (i.e., `bm teams sync --repos`), a GitHub repo is
/// created via `gh repo create`. When false, the workspace is local-only.
pub fn create_workspace_repo(params: &WorkspaceRepoParams) -> Result<()> {
    let member_ws = params.workspace_base.join(params.member_dir_name);

    if params.push {
        // Extract org from github_repo (e.g., "myorg/my-team" → "myorg")
        let org = params
            .github_repo
            .and_then(|r| r.split('/').next())
            .unwrap_or("");
        if org.is_empty() {
            bail!(
                "Cannot create workspace repo: no GitHub org found.\n\
                 The team must have a github_repo configured (e.g., 'myorg/my-team')."
            );
        }

        let ws_repo_name = format!("{}/{}-{}", org, params.team_name, params.member_dir_name);

        // Check if repo already exists on GitHub
        let mut view_cmd = Command::new("gh");
        view_cmd.args(["repo", "view", &ws_repo_name, "--json", "name"]);
        if let Some(token) = params.gh_token {
            view_cmd.env("GH_TOKEN", token);
        }
        let view_output = view_cmd
            .output()
            .context("Failed to run `gh repo view`")?;
        let repo_already_exists = view_output.status.success();
        if repo_already_exists {
            eprintln!(
                "Workspace repo '{}' already exists on GitHub, cloning it.",
                ws_repo_name
            );
        } else {
            // Create GitHub repo
            let mut cmd = Command::new("gh");
            cmd.args(["repo", "create", &ws_repo_name, "--private"]);
            if let Some(token) = params.gh_token {
                cmd.env("GH_TOKEN", token);
            }
            let output = cmd.output().context("Failed to run `gh repo create`")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!(
                    "Failed to create workspace repo '{}'.\n{}\n\n\
                     If the repo already exists:\n  \
                     gh repo delete {} --yes\n\
                     Then re-run `bm teams sync --repos`.",
                    ws_repo_name,
                    stderr.trim(),
                    ws_repo_name,
                );
            }
        }

        // Clone the repo — use --recursive for existing repos to init submodules
        let clone_url = format!("https://github.com/{}.git", ws_repo_name);
        let ws_path_str = member_ws.to_string_lossy().to_string();
        if repo_already_exists {
            git_cmd(params.workspace_base, &["clone", "--recursive", &clone_url, &ws_path_str])
        } else {
            git_cmd(params.workspace_base, &["clone", &clone_url, &ws_path_str])
        }
        .with_context(|| {
            format!(
                "Failed to clone workspace repo {}\n\n\
                 To verify: gh repo view {}",
                ws_repo_name, ws_repo_name
            )
        })?;

        // If repo already existed, submodules + files are already present from clone
        if repo_already_exists {
            return Ok(());
        }
    } else {
        // Local-only mode: git init
        fs::create_dir_all(&member_ws)
            .with_context(|| format!("Failed to create workspace dir {}", member_ws.display()))?;
        git_cmd(&member_ws, &["init", "-b", "main"])?;
    }

    // Configure git user for the workspace repo
    git_cmd(&member_ws, &["config", "user.email", "botminter@local"])?;
    git_cmd(&member_ws, &["config", "user.name", "BotMinter"])?;

    // Add team repo as submodule at `team/`
    let team_repo_url = if params.push {
        // Use the GitHub URL for the team repo
        params
            .github_repo
            .map(|r| format!("https://github.com/{}.git", r))
            .unwrap_or_else(|| {
                fs::canonicalize(params.team_repo_path)
                    .unwrap_or_else(|_| params.team_repo_path.to_path_buf())
                    .to_string_lossy()
                    .to_string()
            })
    } else {
        // Use local path for the team repo
        fs::canonicalize(params.team_repo_path)
            .unwrap_or_else(|_| params.team_repo_path.to_path_buf())
            .to_string_lossy()
            .to_string()
    };

    git_submodule_add(&member_ws, &team_repo_url, "team")
        .with_context(|| {
            format!(
                "Failed to add team repo submodule.\n\n\
                 To verify the team repo: git ls-remote {}",
                team_repo_url
            )
        })?;

    // Checkout member branch in team submodule
    let team_sub = member_ws.join("team");
    if git_cmd(&team_sub, &["checkout", params.member_dir_name]).is_err() {
        git_cmd(&team_sub, &["checkout", "-b", params.member_dir_name])?;
    }

    // Add project submodules
    if !params.projects.is_empty() {
        let projects_dir = member_ws.join("projects");
        fs::create_dir_all(&projects_dir)
            .with_context(|| format!("Failed to create projects dir {}", projects_dir.display()))?;

        for &(project_name, fork_url) in params.projects {
            let submodule_path = format!("projects/{}", project_name);
            git_submodule_add(&member_ws, fork_url, &submodule_path)
                .with_context(|| {
                    format!(
                        "Failed to add project submodule '{}' from {}\n\n\
                         To verify the fork: gh repo view {}",
                        project_name, fork_url, fork_url
                    )
                })?;

            // Checkout member branch in project submodule
            let proj_sub = member_ws.join("projects").join(project_name);
            if git_cmd(&proj_sub, &["checkout", params.member_dir_name]).is_err() {
                git_cmd(&proj_sub, &["checkout", "-b", params.member_dir_name])?;
            }
        }
    }

    // Assemble context files, agent dir, gitignore, and marker
    let project_names: Vec<&str> = params.projects.iter().map(|(name, _)| *name).collect();
    assemble_workspace_repo_context(
        &member_ws,
        params.member_dir_name,
        &project_names,
        params.coding_agent,
    )?;

    // Commit all workspace files
    git_cmd(&member_ws, &["add", "-A"])?;
    git_cmd(
        &member_ws,
        &["commit", "-m", "Initial workspace setup"],
    )?;

    // Push if remote is configured
    if params.push {
        git_cmd(&member_ws, &["push", "-u", "origin", "main"])?;
    }

    Ok(())
}

/// Assembles workspace context files, agent dir, gitignore, and marker for the
/// submodule-based workspace model.
///
/// Context files (context_file, PROMPT.md, ralph.yml) are **copied** from the
/// team submodule — they're tracked first-class citizens in the workspace repo.
/// Agent dir entries are **symlinked** into the team submodule paths.
pub fn assemble_workspace_repo_context(
    ws_root: &Path,
    member_dir_name: &str,
    project_names: &[&str],
    coding_agent: &CodingAgentDef,
) -> Result<()> {
    let team_sub = ws_root.join("team");
    let member_src = team_sub.join("members").join(member_dir_name);

    // Copy context files from team/members/<member>/ to workspace root
    let context_src = member_src.join(&coding_agent.context_file);
    if context_src.exists() {
        fs::copy(&context_src, ws_root.join(&coding_agent.context_file))
            .with_context(|| format!("Failed to copy {}", coding_agent.context_file))?;
    }

    let prompt_src = member_src.join("PROMPT.md");
    if prompt_src.exists() {
        fs::copy(&prompt_src, ws_root.join("PROMPT.md"))
            .context("Failed to copy PROMPT.md")?;
    }

    let ralph_src = member_src.join("ralph.yml");
    if ralph_src.exists() {
        fs::copy(&ralph_src, ws_root.join("ralph.yml"))
            .context("Failed to copy ralph.yml")?;
    }

    // Assemble agent dir with symlinks into team/ submodule
    assemble_agent_dir_submodule(ws_root, member_dir_name, project_names, coding_agent)?;

    // Write .botminter.workspace marker
    write_workspace_marker(ws_root, member_dir_name)?;

    Ok(())
}

/// Assembles the agent directory from three scopes using `team/` submodule paths.
///
/// 1. Team-level: `team/coding-agent/agents/*.md`
/// 2. Project-level: `team/projects/{project}/coding-agent/agents/*.md`
/// 3. Member-level: `team/members/{member}/coding-agent/agents/*.md`
///
/// Also copies `settings.local.json` from the member's coding-agent dir if present.
fn assemble_agent_dir_submodule(
    ws_root: &Path,
    member_dir_name: &str,
    project_names: &[&str],
    coding_agent: &CodingAgentDef,
) -> Result<()> {
    let agents_subdir = ws_root.join(&coding_agent.agent_dir).join("agents");

    // Remove and recreate for idempotency
    if agents_subdir.exists() {
        fs::remove_dir_all(&agents_subdir).ok();
    }
    fs::create_dir_all(&agents_subdir)
        .with_context(|| format!("Failed to create {}/agents/", coding_agent.agent_dir))?;

    let team_sub = ws_root.join("team");

    // 1. Team-level agents
    symlink_md_files(&team_sub.join("coding-agent").join("agents"), &agents_subdir)?;

    // 2. Project-level agents (all assigned projects)
    for project in project_names {
        symlink_md_files(
            &team_sub
                .join("projects")
                .join(project)
                .join("coding-agent")
                .join("agents"),
            &agents_subdir,
        )?;
    }

    // 3. Member-level agents
    symlink_md_files(
        &team_sub
            .join("members")
            .join(member_dir_name)
            .join("coding-agent")
            .join("agents"),
        &agents_subdir,
    )?;

    // 4. Copy settings.local.json if present
    let settings_src = team_sub
        .join("members")
        .join(member_dir_name)
        .join("coding-agent")
        .join("settings.local.json");
    if settings_src.exists() {
        let dst = ws_root
            .join(&coding_agent.agent_dir)
            .join("settings.local.json");
        fs::copy(&settings_src, &dst).context("Failed to copy settings.local.json")?;
    }

    Ok(())
}

/// Writes the `.botminter.workspace` marker file with workspace metadata.
/// Lists member directory names from a members/ directory.
/// Returns sorted directory names, skipping hidden entries.
pub fn list_member_dirs(members_dir: &Path) -> Result<Vec<String>> {
    let mut dirs = Vec::new();
    if !members_dir.is_dir() {
        return Ok(dirs);
    }
    for entry in fs::read_dir(members_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with('.') {
            dirs.push(name);
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// Finds the workspace path for a member.
/// Returns Some if the member workspace dir exists and has the `.botminter.workspace` marker.
pub fn find_workspace(team_ws_base: &Path, member_dir_name: &str) -> Option<PathBuf> {
    let member_ws = team_ws_base.join(member_dir_name);
    if member_ws.is_dir() && member_ws.join(".botminter.workspace").exists() {
        Some(member_ws)
    } else {
        None
    }
}

fn write_workspace_marker(ws_root: &Path, member_dir_name: &str) -> Result<()> {
    let content = format!(
        "# BotMinter workspace marker — do not delete\nmember: {}\n",
        member_dir_name,
    );
    fs::write(ws_root.join(".botminter.workspace"), content)
        .context("Failed to write .botminter.workspace marker")
}

/// Syncs an existing workspace by updating submodules, re-copying context files,
/// re-assembling agent directory, and committing+pushing any changes.
///
/// Uses the `team/` submodule model. Updates submodules to latest remote content,
/// checks out member branches, re-copies context files when newer, and rebuilds
/// agent dir symlinks idempotently.
pub fn sync_workspace(
    ws_root: &Path,
    member_dir_name: &str,
    coding_agent: &CodingAgentDef,
    verbose: bool,
    push: bool,
) -> Result<()> {
    let team_dir = ws_root.join("team");

    // Update submodules to latest remote content
    if team_dir.is_dir() {
        if verbose {
            println!("  Updating team/ submodule...");
        }
        // Fetch and update to latest remote tracking branch
        git_cmd(ws_root, &[
            "-c", "protocol.file.allow=always",
            "submodule", "update", "--remote", "--merge", "team",
        ]).ok();

        // Checkout member branch (avoid detached HEAD)
        checkout_member_branch(&team_dir, member_dir_name, verbose)?;
    }

    // Update project submodules
    let projects_dir = ws_root.join("projects");
    if projects_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let project_name = entry.file_name().to_string_lossy().to_string();
                    let project_path = format!("projects/{}", project_name);
                    if verbose {
                        println!("  Updating {} submodule...", project_path);
                    }
                    git_cmd(ws_root, &[
                        "-c", "protocol.file.allow=always",
                        "submodule", "update", "--remote", "--merge", &project_path,
                    ]).ok();

                    // Checkout member branch in project submodule
                    checkout_member_branch(&entry.path(), member_dir_name, verbose)?;
                }
            }
        }
    }

    // Re-copy context files from team/members/<member>/
    let member_src = team_dir.join("members").join(member_dir_name);
    let files_to_sync = [
        (member_src.join("ralph.yml"), ws_root.join("ralph.yml"), "ralph.yml"),
        (
            member_src.join(&coding_agent.context_file),
            ws_root.join(&coding_agent.context_file),
            coding_agent.context_file.as_str(),
        ),
        (member_src.join("PROMPT.md"), ws_root.join("PROMPT.md"), "PROMPT.md"),
    ];

    for (src, dst, name) in &files_to_sync {
        let copied = copy_if_newer_verbose(src, dst)?;
        if verbose {
            if copied {
                println!("  Copied {} (newer)", name);
            } else if src.exists() {
                println!("  Skipped {} (up-to-date)", name);
            }
        }
    }

    // Re-copy settings.local.json if source is newer
    let settings_src = member_src
        .join("coding-agent")
        .join("settings.local.json");
    let settings_dst = ws_root
        .join(&coding_agent.agent_dir)
        .join("settings.local.json");
    let settings_copied = copy_if_newer_verbose(&settings_src, &settings_dst)?;
    if verbose && settings_src.exists() {
        if settings_copied {
            println!("  Copied settings.local.json (newer)");
        } else {
            println!("  Skipped settings.local.json (up-to-date)");
        }
    }

    // Discover project names from projects/ submodules
    let project_names: Vec<String> = if projects_dir.is_dir() {
        fs::read_dir(&projects_dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let project_name_refs: Vec<&str> = project_names.iter().map(|s| s.as_str()).collect();

    // Re-assemble agent dir from team/ submodule paths (idempotent)
    assemble_agent_dir_submodule(ws_root, member_dir_name, &project_name_refs, coding_agent)?;
    if verbose {
        println!("  Rebuilt agent dir symlinks");
    }

    // Commit changes if any, then push
    git_cmd(ws_root, &["add", "-A"])?;
    let has_changes = git_cmd(ws_root, &["diff", "--cached", "--quiet"]).is_err();
    if has_changes {
        git_cmd(ws_root, &["commit", "-m", "Sync workspace with team repo"])?;
        if verbose {
            println!("  Committed workspace changes");
        }
        if push {
            git_cmd(ws_root, &["push"]).with_context(|| {
                "Failed to push workspace changes. \
                 Ensure the workspace repo has a remote configured."
            })?;
            if verbose {
                println!("  Pushed to remote");
            }
        }
    } else if verbose {
        println!("  No changes to commit");
    }

    Ok(())
}

/// Checks out the member branch in a submodule, creating it if needed.
/// Avoids leaving the submodule in detached HEAD state.
fn checkout_member_branch(sub_dir: &Path, member_dir_name: &str, verbose: bool) -> Result<()> {
    // Check current branch
    let current = git_cmd_output(sub_dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_default();
    let current = current.trim();

    if current == member_dir_name {
        if verbose {
            println!("    Branch: {} (already on it)", member_dir_name);
        }
        return Ok(());
    }

    // Try checkout existing, fall back to creating
    if git_cmd(sub_dir, &["checkout", member_dir_name]).is_ok() {
        if verbose {
            println!("    Branch: {} (checked out)", member_dir_name);
        }
    } else {
        git_cmd(sub_dir, &["checkout", "-b", member_dir_name])?;
        if verbose {
            println!("    Branch: {} (created)", member_dir_name);
        }
    }
    Ok(())
}

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

// ── Private helpers ──────────────────────────────────────────────────

/// Symlinks all `.md` files from `src_dir` into `dst_dir` using relative paths.
/// Silently returns Ok if `src_dir` does not exist.
fn symlink_md_files(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    if !src_dir.is_dir() {
        return Ok(());
    }

    let canonical_src = fs::canonicalize(src_dir)
        .with_context(|| format!("Failed to canonicalize {}", src_dir.display()))?;
    let canonical_dst = fs::canonicalize(dst_dir)
        .with_context(|| format!("Failed to canonicalize {}", dst_dir.display()))?;
    let rel = relative_path(&canonical_dst, &canonical_src);

    for entry in fs::read_dir(&canonical_src)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let filename = path.file_name().unwrap();
            let dst = dst_dir.join(filename);
            if dst.symlink_metadata().is_ok() {
                fs::remove_file(&dst).ok();
            }
            let rel_target = rel.join(filename);
            unix_fs::symlink(&rel_target, &dst).with_context(|| {
                format!("Failed to symlink {} → {}", dst.display(), rel_target.display())
            })?;
        }
    }

    Ok(())
}

/// Computes a relative path from `from_dir` to `to_path`.
/// Both paths must be absolute (canonicalized).
/// Example: relative_path("/a/b/c", "/a/b/d/e") → "../d/e"
fn relative_path(from_dir: &Path, to_path: &Path) -> PathBuf {
    let from_components: Vec<_> = from_dir.components().collect();
    let to_components: Vec<_> = to_path.components().collect();

    // Find common prefix length
    let common = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut result = PathBuf::new();
    // Go up from `from_dir` to the common ancestor
    for _ in common..from_components.len() {
        result.push("..");
    }
    // Go down from the common ancestor to `to_path`
    for component in &to_components[common..] {
        result.push(component);
    }
    result
}

/// Creates a symlink: `link_path` → `target`.
/// Target can be relative or absolute. Removes existing link/file at `link_path` first.
/// Skips if `target` doesn't exist (resolved relative to link's parent for relative targets).
#[cfg(test)]
fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    // For relative targets, resolve against the link's parent to check existence
    let check_path = if target.is_relative() {
        link_path
            .parent()
            .map(|p| p.join(target))
            .unwrap_or_else(|| target.to_path_buf())
    } else {
        target.to_path_buf()
    };
    if !check_path.exists() {
        return Ok(());
    }
    if link_path.symlink_metadata().is_ok() {
        fs::remove_file(link_path).ok();
    }
    unix_fs::symlink(target, link_path).with_context(|| {
        format!(
            "Failed to symlink {} → {}",
            link_path.display(),
            target.display()
        )
    })
}

/// Copies `src` to `dst` only if `src` exists and is newer than `dst`.
#[cfg(test)]
fn copy_if_newer(src: &Path, dst: &Path) -> Result<()> {
    copy_if_newer_verbose(src, dst)?;
    Ok(())
}

/// Copies `src` to `dst` only if `src` exists and is newer than `dst`.
/// Returns `true` if a copy was made, `false` if skipped.
fn copy_if_newer_verbose(src: &Path, dst: &Path) -> Result<bool> {
    if !src.exists() {
        return Ok(false);
    }
    let should_copy = if dst.exists() {
        let src_mod = fs::metadata(src)?.modified()?;
        let dst_mod = fs::metadata(dst)?.modified()?;
        src_mod > dst_mod
    } else {
        true
    };
    if should_copy {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst).with_context(|| {
            format!("Failed to copy {} → {}", src.display(), dst.display())
        })?;
    }
    Ok(should_copy)
}

/// Verifies a symlink points to the expected target. Re-creates as relative if wrong or broken.
#[cfg(test)]
fn verify_symlink(link: &Path, expected_target: &Path) -> Result<()> {
    if !expected_target.exists() {
        return Ok(());
    }
    let canonical_target = fs::canonicalize(expected_target)
        .with_context(|| format!("Failed to canonicalize {}", expected_target.display()))?;

    // Compute the relative path from the link's parent to the target
    let link_parent = link.parent().unwrap_or(Path::new("."));
    let canonical_parent = fs::canonicalize(link_parent).unwrap_or_else(|_| link_parent.to_path_buf());
    let rel = relative_path(&canonical_parent, &canonical_target);

    let needs_fix = match fs::read_link(link) {
        Ok(current) => {
            // Fix if absolute (we want relative) or if it resolves to a different file
            if current.is_absolute() {
                true
            } else {
                let resolved = link_parent.join(&current);
                match fs::canonicalize(&resolved) {
                    Ok(c) => c != canonical_target,
                    Err(_) => true, // broken symlink
                }
            }
        }
        Err(_) => true,
    };

    if needs_fix {
        if link.symlink_metadata().is_ok() {
            fs::remove_file(link).ok();
        }
        unix_fs::symlink(&rel, link)
            .with_context(|| format!("Failed to re-create symlink {}", link.display()))?;
    }
    Ok(())
}

/// Adds a git submodule, allowing the file protocol for local paths.
///
/// Git 2.38.1+ blocks `file://` transport in submodule adds by default
/// (CVE-2022-39253). We allow it since local clones are intentional here
/// (during `bm teams sync --repos` for local workspace setup). For remote
/// repos, URLs are HTTPS and this config has no effect.
fn git_submodule_add(dir: &Path, url: &str, path: &str) -> Result<()> {
    // Check if submodule already exists via git
    let status = Command::new("git")
        .args(["submodule", "status", path])
        .current_dir(dir)
        .output();
    if let Ok(ref out) = status {
        if out.status.success() {
            // Submodule already registered — skip
            return Ok(());
        }
    }

    let output = Command::new("git")
        .args([
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            url,
            path,
        ])
        .current_dir(dir)
        .output()
        .with_context(|| format!("Failed to run git submodule add {} {}", url, path))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git submodule add {} {} failed: {}",
            url,
            path,
            stderr.trim()
        );
    }
    Ok(())
}

/// Returns the current git branch name for a workspace, or "unknown" on failure.
pub fn workspace_git_branch(ws_root: &Path) -> String {
    git_cmd_output(ws_root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Describes the status of a single git submodule.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmoduleStatus {
    pub name: String,
    pub status: SubmoduleState,
}

/// Whether a submodule is up-to-date or has new commits available.
#[derive(Debug, Clone, PartialEq)]
pub enum SubmoduleState {
    UpToDate,
    Behind,
    Modified,
    Uninitialized,
}

impl SubmoduleState {
    pub fn label(&self) -> &'static str {
        match self {
            SubmoduleState::UpToDate => "up-to-date",
            SubmoduleState::Behind => "behind",
            SubmoduleState::Modified => "modified",
            SubmoduleState::Uninitialized => "uninitialized",
        }
    }
}

/// Returns submodule status for all submodules in a workspace.
///
/// Uses `git submodule status` which prefixes each line with:
/// - ' ' (space) = up-to-date
/// - '+' = checked out to different commit than recorded
/// - '-' = not initialized
/// - 'U' = merge conflict
pub fn workspace_submodule_status(ws_root: &Path) -> Vec<SubmoduleStatus> {
    let output = match git_cmd_output(ws_root, &["submodule", "status"]) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    output
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            // Format: " <hash> <path> (<desc>)" or "+<hash> <path> (<desc>)"
            let first_char = line.chars().next()?;
            let state = match first_char {
                ' ' => SubmoduleState::UpToDate,
                '+' => SubmoduleState::Modified,
                '-' => SubmoduleState::Uninitialized,
                _ => SubmoduleState::Behind,
            };
            // Extract the path (second whitespace-delimited field after the hash)
            let rest = line[1..].trim();
            let path = rest.split_whitespace().nth(1)?;
            Some(SubmoduleStatus {
                name: path.to_string(),
                status: state,
            })
        })
        .collect()
}

/// Returns the remote URL for a workspace repo, or None if not available.
pub fn workspace_remote_url(ws_root: &Path) -> Option<String> {
    git_cmd_output(ws_root, &["remote", "get-url", "origin"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Runs a git command in the given directory. Returns `Ok(())` on success.
fn git_cmd(dir: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("Failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

/// Runs a git command and returns stdout as a String.
fn git_cmd_output(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("Failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a `CodingAgentDef` for Claude Code, used by most tests.
    fn claude_code_agent() -> CodingAgentDef {
        CodingAgentDef {
            name: "claude-code".into(),
            display_name: "Claude Code".into(),
            context_file: "CLAUDE.md".into(),
            agent_dir: ".claude".into(),
            binary: "claude".into(),
        }
    }


    // ── Symlink edge cases ──────────────────────────────────────────

    #[test]
    fn create_symlink_replaces_regular_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("real.md");
        fs::write(&target, "# Real").unwrap();

        let link = tmp.path().join("link.md");
        fs::write(&link, "# Regular file occupying the path").unwrap();

        // create_symlink should remove the regular file and create a symlink
        create_symlink(&target, &link).unwrap();

        assert!(
            link.symlink_metadata().unwrap().file_type().is_symlink(),
            "Should be a symlink, not a regular file"
        );
        assert_eq!(fs::read_to_string(&link).unwrap(), "# Real");
    }

    #[test]
    fn verify_symlink_fixes_broken_link() {
        let tmp = tempfile::tempdir().unwrap();

        // Create the correct target
        let correct_target = tmp.path().join("correct.md");
        fs::write(&correct_target, "# Correct").unwrap();

        // Create a broken symlink (pointing to a non-existent path)
        let link = tmp.path().join("link.md");
        let ghost = tmp.path().join("ghost.md");
        unix_fs::symlink(&ghost, &link).unwrap();

        // Link exists as symlink but is broken (ghost doesn't exist)
        assert!(link.symlink_metadata().is_ok(), "Symlink metadata readable");
        assert!(!link.exists(), "Broken symlink — target doesn't exist");

        // verify_symlink should detect the mismatch and re-create
        verify_symlink(&link, &correct_target).unwrap();

        assert!(link.exists(), "Link should now resolve");
        assert_eq!(fs::read_to_string(&link).unwrap(), "# Correct");
    }

    #[test]
    fn verify_symlink_fixes_wrong_target() {
        let tmp = tempfile::tempdir().unwrap();

        let correct = tmp.path().join("correct.md");
        fs::write(&correct, "# Correct").unwrap();

        let wrong = tmp.path().join("wrong.md");
        fs::write(&wrong, "# Wrong").unwrap();

        let link = tmp.path().join("link.md");
        unix_fs::symlink(&wrong, &link).unwrap();
        assert_eq!(fs::read_to_string(&link).unwrap(), "# Wrong");

        verify_symlink(&link, &correct).unwrap();

        assert_eq!(
            fs::read_to_string(&link).unwrap(),
            "# Correct",
            "verify_symlink should re-point to the correct target"
        );
    }

    // ── Sync behavior (submodule model) ─────────────────────────────

    /// Helper: create a workspace using the submodule model for sync tests.
    fn setup_syncable_workspace(tmp: &Path) -> (std::path::PathBuf, String, CodingAgentDef) {
        let member = "arch-01"; // Must match setup_team_repo_for_ws member
        let team_repo = setup_team_repo_for_ws(tmp);
        let workspace_base = tmp.join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();
        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, member, &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join(member);
        (ws, member.to_string(), agent)
    }

    #[test]
    fn sync_recopies_changed_ralph_yml() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Verify initial content
        assert_eq!(
            fs::read_to_string(ws.join("ralph.yml")).unwrap(),
            "v: 1"
        );

        // Modify ralph.yml in team/ submodule (simulating upstream change)
        let source = ws.join("team/members").join(&member).join("ralph.yml");
        fs::write(&source, "updated: true").unwrap();

        // Ensure source is newer
        let now = filetime::FileTime::from_unix_time(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                + 2,
            0,
        );
        filetime::set_file_mtime(&source, now).unwrap();

        sync_workspace(&ws, &member, &agent, false, false).unwrap();

        assert_eq!(
            fs::read_to_string(ws.join("ralph.yml")).unwrap(),
            "updated: true",
            "Sync should re-copy the updated ralph.yml"
        );
    }

    #[test]
    fn sync_reassembles_agent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        let agents_dir = ws.join(".claude/agents");
        let initial_count = fs::read_dir(&agents_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .count();

        // Add a new agent file in team/ submodule
        let member_agents = ws
            .join("team/members")
            .join(&member)
            .join("coding-agent/agents");
        fs::create_dir_all(&member_agents).unwrap();
        fs::write(member_agents.join("new-agent.md"), "# New Agent").unwrap();

        sync_workspace(&ws, &member, &agent, false, false).unwrap();

        let new_count = fs::read_dir(&agents_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .count();
        assert!(
            new_count > initial_count,
            "Agent count should increase after sync: {} > {}",
            new_count,
            initial_count
        );
        assert!(
            agents_dir.join("new-agent.md").exists(),
            "new-agent.md should be symlinked after sync"
        );
    }

    #[test]
    fn sync_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Run sync twice
        sync_workspace(&ws, &member, &agent, false, false).unwrap();
        sync_workspace(&ws, &member, &agent, false, false).unwrap();

        // Verify workspace is still correct
        assert!(ws.join("PROMPT.md").exists());
        assert!(ws.join("CLAUDE.md").exists());
        assert!(ws.join("ralph.yml").exists());
        assert!(ws.join(".claude/agents").is_dir());
        assert_eq!(fs::read_to_string(ws.join("PROMPT.md")).unwrap(), "# P");
    }

    #[test]
    fn sync_commits_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Modify a context file in the team submodule and commit it
        // (simulating an upstream change that arrives via submodule update)
        let team_sub = ws.join("team");
        let source = team_sub.join("members").join(&member).join("ralph.yml");
        fs::write(&source, "updated: true").unwrap();
        git_cmd(&team_sub, &["add", "-A"]).unwrap();
        git_cmd(&team_sub, &["commit", "-m", "upstream change"]).unwrap();

        // Ensure source is newer than workspace copy
        let now = filetime::FileTime::from_unix_time(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                + 2,
            0,
        );
        filetime::set_file_mtime(&source, now).unwrap();

        sync_workspace(&ws, &member, &agent, false, false).unwrap();

        // The workspace ralph.yml should have the updated content
        assert_eq!(
            fs::read_to_string(ws.join("ralph.yml")).unwrap(),
            "updated: true",
            "Sync should re-copy the updated ralph.yml"
        );

        // Working tree should be clean after sync (changes committed)
        let status = git_cmd_output(&ws, &["status", "--porcelain"]).unwrap();
        assert!(
            status.trim().is_empty(),
            "Working tree should be clean after sync, got: {}",
            status
        );
    }

    #[test]
    fn sync_skips_unchanged_files() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Sync once — no changes expected
        sync_workspace(&ws, &member, &agent, false, false).unwrap();

        // Count commits before and after a second sync
        let log_before = git_cmd_output(&ws, &["rev-list", "--count", "HEAD"]).unwrap();

        sync_workspace(&ws, &member, &agent, false, false).unwrap();

        let log_after = git_cmd_output(&ws, &["rev-list", "--count", "HEAD"]).unwrap();
        assert_eq!(
            log_before.trim(),
            log_after.trim(),
            "No new commits should be created when nothing changed"
        );
    }

    #[test]
    fn sync_member_branch_in_team_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // After initial create, team/ submodule should be on member branch
        let team_sub = ws.join("team");
        let branch = git_cmd_output(&team_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(branch.trim(), member, "team/ should be on member branch");

        // Sync and verify branch is still correct (not detached HEAD)
        sync_workspace(&ws, &member, &agent, false, false).unwrap();

        let branch = git_cmd_output(&team_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch.trim(),
            member,
            "team/ should remain on member branch after sync (not detached HEAD)"
        );
    }

    #[test]
    fn sync_verbose_runs_without_error() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Verbose mode should complete without error
        sync_workspace(&ws, &member, &agent, true, false).unwrap();

        // Workspace should still be valid
        assert!(ws.join("PROMPT.md").exists());
        assert!(ws.join("CLAUDE.md").exists());
        assert!(ws.join("ralph.yml").exists());
    }

    // ── copy_if_newer ───────────────────────────────────────────────

    #[test]
    fn copy_if_newer_skips_when_dest_newer() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        fs::write(&src, "old source").unwrap();
        fs::write(&dst, "newer dest").unwrap();

        // Make source older than dest
        let old_time = filetime::FileTime::from_unix_time(1_000_000, 0);
        filetime::set_file_mtime(&src, old_time).unwrap();
        let new_time = filetime::FileTime::from_unix_time(2_000_000, 0);
        filetime::set_file_mtime(&dst, new_time).unwrap();

        copy_if_newer(&src, &dst).unwrap();

        assert_eq!(
            fs::read_to_string(&dst).unwrap(),
            "newer dest",
            "Destination should be unchanged when it is newer"
        );
    }

    #[test]
    fn copy_if_newer_copies_when_source_newer() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        fs::write(&src, "newer source").unwrap();
        fs::write(&dst, "old dest").unwrap();

        // Make source newer than dest
        let old_time = filetime::FileTime::from_unix_time(1_000_000, 0);
        filetime::set_file_mtime(&dst, old_time).unwrap();
        let new_time = filetime::FileTime::from_unix_time(2_000_000, 0);
        filetime::set_file_mtime(&src, new_time).unwrap();

        copy_if_newer(&src, &dst).unwrap();

        assert_eq!(
            fs::read_to_string(&dst).unwrap(),
            "newer source",
            "Destination should be overwritten when source is newer"
        );
    }

    #[test]
    fn copy_if_newer_copies_when_dest_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        fs::write(&src, "content").unwrap();

        assert!(!dst.exists());
        copy_if_newer(&src, &dst).unwrap();

        assert_eq!(
            fs::read_to_string(&dst).unwrap(),
            "content",
            "Should copy when destination does not exist"
        );
    }

    #[test]
    fn copy_if_newer_skips_when_source_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("nonexistent.txt");
        let dst = tmp.path().join("dst.txt");
        fs::write(&dst, "preserved").unwrap();

        copy_if_newer(&src, &dst).unwrap();

        assert_eq!(
            fs::read_to_string(&dst).unwrap(),
            "preserved",
            "Should be a no-op when source doesn't exist"
        );
    }


    // ── Workspace repo (submodule model) ──────────────────────────────

    /// Helper: creates a minimal team repo with git, member config, and optional projects.
    fn setup_team_repo_for_ws(tmp: &Path) -> PathBuf {
        let team_repo = tmp.join("team_repo");
        let member_cfg = team_repo.join("members/arch-01");
        fs::create_dir_all(&member_cfg).unwrap();
        fs::write(member_cfg.join("PROMPT.md"), "# P").unwrap();
        fs::write(member_cfg.join("CLAUDE.md"), "# C").unwrap();
        fs::write(member_cfg.join("ralph.yml"), "v: 1").unwrap();
        fs::create_dir_all(member_cfg.join("coding-agent/agents")).unwrap();
        fs::create_dir_all(team_repo.join("coding-agent/agents")).unwrap();

        git_cmd(&team_repo, &["init", "-b", "main"]).unwrap();
        git_cmd(&team_repo, &["config", "user.email", "test@test"]).unwrap();
        git_cmd(&team_repo, &["config", "user.name", "Test"]).unwrap();
        git_cmd(&team_repo, &["add", "-f", "-A"]).unwrap();
        git_cmd(&team_repo, &["commit", "-m", "init"]).unwrap();

        team_repo
    }

    /// Helper: creates a fake fork repo for submodule tests.
    fn setup_fork_repo(tmp: &Path, name: &str) -> PathBuf {
        let fork = tmp.join(name);
        fs::create_dir_all(&fork).unwrap();
        git_cmd(&fork, &["init", "-b", "main"]).unwrap();
        git_cmd(&fork, &["config", "user.email", "test@test"]).unwrap();
        git_cmd(&fork, &["config", "user.name", "Test"]).unwrap();
        fs::write(fork.join("README.md"), format!("# {}", name)).unwrap();
        git_cmd(&fork, &["add", "-A"]).unwrap();
        git_cmd(&fork, &["commit", "-m", "init fork"]).unwrap();
        fork
    }

    /// Helper: creates workspace repo params for tests (local-only mode).
    fn test_ws_params<'a>(
        team_repo: &'a Path,
        workspace_base: &'a Path,
        member: &'a str,
        projects: &'a [(&'a str, &'a str)],
        coding_agent: &'a CodingAgentDef,
    ) -> WorkspaceRepoParams<'a> {
        WorkspaceRepoParams {
            team_repo_path: team_repo,
            workspace_base,
            member_dir_name: member,
            team_name: "my-team",
            projects,
            github_repo: None,
            push: false,
            gh_token: None,
            coding_agent,
        }
    }

    #[test]
    fn workspace_repo_creates_git_repo_with_team_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");

        // Workspace should be a git repo
        assert!(ws.join(".git").exists(), "workspace should be a git repo");

        // team/ should be a submodule
        assert!(ws.join("team").is_dir(), "team/ submodule should exist");
        assert!(ws.join(".gitmodules").exists(), ".gitmodules should exist");

        // .gitmodules should reference the team submodule
        let gitmodules = fs::read_to_string(ws.join(".gitmodules")).unwrap();
        assert!(
            gitmodules.contains("[submodule \"team\"]"),
            ".gitmodules should contain team submodule entry"
        );
    }

    #[test]
    fn workspace_repo_member_branch_in_team_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let team_sub = workspace_base.join("arch-01/team");
        let branch = git_cmd_output(&team_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch.trim(),
            "arch-01",
            "team submodule should be on the member branch"
        );
    }

    #[test]
    fn workspace_repo_with_projects_creates_submodules() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let fork_a = setup_fork_repo(tmp.path(), "project-a");
        let fork_b = setup_fork_repo(tmp.path(), "project-b");

        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let fork_a_url = fork_a.to_string_lossy().to_string();
        let fork_b_url = fork_b.to_string_lossy().to_string();
        let projects: Vec<(&str, &str)> = vec![
            ("project-a", &fork_a_url),
            ("project-b", &fork_b_url),
        ];

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &projects, &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");

        // Project submodules should exist
        assert!(
            ws.join("projects/project-a").is_dir(),
            "projects/project-a/ should exist"
        );
        assert!(
            ws.join("projects/project-b").is_dir(),
            "projects/project-b/ should exist"
        );

        // .gitmodules should reference all submodules
        let gitmodules = fs::read_to_string(ws.join(".gitmodules")).unwrap();
        assert!(
            gitmodules.contains("projects/project-a"),
            ".gitmodules should contain project-a"
        );
        assert!(
            gitmodules.contains("projects/project-b"),
            ".gitmodules should contain project-b"
        );

        // Project submodules should contain the fork content
        assert!(
            ws.join("projects/project-a/README.md").exists(),
            "project-a should have README.md from fork"
        );
        assert!(
            ws.join("projects/project-b/README.md").exists(),
            "project-b should have README.md from fork"
        );
    }

    #[test]
    fn workspace_repo_member_branch_in_project_submodules() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let fork = setup_fork_repo(tmp.path(), "my-project");

        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let fork_url = fork.to_string_lossy().to_string();
        let projects: Vec<(&str, &str)> = vec![("my-project", &fork_url)];

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &projects, &agent);
        create_workspace_repo(&params).unwrap();

        let proj_sub = workspace_base.join("arch-01/projects/my-project");
        let branch =
            git_cmd_output(&proj_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch.trim(),
            "arch-01",
            "project submodule should be on the member branch"
        );
    }

    #[test]
    fn workspace_repo_no_projects_has_no_projects_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");
        assert!(
            !ws.join("projects").exists(),
            "projects/ should not exist when no projects are configured"
        );
    }

    #[test]
    fn workspace_repo_team_submodule_has_member_files() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");

        // The team submodule should contain the team repo content
        assert!(
            ws.join("team/members/arch-01/PROMPT.md").exists(),
            "team submodule should have member PROMPT.md"
        );
        assert!(
            ws.join("team/members/arch-01/CLAUDE.md").exists(),
            "team submodule should have member CLAUDE.md"
        );
        assert!(
            ws.join("team/members/arch-01/ralph.yml").exists(),
            "team submodule should have member ralph.yml"
        );
    }

    // ── Context assembly (submodule model) ────────────────────────────

    #[test]
    fn workspace_repo_copies_context_files_to_root() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");

        // Context files should exist at workspace root as copies (not symlinks)
        assert!(ws.join("CLAUDE.md").exists(), "CLAUDE.md at workspace root");
        assert!(ws.join("PROMPT.md").exists(), "PROMPT.md at workspace root");
        assert!(ws.join("ralph.yml").exists(), "ralph.yml at workspace root");

        // Verify they are regular files, not symlinks
        assert!(
            !ws.join("CLAUDE.md").symlink_metadata().unwrap().file_type().is_symlink(),
            "CLAUDE.md should be a copy, not a symlink"
        );
        assert!(
            !ws.join("PROMPT.md").symlink_metadata().unwrap().file_type().is_symlink(),
            "PROMPT.md should be a copy, not a symlink"
        );

        // Verify content matches source
        assert_eq!(fs::read_to_string(ws.join("CLAUDE.md")).unwrap(), "# C");
        assert_eq!(fs::read_to_string(ws.join("PROMPT.md")).unwrap(), "# P");
        assert_eq!(fs::read_to_string(ws.join("ralph.yml")).unwrap(), "v: 1");
    }

    #[test]
    fn workspace_repo_assembles_agent_dir_from_team_submodule() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a team repo with agent files at team and member levels
        let team_repo = tmp.path().join("team_repo");
        let member_cfg = team_repo.join("members/arch-01");
        fs::create_dir_all(&member_cfg).unwrap();
        fs::write(member_cfg.join("PROMPT.md"), "# P").unwrap();
        fs::write(member_cfg.join("CLAUDE.md"), "# C").unwrap();
        fs::write(member_cfg.join("ralph.yml"), "v: 1").unwrap();

        // Team-level agent
        let team_agents = team_repo.join("coding-agent/agents");
        fs::create_dir_all(&team_agents).unwrap();
        fs::write(team_agents.join("team-agent.md"), "# Team Agent").unwrap();

        // Member-level agent
        let member_agents = member_cfg.join("coding-agent/agents");
        fs::create_dir_all(&member_agents).unwrap();
        fs::write(member_agents.join("member-agent.md"), "# Member Agent").unwrap();

        git_cmd(&team_repo, &["init", "-b", "main"]).unwrap();
        git_cmd(&team_repo, &["config", "user.email", "test@test"]).unwrap();
        git_cmd(&team_repo, &["config", "user.name", "Test"]).unwrap();
        git_cmd(&team_repo, &["add", "-f", "-A"]).unwrap();
        git_cmd(&team_repo, &["commit", "-m", "init"]).unwrap();

        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");
        let agents_dir = ws.join(".claude/agents");

        // Agent dir should exist
        assert!(agents_dir.is_dir(), ".claude/agents/ should exist");

        // Team-level and member-level agents should be symlinked
        assert!(agents_dir.join("team-agent.md").exists(), "team-agent.md should exist");
        assert!(agents_dir.join("member-agent.md").exists(), "member-agent.md should exist");

        // They should be symlinks
        assert!(
            agents_dir.join("team-agent.md").symlink_metadata().unwrap().file_type().is_symlink(),
            "team-agent.md should be a symlink"
        );
        assert!(
            agents_dir.join("member-agent.md").symlink_metadata().unwrap().file_type().is_symlink(),
            "member-agent.md should be a symlink"
        );

        // Symlinks should resolve to content in team/ submodule
        assert_eq!(
            fs::read_to_string(agents_dir.join("team-agent.md")).unwrap(),
            "# Team Agent"
        );
        assert_eq!(
            fs::read_to_string(agents_dir.join("member-agent.md")).unwrap(),
            "# Member Agent"
        );
    }

    #[test]
    fn workspace_repo_agent_dir_three_scopes() {
        let tmp = tempfile::tempdir().unwrap();

        // Create team repo with agent files at all three scopes
        let team_repo = tmp.path().join("team_repo");
        let member_cfg = team_repo.join("members/arch-01");
        fs::create_dir_all(&member_cfg).unwrap();
        fs::write(member_cfg.join("PROMPT.md"), "# P").unwrap();
        fs::write(member_cfg.join("CLAUDE.md"), "# C").unwrap();
        fs::write(member_cfg.join("ralph.yml"), "v: 1").unwrap();

        // Team-level
        let team_agents = team_repo.join("coding-agent/agents");
        fs::create_dir_all(&team_agents).unwrap();
        fs::write(team_agents.join("team-wide.md"), "# Team").unwrap();

        // Project-level
        let proj_agents = team_repo.join("projects/myproj/coding-agent/agents");
        fs::create_dir_all(&proj_agents).unwrap();
        fs::write(proj_agents.join("project-specific.md"), "# Project").unwrap();

        // Member-level
        let member_agents = member_cfg.join("coding-agent/agents");
        fs::create_dir_all(&member_agents).unwrap();
        fs::write(member_agents.join("member-only.md"), "# Member").unwrap();

        git_cmd(&team_repo, &["init", "-b", "main"]).unwrap();
        git_cmd(&team_repo, &["config", "user.email", "test@test"]).unwrap();
        git_cmd(&team_repo, &["config", "user.name", "Test"]).unwrap();
        git_cmd(&team_repo, &["add", "-f", "-A"]).unwrap();
        git_cmd(&team_repo, &["commit", "-m", "init"]).unwrap();

        // Create a fake project fork
        let fork = setup_fork_repo(tmp.path(), "myproj");
        let fork_url = fork.to_string_lossy().to_string();
        let projects: Vec<(&str, &str)> = vec![("myproj", &fork_url)];

        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &projects, &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");
        let agents_dir = ws.join(".claude/agents");

        // All three scopes should be present
        assert!(agents_dir.join("team-wide.md").exists(), "Team agent missing");
        assert!(agents_dir.join("project-specific.md").exists(), "Project agent missing");
        assert!(agents_dir.join("member-only.md").exists(), "Member agent missing");

        // All should be symlinks
        for name in &["team-wide.md", "project-specific.md", "member-only.md"] {
            assert!(
                agents_dir.join(name).symlink_metadata().unwrap().file_type().is_symlink(),
                "{} should be a symlink",
                name
            );
        }
    }


    #[test]
    fn workspace_repo_writes_marker_file() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");
        assert!(ws.join(".botminter.workspace").exists(), ".botminter.workspace marker should exist");

        let marker = fs::read_to_string(ws.join(".botminter.workspace")).unwrap();
        assert!(marker.contains("member: arch-01"), "marker should contain member name");
    }

    #[test]
    fn workspace_repo_commits_all_files() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");

        // Check that there's a commit
        let log = git_cmd_output(&ws, &["log", "--oneline", "-1"]).unwrap();
        assert!(
            log.contains("Initial workspace setup"),
            "should have initial commit, got: {}",
            log.trim()
        );

        // Working tree should be clean (all files committed)
        let status = git_cmd_output(&ws, &["status", "--porcelain"]).unwrap();
        assert!(
            status.trim().is_empty(),
            "working tree should be clean after commit, got: {}",
            status.trim()
        );
    }

    #[test]
    fn workspace_repo_symlinks_resolve_into_team_submodule() {
        let tmp = tempfile::tempdir().unwrap();

        // Create team repo with a team-level agent
        let team_repo = tmp.path().join("team_repo");
        let member_cfg = team_repo.join("members/arch-01");
        fs::create_dir_all(&member_cfg).unwrap();
        fs::write(member_cfg.join("PROMPT.md"), "# P").unwrap();
        fs::write(member_cfg.join("CLAUDE.md"), "# C").unwrap();
        fs::write(member_cfg.join("ralph.yml"), "v: 1").unwrap();
        fs::create_dir_all(member_cfg.join("coding-agent/agents")).unwrap();

        let team_agents = team_repo.join("coding-agent/agents");
        fs::create_dir_all(&team_agents).unwrap();
        fs::write(team_agents.join("checker.md"), "# Check").unwrap();

        git_cmd(&team_repo, &["init", "-b", "main"]).unwrap();
        git_cmd(&team_repo, &["config", "user.email", "test@test"]).unwrap();
        git_cmd(&team_repo, &["config", "user.name", "Test"]).unwrap();
        git_cmd(&team_repo, &["add", "-f", "-A"]).unwrap();
        git_cmd(&team_repo, &["commit", "-m", "init"]).unwrap();

        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();

        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, "arch-01", &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join("arch-01");
        let link = ws.join(".claude/agents/checker.md");

        // The symlink target should contain "team/" (pointing into the submodule)
        let target = fs::read_link(&link).unwrap();
        let target_str = target.to_string_lossy();
        assert!(
            target_str.contains("team/"),
            "symlink should point into team/ submodule, got: {}",
            target_str
        );
    }

    // ── workspace_git_branch ──────────────────────────────────────

    #[test]
    fn workspace_git_branch_returns_branch_name() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        git_cmd(ws, &["init", "-b", "my-feature"]).unwrap();
        git_cmd(ws, &["config", "user.email", "test@test.com"]).unwrap();
        git_cmd(ws, &["config", "user.name", "Test"]).unwrap();
        fs::write(ws.join("README.md"), "hello").unwrap();
        git_cmd(ws, &["add", "."]).unwrap();
        git_cmd(ws, &["commit", "-m", "init"]).unwrap();

        let branch = workspace_git_branch(ws);
        assert_eq!(branch, "my-feature");
    }

    #[test]
    fn workspace_git_branch_returns_unknown_for_non_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let branch = workspace_git_branch(tmp.path());
        assert_eq!(branch, "unknown");
    }

    // ── workspace_submodule_status ────────────────────────────────

    #[test]
    fn workspace_submodule_status_with_submodule() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a "remote" repo to use as a submodule
        let remote = tmp.path().join("remote");
        fs::create_dir_all(&remote).unwrap();
        git_cmd(&remote, &["init", "-b", "main"]).unwrap();
        git_cmd(&remote, &["config", "user.email", "test@test.com"]).unwrap();
        git_cmd(&remote, &["config", "user.name", "Test"]).unwrap();
        fs::write(remote.join("file.txt"), "hello").unwrap();
        git_cmd(&remote, &["add", "."]).unwrap();
        git_cmd(&remote, &["commit", "-m", "init"]).unwrap();

        // Create workspace repo with a submodule
        let ws = tmp.path().join("ws");
        fs::create_dir_all(&ws).unwrap();
        git_cmd(&ws, &["init", "-b", "main"]).unwrap();
        git_cmd(&ws, &["config", "user.email", "test@test.com"]).unwrap();
        git_cmd(&ws, &["config", "user.name", "Test"]).unwrap();
        git_cmd(
            &ws,
            &[
                "-c", "protocol.file.allow=always",
                "submodule", "add", remote.to_str().unwrap(), "team",
            ],
        )
        .unwrap();
        git_cmd(&ws, &["commit", "-m", "add submodule"]).unwrap();

        let subs = workspace_submodule_status(&ws);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "team");
        assert_eq!(subs[0].status, SubmoduleState::UpToDate);
    }

    #[test]
    fn workspace_submodule_status_empty_for_non_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let subs = workspace_submodule_status(tmp.path());
        assert!(subs.is_empty());
    }

    // ── workspace_remote_url ──────────────────────────────────────

    #[test]
    fn workspace_remote_url_returns_none_without_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        git_cmd(ws, &["init", "-b", "main"]).unwrap();
        git_cmd(ws, &["config", "user.email", "test@test.com"]).unwrap();
        git_cmd(ws, &["config", "user.name", "Test"]).unwrap();

        let url = workspace_remote_url(ws);
        assert!(url.is_none());
    }

    #[test]
    fn workspace_remote_url_returns_url_with_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        git_cmd(ws, &["init", "-b", "main"]).unwrap();
        git_cmd(ws, &["config", "user.email", "test@test.com"]).unwrap();
        git_cmd(ws, &["config", "user.name", "Test"]).unwrap();
        git_cmd(
            ws,
            &["remote", "add", "origin", "https://github.com/org/repo.git"],
        )
        .unwrap();

        let url = workspace_remote_url(ws);
        assert_eq!(url, Some("https://github.com/org/repo.git".to_string()));
    }

    // ── SubmoduleState ────────────────────────────────────────────

    #[test]
    fn submodule_state_labels() {
        assert_eq!(SubmoduleState::UpToDate.label(), "up-to-date");
        assert_eq!(SubmoduleState::Behind.label(), "behind");
        assert_eq!(SubmoduleState::Modified.label(), "modified");
        assert_eq!(SubmoduleState::Uninitialized.label(), "uninitialized");
    }

    // ── inject_robot_config ──────────────────────────────────────

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
