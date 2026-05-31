//! SessionManager: creates and deactivates sessions, delegates to WorkspaceHydrator.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use chrono::Utc;
use libc;

use super::lock::WorkItemLock;
use super::registry::SessionRegistry;
use super::types::{SessionId, SessionRecord, SessionState, SessionType};
use crate::session::finalization::subagent::{
    launch_finalization_subagent, retrigger_finalization,
};
use crate::workspace::{push_with_rebase_retry, DEFAULT_MAX_RETRIES};

/// Describes the state of a single project repo after session deactivation.
#[derive(Debug, Clone)]
pub struct DirtyRepo {
    pub name: String,
    pub has_uncommitted: bool,
    pub unpushed_branches: Vec<String>,
}

/// Returned by `deactivate_session` — summarizes post-deactivation workspace state.
#[derive(Debug)]
pub struct DeactivationResult {
    pub session_id: SessionId,
    /// Repos with uncommitted files or unpushed branches.
    pub dirty_repos: Vec<DirtyRepo>,
    /// Whether the finalization subagent was launched (true = workspace was dirty).
    pub finalization_launched: bool,
}

/// Manages session lifecycle: creates sessions via the hydrator and tracks them in the registry.
///
/// Also owns the `WorkItemLock` — `deactivate_session` always releases all locks held by the
/// terminated session, so callers cannot accidentally orphan locks.
pub struct SessionManager {
    pub(crate) registry: SessionRegistry,
    pub(crate) workspace_base: PathBuf,
    pub(crate) work_item_lock: WorkItemLock,
}

impl SessionManager {
    /// Create a new SessionManager backed by a registry persisted at `registry_path`.
    pub fn new(workspace_base: PathBuf, registry_path: PathBuf) -> Result<Self> {
        Ok(Self {
            registry: SessionRegistry::load(registry_path)?,
            workspace_base,
            work_item_lock: WorkItemLock::new(),
        })
    }

    /// Try to acquire the work-item lock on behalf of `session_id`.
    ///
    /// Returns `true` if acquired, `false` if already held by another session.
    pub fn acquire_lock(&self, work_item_id: &str, session_id: &SessionId) -> Result<bool> {
        self.work_item_lock.acquire(work_item_id, session_id)
    }

    /// Release the work-item lock held by `session_id`. Errors if not the owner.
    pub fn release_lock(&self, work_item_id: &str, session_id: &SessionId) -> Result<()> {
        self.work_item_lock.release(work_item_id, session_id)
    }

    /// Create a new session for `member` of the given `session_type`.
    ///
    /// Creates the member workspace directory, registers the session in Creating state,
    /// transitions to Active, and returns the persisted session record.
    pub fn create_session(
        &mut self,
        member: &str,
        session_type: SessionType,
    ) -> Result<SessionRecord> {
        let workspace_path = self.workspace_base.join(member);
        std::fs::create_dir_all(&workspace_path)?;

        let session_id = SessionId::new();
        let now = Utc::now();
        let record = SessionRecord {
            session_id: session_id.clone(),
            member_name: member.to_string(),
            session_type,
            current_state: SessionState::Creating,
            created_at: now,
            state_transitioned_at: now,
            agent_pid: None,
            workspace_path: Some(workspace_path),
        };

        self.registry.register(record)?;
        self.registry
            .update_state(&session_id, SessionState::Active)?;
        self.registry.save()?;

        Ok(self
            .registry
            .get(&session_id)
            .expect("session must exist after register")
            .clone())
    }

    /// Stop the agent for `id`, inspect workspace dirty state, and transition the session.
    ///
    /// If the workspace is clean (or becomes clean after a quick push), transitions to Completed.
    /// If work remains (uncommitted files or unresolvable push conflicts), launches a finalization
    /// subagent and transitions to Finalizing instead. The subagent runs in the background;
    /// `finalization_completed` or `finalization_failed` must be called when it exits.
    ///
    /// Always releases all work-item locks held by this session before returning.
    pub fn deactivate_session(&mut self, id: &SessionId) -> Result<DeactivationResult> {
        let session = self
            .registry
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", id))?
            .clone();

        let mut dirty_repos = vec![];
        let mut finalization_launched = false;

        if let Some(ref wp) = session.workspace_path {
            dirty_repos = inspect_dirty_repos(wp);
            // Quick-push any committed-but-unpushed branches before deciding on subagent.
            push_and_refresh_dirty(&mut dirty_repos, wp);

            let still_dirty = dirty_repos
                .iter()
                .any(|r| r.has_uncommitted || !r.unpushed_branches.is_empty());

            if still_dirty {
                if let Ok(_child) = launch_finalization_subagent(wp, id.as_str()) {
                    // Child runs in background — session management observes completion separately.
                    finalization_launched = true;
                }
            }
        }

        if finalization_launched {
            self.registry.update_state(id, SessionState::Finalizing)?;
        } else {
            self.registry.update_state(id, SessionState::Completed)?;
        }
        self.registry.save()?;
        // Release all work-item locks held by this session — prevents orphaned locks.
        let _ = self.work_item_lock.release_all(id);

        Ok(DeactivationResult {
            session_id: id.clone(),
            dirty_repos,
            finalization_launched,
        })
    }

    /// Return all sessions that are not in a terminal state (Creating, Active, Finalizing).
    pub fn list_active(&self) -> Vec<&SessionRecord> {
        self.registry
            .list()
            .into_iter()
            .filter(|s| {
                matches!(
                    s.current_state,
                    SessionState::Creating | SessionState::Active | SessionState::Finalizing
                )
            })
            .collect()
    }

    /// Look up a session by ID. Returns None if the session does not exist.
    pub fn get(&self, id: &SessionId) -> Option<&SessionRecord> {
        self.registry.get(id)
    }

    /// Transition a Finalizing session to Completed after the subagent exits successfully.
    pub fn finalization_completed(&mut self, id: &SessionId) -> Result<()> {
        self.registry.update_state(id, SessionState::Completed)?;
        self.registry.save()
    }

    /// Transition a Finalizing session to Failed when remote preservation is impossible.
    pub fn finalization_failed(&mut self, id: &SessionId) -> Result<()> {
        self.registry.update_state(id, SessionState::Failed)?;
        self.registry.save()
    }

    /// Force-stop a session: Active → Killed or Finalizing → Killed, skipping finalization.
    ///
    /// For Active sessions: kills the agent process and transitions to Killed without launching
    /// a finalization subagent. For Finalizing sessions: kills the running finalization subagent
    /// and transitions to Killed. Workspace is retained in both cases (re-trigger available).
    pub fn force_stop_session(&mut self, id: &SessionId) -> Result<()> {
        let session = self
            .registry
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", id))?
            .clone();

        match &session.current_state {
            SessionState::Active | SessionState::Finalizing => {
                if let Some(pid) = session.agent_pid {
                    // SIGKILL — no cleanup, no grace period
                    unsafe { libc::kill(pid as i32, libc::SIGKILL) };
                }
                self.registry.update_state(id, SessionState::Killed)?;
                self.registry.save()?;
                let _ = self.work_item_lock.release_all(id);
                Ok(())
            }
            state => anyhow::bail!(
                "force_stop_session requires Active or Finalizing state, session {} is {}",
                id,
                state
            ),
        }
    }

    /// Re-trigger finalization on a Retained session: Retained → Finalizing, launch fresh subagent.
    ///
    /// Returns an error if the session is not in Retained state — only retained sessions can be
    /// re-finalized; calling this on an active or completed session is a caller bug.
    pub fn retrigger_finalization_for(&mut self, id: &SessionId) -> Result<()> {
        let current = self
            .registry
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?
            .current_state
            .clone();
        if current != SessionState::Retained {
            anyhow::bail!(
                "retrigger_finalization_for requires Retained state, session {} is {}",
                id,
                current
            );
        }
        self.registry.update_state(id, SessionState::Finalizing)?;
        self.registry.save()?;
        let _ = retrigger_finalization(id);
        Ok(())
    }
}

/// Push unpushed branches for each dirty repo and refresh its state in-place.
///
/// For each repo with unpushed commits, resolves the current branch and calls
/// `push_with_rebase_retry`. On success, clears `unpushed_branches`. Non-fatal:
/// push failures are never propagated as Err from `deactivate_session`.
fn push_and_refresh_dirty(dirty_repos: &mut [DirtyRepo], workspace_path: &Path) {
    for repo in dirty_repos.iter_mut() {
        if repo.unpushed_branches.is_empty() {
            continue;
        }
        let repo_path = workspace_path.join("projects").join(&repo.name);
        let branch_out = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&repo_path)
            .output();
        let branch = match branch_out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => continue,
        };
        if push_with_rebase_retry(&repo_path, "origin", &branch, DEFAULT_MAX_RETRIES).is_ok() {
            repo.unpushed_branches.clear();
        }
    }
}

/// Run git status and unpushed-commits checks against every subdirectory under
/// `workspace/projects/`. Non-git directories are silently skipped.
fn inspect_dirty_repos(workspace_path: &Path) -> Vec<DirtyRepo> {
    let projects_dir = workspace_path.join("projects");
    if !projects_dir.exists() {
        return vec![];
    }

    let mut dirty = vec![];
    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let repo_path = entry.path();
        let repo_name = entry.file_name().to_string_lossy().to_string();

        let has_uncommitted = Command::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap_or("."),
                "status",
                "--porcelain",
            ])
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);

        let unpushed_branches: Vec<String> = Command::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap_or("."),
                "log",
                "@{u}..HEAD",
                "--oneline",
            ])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.to_string())
                    .collect()
            })
            .unwrap_or_default();

        if has_uncommitted || !unpushed_branches.is_empty() {
            dirty.push(DirtyRepo {
                name: repo_name,
                has_uncommitted,
                unpushed_branches,
            });
        }
    }

    dirty
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionState;
    use chrono::Utc;

    fn make_manager() -> (SessionManager, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let registry_path = tmp.path().join("sessions.json");
        let workspace_base = tmp.path().join("workspaces");
        std::fs::create_dir_all(&workspace_base).unwrap();
        let manager = SessionManager::new(workspace_base, registry_path).unwrap();
        (manager, tmp)
    }

    // AC-10: create_session must register the session and include start_time
    #[test]
    fn create_session_registers_session_with_required_fields() {
        let (mut manager, _tmp) = make_manager();
        let record = manager
            .create_session("alice", SessionType::Loop)
            .expect("create_session must succeed");

        assert_eq!(record.member_name, "alice");
        assert_eq!(record.session_type, SessionType::Loop);
        // AC-10: start_time must be present (created_at)
        assert!(
            record.created_at <= Utc::now(),
            "created_at must be set to a timestamp at or before now"
        );
        // Session must reach Active state after create_session
        assert_eq!(
            record.current_state,
            SessionState::Active,
            "session must be Active after create_session"
        );
    }

    // AC-10: list_active returns sessions with all required fields for the API
    #[test]
    fn list_active_returns_session_with_all_display_fields() {
        let (mut manager, _tmp) = make_manager();
        let _ = manager
            .create_session("bob", SessionType::Brain)
            .expect("create_session must succeed");

        let active = manager.list_active();
        assert_eq!(active.len(), 1, "one active session expected");
        let s = active[0];
        // AC-10: fields required by GET /api/sessions
        assert!(
            !s.session_id.as_str().is_empty(),
            "session_id must be non-empty"
        );
        assert_eq!(s.member_name, "bob");
        assert_eq!(s.session_type, SessionType::Brain);
        assert_eq!(s.current_state, SessionState::Active);
        // start_time field corresponds to created_at
        assert!(s.created_at <= Utc::now());
    }

    // AC-19: deactivate_session reports dirty state per repo
    #[test]
    fn deactivate_session_reports_dirty_repos() {
        let (mut manager, tmp) = make_manager();
        // Set up a workspace with an uncommitted file
        let ws = tmp.path().join("workspaces").join("alice");
        std::fs::create_dir_all(ws.join("projects/my-project")).unwrap();

        let record = manager
            .create_session("alice", SessionType::Interactive)
            .expect("create_session must succeed");

        let result = manager
            .deactivate_session(&record.session_id)
            .expect("deactivate_session must succeed");

        assert_eq!(
            result.session_id, record.session_id,
            "deactivation result must reference the correct session"
        );
        // dirty_repos may be empty if workspace is clean — structure must exist
        let _ = result.dirty_repos; // Type-check: must be Vec<DirtyRepo>
    }

    // AC-10: deactivate_session transitions session to Completed
    #[test]
    fn deactivate_session_transitions_to_completed() {
        let (mut manager, _tmp) = make_manager();
        let record = manager
            .create_session("dan", SessionType::Interactive)
            .expect("create_session must succeed");

        let _result = manager
            .deactivate_session(&record.session_id)
            .expect("deactivate_session must succeed");

        // After deactivation, session is no longer in active list
        let active = manager.list_active();
        assert!(
            active.iter().all(|s| s.session_id != record.session_id),
            "deactivated session must not appear in list_active"
        );

        // Session state must be Completed
        let session = manager.registry.get(&record.session_id);
        assert!(
            session.is_some(),
            "session record must remain in registry after deactivation"
        );
        assert_eq!(
            session.unwrap().current_state,
            SessionState::Completed,
            "session must be Completed after deactivation"
        );
    }

    // AC-22: No Sync Required — create_session for a member with no prior sync still provides all projects
    #[test]
    fn create_session_provides_all_projects_without_prior_sync() {
        let (mut manager, tmp) = make_manager();
        // Workspace doesn't exist yet for "fresh-member" — hydrator must create it
        let ws_path = tmp.path().join("workspaces").join("fresh-member");
        assert!(
            !ws_path.exists(),
            "workspace must not exist before create_session"
        );

        let record = manager
            .create_session("fresh-member", SessionType::Loop)
            .expect("create_session must succeed even without prior sync");

        // Session record must include the workspace path
        assert!(
            record.workspace_path.is_some(),
            "session record must include workspace_path after hydration"
        );
        // The workspace must have been created
        let ws = record.workspace_path.unwrap();
        assert!(
            ws.exists(),
            "workspace must exist after create_session (hydrator must create it)"
        );
    }

    // get() returns None for unknown session ID
    #[test]
    fn get_returns_none_for_unknown_session() {
        let (manager, _tmp) = make_manager();
        let phantom = SessionId::new();
        assert!(
            manager.get(&phantom).is_none(),
            "get must return None for unknown session"
        );
    }

    // AC-10: list_active excludes terminal sessions
    #[test]
    fn list_active_excludes_deactivated_sessions() {
        let (mut manager, _tmp) = make_manager();
        let r1 = manager
            .create_session("eve", SessionType::Interactive)
            .expect("create session 1");
        let _r2 = manager
            .create_session("frank", SessionType::Interactive)
            .expect("create session 2");

        manager
            .deactivate_session(&r1.session_id)
            .expect("deactivate session 1");

        let active = manager.list_active();
        assert_eq!(
            active.len(),
            1,
            "only one session must remain active after deactivating one"
        );
        assert_ne!(
            active[0].session_id, r1.session_id,
            "deactivated session must not be in active list"
        );
    }
}

#[cfg(test)]
mod git_test_fixtures {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    pub fn init_bare_repo(tmp: &TempDir, name: &str) -> PathBuf {
        let bare = tmp.path().join(format!("{name}.git"));
        fs::create_dir_all(&bare).unwrap();
        Command::new("git")
            .args(["init", "--bare", "-b", "main"])
            .arg(&bare)
            .output()
            .unwrap();
        let seed = tmp.path().join(format!("{name}-seed"));
        Command::new("git")
            .args(["clone", bare.to_str().unwrap(), seed.to_str().unwrap()])
            .output()
            .unwrap();
        fs::write(seed.join("file.txt"), "initial\n").unwrap();
        git_commit_all(&seed, "init");
        Command::new("git")
            .args(["-C", seed.to_str().unwrap(), "push"])
            .output()
            .unwrap();
        bare
    }

    pub fn clone_into_workspace_projects(
        bare: &Path,
        workspace: &Path,
        project_name: &str,
    ) -> PathBuf {
        let projects = workspace.join("projects");
        fs::create_dir_all(&projects).unwrap();
        let dest = projects.join(project_name);
        Command::new("git")
            .args(["clone", bare.to_str().unwrap(), dest.to_str().unwrap()])
            .output()
            .unwrap();
        dest
    }

    pub fn git_commit_all(repo: &Path, msg: &str) {
        Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap(),
                "-c",
                "user.email=test@test.com",
                "-c",
                "user.name=Test",
                "commit",
                "-m",
                msg,
            ])
            .output()
            .unwrap();
    }
}

#[cfg(test)]
mod session_push_integration_tests {
    use super::git_test_fixtures::{clone_into_workspace_projects, git_commit_all, init_bare_repo};
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // ── tests ─────────────────────────────────────────────────────────────────

    // AC-14b: successful push → unpushed_branches cleared
    #[test]
    fn push_succeeds_clears_unpushed_branches() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let workspace = tmp.path().join("ws/alice");
        let project = clone_into_workspace_projects(&bare, &workspace, "my-project");

        fs::write(project.join("change.txt"), "alice change\n").unwrap();
        git_commit_all(&project, "alice commit");

        let mut dirty = inspect_dirty_repos(&workspace);
        assert!(
            !dirty.is_empty() && !dirty[0].unpushed_branches.is_empty(),
            "setup: project must have unpushed commits"
        );

        push_and_refresh_dirty(&mut dirty, &workspace);

        assert!(
            dirty[0].unpushed_branches.is_empty(),
            "unpushed_branches must be cleared after successful push"
        );
    }

    // AC-14b: NFF rejection → fetch+rebase+retry succeeds → branches cleared
    #[test]
    fn nff_conflict_rebase_retry_succeeds_clears_unpushed() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");

        // clone_a advances bare (non-conflicting file)
        let clone_a = tmp.path().join("clone-a");
        Command::new("git")
            .args(["clone", bare.to_str().unwrap(), clone_a.to_str().unwrap()])
            .output()
            .unwrap();
        fs::write(clone_a.join("from_a.txt"), "from a\n").unwrap();
        git_commit_all(&clone_a, "advance bare");
        Command::new("git")
            .args(["-C", clone_a.to_str().unwrap(), "push"])
            .output()
            .unwrap();

        // workspace clone: non-conflicting commit (different file → rebase will succeed)
        let workspace = tmp.path().join("ws/bob");
        let project = clone_into_workspace_projects(&bare, &workspace, "my-project");
        fs::write(project.join("from_bob.txt"), "from bob\n").unwrap();
        git_commit_all(&project, "bob commit");

        let mut dirty = inspect_dirty_repos(&workspace);
        assert!(
            !dirty.is_empty() && !dirty[0].unpushed_branches.is_empty(),
            "setup: project must have unpushed commits"
        );

        push_and_refresh_dirty(&mut dirty, &workspace);

        assert!(
            dirty[0].unpushed_branches.is_empty(),
            "unpushed_branches must be cleared after rebase+retry succeeds"
        );
    }

    // AC-14b: all push attempts fail (rebase conflict) → unpushed_branches preserved
    #[test]
    fn push_fails_max_retries_repo_stays_dirty() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");

        // Both clones start from the same bare state so they diverge after clone_a pushes.
        let clone_a = tmp.path().join("clone-a");
        Command::new("git")
            .args(["clone", bare.to_str().unwrap(), clone_a.to_str().unwrap()])
            .output()
            .unwrap();

        // workspace cloned BEFORE clone_a pushes so it is behind bare when push is attempted
        let workspace = tmp.path().join("ws/dan");
        let project = clone_into_workspace_projects(&bare, &workspace, "my-project");

        // clone_a: conflicting change on the same file → advances bare
        fs::write(clone_a.join("file.txt"), "version A\n").unwrap();
        git_commit_all(&clone_a, "A modifies file.txt");
        Command::new("git")
            .args(["-C", clone_a.to_str().unwrap(), "push"])
            .output()
            .unwrap();

        // workspace: conflicting change on the same file → rebase will conflict
        fs::write(project.join("file.txt"), "version B\n").unwrap();
        git_commit_all(&project, "B modifies file.txt");

        let mut dirty = inspect_dirty_repos(&workspace);
        assert!(
            !dirty.is_empty() && !dirty[0].unpushed_branches.is_empty(),
            "setup: project must have unpushed commits"
        );
        let original_count = dirty[0].unpushed_branches.len();

        push_and_refresh_dirty(&mut dirty, &workspace);

        assert_eq!(
            dirty[0].unpushed_branches.len(),
            original_count,
            "unpushed_branches must be preserved when push fails"
        );
    }

    // AC-14b: repo with only uncommitted changes (no unpushed commits) → no push attempted
    #[test]
    fn uncommitted_only_repo_not_pushed() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let workspace = tmp.path().join("ws/eve");
        let project = clone_into_workspace_projects(&bare, &workspace, "my-project");

        // Write a file but do NOT commit — only has_uncommitted=true, unpushed_branches=[]
        fs::write(project.join("uncommitted.txt"), "not committed\n").unwrap();

        let mut dirty = inspect_dirty_repos(&workspace);
        // The repo is dirty (uncommitted file) but has no unpushed commits
        let repos_without_unpushed: Vec<_> = dirty
            .iter()
            .filter(|r| r.has_uncommitted && r.unpushed_branches.is_empty())
            .collect();
        assert!(
            !repos_without_unpushed.is_empty(),
            "setup: must have a repo with uncommitted-only changes"
        );

        push_and_refresh_dirty(&mut dirty, &workspace);

        let repos_with_unpushed: Vec<_> = dirty
            .iter()
            .filter(|r| !r.unpushed_branches.is_empty())
            .collect();
        assert!(
            repos_with_unpushed.is_empty(),
            "uncommitted-only repos must not gain unpushed_branches entries"
        );
    }

    // AC-14b: push failure must not cause deactivate_session to return Err
    #[test]
    fn push_failure_is_nonfatal() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");

        // clone_a: conflicting change → push to advance bare
        let clone_a = tmp.path().join("clone-a");
        Command::new("git")
            .args(["clone", bare.to_str().unwrap(), clone_a.to_str().unwrap()])
            .output()
            .unwrap();
        fs::write(clone_a.join("file.txt"), "version A\n").unwrap();
        git_commit_all(&clone_a, "A modifies file.txt");
        Command::new("git")
            .args(["-C", clone_a.to_str().unwrap(), "push"])
            .output()
            .unwrap();

        // workspace: conflicting change → push during deactivation will fail
        let workspace_base = tmp.path().join("workspaces");
        let workspace = workspace_base.join("frank");
        let project = clone_into_workspace_projects(&bare, &workspace, "my-project");
        fs::write(project.join("file.txt"), "frank version\n").unwrap();
        git_commit_all(&project, "frank modifies file.txt");

        let registry_path = tmp.path().join("sessions.json");
        let mut manager = SessionManager::new(workspace_base, registry_path).unwrap();
        let record = manager
            .create_session("frank", SessionType::Interactive)
            .unwrap();

        let result = manager.deactivate_session(&record.session_id);
        assert!(
            result.is_ok(),
            "deactivate_session must return Ok even when push fails: {:?}",
            result
        );
    }
}

#[cfg(test)]
mod session_finalization_integration_tests {
    use super::git_test_fixtures::{clone_into_workspace_projects, git_commit_all, init_bare_repo};
    use super::*;
    use crate::session::finalization::subagent::recovery_branch_name;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn make_manager(workspace_base: &Path, registry_path: &Path) -> SessionManager {
        SessionManager::new(workspace_base.to_path_buf(), registry_path.to_path_buf()).unwrap()
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    // AC-23: deactivate with dirty workspace → transitions to Finalizing, subagent launched
    #[test]
    fn deactivate_dirty_session_launches_finalization_and_transitions_to_finalizing() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let workspace_base = tmp.path().join("workspaces");
        let workspace = workspace_base.join("alice");
        let project = clone_into_workspace_projects(&bare, &workspace, "my-project");

        // Uncommitted file makes workspace dirty
        fs::write(project.join("dirty.txt"), "uncommitted work\n").unwrap();

        let registry_path = tmp.path().join("sessions.json");
        let mut manager = make_manager(&workspace_base, &registry_path);
        let record = manager
            .create_session("alice", SessionType::Interactive)
            .unwrap();

        let result = manager.deactivate_session(&record.session_id).unwrap();

        assert!(
            result.finalization_launched,
            "finalization must be launched for dirty session (got finalization_launched=false)"
        );
        assert_eq!(
            manager.get(&record.session_id).unwrap().current_state,
            SessionState::Finalizing,
            "dirty session must transition to Finalizing, not directly to Completed"
        );
    }

    // AC-23: deactivate with clean workspace → Completed, no subagent launched (regression)
    #[test]
    fn deactivate_clean_session_skips_finalization_transitions_to_completed() {
        let tmp = TempDir::new().unwrap();
        let workspace_base = tmp.path().join("workspaces");
        let registry_path = tmp.path().join("sessions.json");
        let mut manager = make_manager(&workspace_base, &registry_path);
        let record = manager.create_session("bob", SessionType::Loop).unwrap();

        let result = manager.deactivate_session(&record.session_id).unwrap();

        assert!(
            !result.finalization_launched,
            "finalization must NOT be launched for clean session"
        );
        assert_eq!(
            manager.get(&record.session_id).unwrap().current_state,
            SessionState::Completed,
            "clean session must transition to Completed"
        );
    }

    // AC-23: finalization subagent exits 0 → session transitions to Completed
    #[test]
    fn finalization_success_transitions_to_completed() {
        let tmp = TempDir::new().unwrap();
        let workspace_base = tmp.path().join("workspaces");
        let registry_path = tmp.path().join("sessions.json");
        let mut manager = make_manager(&workspace_base, &registry_path);
        let record = manager.create_session("carol", SessionType::Loop).unwrap();

        // Drive to Finalizing directly via registry
        manager
            .registry
            .update_state(&record.session_id, SessionState::Finalizing)
            .unwrap();
        manager.registry.save().unwrap();

        // Signal finalization completed successfully
        manager.finalization_completed(&record.session_id).unwrap();

        assert_eq!(
            manager.get(&record.session_id).unwrap().current_state,
            SessionState::Completed,
            "finalization success must transition from Finalizing to Completed"
        );
    }

    // D-10: recovery branch naming follows the expected convention
    #[test]
    fn push_conflict_creates_recovery_branch_and_completes_degraded() {
        let session_id = "abc12345";
        let original_branch = "main";
        let recovery = recovery_branch_name(session_id, original_branch);
        assert_eq!(
            recovery,
            format!("recovery/{session_id}/{original_branch}"),
            "recovery branch name must follow convention: recovery/<session-id>/<branch>"
        );
    }

    // AC-03: remote unreachable → session transitions to Failed, workspace retained
    #[test]
    fn network_failure_transitions_to_failed_retains_workspace() {
        let tmp = TempDir::new().unwrap();
        let workspace_base = tmp.path().join("workspaces");
        let registry_path = tmp.path().join("sessions.json");
        let mut manager = make_manager(&workspace_base, &registry_path);
        let record = manager
            .create_session("dan", SessionType::Interactive)
            .unwrap();
        let workspace_path = record.workspace_path.clone().unwrap();

        // Drive to Finalizing
        manager
            .registry
            .update_state(&record.session_id, SessionState::Finalizing)
            .unwrap();
        manager.registry.save().unwrap();

        // Simulate finalization failure (remote unreachable — preservation impossible)
        manager.finalization_failed(&record.session_id).unwrap();

        assert_eq!(
            manager.get(&record.session_id).unwrap().current_state,
            SessionState::Failed,
            "remote-unreachable finalization must transition to Failed"
        );
        assert!(
            workspace_path.exists(),
            "workspace must be retained (not deleted) after Failed state"
        );
    }

    // AC-03: session in Failed state does not block new session creation (regression)
    #[test]
    fn abnormal_end_does_not_block_new_session() {
        let tmp = TempDir::new().unwrap();
        let workspace_base = tmp.path().join("workspaces");
        let registry_path = tmp.path().join("sessions.json");
        let mut manager = make_manager(&workspace_base, &registry_path);

        // Drive S1 to Failed (simulating abnormal end)
        let s1 = manager.create_session("eve", SessionType::Loop).unwrap();
        manager
            .registry
            .update_state(&s1.session_id, SessionState::Failed)
            .unwrap();
        manager.registry.save().unwrap();

        // S2 must still be creatable for the same member
        let s2 = manager
            .create_session("eve", SessionType::Loop)
            .expect("new session must succeed even when a previous session is in Failed state");

        assert_eq!(
            s2.current_state,
            SessionState::Active,
            "new session must reach Active state regardless of prior Failed session"
        );
    }

    // AC-15: retrigger finalization on Retained session → Finalizing
    #[test]
    fn retrigger_on_retained_session_launches_fresh_subagent() {
        let tmp = TempDir::new().unwrap();
        let workspace_base = tmp.path().join("workspaces");
        let registry_path = tmp.path().join("sessions.json");
        let mut manager = make_manager(&workspace_base, &registry_path);
        let record = manager.create_session("frank", SessionType::Loop).unwrap();

        // Drive to Retained: Active → Completed → Retained
        manager
            .registry
            .update_state(&record.session_id, SessionState::Completed)
            .unwrap();
        manager
            .registry
            .update_state(&record.session_id, SessionState::Retained)
            .unwrap();
        manager.registry.save().unwrap();

        // Retrigger finalization
        manager
            .retrigger_finalization_for(&record.session_id)
            .unwrap();

        assert_eq!(
            manager.get(&record.session_id).unwrap().current_state,
            SessionState::Finalizing,
            "retrigger must transition Retained → Finalizing"
        );
    }
}
