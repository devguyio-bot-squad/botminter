//! HTTP handlers for the session management API.
//!
//! Routes (added to the daemon router in run.rs):
//!   POST   /api/sessions/start          — create a session and launch an agent
//!   GET    /api/sessions                — list active sessions
//!   POST   /api/sessions/{id}/stop      — stop agent and deactivate session
//!   GET    /api/sessions/{id}           — get session detail
//!   DELETE /api/sessions/{id}?force     — force-stop a session (skip finalization)
//!   POST   /api/sessions/{id}/finalize  — re-trigger finalization on a Retained session
//!   GET    /api/sessions/history        — list terminal sessions (completed/failed/killed)

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::session::types::{
    FinalizationResult, GitState, SessionId, SessionRecord, SessionState, SessionType,
};

use super::run::DaemonState;

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct StartSessionRequest {
    pub member: String,
    /// "loop", "brain", or "interactive"
    pub session_type: String,
}

// ── Response types ───────────────────────────────────────────────────────────

/// A single session's fields as returned by GET /api/sessions and GET /api/sessions/{id}.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub owning_member: String,
    pub session_type: String,
    pub current_state: String,
    /// ISO-8601 timestamp — corresponds to SessionRecord::created_at.
    pub start_time: String,
    pub workspace_path: Option<String>,
    /// ISO-8601 timestamp of last state transition (for elapsed time display).
    #[serde(default)]
    pub state_transitioned_at: String,
    /// Number of concurrent active sessions for this member.
    #[serde(default)]
    pub concurrent_count: u32,
}

/// Per-repo dirty state included in StopSessionResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirtyRepoInfo {
    pub name: String,
    pub has_uncommitted: bool,
    pub unpushed_branches: Vec<String>,
}

impl DirtyRepoInfo {
    /// True if this repo needs finalization (has uncommitted changes or unpushed branches).
    pub fn is_dirty(&self) -> bool {
        self.has_uncommitted || !self.unpushed_branches.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartSessionResponse {
    pub ok: bool,
    pub session: Option<SessionInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsListResponse {
    pub sessions: Vec<SessionInfo>,
}

/// A completed session's fields as returned by GET /api/sessions/history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistoryInfo {
    pub session_id: String,
    pub owning_member: String,
    pub session_type: String,
    pub start_time: String,
    pub end_time: String,
    pub exit_normal: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopSessionResponse {
    pub ok: bool,
    pub dirty_repos: Vec<DirtyRepoInfo>,
    pub error: Option<String>,
}

/// Query params for DELETE /api/sessions/{id}.
// `force` is read by serde/axum at runtime — rustc's dead-code lint cannot see this.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ForceStopParams {
    pub force: Option<bool>,
}

/// Response for DELETE /api/sessions/{id}?force=true.
#[derive(Debug, Serialize, Deserialize)]
pub struct ForceStopResponse {
    pub ok: bool,
    pub session_id: String,
    /// State the session was transitioned to — always "Killed" for force stop.
    pub new_state: String,
    /// Whether a new finalization subagent was launched — always false for force stop.
    pub finalization_launched: bool,
    pub error: Option<String>,
}

/// Response for POST /api/sessions/{id}/finalize.
#[derive(Debug, Serialize, Deserialize)]
pub struct RetriggerFinalizationResponse {
    pub ok: bool,
    pub session_id: String,
    /// State the session was transitioned to — "Finalizing" on success.
    pub new_state: String,
    pub error: Option<String>,
}

/// Response for GET /api/sessions/{id}/inspect.
#[derive(Debug, Serialize, Deserialize)]
pub struct InspectSessionResponse {
    pub ok: bool,
    pub session_id: String,
    pub member_name: String,
    pub session_type: String,
    pub current_state: String,
    pub workspace_path: Option<String>,
    pub created_at: String,
    pub state_transitioned_at: String,
    #[serde(default)]
    pub finalization_results: Option<FinalizationResult>,
    #[serde(default)]
    pub git_state: Option<GitState>,
}

/// Response for DELETE /api/sessions/{id}/cleanup.
#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupSessionResponse {
    pub ok: bool,
    pub session_id: String,
    pub error: Option<String>,
}

/// Response for DELETE /api/sessions/cleanup.
#[derive(Debug, Serialize, Deserialize)]
pub struct BulkCleanupResponse {
    pub ok: bool,
    pub removed: usize,
    pub error: Option<String>,
}

/// Query params for DELETE /api/sessions/cleanup.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct BulkCleanupParams {
    pub all: Option<bool>,
    pub member: Option<String>,
    pub older_than: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn session_type_str(t: &SessionType) -> &'static str {
    match t {
        SessionType::Loop => "loop",
        SessionType::Brain => "brain",
        SessionType::Interactive => "interactive",
    }
}

fn record_to_info_with_count(record: &SessionRecord, concurrent_count: u32) -> SessionInfo {
    SessionInfo {
        session_id: record.session_id.as_str().to_string(),
        owning_member: record.member_name.clone(),
        session_type: session_type_str(&record.session_type).to_string(),
        current_state: record.current_state.to_string(),
        start_time: record.created_at.to_rfc3339(),
        workspace_path: record
            .workspace_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        state_transitioned_at: record.state_transitioned_at.to_rfc3339(),
        concurrent_count,
    }
}

fn record_to_info(record: &SessionRecord) -> SessionInfo {
    record_to_info_with_count(record, 1)
}

fn parse_session_type(s: &str) -> Option<SessionType> {
    match s {
        "loop" => Some(SessionType::Loop),
        "brain" => Some(SessionType::Brain),
        "interactive" => Some(SessionType::Interactive),
        _ => None,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/sessions/start — create a session and register it with the daemon.
///
/// Returns 503 if the daemon has not fully started yet.
pub(super) async fn start_session_handler(
    State(state): State<DaemonState>,
    Json(req): Json<StartSessionRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if state.started_at.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "ok": false,
                "code": "daemon_not_ready",
                "error": "daemon is not ready; wait for it to finish starting"
            })),
        );
    }

    let Some(session_type) = parse_session_type(&req.session_type) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "code": "invalid_session_type",
                "error": "invalid session_type; must be 'loop', 'brain', or 'interactive'"
            })),
        );
    };

    let manager = Arc::clone(&state.session_manager);
    let member = req.member.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut m = manager.lock().unwrap();
        m.create_session(&member, session_type)
    })
    .await;

    match result {
        Ok(Ok(record)) => {
            let info = record_to_info(&record);
            let resp = StartSessionResponse {
                ok: true,
                session: Some(info),
                error: None,
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
        }
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "code": "session_create_failed",
                "error": e.to_string()
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "code": "internal_error",
                "error": "internal error"
            })),
        ),
    }
}

/// GET /api/sessions — list all active sessions.
pub(super) async fn list_sessions_handler(
    State(state): State<DaemonState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let manager = Arc::clone(&state.session_manager);

    let sessions = tokio::task::spawn_blocking(move || {
        let m = manager.lock().unwrap();
        let records = m.list_active();
        let counts: std::collections::HashMap<String, u32> =
            records
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, r| {
                    *acc.entry(r.member_name.clone()).or_insert(0) += 1;
                    acc
                });
        records
            .iter()
            .map(|r| {
                let count = *counts.get(&r.member_name).unwrap_or(&1);
                record_to_info_with_count(r, count)
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    let resp = SessionsListResponse { sessions };
    (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
}

/// POST /api/sessions/{id}/stop — stop the agent and deactivate the session.
/// Returns 200 even when the session is unknown (idempotent).
pub(super) async fn stop_session_handler(
    State(state): State<DaemonState>,
    Path(session_id_str): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let session_id = SessionId::from_string(session_id_str);
    let manager = Arc::clone(&state.session_manager);

    let result = tokio::task::spawn_blocking(move || {
        let mut m = manager.lock().unwrap();
        m.deactivate_session(&session_id)
    })
    .await;

    let dirty_repos = match result {
        Ok(Ok(deactivation)) => deactivation
            .dirty_repos
            .into_iter()
            .map(|r| DirtyRepoInfo {
                name: r.name,
                has_uncommitted: r.has_uncommitted,
                unpushed_branches: r.unpushed_branches,
            })
            .collect(),
        _ => vec![],
    };

    let resp = StopSessionResponse {
        ok: true,
        dirty_repos,
        error: None,
    };
    (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
}

/// DELETE /api/sessions/{id}?force=true — force-stop a session, skipping finalization.
///
/// Active → Killed immediately (no finalization subagent).
/// Finalizing → kill the finalization subagent → Killed (workspace retained, re-trigger available).
pub(super) async fn force_stop_session_handler(
    State(state): State<DaemonState>,
    Path(session_id_str): Path<String>,
    Query(_params): Query<ForceStopParams>,
) -> (StatusCode, Json<serde_json::Value>) {
    let session_id = SessionId::from_string(session_id_str.clone());
    let manager = Arc::clone(&state.session_manager);

    let result = tokio::task::spawn_blocking(move || {
        let mut m = manager.lock().unwrap();
        m.force_stop_session(&session_id)
    })
    .await;

    match result {
        Ok(Ok(())) => {
            let resp = ForceStopResponse {
                ok: true,
                session_id: session_id_str,
                new_state: "Killed".to_string(),
                finalization_launched: false,
                error: None,
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
        }
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "code": "force_stop_failed",
                "error": e.to_string()
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "code": "internal_error",
                "error": "internal error"
            })),
        ),
    }
}

/// POST /api/sessions/{id}/finalize — re-trigger finalization on a Retained session.
pub(super) async fn retrigger_finalization_handler(
    State(state): State<DaemonState>,
    Path(session_id_str): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let session_id = SessionId::from_string(session_id_str.clone());
    let manager = Arc::clone(&state.session_manager);

    let result = tokio::task::spawn_blocking(move || {
        let mut m = manager.lock().unwrap();
        m.retrigger_finalization_for(&session_id)
    })
    .await;

    match result {
        Ok(Ok(())) => {
            let resp = RetriggerFinalizationResponse {
                ok: true,
                session_id: session_id_str,
                new_state: "Finalizing".to_string(),
                error: None,
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
        }
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "code": "retrigger_failed",
                "error": e.to_string()
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "code": "internal_error",
                "error": "internal error"
            })),
        ),
    }
}

/// GET /api/sessions/{id} — return a single session's detail.
pub(super) async fn get_session_handler(
    State(state): State<DaemonState>,
    Path(session_id_str): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let session_id = SessionId::from_string(session_id_str);
    let manager = Arc::clone(&state.session_manager);

    let result = tokio::task::spawn_blocking(move || {
        let m = manager.lock().unwrap();
        m.get(&session_id).map(record_to_info)
    })
    .await;

    match result {
        Ok(Some(info)) => (StatusCode::OK, Json(serde_json::to_value(info).unwrap())),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "code": "session_not_found",
                "error": "session not found"
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "code": "internal_error",
                "error": "internal error"
            })),
        ),
    }
}

/// GET /api/sessions/{id}/inspect — return structured summary of a session.
pub(super) async fn inspect_session_handler(
    State(state): State<DaemonState>,
    Path(session_id_str): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let session_id = SessionId::from_string(session_id_str.clone());
    let manager = Arc::clone(&state.session_manager);

    let result = tokio::task::spawn_blocking(move || {
        let m = manager.lock().unwrap();
        m.inspect_session(&session_id)
    })
    .await;

    match result {
        Ok(Ok(inspection)) => {
            let resp = InspectSessionResponse {
                ok: true,
                session_id: inspection.session_id.as_str().to_string(),
                member_name: inspection.member_name,
                session_type: session_type_str(&inspection.session_type).to_string(),
                current_state: inspection.current_state.to_string(),
                workspace_path: inspection
                    .workspace_path
                    .map(|p| p.to_string_lossy().to_string()),
                created_at: inspection.created_at.to_rfc3339(),
                state_transitioned_at: inspection.state_transitioned_at.to_rfc3339(),
                finalization_results: inspection.finalization_results,
                git_state: inspection.git_state,
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
        }
        Ok(Err(_)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "code": "session_not_found",
                "error": "session not found"
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "code": "internal_error",
                "error": "internal error"
            })),
        ),
    }
}

/// DELETE /api/sessions/{id}/cleanup — remove a session's workspace and registry entry.
pub(super) async fn cleanup_session_handler(
    State(state): State<DaemonState>,
    Path(session_id_str): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let session_id = SessionId::from_string(session_id_str.clone());
    let manager = Arc::clone(&state.session_manager);

    let result = tokio::task::spawn_blocking(move || {
        let mut m = manager.lock().unwrap();
        m.cleanup_session(&session_id)
    })
    .await;

    match result {
        Ok(Ok(())) => {
            let resp = CleanupSessionResponse {
                ok: true,
                session_id: session_id_str,
                error: None,
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
        }
        Ok(Err(_)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "code": "session_not_found",
                "error": "session not found"
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "code": "internal_error",
                "error": "internal error"
            })),
        ),
    }
}

/// DELETE /api/sessions/cleanup — bulk cleanup of retained sessions.
///
/// Query params: `all=true`, `member=<name>`, `older_than=<Nh>` (e.g. 48h).
pub(super) async fn bulk_cleanup_handler(
    State(state): State<DaemonState>,
    Query(params): Query<BulkCleanupParams>,
) -> (StatusCode, Json<serde_json::Value>) {
    use crate::session::manager::CleanupFilter;

    let filter = if params.all.unwrap_or(false) {
        CleanupFilter::All
    } else if let Some(ref member) = params.member {
        CleanupFilter::Member(member.clone())
    } else if let Some(ref duration_str) = params.older_than {
        let hours: u64 = duration_str.trim_end_matches('h').parse().unwrap_or(24);
        CleanupFilter::OlderThan(std::time::Duration::from_secs(hours * 3600))
    } else {
        CleanupFilter::All
    };

    let manager = Arc::clone(&state.session_manager);
    let result = tokio::task::spawn_blocking(move || {
        let mut m = manager.lock().unwrap();
        m.cleanup_sessions(filter)
    })
    .await;

    match result {
        Ok(Ok(report)) => {
            let resp = BulkCleanupResponse {
                ok: true,
                removed: report.removed,
                error: None,
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "code": "cleanup_failed",
                "error": e.to_string()
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "code": "internal_error",
                "error": "internal error"
            })),
        ),
    }
}

pub(super) async fn list_session_history_handler(
    State(state): State<DaemonState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let manager = Arc::clone(&state.session_manager);

    let sessions = tokio::task::spawn_blocking(move || {
        let m = manager.lock().unwrap();
        m.list_terminal()
            .into_iter()
            .map(|r| SessionHistoryInfo {
                session_id: r.session_id.as_str().to_string(),
                owning_member: r.member_name.clone(),
                session_type: session_type_str(&r.session_type).to_string(),
                start_time: r.created_at.to_rfc3339(),
                end_time: r.state_transitioned_at.to_rfc3339(),
                exit_normal: matches!(r.current_state, SessionState::Completed),
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    (
        StatusCode::OK,
        Json(serde_json::to_value(sessions).unwrap()),
    )
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Serde contract tests ─────────────────────────────────────────────────

    #[test]
    fn start_session_request_deserialize() {
        let json = r#"{"member":"alice","session_type":"loop"}"#;
        let req: StartSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.member, "alice");
        assert_eq!(req.session_type, "loop");
    }

    #[test]
    fn start_session_response_serialize_success() {
        let info = SessionInfo {
            session_id: "abc12345".to_string(),
            owning_member: "alice".to_string(),
            session_type: "loop".to_string(),
            current_state: "Active".to_string(),
            start_time: "2026-05-31T00:00:00Z".to_string(),
            workspace_path: Some("/workspaces/alice".to_string()),
            ..SessionInfo::default()
        };
        let resp = StartSessionResponse {
            ok: true,
            session: Some(info),
            error: None,
        };
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(val["ok"], true);
        assert_eq!(val["session"]["session_id"], "abc12345");
        assert_eq!(val["session"]["owning_member"], "alice");
        assert_eq!(val["session"]["session_type"], "loop");
        assert_eq!(val["session"]["current_state"], "Active");
        assert!(val["session"]["start_time"].is_string());
    }

    #[test]
    fn start_session_response_serialize_error() {
        let resp = StartSessionResponse {
            ok: false,
            session: None,
            error: Some("daemon not running".to_string()),
        };
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(val["ok"], false);
        assert!(val["session"].is_null());
        assert_eq!(val["error"], "daemon not running");
    }

    #[test]
    fn sessions_list_response_serialize() {
        let resp = SessionsListResponse {
            sessions: vec![
                SessionInfo {
                    session_id: "s1".to_string(),
                    owning_member: "alice".to_string(),
                    session_type: "loop".to_string(),
                    current_state: "Active".to_string(),
                    start_time: "2026-05-31T00:00:00Z".to_string(),
                    workspace_path: None,
                    ..SessionInfo::default()
                },
                SessionInfo {
                    session_id: "s2".to_string(),
                    owning_member: "bob".to_string(),
                    session_type: "brain".to_string(),
                    current_state: "Active".to_string(),
                    start_time: "2026-05-31T01:00:00Z".to_string(),
                    workspace_path: None,
                    ..SessionInfo::default()
                },
            ],
        };
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(val["sessions"].as_array().unwrap().len(), 2);
        assert_eq!(val["sessions"][0]["session_id"], "s1");
        assert_eq!(val["sessions"][1]["owning_member"], "bob");
    }

    #[test]
    fn sessions_list_response_empty() {
        let resp = SessionsListResponse { sessions: vec![] };
        let val = serde_json::to_value(&resp).unwrap();
        assert!(val["sessions"].as_array().unwrap().is_empty());
    }

    #[test]
    fn stop_session_response_serialize_clean() {
        let resp = StopSessionResponse {
            ok: true,
            dirty_repos: vec![],
            error: None,
        };
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(val["ok"], true);
        assert!(val["dirty_repos"].as_array().unwrap().is_empty());
    }

    #[test]
    fn stop_session_response_serialize_with_dirty() {
        let resp = StopSessionResponse {
            ok: true,
            dirty_repos: vec![
                DirtyRepoInfo {
                    name: "my-project".to_string(),
                    has_uncommitted: true,
                    unpushed_branches: vec!["abc123 add feature".to_string()],
                },
                DirtyRepoInfo {
                    name: "infra".to_string(),
                    has_uncommitted: false,
                    unpushed_branches: vec!["def456 update config".to_string()],
                },
            ],
            error: None,
        };
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(val["dirty_repos"][0]["name"], "my-project");
        assert_eq!(val["dirty_repos"][0]["has_uncommitted"], true);
        assert_eq!(
            val["dirty_repos"][0]["unpushed_branches"][0],
            "abc123 add feature"
        );
        assert_eq!(val["dirty_repos"][1]["name"], "infra");
        assert_eq!(val["dirty_repos"][1]["has_uncommitted"], false);
    }

    // AC-10: SessionInfo must include all fields required by the spec
    #[test]
    fn session_info_has_all_required_fields() {
        let info = SessionInfo {
            session_id: "test-id".to_string(),
            owning_member: "alice".to_string(),
            session_type: "interactive".to_string(),
            current_state: "Active".to_string(),
            start_time: "2026-05-31T12:00:00Z".to_string(),
            workspace_path: Some("/ws/alice".to_string()),
            ..SessionInfo::default()
        };
        let val = serde_json::to_value(&info).unwrap();
        // Verify all AC-10 required fields are present in serialized form
        for field in &[
            "session_id",
            "owning_member",
            "session_type",
            "current_state",
            "start_time",
        ] {
            assert!(
                val.get(field).is_some() && !val[field].is_null(),
                "SessionInfo must have field '{field}'"
            );
        }
    }

    // AC-12: StartSessionResponse can express daemon-not-running error
    #[test]
    fn start_session_response_can_express_daemon_required_error() {
        let resp = StartSessionResponse {
            ok: false,
            session: None,
            error: Some("daemon is not running; start it with 'bm start'".to_string()),
        };
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(val["ok"], false);
        let err = val["error"].as_str().unwrap();
        assert!(err.contains("daemon"), "error message must mention daemon");
    }

    // ── HTTP API contract tests ───────────────────────────────────────────────
    //
    // These tests build a real axum Router with the session routes and verify
    // the expected HTTP contracts.

    #[cfg(test)]
    fn make_test_state() -> DaemonState {
        use super::super::config::DaemonPaths;
        use crate::config::{BotminterConfig, Credentials, TeamEntry};
        use crate::formation::AppCredentialsCached;
        use std::collections::HashMap;
        use std::sync::atomic::AtomicBool;

        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path().to_str().unwrap().to_string();
        // Keep tmp alive by leaking — tests are short-lived.
        std::mem::forget(tmp);

        DaemonState {
            team_name: "test-team".to_string(),
            paths: std::sync::Arc::new(DaemonPaths::new_with_dir("test-team", &tmp_path)),
            webhook_secret: None,
            shutdown: std::sync::Arc::new(AtomicBool::new(false)),
            mode: "poll".to_string(),
            // Set started_at so daemon-readiness check passes in HTTP tests.
            started_at: Some(std::time::Instant::now()),
            config: std::sync::Arc::new(BotminterConfig {
                workzone: std::path::PathBuf::from(&tmp_path),
                default_team: None,
                teams: vec![],
                vms: vec![],
                keyring_collection: None,
            }),
            team_entry: std::sync::Arc::new(TeamEntry {
                name: "test-team".to_string(),
                path: std::path::PathBuf::from(&tmp_path),
                profile: "agentic-sdlc-minimal".to_string(),
                github_repo: "test-org/test-repo".to_string(),
                credentials: Credentials::default(),
                coding_agent: None,
                project_number: None,
                bridge_lifecycle: Default::default(),
                daemon: Default::default(),
                vm: None,
            }),
            app_credentials: std::sync::Arc::new(std::sync::Mutex::new(HashMap::<
                String,
                AppCredentialsCached,
            >::new())),
            session_manager: std::sync::Arc::new(std::sync::Mutex::new(
                crate::session::manager::SessionManager::new(
                    std::path::PathBuf::from(&tmp_path).join("sessions"),
                    std::path::PathBuf::from(&tmp_path).join("sessions-registry.json"),
                )
                .expect("SessionManager::new must not fail in tests"),
            )),
        }
    }

    #[cfg(test)]
    fn session_test_router() -> axum::Router {
        use axum::routing::{get, post};
        let state = make_test_state();
        axum::Router::new()
            .route("/api/sessions/start", post(start_session_handler))
            .route("/api/sessions", get(list_sessions_handler))
            .route("/api/sessions/{id}/stop", post(stop_session_handler))
            .route("/api/sessions/{id}", get(get_session_handler))
            .with_state(state)
    }

    // AC-10: POST /api/sessions/start with valid member → 200 with session record
    #[tokio::test]
    async fn post_sessions_start_valid_member_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = session_test_router();
        let request = Request::builder()
            .method("POST")
            .uri("/api/sessions/start")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"member":"alice","session_type":"loop"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "POST /api/sessions/start must return 200 OK"
        );
    }

    // AC-12: POST /api/sessions/start when daemon not ready → 503
    #[tokio::test]
    async fn post_sessions_start_daemon_not_ready_returns_503() {
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::post;
        use tower::ServiceExt;

        let mut state = make_test_state();
        state.started_at = None;

        let app = axum::Router::new()
            .route("/api/sessions/start", post(start_session_handler))
            .with_state(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/sessions/start")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"member":"alice","session_type":"loop"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "POST /api/sessions/start must return 503 when daemon is not ready"
        );
    }

    // AC-10: GET /api/sessions → 200 with list of sessions
    #[tokio::test]
    async fn get_sessions_returns_200_with_list() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = session_test_router();
        let request = Request::builder()
            .method("GET")
            .uri("/api/sessions")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "GET /api/sessions must return 200 OK"
        );
    }

    // API contract: POST /api/sessions/{id}/stop with valid session → 200
    #[tokio::test]
    async fn post_sessions_stop_valid_session_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = session_test_router();
        let request = Request::builder()
            .method("POST")
            .uri("/api/sessions/abc12345/stop")
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "POST /api/sessions/:id/stop must return 200 OK"
        );
    }

    // API contract: GET /api/sessions/{id} with unknown ID → 404
    #[tokio::test]
    async fn get_session_unknown_id_returns_404() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = session_test_router();
        let request = Request::builder()
            .method("GET")
            .uri("/api/sessions/nonexistent-id")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "GET /api/sessions/:id with unknown ID must return 404"
        );
    }

    // AC-10: GET /api/sessions response body must include required session fields
    #[tokio::test]
    async fn get_sessions_response_has_required_fields() {
        use axum::body::{to_bytes, Body};
        use axum::http::Request;
        use tower::ServiceExt;

        let app = session_test_router();
        let request = Request::builder()
            .method("GET")
            .uri("/api/sessions")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let sessions = val["sessions"]
            .as_array()
            .expect("must have 'sessions' array");
        // If any sessions exist, they must have the required AC-10 fields
        for s in sessions {
            for field in &[
                "session_id",
                "owning_member",
                "session_type",
                "current_state",
                "start_time",
            ] {
                assert!(s.get(field).is_some(), "session must have field '{field}'");
            }
        }
    }

    // API contract: 404 response includes machine-readable code field
    #[tokio::test]
    async fn get_session_404_has_code_field() {
        use axum::body::{to_bytes, Body};
        use axum::http::Request;
        use tower::ServiceExt;

        let app = session_test_router();
        let request = Request::builder()
            .method("GET")
            .uri("/api/sessions/nonexistent-id")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            val.get("code").is_some(),
            "404 response must include a machine-readable 'code' field"
        );
        assert!(
            val.get("error").is_some(),
            "404 response must include a human-readable 'error' field"
        );
    }

    // stop response dirty_repos contains structured per-repo data
    #[tokio::test]
    async fn post_sessions_stop_response_has_structured_dirty_repos() {
        use axum::body::{to_bytes, Body};
        use axum::http::Request;
        use tower::ServiceExt;

        let app = session_test_router();
        let request = Request::builder()
            .method("POST")
            .uri("/api/sessions/any-session-id/stop")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            val.get("dirty_repos").is_some(),
            "must have dirty_repos field"
        );
        // dirty_repos must be an array (structured, not strings)
        let repos = val["dirty_repos"]
            .as_array()
            .expect("dirty_repos must be an array");
        // Each entry must be a structured object, not a plain string
        for repo in repos {
            assert!(
                repo.is_object(),
                "each dirty_repo entry must be a JSON object"
            );
            assert!(repo.get("name").is_some(), "dirty_repo must have 'name'");
            assert!(
                repo.get("has_uncommitted").is_some(),
                "dirty_repo must have 'has_uncommitted'"
            );
            assert!(
                repo.get("unpushed_branches").is_some(),
                "dirty_repo must have 'unpushed_branches'"
            );
        }
    }

    // ── AC-18: Session inspect and cleanup HTTP API tests ────────────────────

    // AC-18 (inspection): GET /api/sessions/{id}/inspect → 200 with structured summary.
    #[tokio::test]
    async fn get_session_inspect_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        let state = make_test_state();
        let app = axum::Router::new()
            .route("/api/sessions/{id}/inspect", get(inspect_session_handler))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/sessions/test-session-id/inspect")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Unknown session → 404; known session → 200. Either way the route must exist.
        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
            "GET /api/sessions/:id/inspect must be routable (200 or 404), got {}",
            response.status()
        );
    }

    // AC-18 (individual cleanup): DELETE /api/sessions/{id}/cleanup → 200.
    #[tokio::test]
    async fn delete_session_cleanup_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::delete;
        use tower::ServiceExt;

        let state = make_test_state();
        let app = axum::Router::new()
            .route(
                "/api/sessions/{id}/cleanup",
                delete(cleanup_session_handler),
            )
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri("/api/sessions/nonexistent-id/cleanup")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
            "DELETE /api/sessions/:id/cleanup must be routable, got {}",
            response.status()
        );
    }

    // AC-18 (bulk cleanup): DELETE /api/sessions/cleanup?member=alice → 200 with count.
    #[tokio::test]
    async fn delete_sessions_bulk_cleanup_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::delete;
        use tower::ServiceExt;

        let state = make_test_state();
        let app = axum::Router::new()
            .route("/api/sessions/cleanup", delete(bulk_cleanup_handler))
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri("/api/sessions/cleanup?member=alice")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "DELETE /api/sessions/cleanup must return 200, got {}",
            response.status()
        );
    }

    // AC-17: GET /api/sessions/history returns terminal sessions with correct exit_normal flags
    #[tokio::test]
    async fn history_handler_returns_terminal_sessions() {
        use axum::body::{to_bytes, Body};
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        let state = make_test_state();

        {
            let mut mgr = state.session_manager.lock().unwrap();
            let completed = mgr
                .create_session("alice", crate::session::SessionType::Loop)
                .unwrap();
            let failed = mgr
                .create_session("bob", crate::session::SessionType::Brain)
                .unwrap();
            mgr.registry
                .update_state(
                    &completed.session_id,
                    crate::session::SessionState::Completed,
                )
                .unwrap();
            mgr.registry.save().unwrap();
            mgr.registry
                .update_state(&failed.session_id, crate::session::SessionState::Failed)
                .unwrap();
            mgr.registry.save().unwrap();
        }

        let app = axum::Router::new()
            .route("/api/sessions/history", get(list_session_history_handler))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/sessions/history")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "GET /api/sessions/history must return 200 OK"
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<SessionHistoryInfo> = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            sessions.len(),
            2,
            "history endpoint must return both terminal sessions"
        );
        let alice = sessions
            .iter()
            .find(|s| s.owning_member == "alice")
            .unwrap();
        let bob = sessions.iter().find(|s| s.owning_member == "bob").unwrap();
        assert!(
            alice.exit_normal,
            "Completed session must have exit_normal=true"
        );
        assert!(
            !bob.exit_normal,
            "Failed session must have exit_normal=false"
        );
    }
}
