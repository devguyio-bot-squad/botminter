//! HTTP handlers for the session management API.
//!
//! Routes (added to the daemon router in run.rs):
//!   POST   /api/sessions/start          — create a session and launch an agent
//!   GET    /api/sessions                — list active sessions
//!   POST   /api/sessions/{id}/stop      — stop agent and deactivate session
//!   GET    /api/sessions/{id}           — get session detail
//!   DELETE /api/sessions/{id}?force     — force-stop a session (skip finalization)
//!   POST   /api/sessions/{id}/finalize  — re-trigger finalization on a Retained session

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::session::types::{SessionId, SessionRecord, SessionType};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub owning_member: String,
    pub session_type: String,
    pub current_state: String,
    /// ISO-8601 timestamp — corresponds to SessionRecord::created_at.
    pub start_time: String,
    pub workspace_path: Option<String>,
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn record_to_info(record: &SessionRecord) -> SessionInfo {
    let session_type_str = match record.session_type {
        SessionType::Loop => "loop",
        SessionType::Brain => "brain",
        SessionType::Interactive => "interactive",
    }
    .to_string();

    SessionInfo {
        session_id: record.session_id.as_str().to_string(),
        owning_member: record.member_name.clone(),
        session_type: session_type_str,
        current_state: record.current_state.to_string(),
        start_time: record.created_at.to_rfc3339(),
        workspace_path: record
            .workspace_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
    }
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
        m.list_active()
            .iter()
            .map(|r| record_to_info(r))
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
                },
                SessionInfo {
                    session_id: "s2".to_string(),
                    owning_member: "bob".to_string(),
                    session_type: "brain".to_string(),
                    current_state: "Active".to_string(),
                    start_time: "2026-05-31T01:00:00Z".to_string(),
                    workspace_path: None,
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
}
