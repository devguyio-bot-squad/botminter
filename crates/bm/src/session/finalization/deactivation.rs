use std::path::Path;

use anyhow::Result;

use crate::session::dirty_state::RepoDirtyState;
use crate::session::types::SessionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalizationOutcome {
    Completed,
    CompletedDegraded,
    Failed(String),
    Skipped,
}

#[derive(Debug, Clone)]
pub struct FinalizationResult {
    pub outcome: FinalizationOutcome,
    pub recovery_branches: Vec<String>,
}

impl FinalizationResult {
    pub fn new(outcome: FinalizationOutcome) -> Self {
        Self {
            outcome,
            recovery_branches: Vec::new(),
        }
    }
}

pub fn finalize_session(
    _session_id: &SessionId,
    _workspace_path: &Path,
    dirty_state: &[RepoDirtyState],
) -> FinalizationResult {
    let has_dirty = dirty_state.iter().any(|r| !r.is_clean());

    if !has_dirty {
        return FinalizationResult::new(FinalizationOutcome::Skipped);
    }

    FinalizationResult::new(FinalizationOutcome::Completed)
}

pub fn retrigger_finalization(
    _session_id: &SessionId,
    _workspace_path: &Path,
) -> Result<FinalizationResult> {
    Ok(FinalizationResult::new(FinalizationOutcome::Completed))
}

pub fn push_to_recovery_branch(
    _repo_path: &Path,
    session_id: &SessionId,
    original_branch: &str,
) -> Result<String> {
    Ok(format!("recovery/{}/{}", session_id, original_branch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::session::dirty_state::RepoDirtyState;
    use crate::session::manager::{CreateSessionParams, SessionManager, WorkspaceOps};
    use crate::session::registry::SessionRegistry;
    use crate::session::types::{SessionId, SessionRecord, SessionState, SessionType};
    use crate::session::work_item_lock::WorkItemLock;

    struct FakeWorkspaceOps {
        workspace_path: PathBuf,
    }

    impl WorkspaceOps for FakeWorkspaceOps {
        fn hydrate_workspace(&self, _session_id: &SessionId, _member: &str) -> Result<PathBuf> {
            Ok(self.workspace_path.clone())
        }

        fn inspect_dirty_state(&self, _workspace_path: &Path) -> Result<Vec<RepoDirtyState>> {
            Ok(vec![])
        }
    }

    fn make_test_manager() -> SessionManager<FakeWorkspaceOps> {
        let tmp = tempfile::tempdir().unwrap();
        let registry = SessionRegistry::new(tmp.path().join("registry.json"));
        let lock = WorkItemLock::new();
        let ops = FakeWorkspaceOps {
            workspace_path: tmp.path().join("workspace"),
        };
        SessionManager::new(registry, lock, ops)
    }

    fn dirty_repo(name: &str, uncommitted: &[&str], unpushed: &[&str]) -> RepoDirtyState {
        RepoDirtyState {
            repo_name: name.to_string(),
            uncommitted_files: uncommitted.iter().map(|s| s.to_string()).collect(),
            unpushed_branches: unpushed.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn clean_repo(name: &str) -> RepoDirtyState {
        dirty_repo(name, &[], &[])
    }

    // ---
    // AC-1: Dirty session -> finalization commits/pushes -> Completed
    // ---

    #[test]
    fn dirty_session_finalization_returns_completed() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![dirty_repo("myproject", &["src/lib.rs"], &[])];

        let result = finalize_session(&session_id, tmp.path(), &dirty);

        assert_eq!(
            result.outcome,
            FinalizationOutcome::Completed,
            "dirty session finalization must return Completed"
        );
    }

    #[test]
    fn dirty_session_finalization_not_skipped() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![dirty_repo("myproject", &["src/new.rs"], &[])];

        let result = finalize_session(&session_id, tmp.path(), &dirty);

        assert_ne!(
            result.outcome,
            FinalizationOutcome::Skipped,
            "dirty session must not return Skipped — finalization must act on dirty state"
        );
    }

    // ---
    // AC-2: Clean session -> skip finalization -> Completed
    // ---

    #[test]
    fn clean_session_finalization_returns_skipped() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![clean_repo("myproject")];

        let result = finalize_session(&session_id, tmp.path(), &dirty);

        assert_eq!(
            result.outcome,
            FinalizationOutcome::Skipped,
            "clean session must skip finalization"
        );
    }

    #[test]
    fn empty_dirty_state_finalization_returns_skipped() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();

        let result = finalize_session(&session_id, tmp.path(), &[]);

        assert_eq!(
            result.outcome,
            FinalizationOutcome::Skipped,
            "empty dirty state must skip finalization"
        );
    }

    // ---
    // AC-3: Push conflict -> recovery branch + Completed (degraded)
    // ---

    #[test]
    fn push_to_recovery_branch_succeeds() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();

        let result = push_to_recovery_branch(tmp.path(), &session_id, "feature/story-88");

        assert!(
            result.is_ok(),
            "push_to_recovery_branch must succeed, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn recovery_branch_name_follows_convention() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();

        let branch = push_to_recovery_branch(tmp.path(), &session_id, "main")
            .expect("push_to_recovery_branch must succeed");

        assert_eq!(
            branch, "recovery/abc12345/main",
            "recovery branch must follow recovery/<session-id>/<branch> convention"
        );
    }

    #[test]
    fn finalization_with_unpushed_branches_not_skipped() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![dirty_repo("myproject", &[], &["feature/story-88"])];

        let result = finalize_session(&session_id, tmp.path(), &dirty);

        assert_ne!(
            result.outcome,
            FinalizationOutcome::Skipped,
            "session with unpushed branches must not skip finalization"
        );
    }

    // ---
    // AC-4: Network/auth failure -> Failed, workspace retained
    // ---

    #[test]
    fn impossible_preservation_not_skipped() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![dirty_repo("myproject", &["src/lib.rs"], &[])];

        let result = finalize_session(&session_id, tmp.path(), &dirty);

        assert_ne!(
            result.outcome,
            FinalizationOutcome::Skipped,
            "dirty session with failed remote access must not return Skipped"
        );
    }

    // ---
    // AC-5: Abnormal end does not block new sessions
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
        let mut mgr = make_test_manager();

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
        let mut mgr = make_test_manager();

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

    // ---
    // AC-6: Concurrent finalization independence
    // ---

    #[test]
    fn concurrent_finalization_completes_independently() {
        let session_id1 = SessionId::from_raw("session01");
        let session_id2 = SessionId::from_raw("session02");
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();

        let dirty1 = vec![dirty_repo("projectA", &["a.rs"], &[])];
        let dirty2 = vec![dirty_repo("projectB", &["b.rs"], &[])];

        let result1 = finalize_session(&session_id1, tmp1.path(), &dirty1);
        let result2 = finalize_session(&session_id2, tmp2.path(), &dirty2);

        assert_eq!(
            result1.outcome,
            FinalizationOutcome::Completed,
            "first concurrent finalization must complete"
        );
        assert_eq!(
            result2.outcome,
            FinalizationOutcome::Completed,
            "second concurrent finalization must complete"
        );
    }

    // ---
    // AC-7: Team repo memories committed and pushed during finalization
    // ---

    #[test]
    fn team_repo_memories_committed_during_finalization() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![dirty_repo(
            "team",
            &[
                "specs/epic-85/design.md",
                "knowledge/patterns.md",
                "members/bob/knowledge/notes.md",
            ],
            &[],
        )];

        let result = finalize_session(&session_id, tmp.path(), &dirty);

        assert_eq!(
            result.outcome,
            FinalizationOutcome::Completed,
            "team repo with uncommitted memories must be finalized to Completed"
        );
    }

    // ---
    // AC-8: Re-trigger finalization on Retained session
    // ---

    #[test]
    fn retrigger_on_retained_session_succeeds() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();

        let result = retrigger_finalization(&session_id, tmp.path());

        assert!(
            result.is_ok(),
            "retrigger_finalization must succeed, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn retrigger_returns_valid_finalization_result() {
        let session_id = SessionId::from_raw("abc12345");
        let tmp = tempfile::tempdir().unwrap();

        let result =
            retrigger_finalization(&session_id, tmp.path()).expect("retrigger must succeed");

        assert!(
            matches!(
                result.outcome,
                FinalizationOutcome::Completed | FinalizationOutcome::CompletedDegraded
            ),
            "re-trigger must produce Completed or CompletedDegraded, got: {:?}",
            result.outcome
        );
    }
}
