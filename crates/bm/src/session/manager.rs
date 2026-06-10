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
            finalization_result: None,
        };

        self.registry.register(record)?;
        self.registry
            .update_state(&session_id, SessionState::Active)?;

        Ok(self.registry.get(&session_id).unwrap().clone())
    }

    /// Return sessions in terminal states (Completed, Failed, Killed).
    pub fn list_terminal(&self) -> Vec<SessionRecord> {
        self.registry
            .list()
            .into_iter()
            .filter(|r| r.current_state.is_terminal())
            .cloned()
            .collect()
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

    #[test]
    fn list_terminal_returns_only_terminal_state_sessions() {
        let mut mgr = make_manager(vec![]);

        // Active session — NOT terminal
        let active_id = SessionId::new();
        let active = SessionRecord {
            session_id: active_id.clone(),
            member_name: "alice".to_string(),
            session_type: SessionType::Interactive,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: None,
            finalization_result: None,
        };
        mgr.registry.register(active).unwrap();
        mgr.registry
            .update_state(&active_id, SessionState::Active)
            .unwrap();

        // Completed session — terminal
        let completed_id = SessionId::new();
        let completed = SessionRecord {
            session_id: completed_id.clone(),
            member_name: "bob".to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: None,
            finalization_result: None,
        };
        mgr.registry.register(completed).unwrap();
        mgr.registry
            .update_state(&completed_id, SessionState::Active)
            .unwrap();
        mgr.registry
            .update_state(&completed_id, SessionState::Completed)
            .unwrap();

        // Failed session — terminal
        let failed_id = SessionId::new();
        let failed = SessionRecord {
            session_id: failed_id.clone(),
            member_name: "carol".to_string(),
            session_type: SessionType::Brain,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: None,
            finalization_result: None,
        };
        mgr.registry.register(failed).unwrap();
        mgr.registry
            .update_state(&failed_id, SessionState::Active)
            .unwrap();
        mgr.registry
            .update_state(&failed_id, SessionState::Failed)
            .unwrap();

        // Killed session — terminal
        let killed_id = SessionId::new();
        let killed = SessionRecord {
            session_id: killed_id.clone(),
            member_name: "dan".to_string(),
            session_type: SessionType::Interactive,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: None,
            finalization_result: None,
        };
        mgr.registry.register(killed).unwrap();
        mgr.registry
            .update_state(&killed_id, SessionState::Active)
            .unwrap();
        mgr.registry
            .update_state(&killed_id, SessionState::Killed)
            .unwrap();

        // Finalizing session — NOT terminal
        let finalizing_id = SessionId::new();
        let finalizing = SessionRecord {
            session_id: finalizing_id.clone(),
            member_name: "eve".to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: None,
            finalization_result: None,
        };
        mgr.registry.register(finalizing).unwrap();
        mgr.registry
            .update_state(&finalizing_id, SessionState::Active)
            .unwrap();
        mgr.registry
            .update_state(&finalizing_id, SessionState::Finalizing)
            .unwrap();

        // Retained session — NOT terminal
        let retained_id = SessionId::new();
        let retained = SessionRecord {
            session_id: retained_id.clone(),
            member_name: "frank".to_string(),
            session_type: SessionType::Brain,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: None,
            finalization_result: None,
        };
        mgr.registry.register(retained).unwrap();
        mgr.registry
            .update_state(&retained_id, SessionState::Active)
            .unwrap();
        mgr.registry
            .update_state(&retained_id, SessionState::Completed)
            .unwrap();
        mgr.registry
            .update_state(&retained_id, SessionState::Retained)
            .unwrap();

        let terminals = mgr.list_terminal();

        assert_eq!(
            terminals.len(),
            3,
            "should return exactly 3 terminal sessions (Completed, Failed, Killed)"
        );

        let ids: Vec<String> = terminals.iter().map(|r| r.session_id.to_string()).collect();
        assert!(
            ids.contains(&completed_id.to_string()),
            "Completed session must be included"
        );
        assert!(
            ids.contains(&failed_id.to_string()),
            "Failed session must be included"
        );
        assert!(
            ids.contains(&killed_id.to_string()),
            "Killed session must be included"
        );
    }

    // ---
    // State transition invariants
    // ---

    #[test]
    fn retained_to_finalizing_is_valid_transition() {
        assert!(
            SessionState::Retained.can_transition_to(&SessionState::Finalizing),
            "Retained -> Finalizing must be valid to support re-trigger finalization"
        );
    }

    #[test]
    fn new_session_while_old_is_finalizing() {
        let mut mgr = make_manager(vec![]);

        let session_id = SessionId::new();
        let record = SessionRecord {
            session_id: session_id.clone(),
            member_name: "alice".to_string(),
            session_type: SessionType::Interactive,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: Some(PathBuf::from("/tmp/ws1")),
            finalization_result: None,
        };
        mgr.registry.register(record).unwrap();
        mgr.registry
            .update_state(&session_id, SessionState::Active)
            .unwrap();
        mgr.registry
            .update_state(&session_id, SessionState::Finalizing)
            .unwrap();

        let params = CreateSessionParams {
            member_name: "alice".to_string(),
            session_type: SessionType::Interactive,
            work_item_id: None,
        };
        let result = mgr.create_session(params);

        assert!(
            result.is_ok(),
            "new session must succeed when old session is Finalizing"
        );
    }

    #[test]
    fn new_session_while_old_is_failed() {
        let mut mgr = make_manager(vec![]);

        let session_id = SessionId::new();
        let record = SessionRecord {
            session_id: session_id.clone(),
            member_name: "alice".to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Creating,
            created_at: chrono::Utc::now(),
            state_transitioned_at: chrono::Utc::now(),
            agent_pid: None,
            workspace_path: Some(PathBuf::from("/tmp/ws1")),
            finalization_result: None,
        };
        mgr.registry.register(record).unwrap();
        mgr.registry
            .update_state(&session_id, SessionState::Active)
            .unwrap();
        mgr.registry
            .update_state(&session_id, SessionState::Failed)
            .unwrap();

        let params = CreateSessionParams {
            member_name: "alice".to_string(),
            session_type: SessionType::Interactive,
            work_item_id: None,
        };
        let result = mgr.create_session(params);

        assert!(
            result.is_ok(),
            "new session must succeed when old session is Failed"
        );
    }
}
