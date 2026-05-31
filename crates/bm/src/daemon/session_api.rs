//! HTTP handlers for the session management API.
//!
//! Routes (added to the daemon router in run.rs):
//!   POST   /api/sessions/start     — create a session and launch an agent
//!   GET    /api/sessions           — list active sessions
//!   POST   /api/sessions/:id/stop  — stop agent and deactivate session
//!   GET    /api/sessions/:id       — get session detail

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::run::DaemonState;

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct StartSessionRequest {
    pub member: String,
    /// "loop", "brain", or "interactive"
    pub session_type: String,
}

// ── Response types ───────────────────────────────────────────────────────────

/// A single session's fields as returned by GET /api/sessions and GET /api/sessions/:id.
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
    pub dirty_repos: Vec<String>,
    pub error: Option<String>,
}


// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/sessions/start — create a session and launch the appropriate agent.
pub(super) async fn start_session_handler(
    State(_state): State<DaemonState>,
    Json(_req): Json<StartSessionRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Not yet implemented — returns 501 until the GREEN phase adds real logic.
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"ok": false, "error": "not implemented"})),
    )
}

/// GET /api/sessions — list all active sessions.
pub(super) async fn list_sessions_handler(
    State(_state): State<DaemonState>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"ok": false, "error": "not implemented"})),
    )
}

/// POST /api/sessions/:id/stop — stop the agent and deactivate the session.
pub(super) async fn stop_session_handler(
    State(_state): State<DaemonState>,
    Path(_session_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"ok": false, "error": "not implemented"})),
    )
}

/// GET /api/sessions/:id — return a single session's detail.
pub(super) async fn get_session_handler(
    State(_state): State<DaemonState>,
    Path(_session_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"ok": false, "error": "not implemented"})),
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
            dirty_repos: vec!["my-project".to_string(), "infra".to_string()],
            error: None,
        };
        let val = serde_json::to_value(&resp).unwrap();
        assert_eq!(val["dirty_repos"][0], "my-project");
        assert_eq!(val["dirty_repos"][1], "infra");
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
        for field in &["session_id", "owning_member", "session_type", "current_state", "start_time"] {
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
    // the expected HTTP contracts. They fail in RED because the handlers return
    // 501 Not Implemented — GREEN will replace them with real implementations.

    #[cfg(test)]
    fn make_test_state() -> DaemonState {
        use std::collections::HashMap;
        use std::sync::atomic::AtomicBool;
        use crate::config::{BotminterConfig, Credentials, TeamEntry};
        use crate::formation::AppCredentialsCached;
        use super::super::config::DaemonPaths;

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
            started_at: None,
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
            app_credentials: std::sync::Arc::new(std::sync::Mutex::new(
                HashMap::<String, AppCredentialsCached>::new(),
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
            .route("/api/sessions/:id/stop", post(stop_session_handler))
            .route("/api/sessions/:id", get(get_session_handler))
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
            "POST /api/sessions/start must return 200 OK (currently returns 501 — RED)"
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
            "GET /api/sessions must return 200 OK (currently returns 501 — RED)"
        );
    }

    // API contract: POST /api/sessions/:id/stop with valid session → 200
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
            "POST /api/sessions/:id/stop must return 200 OK (currently returns 501 — RED)"
        );
    }

    // API contract: GET /api/sessions/:id with unknown ID → 404
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
            "GET /api/sessions/:id with unknown ID must return 404 (currently returns 501 — RED)"
        );
    }

    // AC-10: GET /api/sessions response body must include required session fields
    #[tokio::test]
    async fn get_sessions_response_has_required_fields() {
        use axum::body::{Body, to_bytes};
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
        let sessions = val["sessions"].as_array().expect("must have 'sessions' array");
        // If any sessions exist, they must have the required AC-10 fields
        for s in sessions {
            for field in &["session_id", "owning_member", "session_type", "current_state", "start_time"] {
                assert!(
                    s.get(field).is_some(),
                    "session must have field '{field}'"
                );
            }
        }
    }
}
