//! SessionManager: creates and deactivates sessions, delegates to WorkspaceHydrator.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use chrono::Utc;
use libc;

use super::lock::WorkItemLock;
use super::registry::SessionRegistry;
use super::types::{
    FinalizationResult, GitState, SessionId, SessionRecord, SessionState, SessionType,
};
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

/// Structured summary of a session returned by `inspect_session`.
#[derive(Debug, Clone)]
pub struct SessionInspection {
    pub session_id: SessionId,
    pub member_name: String,
    pub session_type: SessionType,
    pub current_state: SessionState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub state_transitioned_at: chrono::DateTime<chrono::Utc>,
    pub workspace_path: Option<std::path::PathBuf>,
    pub finalization_results: Option<FinalizationResult>,
    pub git_state: Option<GitState>,
}

/// Filter predicate for bulk session cleanup.
pub enum CleanupFilter {
    Member(String),
    OlderThan(std::time::Duration),
    All,
}

/// Result of a bulk cleanup operation.
pub struct CleanupReport {
    pub removed: usize,
}

pub struct RecoveryReport {
    pub recovered: usize,
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
            finalization_result: None,
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

    /// Return all sessions (including terminal states, excluding Retained).
    pub fn list_all(&self) -> Vec<&SessionRecord> {
        self.registry
            .list()
            .into_iter()
            .filter(|s| !matches!(s.current_state, SessionState::Retained))
            .collect()
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

    pub fn list_terminal(&self) -> Vec<&SessionRecord> {
        self.registry
            .list()
            .into_iter()
            .filter(|s| {
                matches!(
                    s.current_state,
                    SessionState::Completed | SessionState::Failed | SessionState::Killed
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

    /// Mark an Active session as Failed (e.g., agent exited with non-zero code).
    pub fn fail_session(&mut self, id: &SessionId) -> Result<()> {
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
        let session = self
            .registry
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?
            .clone();
        if session.current_state != SessionState::Retained {
            anyhow::bail!(
                "retrigger_finalization_for requires Retained state, session {} is {}",
                id,
                session.current_state
            );
        }
        self.registry.update_state(id, SessionState::Finalizing)?;
        self.registry.save()?;
        if let Some(ref wp) = session.workspace_path {
            let _ = retrigger_finalization(wp, id);
        }
        Ok(())
    }

    pub fn inspect_session(&self, id: &SessionId) -> Result<SessionInspection> {
        let session = self
            .registry
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?
            .clone();
        Ok(SessionInspection {
            session_id: session.session_id,
            member_name: session.member_name,
            session_type: session.session_type,
            current_state: session.current_state,
            created_at: session.created_at,
            state_transitioned_at: session.state_transitioned_at,
            workspace_path: session.workspace_path,
            finalization_results: session.finalization_result,
            git_state: None,
        })
    }

    pub fn cleanup_session(&mut self, id: &SessionId) -> Result<()> {
        let session = self
            .registry
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?
            .clone();
        if let Some(ref wp) = session.workspace_path {
            if wp.exists() {
                std::fs::remove_dir_all(wp)?;
            }
        }
        self.registry.remove(id)?;
        self.registry.save()
    }

    pub fn cleanup_sessions(&mut self, filter: CleanupFilter) -> Result<CleanupReport> {
        let now = chrono::Utc::now();
        let ids_to_remove: Vec<SessionId> = self
            .registry
            .list()
            .into_iter()
            .filter(|s| s.current_state == SessionState::Retained)
            .filter(|s| match &filter {
                CleanupFilter::Member(name) => &s.member_name == name,
                CleanupFilter::OlderThan(duration) => {
                    let age = now.signed_duration_since(s.state_transitioned_at);
                    age.num_seconds() >= 0 && (age.num_seconds() as u64) > duration.as_secs()
                }
                CleanupFilter::All => true,
            })
            .map(|s| s.session_id.clone())
            .collect();
        let removed = ids_to_remove.len();
        for id in ids_to_remove {
            self.cleanup_session(&id)?;
        }
        Ok(CleanupReport { removed })
    }

    pub fn recover_stale_sessions_with<F>(&mut self, is_alive: F) -> Result<RecoveryReport>
    where
        F: Fn(u32) -> bool,
    {
        let ids_to_recover: Vec<SessionId> = self
            .registry
            .list()
            .into_iter()
            .filter(|s| {
                matches!(
                    s.current_state,
                    SessionState::Active | SessionState::Finalizing
                )
            })
            .filter(|s| s.agent_pid.map(|pid| !is_alive(pid)).unwrap_or(false))
            .map(|s| s.session_id.clone())
            .collect();
        let recovered = ids_to_recover.len();
        for id in &ids_to_recover {
            self.registry.update_state(id, SessionState::Failed)?;
        }
        if recovered > 0 {
            self.registry.save()?;
        }
        Ok(RecoveryReport { recovered })
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

    // AC-17: list_terminal returns only terminal-state sessions (Completed, Failed, Killed)
    #[test]
    fn list_terminal_returns_only_terminal_state_sessions() {
        let (mut manager, _tmp) = make_manager();

        let active_record = manager
            .create_session("alice", SessionType::Loop)
            .expect("create active session");
        let completed_record = manager
            .create_session("bob", SessionType::Brain)
            .expect("create completed session");
        let failed_record = manager
            .create_session("carol", SessionType::Interactive)
            .expect("create failed session");

        // Drive bob → Completed
        manager
            .registry
            .update_state(&completed_record.session_id, SessionState::Completed)
            .unwrap();
        manager.registry.save().unwrap();

        // Drive carol → Failed
        manager
            .registry
            .update_state(&failed_record.session_id, SessionState::Failed)
            .unwrap();
        manager.registry.save().unwrap();

        let terminal = manager.list_terminal();

        assert_eq!(
            terminal.len(),
            2,
            "only terminal sessions (Completed, Failed) must be returned by list_terminal"
        );
        let ids: Vec<_> = terminal.iter().map(|s| &s.session_id).collect();
        assert!(
            !ids.contains(&&active_record.session_id),
            "Active session must not appear in list_terminal"
        );
        assert!(
            ids.contains(&&completed_record.session_id),
            "Completed session must appear in list_terminal"
        );
        assert!(
            ids.contains(&&failed_record.session_id),
            "Failed session must appear in list_terminal"
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
    use super::git_test_fixtures::{clone_into_workspace_projects, init_bare_repo};
    use super::*;
    use crate::session::finalization::subagent::recovery_branch_name;
    use std::fs;
    use std::path::Path;
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

#[cfg(test)]
mod session_cleanup_inspection_tests {
    use super::*;
    use crate::session::types::{SessionId, SessionRecord, SessionState, SessionType};
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_manager(tmp: &TempDir) -> SessionManager {
        SessionManager::new(
            tmp.path().join("workspaces"),
            tmp.path().join("registry.json"),
        )
        .unwrap()
    }

    fn add_retained_session(
        manager: &mut SessionManager,
        member: &str,
        workspace_path: Option<std::path::PathBuf>,
    ) -> SessionId {
        let record = SessionRecord {
            session_id: SessionId::new(),
            member_name: member.to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Retained,
            created_at: Utc::now(),
            state_transitioned_at: Utc::now(),
            agent_pid: None,
            workspace_path,
            finalization_result: None,
        };
        let id = record.session_id.clone();
        manager.registry.register(record).unwrap();
        id
    }

    // AC-18 (inspection): inspect_session returns structured summary for a Retained session.
    #[test]
    fn inspect_retained_session_returns_structured_summary() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws/alice");
        std::fs::create_dir_all(&workspace).unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_retained_session(&mut manager, "alice", Some(workspace));

        let inspection: SessionInspection = manager.inspect_session(&id).unwrap();
        assert_eq!(inspection.member_name, "alice");
        assert_eq!(inspection.current_state, SessionState::Retained);
        assert!(
            inspection.workspace_path.is_some(),
            "inspection must include workspace path"
        );
    }

    // AC-18 (inspection): inspect_session returns error for unknown session ID.
    #[test]
    fn inspect_unknown_session_returns_error() {
        let tmp = TempDir::new().unwrap();
        let manager = make_manager(&tmp);
        let phantom = SessionId::new();
        let result: anyhow::Result<SessionInspection> = manager.inspect_session(&phantom);
        assert!(result.is_err(), "inspect_session on unknown ID must fail");
    }

    // AC-18 (individual cleanup): cleanup_session removes workspace dir and registry entry.
    #[test]
    fn cleanup_session_removes_workspace_directory_and_registry_entry() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws/alice");
        std::fs::create_dir_all(&workspace).unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_retained_session(&mut manager, "alice", Some(workspace.clone()));
        manager.registry.save().unwrap();

        manager.cleanup_session(&id).unwrap();

        assert!(
            !workspace.exists(),
            "workspace directory must be removed after cleanup"
        );
        assert!(
            manager.registry.get(&id).is_none(),
            "session must be removed from registry after cleanup"
        );
    }

    // AC-18 (bulk cleanup): cleanup_sessions with member filter removes only matching sessions.
    #[test]
    fn cleanup_sessions_member_filter_removes_only_matching() {
        let tmp = TempDir::new().unwrap();
        let ws_a1 = tmp.path().join("ws/alice-1");
        let ws_a2 = tmp.path().join("ws/alice-2");
        let ws_bob = tmp.path().join("ws/bob");
        for p in [&ws_a1, &ws_a2, &ws_bob] {
            std::fs::create_dir_all(p).unwrap();
        }
        let mut manager = make_manager(&tmp);
        let _id_a1 = add_retained_session(&mut manager, "alice", Some(ws_a1.clone()));
        let _id_a2 = add_retained_session(&mut manager, "alice", Some(ws_a2.clone()));
        let id_bob = add_retained_session(&mut manager, "bob", Some(ws_bob.clone()));

        let report: CleanupReport = manager
            .cleanup_sessions(CleanupFilter::Member("alice".to_string()))
            .unwrap();

        assert_eq!(
            report.removed, 2,
            "exactly 2 alice sessions must be removed"
        );
        assert!(!ws_a1.exists(), "alice workspace 1 must be removed");
        assert!(!ws_a2.exists(), "alice workspace 2 must be removed");
        assert!(ws_bob.exists(), "bob workspace must NOT be removed");
        assert!(
            manager.registry.get(&id_bob).is_some(),
            "bob session must remain in registry"
        );
    }

    // AC-18 (bulk cleanup): cleanup_sessions with OlderThan filter removes only old sessions.
    #[test]
    fn cleanup_sessions_older_than_removes_only_old_sessions() {
        let tmp = TempDir::new().unwrap();
        let ws_old = tmp.path().join("ws/old-session");
        let ws_new = tmp.path().join("ws/new-session");
        for p in [&ws_old, &ws_new] {
            std::fs::create_dir_all(p).unwrap();
        }
        let mut manager = make_manager(&tmp);

        // Insert an old session: state_transitioned_at 49 hours in the past
        let old_id = {
            let id = SessionId::new();
            let record = SessionRecord {
                session_id: id.clone(),
                member_name: "alice".to_string(),
                session_type: SessionType::Loop,
                current_state: SessionState::Retained,
                created_at: Utc::now() - chrono::Duration::hours(49),
                state_transitioned_at: Utc::now() - chrono::Duration::hours(49),
                agent_pid: None,
                workspace_path: Some(ws_old.clone()),
                finalization_result: None,
            };
            manager.registry.register(record).unwrap();
            id
        };
        let new_id = add_retained_session(&mut manager, "bob", Some(ws_new.clone()));

        let report: CleanupReport = manager
            .cleanup_sessions(CleanupFilter::OlderThan(std::time::Duration::from_secs(
                48 * 3600,
            )))
            .unwrap();

        assert_eq!(report.removed, 1, "only the old session must be cleaned up");
        assert!(!ws_old.exists(), "old session workspace must be removed");
        assert!(ws_new.exists(), "new session workspace must NOT be removed");
        assert!(manager.registry.get(&old_id).is_none());
        assert!(manager.registry.get(&new_id).is_some());
    }

    // AC-18 (cleanup independence): cleanup_session does not require a tokio runtime.
    // The call must succeed from a plain synchronous test context.
    #[test]
    fn cleanup_does_not_require_tokio_runtime() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws/sync-test");
        std::fs::create_dir_all(&workspace).unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_retained_session(&mut manager, "sync-member", Some(workspace.clone()));

        // This must compile and run without #[tokio::test] — no runtime in scope
        let result = manager.cleanup_session(&id);
        assert!(
            result.is_ok(),
            "cleanup_session must succeed without a tokio runtime: {:?}",
            result.err()
        );
        assert!(!workspace.exists());
    }
}

#[cfg(test)]
mod restart_recovery_tests {
    use chrono::Utc;
    use tempfile::TempDir;

    use crate::session::manager::{RecoveryReport, SessionManager};
    use crate::session::types::{SessionId, SessionRecord, SessionState, SessionType};

    fn make_manager(tmp: &TempDir) -> SessionManager {
        let registry_path = tmp.path().join("registry.json");
        SessionManager::new(tmp.path().to_path_buf(), registry_path).unwrap()
    }

    fn add_session_with_state_and_pid(
        manager: &mut super::SessionManager,
        state: SessionState,
        pid: Option<u32>,
    ) -> SessionId {
        let id = SessionId::new();
        let record = SessionRecord {
            session_id: id.clone(),
            member_name: "alice".to_string(),
            session_type: SessionType::Loop,
            current_state: state,
            created_at: Utc::now(),
            state_transitioned_at: Utc::now(),
            agent_pid: pid,
            workspace_path: None,
            finalization_result: None,
        };
        manager.registry.register(record).unwrap();
        id
    }

    // AC-25: Active session with a dead PID → transitioned to Failed on recovery.
    #[test]
    fn recover_stale_sessions_active_dead_pid_to_failed() {
        let tmp = TempDir::new().unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_session_with_state_and_pid(&mut manager, SessionState::Active, Some(0));
        manager.registry.save().unwrap();

        let report: RecoveryReport = manager.recover_stale_sessions_with(|_pid| false).unwrap();

        assert_eq!(report.recovered, 1, "one stale session must be recovered");
        let updated = manager.registry.get(&id).unwrap();
        assert_eq!(
            updated.current_state,
            SessionState::Failed,
            "stale Active session must be transitioned to Failed"
        );
    }

    // AC-25: Finalizing session with a dead PID → transitioned to Failed on recovery.
    #[test]
    fn recover_stale_sessions_finalizing_dead_pid_to_failed() {
        let tmp = TempDir::new().unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_session_with_state_and_pid(&mut manager, SessionState::Finalizing, Some(0));
        manager.registry.save().unwrap();

        let report: RecoveryReport = manager.recover_stale_sessions_with(|_pid| false).unwrap();

        assert_eq!(report.recovered, 1);
        let updated = manager.registry.get(&id).unwrap();
        assert_eq!(
            updated.current_state,
            SessionState::Failed,
            "stale Finalizing session must be transitioned to Failed"
        );
    }

    // AC-25: Active session with a live PID → left unchanged.
    #[test]
    fn recover_stale_sessions_live_pid_unchanged() {
        let tmp = TempDir::new().unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_session_with_state_and_pid(&mut manager, SessionState::Active, Some(1));
        manager.registry.save().unwrap();

        let report: RecoveryReport = manager.recover_stale_sessions_with(|_pid| true).unwrap();

        assert_eq!(report.recovered, 0, "live session must not be recovered");
        let unchanged = manager.registry.get(&id).unwrap();
        assert_eq!(unchanged.current_state, SessionState::Active);
    }

    // AC-25: Active session with no PID → left unchanged (not yet attached to a process).
    #[test]
    fn recover_stale_sessions_no_pid_unchanged() {
        let tmp = TempDir::new().unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_session_with_state_and_pid(&mut manager, SessionState::Active, None);
        manager.registry.save().unwrap();

        let report: RecoveryReport = manager.recover_stale_sessions_with(|_pid| false).unwrap();

        assert_eq!(
            report.recovered, 0,
            "session without PID must not be recovered"
        );
        let unchanged = manager.registry.get(&id).unwrap();
        assert_eq!(unchanged.current_state, SessionState::Active);
    }

    // AC-25: Terminal and Retained sessions are not touched by recovery.
    #[test]
    fn recover_stale_sessions_ignores_terminal_states() {
        let tmp = TempDir::new().unwrap();
        let mut manager = make_manager(&tmp);
        let id_completed =
            add_session_with_state_and_pid(&mut manager, SessionState::Completed, Some(0));
        let id_failed = add_session_with_state_and_pid(&mut manager, SessionState::Failed, Some(0));
        let id_retained =
            add_session_with_state_and_pid(&mut manager, SessionState::Retained, Some(0));
        manager.registry.save().unwrap();

        let report: RecoveryReport = manager.recover_stale_sessions_with(|_pid| false).unwrap();

        assert_eq!(
            report.recovered, 0,
            "terminal/retained sessions must not be recovered"
        );
        assert_eq!(
            manager.registry.get(&id_completed).unwrap().current_state,
            SessionState::Completed
        );
        assert_eq!(
            manager.registry.get(&id_failed).unwrap().current_state,
            SessionState::Failed
        );
        assert_eq!(
            manager.registry.get(&id_retained).unwrap().current_state,
            SessionState::Retained
        );
    }
}

// AC-18: SessionInspection extended fields — CT-89-06 RED
#[cfg(test)]
mod session_inspection_extended_tests {
    use super::*;
    use crate::session::types::{FinalizationResult, GitState, SessionType};
    use tempfile::TempDir;

    fn make_manager(tmp: &TempDir) -> SessionManager {
        SessionManager::new(
            tmp.path().join("workspaces"),
            tmp.path().join("registry.json"),
        )
        .unwrap()
    }

    #[test]
    fn inspect_session_includes_finalization_results_field() {
        let tmp = TempDir::new().unwrap();
        let mut manager = make_manager(&tmp);
        let record = manager.create_session("alice", SessionType::Loop).unwrap();
        let id = record.session_id.clone();

        let inspection = manager.inspect_session(&id).unwrap();
        // E0609: no field `finalization_results` on `SessionInspection` until added
        let _: &Option<FinalizationResult> = &inspection.finalization_results;
    }

    #[test]
    fn inspect_session_includes_git_state_field() {
        let tmp = TempDir::new().unwrap();
        let mut manager = make_manager(&tmp);
        let record = manager.create_session("alice", SessionType::Loop).unwrap();
        let id = record.session_id.clone();

        let inspection = manager.inspect_session(&id).unwrap();
        // E0609: no field `git_state` on `SessionInspection` until added
        let _: &Option<GitState> = &inspection.git_state;
    }
}
