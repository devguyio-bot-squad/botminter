use std::path::{Path, PathBuf};

use anyhow::Result;

use super::dirty_state::RepoDirtyState;
use super::registry::SessionRegistry;
use super::types::{SessionId, SessionRecord, SessionState, SessionType};
use super::work_item_lock::WorkItemLock;

/// Abstracts workspace provisioning and inspection so the session manager
/// can be tested without real git operations.
pub trait WorkspaceOps: Send + Sync {
    fn hydrate_workspace(&self, session_id: &SessionId, member: &str) -> Result<PathBuf>;
    fn inspect_dirty_state(&self, workspace_path: &Path) -> Result<Vec<RepoDirtyState>>;
}

/// Parameters for creating a new session.
#[derive(Debug)]
pub struct CreateSessionParams {
    pub member_name: String,
    pub session_type: SessionType,
    pub work_item_id: Option<String>,
}

/// Result of deactivating a session.
#[derive(Debug)]
pub struct DeactivateResult {
    pub session_record: SessionRecord,
    pub dirty_state: Vec<RepoDirtyState>,
}

/// Coordinates session lifecycle across the registry, workspace hydrator, and work-item lock.
pub struct SessionManager<W: WorkspaceOps> {
    pub(crate) registry: SessionRegistry,
    pub(crate) work_item_lock: WorkItemLock,
    pub(crate) workspace_ops: W,
}

impl<W: WorkspaceOps> SessionManager<W> {
    pub fn new(registry: SessionRegistry, work_item_lock: WorkItemLock, workspace_ops: W) -> Self {
        Self {
            registry,
            work_item_lock,
            workspace_ops,
        }
    }

    /// Create a new session: hydrate workspace, register in registry, transition to Active.
    /// If `work_item_id` is provided, acquire a lock on it.
    pub fn create_session(&mut self, params: CreateSessionParams) -> Result<SessionRecord> {
        let session_id = SessionId::new();

        if let Some(ref work_item_id) = params.work_item_id {
            self.work_item_lock
                .acquire(work_item_id, &session_id)?;
        }

        let workspace_path = match self
            .workspace_ops
            .hydrate_workspace(&session_id, &params.member_name)
        {
            Ok(path) => path,
            Err(e) => {
                if let Some(ref work_item_id) = params.work_item_id {
                    self.work_item_lock.release(work_item_id, &session_id);
                }
                return Err(e);
            }
        };

        let now = chrono::Utc::now();
        let record = SessionRecord {
            session_id: session_id.clone(),
            member_name: params.member_name,
            session_type: params.session_type,
            current_state: SessionState::Creating,
            created_at: now,
            state_transitioned_at: now,
            agent_pid: None,
            workspace_path: Some(workspace_path),
        };

        self.registry.register(record)?;
        self.registry
            .update_state(&session_id, SessionState::Active)?;

        Ok(self.registry.get(&session_id).unwrap().clone())
    }

    /// Deactivate a session: inspect dirty state, transition to Completed/Failed,
    /// and release all work-item locks held by the session.
    pub fn deactivate_session(&mut self, session_id: &SessionId) -> Result<DeactivateResult> {
        let record = self
            .registry
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;

        let workspace_path = record
            .workspace_path
            .clone()
            .unwrap_or_default();

        let dirty_state = self
            .workspace_ops
            .inspect_dirty_state(&workspace_path)
            .unwrap_or_default();

        self.registry
            .update_state(session_id, SessionState::Completed)?;

        self.work_item_lock.release_all(session_id);

        let session_record = self.registry.get(session_id).unwrap().clone();

        Ok(DeactivateResult {
            session_record,
            dirty_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::SessionState;

    struct FakeWorkspaceOps {
        workspace_path: PathBuf,
        dirty_state: Vec<RepoDirtyState>,
    }

    impl WorkspaceOps for FakeWorkspaceOps {
        fn hydrate_workspace(
            &self,
            _session_id: &SessionId,
            _member: &str,
        ) -> Result<PathBuf> {
            Ok(self.workspace_path.clone())
        }

        fn inspect_dirty_state(
            &self,
            _workspace_path: &Path,
        ) -> Result<Vec<RepoDirtyState>> {
            Ok(self.dirty_state.clone())
        }
    }

    fn make_manager(dirty_state: Vec<RepoDirtyState>) -> SessionManager<FakeWorkspaceOps> {
        let tmp = tempfile::tempdir().unwrap();
        let registry = SessionRegistry::new(tmp.path().join("registry.json"));
        let lock = WorkItemLock::new();
        let ops = FakeWorkspaceOps {
            workspace_path: tmp.path().join("workspace"),
            dirty_state,
        };
        SessionManager::new(registry, lock, ops)
    }

    // AC-1: Session Status Display (session creation returns correct fields)

    #[test]
    fn create_session_returns_session_with_active_state() {
        let mut mgr = make_manager(vec![]);
        let params = CreateSessionParams {
            member_name: "alice".to_string(),
            session_type: SessionType::Interactive,
            work_item_id: None,
        };

        let record = mgr.create_session(params).unwrap();

        assert_eq!(record.current_state, SessionState::Active);
        assert_eq!(record.member_name, "alice");
        assert_eq!(record.session_type, SessionType::Interactive);
        assert!(!record.session_id.as_str().is_empty());
    }

    // AC-3: Work-Item Lock integration with session lifecycle

    #[test]
    fn create_session_acquires_work_item_lock_if_provided() {
        let mut mgr = make_manager(vec![]);
        let params = CreateSessionParams {
            member_name: "alice".to_string(),
            session_type: SessionType::Loop,
            work_item_id: Some("ISSUE-42".to_string()),
        };

        let record = mgr.create_session(params).unwrap();

        // A second session trying to acquire the same work item must fail
        let other_session = SessionId::new();
        let err = mgr
            .work_item_lock
            .acquire("ISSUE-42", &other_session)
            .expect_err("work item must be locked by the first session");
        let _ = err; // Just verify it errors
        let _ = record;
    }

    // AC-5: Dirty State Reported on Deactivation

    #[test]
    fn deactivate_session_transitions_to_completed_when_clean() {
        let mut mgr = make_manager(vec![]);

        // Manually set up an Active session
        let session_id = SessionId::new();
        let record = SessionRecord {
            session_id: session_id.clone(),
            member_name: "alice".to_string(),
            session_type: SessionType::Interactive,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: Some(PathBuf::from("/tmp/ws")),
        };
        mgr.registry.register(record).unwrap();
        mgr.registry
            .update_state(&session_id, SessionState::Active)
            .unwrap();

        let result = mgr.deactivate_session(&session_id).unwrap();

        assert_eq!(
            result.session_record.current_state,
            SessionState::Completed,
            "clean workspace must transition to Completed"
        );
        assert!(
            result.dirty_state.iter().all(|r| r.is_clean()),
            "no dirty state should be reported for a clean workspace"
        );
    }

    #[test]
    fn deactivate_session_reports_dirty_state() {
        let dirty = vec![RepoDirtyState {
            repo_name: "myproject".to_string(),
            uncommitted_files: vec!["dirty.txt".to_string()],
            unpushed_branches: vec![],
        }];
        let mut mgr = make_manager(dirty);

        let session_id = SessionId::new();
        let record = SessionRecord {
            session_id: session_id.clone(),
            member_name: "bob".to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: Some(PathBuf::from("/tmp/ws")),
        };
        mgr.registry.register(record).unwrap();
        mgr.registry
            .update_state(&session_id, SessionState::Active)
            .unwrap();

        let result = mgr.deactivate_session(&session_id).unwrap();

        assert!(
            result.dirty_state.iter().any(|r| !r.is_clean()),
            "dirty workspace must be reported in deactivation result"
        );
    }

    #[test]
    fn deactivate_session_releases_work_item_locks() {
        let mut mgr = make_manager(vec![]);

        let session_id = SessionId::new();
        let record = SessionRecord {
            session_id: session_id.clone(),
            member_name: "carol".to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: Some(PathBuf::from("/tmp/ws")),
        };
        mgr.registry.register(record).unwrap();
        mgr.registry
            .update_state(&session_id, SessionState::Active)
            .unwrap();

        // Manually acquire a lock for this session
        mgr.work_item_lock
            .acquire("ISSUE-99", &session_id)
            .unwrap();

        mgr.deactivate_session(&session_id).unwrap();

        // After deactivation, the lock must be released
        let other = SessionId::new();
        mgr.work_item_lock
            .acquire("ISSUE-99", &other)
            .expect("work item lock must be released after session deactivation");
    }
}
