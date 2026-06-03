use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::session::registry::SessionRegistry;
use crate::session::types::{SessionId, SessionRecord, SessionState, SessionType};
use crate::session::work_item_lock::WorkItemLock;

struct SessionsInner {
    registry: SessionRegistry,
    work_item_lock: WorkItemLock,
}

/// Shared state for session management API handlers.
#[derive(Clone)]
pub struct SessionsApiState {
    inner: Arc<Mutex<SessionsInner>>,
}

impl SessionsApiState {
    pub fn new(registry_path: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionsInner {
                registry: SessionRegistry::new(registry_path),
                work_item_lock: WorkItemLock::new(),
            })),
        }
    }
}

// ── Request types ───────────────────────────────────────────────────────

/// Request body for `POST /api/sessions/start`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartSessionRequest {
    pub member_name: String,
    pub session_type: String,
    pub work_item_id: Option<String>,
}

// ── Response types ──────────────────────────────────────────────────────

/// Response for `POST /api/sessions/start`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartSessionResponse {
    pub ok: bool,
    pub session_id: Option<String>,
    pub error: Option<String>,
}

/// A single session's summary info for list responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub member_name: String,
    pub session_type: String,
    pub current_state: String,
    pub started_at: String,
}

/// Response for `GET /api/sessions`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionInfo>,
}

/// Response for `GET /api/sessions/:id`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionDetailResponse {
    pub ok: bool,
    pub session: Option<SessionInfo>,
    pub error: Option<String>,
}

/// Response for `POST /api/sessions/:id/stop`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StopSessionResponse {
    pub ok: bool,
    pub error: Option<String>,
}

/// A single session's history entry for the history endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionHistoryInfo {
    pub session_id: String,
    pub member_name: String,
    pub session_type: String,
    pub start_time: String,
    pub end_time: String,
    pub exit_normal: bool,
}

/// Response for `GET /api/sessions/history`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionHistoryResponse {
    pub sessions: Vec<SessionHistoryInfo>,
}

// ── Helpers ────────────────────────────────────────────────────────────

fn record_to_info(r: &SessionRecord) -> SessionInfo {
    SessionInfo {
        session_id: r.session_id.to_string(),
        member_name: r.member_name.clone(),
        session_type: r.session_type.to_string(),
        current_state: r.current_state.to_string(),
        started_at: r.created_at.to_rfc3339(),
    }
}

// ── Handlers ────────────────────────────────────────────────────────────

/// GET /api/sessions/history — lists terminal sessions as history.
pub async fn list_session_history_handler(
    State(state): State<SessionsApiState>,
) -> (StatusCode, Json<SessionHistoryResponse>) {
    let inner = state.inner.lock().unwrap();
    let sessions = inner
        .registry
        .list()
        .into_iter()
        .filter(|r| r.current_state.is_terminal())
        .map(|r| SessionHistoryInfo {
            session_id: r.session_id.to_string(),
            member_name: r.member_name.clone(),
            session_type: r.session_type.to_string(),
            start_time: r.created_at.to_rfc3339(),
            end_time: r.state_transitioned_at.to_rfc3339(),
            exit_normal: r.current_state == SessionState::Completed,
        })
        .collect();

    (StatusCode::OK, Json(SessionHistoryResponse { sessions }))
}

/// POST /api/sessions/start — creates a new session.
pub async fn start_session_handler(
    State(state): State<SessionsApiState>,
    Json(req): Json<StartSessionRequest>,
) -> (StatusCode, Json<StartSessionResponse>) {
    let session_type: SessionType = match req.session_type.parse() {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(StartSessionResponse {
                    ok: false,
                    session_id: None,
                    error: Some(e.to_string()),
                }),
            );
        }
    };

    let mut inner = state.inner.lock().unwrap();
    let session_id = SessionId::new();

    if let Some(ref work_item_id) = req.work_item_id {
        if let Err(e) = inner.work_item_lock.acquire(work_item_id, &session_id) {
            return (
                StatusCode::CONFLICT,
                Json(StartSessionResponse {
                    ok: false,
                    session_id: None,
                    error: Some(e.to_string()),
                }),
            );
        }
    }

    let now = chrono::Utc::now();
    let record = SessionRecord {
        session_id: session_id.clone(),
        member_name: req.member_name,
        session_type,
        current_state: SessionState::Creating,
        created_at: now,
        state_transitioned_at: now,
        agent_pid: None,
        workspace_path: None,
    };

    if let Err(e) = inner.registry.register(record) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(StartSessionResponse {
                ok: false,
                session_id: None,
                error: Some(e.to_string()),
            }),
        );
    }

    if let Err(e) = inner
        .registry
        .update_state(&session_id, SessionState::Active)
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(StartSessionResponse {
                ok: false,
                session_id: None,
                error: Some(e.to_string()),
            }),
        );
    }

    (
        StatusCode::OK,
        Json(StartSessionResponse {
            ok: true,
            session_id: Some(session_id.to_string()),
            error: None,
        }),
    )
}

/// GET /api/sessions — lists active sessions.
pub async fn list_sessions_handler(
    State(state): State<SessionsApiState>,
) -> (StatusCode, Json<SessionListResponse>) {
    let inner = state.inner.lock().unwrap();
    let sessions = inner
        .registry
        .list()
        .iter()
        .map(|r| record_to_info(r))
        .collect();

    (StatusCode::OK, Json(SessionListResponse { sessions }))
}

/// POST /api/sessions/:id/stop — deactivates a session.
pub async fn stop_session_handler(
    State(state): State<SessionsApiState>,
    Path(session_id_str): Path<String>,
) -> (StatusCode, Json<StopSessionResponse>) {
    let mut inner = state.inner.lock().unwrap();
    let session_id = SessionId::from_raw(&session_id_str);

    if inner.registry.get(&session_id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(StopSessionResponse {
                ok: false,
                error: Some(format!("Session {} not found", session_id_str)),
            }),
        );
    }

    if let Err(e) = inner
        .registry
        .update_state(&session_id, SessionState::Completed)
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(StopSessionResponse {
                ok: false,
                error: Some(e.to_string()),
            }),
        );
    }

    inner.work_item_lock.release_all(&session_id);

    (
        StatusCode::OK,
        Json(StopSessionResponse {
            ok: true,
            error: None,
        }),
    )
}

/// GET /api/sessions/:id — returns full session details.
pub async fn session_detail_handler(
    State(state): State<SessionsApiState>,
    Path(session_id_str): Path<String>,
) -> (StatusCode, Json<SessionDetailResponse>) {
    let inner = state.inner.lock().unwrap();
    let session_id = SessionId::from_raw(&session_id_str);

    match inner.registry.get(&session_id) {
        Some(r) => (
            StatusCode::OK,
            Json(SessionDetailResponse {
                ok: true,
                session: Some(record_to_info(r)),
                error: None,
            }),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(SessionDetailResponse {
                ok: false,
                session: None,
                error: Some(format!("Session {} not found", session_id_str)),
            }),
        ),
    }
}

/// Build the sessions API router fragment (merged into the daemon router in green phase).
pub fn sessions_router(state: SessionsApiState) -> Router {
    Router::new()
        .route("/api/sessions/start", post(start_session_handler))
        .route(
            "/api/sessions/history",
            get(list_session_history_handler),
        )
        .route("/api/sessions", get(list_sessions_handler))
        .route("/api/sessions/{id}/stop", post(stop_session_handler))
        .route("/api/sessions/{id}", get(session_detail_handler))
        .with_state(state)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> SessionsApiState {
        let state = SessionsApiState::new(PathBuf::from("/tmp/bm-test-sessions-api.json"));
        {
            let mut inner = state.inner.lock().unwrap();
            let session_id = SessionId::from_raw("test-session-id");
            let now = chrono::Utc::now();
            let record = SessionRecord {
                session_id: session_id.clone(),
                member_name: "test-member".to_string(),
                session_type: SessionType::Interactive,
                current_state: SessionState::Creating,
                created_at: now,
                state_transitioned_at: now,
                agent_pid: None,
                workspace_path: Some(PathBuf::from("/tmp/ws")),
            };
            inner.registry.register(record).unwrap();
            inner
                .registry
                .update_state(&session_id, SessionState::Active)
                .unwrap();
        }
        state
    }

    fn test_router() -> Router {
        sessions_router(test_state())
    }

    // AC-1: Session Status Display — list endpoint returns session metadata

    #[tokio::test]
    async fn post_sessions_start_creates_session_and_returns_id() {
        let app = test_router();
        let body = serde_json::json!({
            "member_name": "alice",
            "session_type": "Interactive"
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/sessions/start")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            json["session_id"].is_string(),
            "response must include a session_id string"
        );
    }

    #[tokio::test]
    async fn get_sessions_lists_active_sessions() {
        let app = test_router();
        let request = Request::builder()
            .uri("/api/sessions")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            json["sessions"].is_array(),
            "response must include a sessions array"
        );
    }

    #[tokio::test]
    async fn post_sessions_stop_deactivates_session() {
        let app = test_router();
        let request = Request::builder()
            .method("POST")
            .uri("/api/sessions/test-session-id/stop")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["ok"], true);
    }

    #[tokio::test]
    async fn get_session_detail_returns_full_info() {
        let app = test_router();
        let request = Request::builder()
            .uri("/api/sessions/test-session-id")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            json["session"].is_object(),
            "response must include a session object"
        );
    }

    #[tokio::test]
    async fn history_handler_returns_terminal_sessions() {
        let state = SessionsApiState::new(PathBuf::from("/tmp/bm-test-history-api.json"));

        {
            let mut inner = state.inner.lock().unwrap();
            let now = chrono::Utc::now();

            // Active session — should NOT appear in history
            let active_id = SessionId::from_raw("active-session");
            let active = SessionRecord {
                session_id: active_id.clone(),
                member_name: "alice".to_string(),
                session_type: SessionType::Interactive,
                current_state: SessionState::Creating,
                created_at: now,
                state_transitioned_at: now,
                agent_pid: None,
                workspace_path: None,
            };
            inner.registry.register(active).unwrap();
            inner
                .registry
                .update_state(&active_id, SessionState::Active)
                .unwrap();

            // Completed session — SHOULD appear with exit_normal: true
            let completed_id = SessionId::from_raw("completed-session");
            let completed = SessionRecord {
                session_id: completed_id.clone(),
                member_name: "bob".to_string(),
                session_type: SessionType::Loop,
                current_state: SessionState::Creating,
                created_at: now,
                state_transitioned_at: now,
                agent_pid: None,
                workspace_path: None,
            };
            inner.registry.register(completed).unwrap();
            inner
                .registry
                .update_state(&completed_id, SessionState::Active)
                .unwrap();
            inner
                .registry
                .update_state(&completed_id, SessionState::Completed)
                .unwrap();

            // Failed session — SHOULD appear with exit_normal: false
            let failed_id = SessionId::from_raw("failed-session");
            let failed = SessionRecord {
                session_id: failed_id.clone(),
                member_name: "carol".to_string(),
                session_type: SessionType::Brain,
                current_state: SessionState::Creating,
                created_at: now,
                state_transitioned_at: now,
                agent_pid: None,
                workspace_path: None,
            };
            inner.registry.register(failed).unwrap();
            inner
                .registry
                .update_state(&failed_id, SessionState::Active)
                .unwrap();
            inner
                .registry
                .update_state(&failed_id, SessionState::Failed)
                .unwrap();
        }

        let app = sessions_router(state);
        let request = Request::builder()
            .uri("/api/sessions/history")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: SessionHistoryResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            json.sessions.len(),
            2,
            "should return only terminal sessions (Completed + Failed), not Active"
        );

        let completed = json
            .sessions
            .iter()
            .find(|s| s.session_id == "completed-session");
        assert!(
            completed.is_some(),
            "Completed session must be in history"
        );
        assert!(
            completed.unwrap().exit_normal,
            "Completed session must have exit_normal: true"
        );

        let failed = json
            .sessions
            .iter()
            .find(|s| s.session_id == "failed-session");
        assert!(failed.is_some(), "Failed session must be in history");
        assert!(
            !failed.unwrap().exit_normal,
            "Failed session must have exit_normal: false"
        );
    }

    #[tokio::test]
    async fn get_session_detail_not_found() {
        let app = test_router();
        let request = Request::builder()
            .uri("/api/sessions/nonexistent-id")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "unknown session ID must return 404"
        );
    }
}
