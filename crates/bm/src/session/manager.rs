//! SessionManager: creates and deactivates sessions, delegates to WorkspaceHydrator.

use std::path::PathBuf;

use anyhow::Result;

use super::registry::SessionRegistry;
use super::types::{SessionId, SessionRecord, SessionType};

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
}

/// Manages session lifecycle: creates sessions via the hydrator and tracks them in the registry.
pub struct SessionManager {
    pub(crate) registry: SessionRegistry,
    pub(crate) workspace_base: PathBuf,
}

impl SessionManager {
    /// Create a new SessionManager backed by a registry persisted at `registry_path`.
    pub fn new(workspace_base: PathBuf, registry_path: PathBuf) -> Result<Self> {
        Ok(Self {
            registry: SessionRegistry::load(registry_path)?,
            workspace_base,
        })
    }

    /// Create a new session for `member` of the given `session_type`.
    ///
    /// Delegates workspace setup to the WorkspaceHydrator (CT-02), then registers
    /// the session in the registry as Creating → Active.
    pub fn create_session(
        &mut self,
        member: &str,
        session_type: SessionType,
    ) -> Result<SessionRecord> {
        let _ = (member, session_type);
        unimplemented!("SessionManager::create_session not yet implemented")
    }

    /// Stop the agent for `id`, inspect workspace dirty state, and transition the session to Completed.
    pub fn deactivate_session(&mut self, id: &SessionId) -> Result<DeactivationResult> {
        let _ = id;
        unimplemented!("SessionManager::deactivate_session not yet implemented")
    }

    /// Return all active (non-terminal) sessions.
    pub fn list_active(&self) -> Vec<&SessionRecord> {
        unimplemented!("SessionManager::list_active not yet implemented")
    }

    /// Look up a session by ID. Returns None if the session does not exist.
    pub fn get(&self, id: &SessionId) -> Option<&SessionRecord> {
        let _ = id;
        unimplemented!("SessionManager::get not yet implemented")
    }
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
        assert!(!s.session_id.as_str().is_empty(), "session_id must be non-empty");
        assert_eq!(s.member_name, "bob");
        assert_eq!(s.session_type, SessionType::Brain);
        assert_eq!(s.current_state, SessionState::Active);
        // start_time field corresponds to created_at
        assert!(s.created_at <= Utc::now());
    }

    // AC-12: Daemon required — if no daemon context exists, create_session fails with a clear error
    #[test]
    fn create_session_fails_without_daemon_context() {
        // This test verifies that a session can only be created when the daemon is running.
        // The manager should detect the missing daemon and return an error.
        // (Daemon detection is checked via the daemon config file / socket — not present in tmp dir)
        let (mut manager, _tmp) = make_manager();

        // Create session with Loop type (requires daemon)
        let result = manager.create_session("carol", SessionType::Loop);
        // For this test to pass in GREEN, the impl must check daemon presence.
        // Currently unimplemented — test fails with panic.
        assert!(
            result.is_err(),
            "create_session for Loop/Brain must fail if daemon is not running"
        );
        let err_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            err_msg.contains("daemon") || err_msg.contains("not running") || err_msg.contains("required"),
            "error must mention daemon, got: {err_msg}"
        );
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
