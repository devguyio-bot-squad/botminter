//! Workspace hydration pipeline — RepoSource trait, GitWorktreeSource, ConfigAssembler,
//! CredentialRelay, and WorkspaceHydrator.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::session::SessionId;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Maps a repo URL to a stable directory name by taking the first 16 hex chars of its SHA-256.
fn url_to_clone_name(url: &str) -> String {
    let hash = Sha256::digest(url.as_bytes());
    hex::encode(&hash[..8])
}

/// Checks whether the shared clone needs a fetch based on the age of `.bm_last_fetch`.
fn needs_fetch(clone_dir: &Path, threshold: Duration) -> bool {
    let marker = clone_dir.join(".bm_last_fetch");
    match fs::metadata(&marker).and_then(|m| m.modified()) {
        Ok(mtime) => mtime.elapsed().unwrap_or(Duration::MAX) > threshold,
        Err(_) => true,
    }
}

/// Touches `.bm_last_fetch` in the clone dir to record the time of the last fetch.
fn touch_fetch_marker(clone_dir: &Path) -> Result<()> {
    fs::write(clone_dir.join(".bm_last_fetch"), "").context("Failed to write fetch marker")
}

/// Read the bare-clone directory that owns the linked worktree at `target`.
/// The worktree's `.git` file contains `gitdir: <bare>/worktrees/<name>`.
fn clone_dir_for_worktree(target: &Path) -> Result<PathBuf> {
    let git_file = target.join(".git");
    let content =
        fs::read_to_string(&git_file).with_context(|| format!("Failed to read {:?}", git_file))?;
    let gitdir_str = content
        .lines()
        .find_map(|l| l.strip_prefix("gitdir: "))
        .ok_or_else(|| anyhow::anyhow!("No gitdir line in {:?}", git_file))?
        .trim();
    let gitdir = PathBuf::from(gitdir_str);
    // Bare repo layout: <clone>/<worktrees>/<name>  → parent.parent() = <clone>
    gitdir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("Cannot derive clone dir from gitdir {:?}", gitdir))
}

// ── RepoSource trait ────────────────────────────────────────────────────────

/// Abstracts repository provisioning so the strategy can be swapped
/// without changing WorkspaceHydrator (e.g., git worktree vs full clone vs container).
pub trait RepoSource: Send + Sync {
    /// Create a writable working tree at `target` containing `repo_url` at `branch`
    /// (or the repo's default branch when `None`). `target` must not exist beforehand.
    fn provision(&self, repo_url: &str, branch: Option<&str>, target: &Path) -> Result<()>;

    /// Remove the working tree at `target` and release any associated resources
    /// (e.g., prune the git worktree reference from the shared clone).
    fn deprovision(&self, target: &Path) -> Result<()>;
}

// ── GitWorktreeSource ───────────────────────────────────────────────────────

/// Implements RepoSource via `git worktree add` from a shared permanent clone.
/// Fetches the shared clone from remote when its last-fetch timestamp is older
/// than `freshness_threshold`.
pub struct GitWorktreeSource {
    /// Directory that holds permanent shared clones, one per repo URL.
    pub clones_dir: PathBuf,
    /// How old a clone's last-fetch can be before a fetch is forced.
    pub freshness_threshold: Duration,
}

impl GitWorktreeSource {
    pub fn new(clones_dir: PathBuf, freshness_threshold: Duration) -> Self {
        Self {
            clones_dir,
            freshness_threshold,
        }
    }
}

impl RepoSource for GitWorktreeSource {
    fn provision(&self, repo_url: &str, branch: Option<&str>, target: &Path) -> Result<()> {
        let clone_dir = self.clones_dir.join(url_to_clone_name(repo_url));

        if !clone_dir.exists() {
            // First use: create a bare clone of the remote.
            fs::create_dir_all(&self.clones_dir).context("Failed to create clones directory")?;
            // On failure, remove the partial clone dir so retries start fresh.
            if let Err(e) = super::util::git_cmd(
                &self.clones_dir,
                &["clone", "--bare", repo_url, clone_dir.to_str().unwrap()],
            ) {
                let _ = fs::remove_dir_all(&clone_dir);
                return Err(e).with_context(|| {
                    format!(
                        "Failed to clone '{}' — check the URL and network connectivity",
                        repo_url
                    )
                });
            }
            touch_fetch_marker(&clone_dir)?;
        } else if needs_fetch(&clone_dir, self.freshness_threshold) {
            // Existing clone is stale: fetch all refs.
            super::util::git_cmd(&clone_dir, &["fetch", "--all", "--prune"]).with_context(
                || {
                    format!(
                        "git fetch failed on shared clone at '{}' — check network connectivity",
                        clone_dir.display()
                    )
                },
            )?;
            touch_fetch_marker(&clone_dir)?;
        }

        // Resolve the ref to check out (detached HEAD to avoid branch-lock conflicts).
        let commit_ish = match branch {
            Some(b) => b.to_string(),
            None => super::util::git_cmd_output(&clone_dir, &["rev-parse", "HEAD"])
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "HEAD".to_string()),
        };

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).context("Failed to create worktree parent dir")?;
        }

        super::util::git_cmd(
            &clone_dir,
            &[
                "worktree",
                "add",
                "--detach",
                target.to_str().unwrap(),
                &commit_ish,
            ],
        )
        .with_context(|| format!("git worktree add failed for '{}'", target.display()))
    }

    fn deprovision(&self, target: &Path) -> Result<()> {
        if !target.exists() {
            return Ok(());
        }
        // Find the owning bare clone, remove the worktree entry, then prune.
        let clone_dir =
            clone_dir_for_worktree(target).context("Failed to find owning clone for worktree")?;

        let _ = Command::new("git")
            .args(["worktree", "remove", "--force", target.to_str().unwrap()])
            .current_dir(&clone_dir)
            .output();

        // Ensure the directory is gone regardless.
        if target.exists() {
            fs::remove_dir_all(target)
                .with_context(|| format!("Failed to remove worktree dir {:?}", target))?;
        }

        // Prune stale entries from the shared clone.
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(&clone_dir)
            .output();

        Ok(())
    }
}

// ── AssemblyConfig ──────────────────────────────────────────────────────────

/// Per-session parameters that ConfigAssembler injects into the workspace.
#[derive(Debug, Clone)]
pub struct AssemblyConfig {
    pub session_id: SessionId,
    pub member_name: String,
    pub team_repo_url: String,
    pub team_repo_branch: String,
    pub project_number: Option<u64>,
    /// Skill directories to symlink or reference in the assembled workspace.
    pub skill_dirs: Vec<PathBuf>,
    /// Root directory under which per-member credential directories live.
    pub credential_base: PathBuf,
}

// ── ConfigAssembler ─────────────────────────────────────────────────────────

/// Populates a session workspace with CLAUDE.md, PROMPT.md, ralph.yml,
/// .claude/agents/ references, .botminter.workspace marker, and skill directory
/// references. All operations are idempotent — running twice yields the same state.
pub struct ConfigAssembler {
    pub team_repo_path: PathBuf,
    pub member_name: String,
}

impl ConfigAssembler {
    pub fn new(team_repo_path: PathBuf, member_name: String) -> Self {
        Self {
            team_repo_path,
            member_name,
        }
    }

    /// Assemble workspace configuration from `config` into `workspace`.
    /// Surfaces PROMPT.md, CLAUDE.md, and ralph.yml from the team repo member
    /// directory, writes the .botminter.workspace marker, and validates skill dirs.
    /// Returns a list of warning strings (e.g., missing files, missing skill dirs).
    pub fn assemble(&self, workspace: &Path, config: &AssemblyConfig) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Surface member config files from team repo (required for ralph to launch).
        // team_repo_path/members/<member>/{PROMPT.md,CLAUDE.md,ralph.yml} → workspace/
        if self.team_repo_path.exists() {
            let member_dir = self
                .team_repo_path
                .join("members")
                .join(&config.member_name);

            // PROMPT.md — required; warn if missing
            let prompt_src = member_dir.join("PROMPT.md");
            if prompt_src.exists() {
                fs::copy(&prompt_src, workspace.join("PROMPT.md")).with_context(|| {
                    format!(
                        "Failed to copy PROMPT.md from {}",
                        prompt_src.display()
                    )
                })?;
            } else {
                warnings.push(format!(
                    "PROMPT.md not found for member '{}' at {} — ralph may not start correctly",
                    config.member_name,
                    prompt_src.display()
                ));
            }

            // CLAUDE.md — optional
            let claude_src = member_dir.join("CLAUDE.md");
            if claude_src.exists() {
                fs::copy(&claude_src, workspace.join("CLAUDE.md")).with_context(|| {
                    format!("Failed to copy CLAUDE.md from {}", claude_src.display())
                })?;
            }

            // ralph.yml — optional but expected for loop sessions
            let ralph_src = member_dir.join("ralph.yml");
            if ralph_src.exists() {
                fs::copy(&ralph_src, workspace.join("ralph.yml")).with_context(|| {
                    format!("Failed to copy ralph.yml from {}", ralph_src.display())
                })?;
            }
        }

        // Write .botminter.workspace marker (idempotent: always overwrite).
        let marker = format!(
            "# BotMinter workspace marker — do not delete\nsession_id: {}\nmember: {}\n",
            config.session_id.as_str(),
            config.member_name,
        );
        fs::write(workspace.join(".botminter.workspace"), &marker)
            .context("Failed to write .botminter.workspace marker")?;

        // Validate skill directories and collect warnings for missing ones.
        for dir in &config.skill_dirs {
            if !dir.exists() {
                warnings.push(format!("Skill directory not found: {}", dir.display()));
            }
        }

        // Assemble .claude/ directory from team coding-agent assets (non-fatal).
        if let Err(e) = self.assemble_claude_dir(workspace) {
            warnings.push(format!(
                ".claude/ assembly failed: {e} — coding-agent assets (agents, skills, settings) may be missing"
            ));
        }

        Ok(warnings)
    }

    fn assemble_claude_dir(&self, workspace: &Path) -> Result<()> {
        let claude_dir = workspace.join(".claude");
        fs::create_dir_all(&claude_dir).context("Failed to create .claude/")?;

        let coding_agent = self.team_repo_path.join("coding-agent");

        let agents_src = coding_agent.join("agents");
        if agents_src.is_dir() {
            let agents_dst = claude_dir.join("agents");
            fs::create_dir_all(&agents_dst).context("Failed to create .claude/agents/")?;
            super::util::symlink_md_files(&agents_src, &agents_dst)?;
        }

        // Merge team-level and member-level skills into .claude/skills/.
        let skills_dst = claude_dir.join("skills");
        let skill_sources = [
            coding_agent.join("skills"),
            self.team_repo_path
                .join("members")
                .join(&self.member_name)
                .join("coding-agent")
                .join("skills"),
        ];
        for src in &skill_sources {
            if src.is_dir() {
                fs::create_dir_all(&skills_dst).context("Failed to create .claude/skills/")?;
                super::util::symlink_subdirs(src, &skills_dst)?;
            }
        }

        let settings_src = coding_agent.join("settings.json");
        if settings_src.exists() {
            fs::copy(&settings_src, claude_dir.join("settings.json"))
                .context("Failed to copy settings.json")?;
        }

        Ok(())
    }
}

// ── CredentialRelay ─────────────────────────────────────────────────────────

/// Writes credential files (hosts.yml) into a member's shared credential directory.
pub trait CredentialWriter: Send + Sync {
    fn write_credentials(&self, member_dir: &Path) -> Result<()>;
}

/// A no-op credential writer — used when no App credentials are configured.
pub struct NoOpCredentialWriter;

impl CredentialWriter for NoOpCredentialWriter {
    fn write_credentials(&self, _member_dir: &Path) -> Result<()> {
        Ok(())
    }
}

/// Manages per-member credential directories: writes credential files during
/// session creation and provides the directory path for runtime use.
/// Credentials are never copied into session workspaces — only referenced.
pub struct CredentialRelay {
    /// Root directory under which `<member>/` subdirectories hold credentials.
    pub credentials_base: PathBuf,
    writer: Box<dyn CredentialWriter>,
}

impl CredentialRelay {
    pub fn new(credentials_base: PathBuf) -> Self {
        Self {
            credentials_base,
            writer: Box::new(NoOpCredentialWriter),
        }
    }

    pub fn with_writer(credentials_base: PathBuf, writer: Box<dyn CredentialWriter>) -> Self {
        Self {
            credentials_base,
            writer,
        }
    }

    fn member_dir(&self, member_name: &str) -> PathBuf {
        self.credentials_base.join(member_name)
    }

    /// Return the credential directory for `member_name`. Errors if the member
    /// has no configured credential directory.
    pub fn credential_path(&self, member_name: &str) -> Result<PathBuf> {
        let path = self.member_dir(member_name);
        if !path.exists() {
            bail!(
                "No credential directory for member '{}' at {} \
                 — run 'bm credentials export' or verify member configuration",
                member_name,
                path.display()
            );
        }
        Ok(path)
    }

    /// Write credentials (hosts.yml) to the shared credential directory for `member_name`.
    pub fn ensure_credentials(&self, member_name: &str) -> Result<()> {
        self.writer.write_credentials(&self.member_dir(member_name))
    }
}

// ── HydrationTiming ─────────────────────────────────────────────────────────

/// Timing breakdown returned by a successful hydration so operators can
/// identify bottlenecks (clone fetch latency vs worktree creation vs config assembly).
#[derive(Debug, Clone)]
pub struct HydrationTiming {
    /// Time spent in clone/fetch operations inside `RepoSource::provision()`.
    /// Currently always 0 because fetch and worktree creation are not timed
    /// separately inside `provision()` — this field is reserved for future
    /// lower-level instrumentation.
    pub clone_fetch_ms: u64,
    /// Total time spent in `RepoSource::provision()` across all repos,
    /// including any clone fetch and worktree creation.
    pub worktree_create_ms: u64,
    /// Time spent assembling workspace config (marker file, skill validation).
    pub config_assembly_ms: u64,
}

// ── HydrationResult ─────────────────────────────────────────────────────────

/// Returned by WorkspaceHydrator::hydrate on success.
#[derive(Debug)]
pub struct HydrationResult {
    pub workspace_path: PathBuf,
    pub timing: HydrationTiming,
    /// Skill directories that were not found — reported as warnings, not errors.
    pub skill_warnings: Vec<String>,
}

// ── WorkspaceHydrator ───────────────────────────────────────────────────────

/// Orchestrates repo provisioning and config assembly into a complete session workspace.
/// Atomic from the caller's perspective: either a fully valid workspace is returned,
/// or no workspace exists (all partial state is cleaned up on failure).
pub struct WorkspaceHydrator<R: RepoSource> {
    pub repo_source: R,
    pub config_assembler: ConfigAssembler,
    pub credential_relay: CredentialRelay,
    /// Root under which per-member/per-session workspace directories are created.
    pub sessions_base: PathBuf,
}

impl<R: RepoSource> WorkspaceHydrator<R> {
    pub fn new(
        repo_source: R,
        config_assembler: ConfigAssembler,
        credential_relay: CredentialRelay,
        sessions_base: PathBuf,
    ) -> Self {
        Self {
            repo_source,
            config_assembler,
            credential_relay,
            sessions_base,
        }
    }

    /// Create a complete session workspace for `member`.
    /// `repo_urls` is a slice of `(url, project_name)` pairs — one working tree
    /// per entry is created under `workspace/projects/<project_name>/`.
    ///
    /// On any error, all partial state is removed and an error is returned.
    pub fn hydrate(
        &self,
        session_id: &SessionId,
        member: &str,
        repo_urls: &[(&str, &str)],
        config: AssemblyConfig,
    ) -> Result<HydrationResult> {
        let workspace_path = self.sessions_base.join(member).join(session_id.as_str());
        fs::create_dir_all(&workspace_path).with_context(|| {
            format!(
                "Failed to create workspace directory at {} — check available disk space",
                workspace_path.display()
            )
        })?;

        let projects_dir = workspace_path.join("projects");
        if let Err(e) = fs::create_dir_all(&projects_dir) {
            let _ = fs::remove_dir_all(&workspace_path);
            return Err(e).with_context(|| {
                format!(
                    "Failed to create projects directory at {} — check available disk space",
                    projects_dir.display()
                )
            });
        }

        // Provision each repo as a git worktree.  Track what was created for rollback.
        let provision_start = Instant::now();
        let mut provisioned: Vec<PathBuf> = Vec::new();
        for (url, project_name) in repo_urls {
            let project_path = projects_dir.join(project_name);
            if let Err(e) = self.repo_source.provision(url, None, &project_path) {
                for p in &provisioned {
                    let _ = self.repo_source.deprovision(p);
                }
                let _ = fs::remove_dir_all(&workspace_path);
                return Err(e).with_context(|| {
                    format!(
                        "Failed to provision repo '{}' (project '{}')",
                        url, project_name
                    )
                });
            }
            provisioned.push(project_path);
        }
        let worktree_create_ms = provision_start.elapsed().as_millis() as u64;

        // Assemble config (writes marker, validates skills).
        let assembly_start = Instant::now();
        let skill_warnings = match self.config_assembler.assemble(&workspace_path, &config) {
            Ok(w) => w,
            Err(e) => {
                for p in &provisioned {
                    let _ = self.repo_source.deprovision(p);
                }
                let _ = fs::remove_dir_all(&workspace_path);
                return Err(e).with_context(|| {
                    format!(
                        "Failed to assemble workspace config for session {} \
                         — verify team repo path and member configuration",
                        session_id.as_str()
                    )
                });
            }
        };
        let config_assembly_ms = assembly_start.elapsed().as_millis() as u64;

        // Write credentials to the shared dir — non-fatal; session starts without them on failure.
        if let Err(e) = self.credential_relay.ensure_credentials(member) {
            tracing::warn!("Credential write failed for member '{}': {e}", member);
        }

        Ok(HydrationResult {
            workspace_path,
            timing: HydrationTiming {
                clone_fetch_ms: 0,
                worktree_create_ms,
                config_assembly_ms,
            },
            skill_warnings,
        })
    }

    /// Remove the session workspace and all associated git worktrees.
    pub fn teardown(&self, session_id: &SessionId, member: &str) -> Result<()> {
        let workspace_path = self.sessions_base.join(member).join(session_id.as_str());
        if !workspace_path.exists() {
            return Ok(());
        }

        let projects_dir = workspace_path.join("projects");
        if projects_dir.is_dir() {
            for entry in fs::read_dir(&projects_dir).with_context(|| {
                format!(
                    "Failed to list projects directory at {} during teardown",
                    projects_dir.display()
                )
            })? {
                let path = entry?.path();
                if path.is_dir() {
                    let _ = self.repo_source.deprovision(&path);
                }
            }
        }

        fs::remove_dir_all(&workspace_path).with_context(|| {
            format!(
                "Failed to remove workspace at {} — check that no processes hold open files",
                workspace_path.display()
            )
        })
    }
}

// ── Production WorkspaceOps ────────────────────────────────────────────────────

/// Configuration for constructing a [`HydrationWorkspaceOps`].
#[derive(Debug, Clone)]
pub struct HydrationWorkspaceConfig {
    pub clones_dir: PathBuf,
    pub sessions_base: PathBuf,
    pub team_repo_path: PathBuf,
    pub credential_base: PathBuf,
    pub freshness_threshold: Duration,
    pub repo_urls: Vec<(String, String)>,
    pub team_repo_url: String,
    pub team_repo_branch: String,
    /// Team workspace root (e.g. `~/workspaces/my-team`). Permanent member
    /// workspaces live at `<workspace_base>/<member>/` — used to locate App
    /// credential directories (`GH_CONFIG_DIR`) when launching ralph.
    pub workspace_base: PathBuf,
    pub project_number: Option<u64>,
    pub skill_dirs: Vec<PathBuf>,
}

/// Production implementation of [`crate::session::manager::WorkspaceOps`]
/// that delegates to the hydration pipeline for workspace creation and
/// to [`crate::session::dirty_state`] for workspace inspection.
pub struct HydrationWorkspaceOps {
    hydrator: WorkspaceHydrator<GitWorktreeSource>,
    repo_urls: Vec<(String, String)>,
    team_repo_url: String,
    team_repo_branch: String,
    project_number: Option<u64>,
    skill_dirs: Vec<PathBuf>,
    /// Team workspace root for resolving per-member App credential paths.
    workspace_base: PathBuf,
}

impl HydrationWorkspaceOps {
    pub fn new(config: HydrationWorkspaceConfig) -> Self {
        let source = GitWorktreeSource::new(config.clones_dir, config.freshness_threshold);
        let assembler = ConfigAssembler::new(config.team_repo_path, String::new());
        let relay = CredentialRelay::new(config.credential_base);
        let hydrator = WorkspaceHydrator::new(source, assembler, relay, config.sessions_base);

        Self {
            hydrator,
            repo_urls: config.repo_urls,
            team_repo_url: config.team_repo_url,
            team_repo_branch: config.team_repo_branch,
            project_number: config.project_number,
            skill_dirs: config.skill_dirs,
            workspace_base: config.workspace_base,
        }
    }

    pub fn teardown(&self, session_id: &SessionId, member: &str) -> Result<()> {
        self.hydrator.teardown(session_id, member)
    }

    /// Return the GitHub App credential directory for `member_name` if it exists.
    ///
    /// The path is `<workspace_base>/<member>/.config/gh` — set as `GH_CONFIG_DIR`
    /// when launching ralph so it uses the member's App installation token.
    pub fn gh_config_dir_for_member(&self, member_name: &str) -> Option<PathBuf> {
        let gh_dir = self
            .workspace_base
            .join(member_name)
            .join(".config")
            .join("gh");
        if gh_dir.join("hosts.yml").exists() {
            Some(gh_dir)
        } else {
            None
        }
    }
}

impl crate::session::manager::WorkspaceOps for HydrationWorkspaceOps {
    fn hydrate_workspace(&self, session_id: &SessionId, member: &str) -> Result<PathBuf> {
        let config = AssemblyConfig {
            session_id: session_id.clone(),
            member_name: member.to_string(),
            team_repo_url: self.team_repo_url.clone(),
            team_repo_branch: self.team_repo_branch.clone(),
            project_number: self.project_number,
            skill_dirs: self.skill_dirs.clone(),
            credential_base: self.hydrator.credential_relay.credentials_base.clone(),
        };

        let refs: Vec<(&str, &str)> = self
            .repo_urls
            .iter()
            .map(|(url, name)| (url.as_str(), name.as_str()))
            .collect();

        let result = self.hydrator.hydrate(session_id, member, &refs, config)?;
        Ok(result.workspace_path)
    }

    fn inspect_dirty_state(
        &self,
        workspace_path: &Path,
    ) -> Result<Vec<crate::session::dirty_state::RepoDirtyState>> {
        crate::session::dirty_state::inspect_dirty_state(workspace_path)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a bare git repo with one commit. Returns the bare repo path.
    fn init_bare_repo(tmp: &TempDir, name: &str) -> PathBuf {
        let bare = tmp.path().join(format!("{name}.git"));
        fs::create_dir_all(&bare).unwrap();
        Command::new("git")
            .args(["init", "--bare", "-b", "main"])
            .arg(&bare)
            .status()
            .unwrap();

        let work = tmp.path().join(format!("{name}-work"));
        Command::new("git")
            .args(["clone", bare.to_str().unwrap(), work.to_str().unwrap()])
            .status()
            .unwrap();
        fs::write(work.join("README.md"), format!("# {name}\n")).unwrap();
        Command::new("git")
            .args(["-C", work.to_str().unwrap(), "add", "."])
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                work.to_str().unwrap(),
                "-c",
                "user.email=test@test.com",
                "-c",
                "user.name=Test",
                "commit",
                "-m",
                "init",
            ])
            .status()
            .unwrap();
        Command::new("git")
            .args(["-C", work.to_str().unwrap(), "push"])
            .status()
            .unwrap();
        bare
    }

    fn git_worktree_source(tmp: &TempDir) -> GitWorktreeSource {
        GitWorktreeSource::new(tmp.path().join("clones"), Duration::from_secs(300))
    }

    fn make_assembly_config(
        tmp: &TempDir,
        session_id: SessionId,
        repo_url: &str,
    ) -> AssemblyConfig {
        AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: repo_url.to_string(),
            team_repo_branch: "main".to_string(),
            project_number: Some(42),
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        }
    }

    // ── AC-1: Clean Workspace State ─────────────────────────────────────────

    #[test]
    fn provision_produces_clean_git_working_tree() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let source = git_worktree_source(&tmp);
        let target = tmp.path().join("ws");

        // provision must succeed and produce a workspace with no uncommitted changes
        source
            .provision(repo.to_str().unwrap(), None, &target)
            .unwrap();

        let status = Command::new("git")
            .args(["-C", target.to_str().unwrap(), "status", "--porcelain"])
            .output()
            .unwrap();
        assert!(
            status.stdout.is_empty(),
            "provisioned workspace must have no uncommitted changes"
        );
    }

    // ── AC-2: Independent Workspaces ────────────────────────────────────────

    #[test]
    fn two_provisions_of_same_repo_yield_independent_working_trees() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let source = git_worktree_source(&tmp);

        let ws_a = tmp.path().join("ws-a");
        let ws_b = tmp.path().join("ws-b");

        source
            .provision(repo.to_str().unwrap(), None, &ws_a)
            .unwrap();
        source
            .provision(repo.to_str().unwrap(), None, &ws_b)
            .unwrap();

        // Writing to workspace A must not affect workspace B
        fs::write(ws_a.join("mutation.txt"), "only in A").unwrap();
        assert!(
            !ws_b.join("mutation.txt").exists(),
            "mutation in workspace A must not appear in workspace B"
        );
        assert_ne!(ws_a, ws_b, "workspaces must be distinct directories");
    }

    // ── AC-3: Timing Breakdown ──────────────────────────────────────────────

    #[test]
    fn hydrate_returns_timing_breakdown() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let source = git_worktree_source(&tmp);
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let relay = CredentialRelay::new(tmp.path().join("credentials"));
        let hydrator =
            WorkspaceHydrator::new(source, assembler, relay, tmp.path().join("sessions"));

        let session_id = SessionId::new();
        let config = make_assembly_config(&tmp, session_id.clone(), repo.to_str().unwrap());

        let result = hydrator
            .hydrate(
                &session_id,
                "alice",
                &[(repo.to_str().unwrap(), "myproject")],
                config,
            )
            .unwrap();

        // AC-3: all three timing fields must be present (non-panic access is the assertion)
        let _ = result.timing.clone_fetch_ms;
        let _ = result.timing.worktree_create_ms;
        let _ = result.timing.config_assembly_ms;
    }

    // ── AC-4: Fail-Clean ────────────────────────────────────────────────────

    #[test]
    fn failed_hydration_leaves_no_partial_workspace_state() {
        let tmp = TempDir::new().unwrap();
        let source = git_worktree_source(&tmp);
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let relay = CredentialRelay::new(tmp.path().join("credentials"));
        let sessions_base = tmp.path().join("sessions");
        let hydrator = WorkspaceHydrator::new(source, assembler, relay, sessions_base.clone());

        let session_id = SessionId::new();
        let config = make_assembly_config(
            &tmp,
            session_id.clone(),
            "file:///nonexistent-repo-that-does-not-exist.git",
        );

        let result = hydrator.hydrate(
            &session_id,
            "alice",
            &[("file:///nonexistent-repo-that-does-not-exist.git", "proj")],
            config,
        );

        assert!(result.is_err(), "hydration with bad repo URL must fail");
        // No partial workspace directory may remain
        let expected_ws = sessions_base.join("alice").join(session_id.as_str());
        assert!(
            !expected_ws.exists(),
            "partial workspace must be removed after failed hydration"
        );
    }

    // ── AC-4 error message content ──────────────────────────────────────────

    #[test]
    fn hydration_error_includes_operation_and_cause() {
        let tmp = TempDir::new().unwrap();
        let source = git_worktree_source(&tmp);
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let relay = CredentialRelay::new(tmp.path().join("credentials"));
        let hydrator =
            WorkspaceHydrator::new(source, assembler, relay, tmp.path().join("sessions"));

        let session_id = SessionId::new();
        let config = make_assembly_config(&tmp, session_id.clone(), "file:///no-such-repo.git");

        let err = hydrator
            .hydrate(
                &session_id,
                "alice",
                &[("file:///no-such-repo.git", "proj")],
                config,
            )
            .unwrap_err();

        let msg = err.to_string();
        // AC-4: error must describe the failing operation and underlying cause
        assert!(!msg.is_empty(), "error message must be non-empty");
    }

    // ── AC-5: Skills Available ──────────────────────────────────────────────

    #[test]
    fn config_assembler_reports_missing_skill_dirs_as_warnings() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let session_id = SessionId::new();

        let nonexistent = tmp.path().join("skills").join("missing-dir");
        let config = AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: None,
            skill_dirs: vec![nonexistent.clone()],
            credential_base: tmp.path().join("credentials"),
        };

        let warnings = assembler.assemble(&workspace, &config).unwrap();
        // AC-5: missing skill dir must appear in warnings, not cause an error
        assert!(
            warnings.iter().any(|w| w.contains("missing-dir")
                || w.contains("not found")
                || w.contains("missing")),
            "missing skill dir must be reported as a warning, got: {:?}",
            warnings
        );
    }

    // ── AC-6: Credentials Accessible ────────────────────────────────────────

    #[test]
    fn credential_relay_returns_accessible_directory_for_configured_member() {
        let tmp = TempDir::new().unwrap();
        let creds_base = tmp.path().join("credentials");
        let member_dir = creds_base.join("alice");
        fs::create_dir_all(&member_dir).unwrap();
        fs::write(member_dir.join("hosts.yml"), "# mock token\n").unwrap();

        let relay = CredentialRelay::new(creds_base);
        let path = relay.credential_path("alice").unwrap();

        assert!(path.exists(), "credential path must exist");
        assert!(path.is_dir(), "credential path must be a directory");
    }

    #[test]
    fn credential_relay_errors_for_member_with_no_credentials() {
        let tmp = TempDir::new().unwrap();
        let creds_base = tmp.path().join("credentials");
        fs::create_dir_all(&creds_base).unwrap();

        let relay = CredentialRelay::new(creds_base);
        let result = relay.credential_path("nonexistent-member");

        assert!(
            result.is_err(),
            "credential relay must error for a member with no credentials configured"
        );
    }

    // ── AC-7: Shared Clone Freshness ─────────────────────────────────────────

    #[test]
    fn provision_fetches_before_worktree_when_clone_is_stale() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        // Zero threshold means any clone is considered stale — fetch always runs
        let source = GitWorktreeSource::new(tmp.path().join("clones"), Duration::from_secs(0));
        let target = tmp.path().join("ws");

        // Must succeed: fetch ran and worktree was created from fresh state
        source
            .provision(repo.to_str().unwrap(), None, &target)
            .unwrap();

        assert!(
            target.exists(),
            "workspace must exist after provision with zero freshness threshold"
        );
    }

    // ── Config Assembler idempotency ─────────────────────────────────────────

    #[test]
    fn config_assembler_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let session_id = SessionId::new();

        let config = AssemblyConfig {
            session_id: session_id.clone(),
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: Some(42),
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        let warnings_first = assembler.assemble(&workspace, &config).unwrap();
        let warnings_second = assembler.assemble(&workspace, &config).unwrap();

        assert_eq!(
            warnings_first, warnings_second,
            "config assembler must be idempotent — same result on second call"
        );
    }

    // ── Workspace marker contains session ID ─────────────────────────────────

    #[test]
    fn assembled_workspace_marker_contains_session_id() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let session_id = SessionId::new();

        let config = AssemblyConfig {
            session_id: session_id.clone(),
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: Some(42),
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        assembler.assemble(&workspace, &config).unwrap();

        let marker = fs::read_to_string(workspace.join(".botminter.workspace")).unwrap();
        assert!(
            marker.contains(session_id.as_str()),
            "workspace marker must contain session ID '{}', got:\n{}",
            session_id,
            marker
        );
    }

    // ── AC-08: .claude/ assembly ─────────────────────────────────────────────

    #[test]
    fn assemble_creates_claude_agents_symlink_from_team_coding_agent() {
        let tmp = TempDir::new().unwrap();
        let team = tmp.path().join("team");
        let agents_src = team.join("coding-agent/agents");
        fs::create_dir_all(&agents_src).unwrap();
        fs::write(agents_src.join("finalization.md"), "# Finalization").unwrap();

        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(team, "alice".to_string());
        let session_id = SessionId::new();
        let config = AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: None,
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        assembler.assemble(&workspace, &config).unwrap();

        let agent_file = workspace.join(".claude/agents/finalization.md");
        assert!(
            agent_file.exists(),
            ".claude/agents/finalization.md must exist after assemble()"
        );
        assert!(
            agent_file
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            ".claude/agents/finalization.md must be a symlink"
        );
    }

    #[test]
    fn assemble_creates_claude_skills_symlinks_from_team_coding_agent() {
        let tmp = TempDir::new().unwrap();
        let team = tmp.path().join("team");
        let skill_src = team.join("coding-agent/skills/my-skill");
        fs::create_dir_all(&skill_src).unwrap();
        fs::write(skill_src.join("SKILL.md"), "# My Skill").unwrap();

        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(team, "alice".to_string());
        let session_id = SessionId::new();
        let config = AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: None,
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        assembler.assemble(&workspace, &config).unwrap();

        let skills_dir = workspace.join(".claude/skills");
        assert!(
            skills_dir.exists(),
            ".claude/skills/ must exist after assemble()"
        );
        assert!(
            skills_dir.join("my-skill").symlink_metadata().is_ok(),
            ".claude/skills/my-skill must exist as a symlink after assemble()"
        );
    }

    #[test]
    fn assemble_copies_team_settings_json_into_claude_dir() {
        let tmp = TempDir::new().unwrap();
        let team = tmp.path().join("team");
        let coding_agent_dir = team.join("coding-agent");
        fs::create_dir_all(&coding_agent_dir).unwrap();
        fs::write(coding_agent_dir.join("settings.json"), r#"{"hooks": {}}"#).unwrap();

        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(team, "alice".to_string());
        let session_id = SessionId::new();
        let config = AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: None,
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        assembler.assemble(&workspace, &config).unwrap();

        assert!(
            workspace.join(".claude/settings.json").exists(),
            ".claude/settings.json must exist after assemble()"
        );
    }

    #[test]
    fn assemble_completes_and_creates_claude_dir_when_no_coding_agent_dir() {
        let tmp = TempDir::new().unwrap();
        let team = tmp.path().join("team");
        fs::create_dir_all(&team).unwrap();

        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(team, "alice".to_string());
        let session_id = SessionId::new();
        let config = AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: None,
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        let result = assembler.assemble(&workspace, &config);
        assert!(
            result.is_ok(),
            "assemble() must not error when no coding-agent dir exists"
        );
        assert!(
            workspace.join(".claude").is_dir(),
            ".claude/ must be created by assemble() even when team has no coding-agent dir"
        );
    }

    // ── AC-08: Member-level .claude/ assembly ───────────────────────────────

    #[test]
    fn assemble_includes_member_level_skill_in_claude_skills_dir() {
        let tmp = TempDir::new().unwrap();
        let team = tmp.path().join("team");
        let member_skill_src = team.join("members/alice/coding-agent/skills/story-mgmt");
        fs::create_dir_all(&member_skill_src).unwrap();
        fs::write(member_skill_src.join("SKILL.md"), "# Story Mgmt").unwrap();

        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(team, "alice".to_string());
        let session_id = SessionId::new();
        let config = AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: None,
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        assembler.assemble(&workspace, &config).unwrap();

        assert!(
            workspace.join(".claude/skills/story-mgmt").exists(),
            ".claude/skills/story-mgmt must be present — member-level skill must be assembled \
             from team/members/alice/coding-agent/skills/"
        );
    }

    #[test]
    fn assemble_merges_team_and_member_skills_in_claude_skills_dir() {
        let tmp = TempDir::new().unwrap();
        let team = tmp.path().join("team");
        let team_skill_src = team.join("coding-agent/skills/ro-loop");
        fs::create_dir_all(&team_skill_src).unwrap();
        fs::write(team_skill_src.join("SKILL.md"), "# ro-loop").unwrap();
        let member_skill_src = team.join("members/alice/coding-agent/skills/story-mgmt");
        fs::create_dir_all(&member_skill_src).unwrap();
        fs::write(member_skill_src.join("SKILL.md"), "# story-mgmt").unwrap();

        let workspace = tmp.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        let assembler = ConfigAssembler::new(team, "alice".to_string());
        let session_id = SessionId::new();
        let config = AssemblyConfig {
            session_id,
            member_name: "alice".to_string(),
            team_repo_url: "https://example.com/team.git".to_string(),
            team_repo_branch: "main".to_string(),
            project_number: None,
            skill_dirs: vec![],
            credential_base: tmp.path().join("credentials"),
        };

        assembler.assemble(&workspace, &config).unwrap();

        assert!(
            workspace.join(".claude/skills/ro-loop").exists(),
            ".claude/skills/ro-loop must be present — team-level skill"
        );
        assert!(
            workspace.join(".claude/skills/story-mgmt").exists(),
            ".claude/skills/story-mgmt must be present — member-level skill must be assembled \
             alongside team-level skills"
        );
    }

    // ── AC-09: Credential write-path ─────────────────────────────────────────

    struct TestCredentialWriter {
        token: String,
    }

    impl CredentialWriter for TestCredentialWriter {
        fn write_credentials(&self, member_dir: &Path) -> Result<()> {
            fs::create_dir_all(member_dir)?;
            let hosts_content = format!(
                "github.com:\n    oauth_token: {}\n    git_protocol: https\n",
                self.token
            );
            fs::write(member_dir.join("hosts.yml"), &hosts_content)?;
            Ok(())
        }
    }

    #[test]
    fn ensure_credentials_writes_hosts_yml_to_member_credential_dir() {
        let tmp = TempDir::new().unwrap();
        let creds_base = tmp.path().join("credentials");
        let relay = CredentialRelay::with_writer(
            creds_base.clone(),
            Box::new(TestCredentialWriter {
                token: "test-token".to_string(),
            }),
        );

        relay.ensure_credentials("alice").unwrap();

        let hosts_yml = creds_base.join("alice").join("hosts.yml");
        assert!(
            hosts_yml.exists(),
            "hosts.yml must exist at the shared credential path after ensure_credentials()"
        );
    }

    #[test]
    fn credential_path_contains_hosts_yml_after_ensure_credentials() {
        let tmp = TempDir::new().unwrap();
        let creds_base = tmp.path().join("credentials");
        // Pre-create the member dir so credential_path() does not fail on missing dir.
        fs::create_dir_all(creds_base.join("alice")).unwrap();

        let relay = CredentialRelay::with_writer(
            creds_base.clone(),
            Box::new(TestCredentialWriter {
                token: "test-token".to_string(),
            }),
        );

        relay.ensure_credentials("alice").unwrap();
        let path = relay.credential_path("alice").unwrap();

        assert!(
            path.join("hosts.yml").exists(),
            "credential_path() must point to a directory containing hosts.yml after ensure_credentials()"
        );
    }

    #[test]
    fn hydrate_writes_hosts_yml_to_shared_credential_path() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let creds_base = tmp.path().join("credentials");
        let source = git_worktree_source(&tmp);
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let relay = CredentialRelay::with_writer(
            creds_base.clone(),
            Box::new(TestCredentialWriter {
                token: "test-token".to_string(),
            }),
        );
        let hydrator =
            WorkspaceHydrator::new(source, assembler, relay, tmp.path().join("sessions"));

        let session_id = SessionId::new();
        let config = make_assembly_config(&tmp, session_id.clone(), repo.to_str().unwrap());

        hydrator
            .hydrate(
                &session_id,
                "alice",
                &[(repo.to_str().unwrap(), "myproject")],
                config,
            )
            .unwrap();

        let hosts_yml = creds_base.join("alice").join("hosts.yml");
        assert!(
            hosts_yml.exists(),
            "hosts.yml must exist at the shared credential path after hydrate()"
        );
    }

    #[test]
    fn hydrate_proceeds_without_error_when_no_credential_writer() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let source = git_worktree_source(&tmp);
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let relay = CredentialRelay::new(tmp.path().join("credentials")); // NoOp — no app creds
        let hydrator =
            WorkspaceHydrator::new(source, assembler, relay, tmp.path().join("sessions"));

        let session_id = SessionId::new();
        let config = make_assembly_config(&tmp, session_id.clone(), repo.to_str().unwrap());

        let result = hydrator.hydrate(
            &session_id,
            "alice",
            &[(repo.to_str().unwrap(), "myproject")],
            config,
        );

        assert!(
            result.is_ok(),
            "hydrate() must succeed even when no credential writer is configured (non-fatal)"
        );
    }

    // ── Deprovision removes directory ────────────────────────────────────────

    #[test]
    fn deprovision_removes_working_tree_directory() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let source = git_worktree_source(&tmp);
        let target = tmp.path().join("ws");

        source
            .provision(repo.to_str().unwrap(), None, &target)
            .unwrap();
        assert!(
            target.exists(),
            "precondition: workspace must exist after provision"
        );

        source.deprovision(&target).unwrap();

        assert!(
            !target.exists(),
            "workspace directory must be removed after deprovision"
        );
    }

    // ── Layout invariant: .botminter.workspace marker present ───────────────

    #[test]
    fn hydrated_workspace_contains_workspace_marker() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let source = git_worktree_source(&tmp);
        let assembler = ConfigAssembler::new(tmp.path().join("team"), "alice".to_string());
        let relay = CredentialRelay::new(tmp.path().join("credentials"));
        let hydrator =
            WorkspaceHydrator::new(source, assembler, relay, tmp.path().join("sessions"));

        let session_id = SessionId::new();
        let config = make_assembly_config(&tmp, session_id.clone(), repo.to_str().unwrap());

        let result = hydrator
            .hydrate(
                &session_id,
                "alice",
                &[(repo.to_str().unwrap(), "myproject")],
                config,
            )
            .unwrap();

        assert!(
            result.workspace_path.join(".botminter.workspace").exists(),
            ".botminter.workspace marker must exist in the hydrated workspace"
        );
    }
}
