use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

use super::dirty_state::RepoDirtyState;
use super::finalization::deactivation::FinalizationResult;
use super::registry::SessionRegistry;
use super::types::{SessionId, SessionState, SessionType};

/// Structured summary of a retained session for operator inspection.
pub struct SessionInspection {
    pub session_id: SessionId,
    pub member_name: String,
    pub session_type: SessionType,
    pub current_state: SessionState,
    pub workspace_path: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub finalization_result: Option<FinalizationResult>,
    pub git_state: Vec<RepoDirtyState>,
}

/// Report of a single session cleanup operation.
pub struct CleanupReport {
    pub session_id: SessionId,
    pub workspace_removed: bool,
    pub registry_removed: bool,
}

/// Filter for bulk cleanup of retained sessions.
pub enum CleanupFilter {
    AllRetained,
    ByMember(String),
    OlderThan(Duration),
}

/// Inspect a retained session, returning a structured summary.
pub fn inspect_session(
    registry: &SessionRegistry,
    session_id: &SessionId,
) -> Result<SessionInspection> {
    let record = registry
        .get(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;

    Ok(SessionInspection {
        session_id: record.session_id.clone(),
        member_name: record.member_name.clone(),
        session_type: record.session_type.clone(),
        current_state: record.current_state.clone(),
        workspace_path: record.workspace_path.clone(),
        created_at: record.created_at,
        finalization_result: None,
        git_state: vec![],
    })
}

/// Clean up a single retained session: remove workspace directory and remove from registry.
pub fn cleanup_session(
    registry: &mut SessionRegistry,
    session_id: &SessionId,
) -> Result<CleanupReport> {
    let record = registry
        .get(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?
        .clone();

    let workspace_removed = record
        .workspace_path
        .as_ref()
        .is_some_and(|p| p.is_dir() && std::fs::remove_dir_all(p).is_ok());

    registry.remove(session_id)?;

    Ok(CleanupReport {
        session_id: record.session_id,
        workspace_removed,
        registry_removed: true,
    })
}

/// Bulk cleanup of retained sessions matching a filter. Returns one report per cleaned session.
pub fn bulk_cleanup(
    registry: &mut SessionRegistry,
    filter: CleanupFilter,
) -> Result<Vec<CleanupReport>> {
    let ids: Vec<SessionId> = registry
        .list()
        .into_iter()
        .filter(|r| r.current_state == SessionState::Retained)
        .filter(|r| match &filter {
            CleanupFilter::AllRetained => true,
            CleanupFilter::ByMember(name) => r.member_name == *name,
            CleanupFilter::OlderThan(duration) => {
                Utc::now() - r.created_at > *duration
            }
        })
        .map(|r| r.session_id.clone())
        .collect();

    let mut reports = Vec::with_capacity(ids.len());
    for id in &ids {
        reports.push(cleanup_session(registry, id)?);
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::{SessionRecord, SessionState, SessionType};

    fn make_retained_record(member: &str, session_type: SessionType) -> SessionRecord {
        SessionRecord {
            session_id: SessionId::new(),
            member_name: member.to_string(),
            session_type,
            current_state: SessionState::Creating,
            created_at: Utc::now(),
            state_transitioned_at: Utc::now(),
            agent_pid: None,
            workspace_path: Some(PathBuf::from("/tmp/workspace-test")),
        }
    }

    fn register_retained(registry: &mut SessionRegistry, record: SessionRecord) -> SessionId {
        let id = record.session_id.clone();
        registry.register(record).unwrap();
        registry.update_state(&id, SessionState::Active).unwrap();
        registry
            .update_state(&id, SessionState::Completed)
            .unwrap();
        registry
            .update_state(&id, SessionState::Retained)
            .unwrap();
        id
    }

    fn new_registry() -> SessionRegistry {
        let tmp = tempfile::tempdir().unwrap();
        SessionRegistry::new(tmp.path().join("registry.json"))
    }

    // --- AC-18 (partial): Session Inspection ---

    #[test]
    fn inspect_retained_session_returns_workspace_path() {
        let mut registry = new_registry();
        let record = make_retained_record("alice", SessionType::Loop);
        let id = register_retained(&mut registry, record);

        let inspection = inspect_session(&registry, &id).unwrap();

        assert_eq!(
            inspection.workspace_path,
            Some(PathBuf::from("/tmp/workspace-test")),
            "inspection must include the session's workspace path"
        );
    }

    #[test]
    fn inspect_retained_session_returns_member_and_type() {
        let mut registry = new_registry();
        let record = make_retained_record("bob", SessionType::Brain);
        let id = register_retained(&mut registry, record);

        let inspection = inspect_session(&registry, &id).unwrap();

        assert_eq!(
            inspection.member_name, "bob",
            "inspection must include the correct member name"
        );
        assert_eq!(
            inspection.session_type,
            SessionType::Brain,
            "inspection must include the correct session type"
        );
        assert_eq!(
            inspection.current_state,
            SessionState::Retained,
            "inspection must reflect the current Retained state"
        );
    }

    #[test]
    fn inspect_session_returns_correct_session_id() {
        let mut registry = new_registry();
        let record = make_retained_record("carol", SessionType::Interactive);
        let id = register_retained(&mut registry, record);

        let inspection = inspect_session(&registry, &id).unwrap();

        assert_eq!(
            inspection.session_id, id,
            "inspection must return the queried session's ID, not a stub"
        );
    }

    #[test]
    fn inspect_nonexistent_session_returns_error() {
        let registry = new_registry();
        let phantom_id = SessionId::from_raw("nonexist");

        let result = inspect_session(&registry, &phantom_id);

        assert!(
            result.is_err(),
            "inspecting a nonexistent session must return an error"
        );
    }

    // --- Individual Cleanup ---

    #[test]
    fn cleanup_session_removes_from_registry() {
        let mut registry = new_registry();
        let record = make_retained_record("alice", SessionType::Loop);
        let id = register_retained(&mut registry, record);

        cleanup_session(&mut registry, &id).unwrap();

        assert!(
            registry.get(&id).is_none(),
            "session must be removed from registry after cleanup"
        );
    }

    #[test]
    fn cleanup_session_reports_registry_removed() {
        let mut registry = new_registry();
        let record = make_retained_record("bob", SessionType::Interactive);
        let id = register_retained(&mut registry, record);

        let report = cleanup_session(&mut registry, &id).unwrap();

        assert!(
            report.registry_removed,
            "cleanup report must indicate registry entry was removed"
        );
        assert_eq!(
            report.session_id, id,
            "cleanup report must reference the correct session ID"
        );
    }

    #[test]
    fn cleanup_nonexistent_session_returns_error() {
        let mut registry = new_registry();
        let phantom_id = SessionId::from_raw("nonexist");

        let result = cleanup_session(&mut registry, &phantom_id);

        assert!(
            result.is_err(),
            "cleaning up a nonexistent session must return an error"
        );
    }

    // --- Bulk Cleanup ---

    #[test]
    fn bulk_cleanup_by_member_returns_only_matching() {
        let mut registry = new_registry();
        let r1 = make_retained_record("alice", SessionType::Loop);
        let r2 = make_retained_record("bob", SessionType::Interactive);
        let r3 = make_retained_record("alice", SessionType::Brain);
        let id1 = register_retained(&mut registry, r1);
        let _id2 = register_retained(&mut registry, r2);
        let id3 = register_retained(&mut registry, r3);

        let reports =
            bulk_cleanup(&mut registry, CleanupFilter::ByMember("alice".to_string())).unwrap();

        assert_eq!(
            reports.len(),
            2,
            "bulk cleanup by member must return exactly the matching sessions"
        );
        let cleaned_ids: Vec<_> = reports.iter().map(|r| r.session_id.as_str().to_string()).collect();
        assert!(
            cleaned_ids.contains(&id1.as_str().to_string()),
            "alice's first session must be cleaned"
        );
        assert!(
            cleaned_ids.contains(&id3.as_str().to_string()),
            "alice's second session must be cleaned"
        );
    }

    #[test]
    fn bulk_cleanup_by_age_returns_only_old() {
        let mut registry = new_registry();

        let mut old_record = make_retained_record("alice", SessionType::Loop);
        old_record.created_at = Utc::now() - chrono::Duration::hours(48);
        let old_id = register_retained(&mut registry, old_record);

        let fresh_record = make_retained_record("bob", SessionType::Interactive);
        let _fresh_id = register_retained(&mut registry, fresh_record);

        let reports = bulk_cleanup(
            &mut registry,
            CleanupFilter::OlderThan(chrono::Duration::hours(24)),
        )
        .unwrap();

        assert_eq!(
            reports.len(),
            1,
            "bulk cleanup by age must return only sessions older than the threshold"
        );
        assert_eq!(
            reports[0].session_id, old_id,
            "only the old session must be cleaned"
        );
    }

    #[test]
    fn bulk_cleanup_all_retained_returns_all() {
        let mut registry = new_registry();
        let r1 = make_retained_record("alice", SessionType::Loop);
        let r2 = make_retained_record("bob", SessionType::Interactive);
        register_retained(&mut registry, r1);
        register_retained(&mut registry, r2);

        let reports = bulk_cleanup(&mut registry, CleanupFilter::AllRetained).unwrap();

        assert_eq!(
            reports.len(),
            2,
            "bulk cleanup AllRetained must return all retained sessions"
        );
    }

    // --- Cleanup Independence from GC ---

    #[test]
    fn cleanup_does_not_conflict_with_concurrent_cleanup() {
        let mut registry = new_registry();
        let r1 = make_retained_record("alice", SessionType::Loop);
        let r2 = make_retained_record("bob", SessionType::Brain);
        let id1 = register_retained(&mut registry, r1);
        let id2 = register_retained(&mut registry, r2);

        let report1 = cleanup_session(&mut registry, &id1);
        let report2 = cleanup_session(&mut registry, &id2);

        assert!(
            report1.is_ok(),
            "first cleanup must succeed independently"
        );
        assert!(
            report2.is_ok(),
            "second cleanup must succeed independently — no conflict with first"
        );
        assert!(
            registry.get(&id1).is_none(),
            "first session must be removed after cleanup"
        );
        assert!(
            registry.get(&id2).is_none(),
            "second session must be removed after cleanup"
        );
    }
}
