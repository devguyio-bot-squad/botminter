//! Workspace hydration pipeline — RepoSource trait, GitWorktreeSource, ConfigAssembler,
//! CredentialRelay, and WorkspaceHydrator. All implementation methods are stubs (todo!())
//! so tests in this module fail until the green phase provides real implementations.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;

use crate::session::SessionId;

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
    fn provision(&self, _repo_url: &str, _branch: Option<&str>, _target: &Path) -> Result<()> {
        todo!("GitWorktreeSource::provision")
    }

    fn deprovision(&self, _target: &Path) -> Result<()> {
        todo!("GitWorktreeSource::deprovision")
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
    /// Returns a list of skill-validation warning strings (e.g., missing skill
    /// directories are warnings, not hard errors).
    pub fn assemble(&self, _workspace: &Path, _config: &AssemblyConfig) -> Result<Vec<String>> {
        todo!("ConfigAssembler::assemble")
    }
}

// ── CredentialRelay ─────────────────────────────────────────────────────────

/// Returns the shared per-member credential directory path.
/// Credentials are never copied into session workspaces — only referenced.
pub struct CredentialRelay {
    /// Root directory under which `<member>/` subdirectories hold credentials.
    pub credentials_base: PathBuf,
}

impl CredentialRelay {
    pub fn new(credentials_base: PathBuf) -> Self {
        Self { credentials_base }
    }

    /// Return the credential directory for `member_name`. Errors if the member
    /// has no configured credential directory.
    pub fn credential_path(&self, _member_name: &str) -> Result<PathBuf> {
        todo!("CredentialRelay::credential_path")
    }
}

// ── HydrationTiming ─────────────────────────────────────────────────────────

/// Timing breakdown returned by a successful hydration so operators can
/// identify bottlenecks (clone fetch latency vs worktree creation vs config assembly).
#[derive(Debug, Clone)]
pub struct HydrationTiming {
    pub clone_fetch_ms: u64,
    pub worktree_create_ms: u64,
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
        _session_id: &SessionId,
        _member: &str,
        _repo_urls: &[(&str, &str)],
        _config: AssemblyConfig,
    ) -> Result<HydrationResult> {
        todo!("WorkspaceHydrator::hydrate")
    }

    /// Remove the session workspace and all associated git worktrees.
    pub fn teardown(&self, _session_id: &SessionId, _member: &str) -> Result<()> {
        todo!("WorkspaceHydrator::teardown")
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

    fn make_assembly_config(tmp: &TempDir, session_id: SessionId, repo_url: &str) -> AssemblyConfig {
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
        source.provision(repo.to_str().unwrap(), None, &target).unwrap();

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

        source.provision(repo.to_str().unwrap(), None, &ws_a).unwrap();
        source.provision(repo.to_str().unwrap(), None, &ws_b).unwrap();

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
        let hydrator =
            WorkspaceHydrator::new(source, assembler, relay, sessions_base.clone());

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
        let config = make_assembly_config(
            &tmp,
            session_id.clone(),
            "file:///no-such-repo.git",
        );

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
        assert!(
            !msg.is_empty(),
            "error message must be non-empty"
        );
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
            warnings
                .iter()
                .any(|w| w.contains("missing-dir") || w.contains("not found") || w.contains("missing")),
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
        source.provision(repo.to_str().unwrap(), None, &target).unwrap();

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

        let marker =
            fs::read_to_string(workspace.join(".botminter.workspace")).unwrap();
        assert!(
            marker.contains(session_id.as_str()),
            "workspace marker must contain session ID '{}', got:\n{}",
            session_id,
            marker
        );
    }

    // ── Deprovision removes directory ────────────────────────────────────────

    #[test]
    fn deprovision_removes_working_tree_directory() {
        let tmp = TempDir::new().unwrap();
        let repo = init_bare_repo(&tmp, "myproject");
        let source = git_worktree_source(&tmp);
        let target = tmp.path().join("ws");

        source.provision(repo.to_str().unwrap(), None, &target).unwrap();
        assert!(target.exists(), "precondition: workspace must exist after provision");

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
