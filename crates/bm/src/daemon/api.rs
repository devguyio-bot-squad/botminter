use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use anyhow::Context;

use super::log::daemon_log;
use super::run::DaemonState;
use crate::formation;
use crate::state;

// ── Request types ────────────────────────────────────────────────────

/// Request body for `POST /api/members/start`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartMembersRequest {
    /// If set, start only this member. If None, start all members.
    pub member: Option<String>,
}

/// Request body for `POST /api/members/stop`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StopMembersRequest {
    /// If set, stop only this member. If None, stop all members.
    pub member: Option<String>,
    /// Force-kill members instead of graceful shutdown.
    #[serde(default)]
    pub force: bool,
}

// ── Response types ───────────────────────────────────────────────────

/// Response for `POST /api/members/start`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartMembersResponse {
    pub ok: bool,
    pub launched: Vec<MemberLaunchedInfo>,
    pub skipped: Vec<MemberSkippedInfo>,
    pub errors: Vec<MemberErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberLaunchedInfo {
    pub name: String,
    pub pid: u32,
    pub brain_mode: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberSkippedInfo {
    pub name: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberErrorInfo {
    pub name: String,
    pub error: String,
}

/// Response for `POST /api/members/stop`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StopMembersResponse {
    pub ok: bool,
    pub stopped: Vec<MemberStoppedInfo>,
    pub errors: Vec<MemberErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberStoppedInfo {
    pub name: String,
    pub already_exited: bool,
    pub forced: bool,
}

/// Response for `GET /api/members`.
#[derive(Debug, Serialize, Deserialize)]
pub struct MembersStatusResponse {
    pub members: Vec<MemberStatusInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberStatusInfo {
    pub name: String,
    pub status: String,
    pub pid: Option<u32>,
    pub workspace: Option<String>,
    pub brain_mode: bool,
    pub started_at: Option<String>,
}

/// Response for `GET /api/health`.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub version: String,
    pub team: String,
    pub daemon_mode: String,
    pub member_count: usize,
    pub uptime_secs: Option<u64>,
}

/// Request body for `POST /api/loops/start`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartLoopRequest {
    /// The prompt to pass to `ralph run -p`.
    pub prompt: String,
    /// If set, run the loop in this member's workspace. Defaults to the first member.
    pub member: Option<String>,
}

/// Response for `POST /api/loops/start`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartLoopResponse {
    pub ok: bool,
    pub loop_id: Option<String>,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

/// Error response body.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// POST /api/members/start — launches team members.
pub(super) async fn start_members_handler(
    State(state): State<DaemonState>,
    Json(req): Json<StartMembersRequest>,
) -> impl IntoResponse {
    let paths = Arc::clone(&state.paths);

    daemon_log(
        &paths,
        "INFO",
        &format!(
            "API: start members (filter: {:?})",
            req.member.as_deref().unwrap_or("all")
        ),
    );

    let cfg = Arc::clone(&state.config);
    let team_entry = Arc::clone(&state.team_entry);

    let result = tokio::task::spawn_blocking(move || {
        let team_repo = team_entry.path.join("team");

        formation::start_local_members(
            &team_entry,
            &cfg,
            &team_repo,
            req.member.as_deref(),
            false,
            None,
        )
    })
    .await;

    match result {
        Ok(Ok(start_result)) => {
            let has_errors = !start_result.errors.is_empty();
            let resp = StartMembersResponse {
                ok: !has_errors,
                launched: start_result
                    .launched
                    .into_iter()
                    .map(|m| MemberLaunchedInfo {
                        name: m.name,
                        pid: m.pid,
                        brain_mode: m.brain_mode,
                    })
                    .collect(),
                skipped: start_result
                    .skipped
                    .into_iter()
                    .map(|m| MemberSkippedInfo {
                        name: m.name,
                        pid: m.pid,
                    })
                    .collect(),
                errors: start_result
                    .errors
                    .into_iter()
                    .map(|m| MemberErrorInfo {
                        name: m.name,
                        error: m.error,
                    })
                    .collect(),
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
        }
        Ok(Err(e)) => {
            daemon_log(&paths, "ERROR", &format!("API start failed: {}", e));
            let resp = ErrorResponse {
                ok: false,
                error: e.to_string(),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
        Err(e) => {
            daemon_log(&paths, "ERROR", &format!("API start panicked: {}", e));
            let resp = ErrorResponse {
                ok: false,
                error: "internal error".to_string(),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
    }
}

/// POST /api/members/stop — stops team members.
pub(super) async fn stop_members_handler(
    State(state): State<DaemonState>,
    Json(req): Json<StopMembersRequest>,
) -> impl IntoResponse {
    let paths = Arc::clone(&state.paths);

    daemon_log(
        &paths,
        "INFO",
        &format!(
            "API: stop members (filter: {:?}, force: {})",
            req.member.as_deref().unwrap_or("all"),
            req.force
        ),
    );

    let cfg = Arc::clone(&state.config);
    let team_entry = Arc::clone(&state.team_entry);

    let result = tokio::task::spawn_blocking(move || {
        formation::stop_local_members(
            &team_entry,
            &cfg,
            req.member.as_deref(),
            req.force,
            false, // bridge_flag — daemon doesn't auto-stop bridge
        )
    })
    .await;

    match result {
        Ok(Ok(stop_result)) => {
            let has_errors = !stop_result.errors.is_empty();
            let resp = StopMembersResponse {
                ok: !has_errors,
                stopped: stop_result
                    .stopped
                    .into_iter()
                    .map(|m| MemberStoppedInfo {
                        name: m.name,
                        already_exited: m.already_exited,
                        forced: m.forced,
                    })
                    .collect(),
                errors: stop_result
                    .errors
                    .into_iter()
                    .map(|m| MemberErrorInfo {
                        name: m.name,
                        error: m.error,
                    })
                    .collect(),
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
        }
        Ok(Err(e)) => {
            daemon_log(&paths, "ERROR", &format!("API stop failed: {}", e));
            let resp = ErrorResponse {
                ok: false,
                error: e.to_string(),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
        Err(e) => {
            daemon_log(&paths, "ERROR", &format!("API stop panicked: {}", e));
            let resp = ErrorResponse {
                ok: false,
                error: "internal error".to_string(),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
    }
}

/// GET /api/members — returns member status.
pub(super) async fn list_members_handler(
    State(state): State<DaemonState>,
) -> impl IntoResponse {
    let team_name = state.team_name.clone();

    let result = tokio::task::spawn_blocking(move || {
        let runtime_state = state::load()?;
        let team_prefix = format!("{}/", team_name);

        let members: Vec<MemberStatusInfo> = runtime_state
            .members
            .iter()
            .filter(|(key, _)| key.starts_with(&team_prefix))
            .map(|(key, rt)| {
                let name = key
                    .strip_prefix(&team_prefix)
                    .unwrap_or(key)
                    .to_string();
                let alive = state::is_alive(rt.pid);
                MemberStatusInfo {
                    name,
                    status: if alive {
                        if rt.brain_mode {
                            "brain".to_string()
                        } else {
                            "running".to_string()
                        }
                    } else {
                        "crashed".to_string()
                    },
                    pid: Some(rt.pid),
                    workspace: Some(rt.workspace.to_string_lossy().to_string()),
                    brain_mode: rt.brain_mode,
                    started_at: Some(rt.started_at.clone()),
                }
            })
            .collect();

        Ok::<_, anyhow::Error>(members)
    })
    .await;

    match result {
        Ok(Ok(members)) => {
            let resp = MembersStatusResponse { members };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
        }
        Ok(Err(e)) => {
            let resp = ErrorResponse {
                ok: false,
                error: e.to_string(),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
        Err(e) => {
            let resp = ErrorResponse {
                ok: false,
                error: format!("internal error: {}", e),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
    }
}

/// GET /api/health — enhanced health check with daemon metadata.
pub(super) async fn health_check_handler(
    State(state): State<DaemonState>,
) -> impl IntoResponse {
    let team_name = state.team_name.clone();
    let mode = state.mode.clone();
    let started_at = state.started_at;

    let result = tokio::task::spawn_blocking(move || {
        let runtime_state = state::load().unwrap_or_default();
        let team_prefix = format!("{}/", team_name);
        let member_count = runtime_state
            .members
            .keys()
            .filter(|k| k.starts_with(&team_prefix))
            .count();
        (member_count, team_name)
    })
    .await;

    let (member_count, team) = match result {
        Ok(r) => r,
        Err(_) => (0, state.team_name.clone()),
    };

    let uptime_secs = started_at.map(|t| {
        let elapsed = std::time::Instant::now().duration_since(t);
        elapsed.as_secs()
    });

    let resp = HealthResponse {
        ok: !state.shutdown.load(Ordering::SeqCst),
        version: env!("CARGO_PKG_VERSION").to_string(),
        team,
        daemon_mode: mode,
        member_count,
        uptime_secs,
    };

    (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
}

/// POST /api/loops/start — spawns a Ralph loop in a member's workspace.
pub(super) async fn start_loop_handler(
    State(state): State<DaemonState>,
    Json(req): Json<StartLoopRequest>,
) -> impl IntoResponse {
    let paths = Arc::clone(&state.paths);

    daemon_log(
        &paths,
        "INFO",
        &format!(
            "API: start loop (member: {:?}, prompt length: {})",
            req.member.as_deref().unwrap_or("default"),
            req.prompt.len()
        ),
    );

    let cfg = Arc::clone(&state.config);
    let team_entry = Arc::clone(&state.team_entry);
    let team_name = state.team_name.clone();

    let result = tokio::task::spawn_blocking(move || {
        start_loop_blocking(&team_name, &cfg, &team_entry, &req)
    })
    .await;

    match result {
        Ok(Ok(resp)) => {
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
        }
        Ok(Err(e)) => {
            daemon_log(&paths, "ERROR", &format!("API start loop failed: {}", e));
            let resp = StartLoopResponse {
                ok: false,
                loop_id: None,
                pid: None,
                error: Some(e.to_string()),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
        Err(e) => {
            daemon_log(&paths, "ERROR", &format!("API start loop panicked: {}", e));
            let resp = StartLoopResponse {
                ok: false,
                loop_id: None,
                pid: None,
                error: Some("internal error".to_string()),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(resp).unwrap()),
            )
                .into_response()
        }
    }
}

/// Blocking implementation for loop spawning.
fn start_loop_blocking(
    team_name: &str,
    cfg: &crate::config::BotminterConfig,
    team_entry: &crate::config::TeamEntry,
    req: &StartLoopRequest,
) -> anyhow::Result<StartLoopResponse> {
    use crate::workspace;

    let team_repo = team_entry.path.join("team");
    let members_dir = team_repo.join("members");

    // Resolve which member's workspace to use
    let member_name = if let Some(ref name) = req.member {
        name.clone()
    } else {
        // Default to the first member found
        let dirs = workspace::list_member_dirs(&members_dir)?;
        dirs.into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No members found in team"))?
    };

    let team_ws_base = cfg.workzone.join(team_name);
    let ws = workspace::find_workspace(&team_ws_base, &member_name)
        .ok_or_else(|| anyhow::anyhow!("No workspace found for member '{}'", member_name))?;

    // Write prompt to a temp file in the workspace
    let prompt_file = ws.join(".ralph-loop-prompt.md");
    std::fs::write(&prompt_file, &req.prompt)
        .with_context(|| format!("Failed to write loop prompt to {}", prompt_file.display()))?;

    let gh_token = team_entry
        .credentials
        .gh_token
        .as_deref()
        .unwrap_or("");

    // Spawn ralph run with the prompt
    let mut cmd = std::process::Command::new("ralph");
    cmd.args(["run", "-p"])
        .arg(&prompt_file)
        .current_dir(&ws)
        .env("GH_TOKEN", gh_token)
        .env_remove("CLAUDECODE")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let child = cmd.spawn().with_context(|| {
        format!("Failed to spawn ralph loop in {}", ws.display())
    })?;

    let pid = child.id();

    Ok(StartLoopResponse {
        ok: true,
        loop_id: Some(format!("loop-{}", pid)),
        pid: Some(pid),
        error: None,
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_request_deserialize_with_member() {
        let json = r#"{"member": "superman"}"#;
        let req: StartMembersRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.member, Some("superman".to_string()));
    }

    #[test]
    fn start_request_deserialize_without_member() {
        let json = r#"{}"#;
        let req: StartMembersRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.member, None);
    }

    #[test]
    fn stop_request_deserialize_defaults() {
        let json = r#"{}"#;
        let req: StopMembersRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.member, None);
        assert!(!req.force);
    }

    #[test]
    fn stop_request_deserialize_with_force() {
        let json = r#"{"member": "alice", "force": true}"#;
        let req: StopMembersRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.member, Some("alice".to_string()));
        assert!(req.force);
    }

    #[test]
    fn start_response_serialize() {
        let resp = StartMembersResponse {
            ok: true,
            launched: vec![MemberLaunchedInfo {
                name: "alice".to_string(),
                pid: 1234,
                brain_mode: false,
            }],
            skipped: vec![MemberSkippedInfo {
                name: "bob".to_string(),
                pid: 5678,
            }],
            errors: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["launched"][0]["name"], "alice");
        assert_eq!(json["launched"][0]["pid"], 1234);
        assert_eq!(json["skipped"][0]["name"], "bob");
        assert!(json["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn stop_response_serialize() {
        let resp = StopMembersResponse {
            ok: true,
            stopped: vec![MemberStoppedInfo {
                name: "alice".to_string(),
                already_exited: false,
                forced: true,
            }],
            errors: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["stopped"][0]["name"], "alice");
        assert!(json["stopped"][0]["forced"].as_bool().unwrap());
    }

    #[test]
    fn members_status_response_serialize() {
        let resp = MembersStatusResponse {
            members: vec![
                MemberStatusInfo {
                    name: "alice".to_string(),
                    status: "running".to_string(),
                    pid: Some(1234),
                    workspace: Some("/tmp/ws/alice".to_string()),
                    brain_mode: false,
                    started_at: Some("2026-03-24T10:00:00Z".to_string()),
                },
                MemberStatusInfo {
                    name: "bob".to_string(),
                    status: "brain".to_string(),
                    pid: Some(5678),
                    workspace: Some("/tmp/ws/bob".to_string()),
                    brain_mode: true,
                    started_at: Some("2026-03-24T10:05:00Z".to_string()),
                },
            ],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["members"][0]["name"], "alice");
        assert_eq!(json["members"][0]["status"], "running");
        assert_eq!(json["members"][1]["brain_mode"], true);
    }

    #[test]
    fn health_response_serialize() {
        let resp = HealthResponse {
            ok: true,
            version: "0.1.0".to_string(),
            team: "my-team".to_string(),
            daemon_mode: "poll".to_string(),
            member_count: 3,
            uptime_secs: Some(120),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["version"], "0.1.0");
        assert_eq!(json["team"], "my-team");
        assert_eq!(json["daemon_mode"], "poll");
        assert_eq!(json["member_count"], 3);
        assert_eq!(json["uptime_secs"], 120);
    }

    #[test]
    fn error_response_serialize() {
        let resp = ErrorResponse {
            ok: false,
            error: "something went wrong".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["error"], "something went wrong");
    }

    #[test]
    fn members_status_with_no_members() {
        let resp = MembersStatusResponse {
            members: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["members"].as_array().unwrap().is_empty());
    }

    #[test]
    fn start_response_with_errors() {
        let resp = StartMembersResponse {
            ok: false,
            launched: vec![],
            skipped: vec![],
            errors: vec![MemberErrorInfo {
                name: "charlie".to_string(),
                error: "no workspace found".to_string(),
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["errors"][0]["name"], "charlie");
        assert_eq!(json["errors"][0]["error"], "no workspace found");
    }

    #[test]
    fn start_loop_request_deserialize_with_member() {
        let json = r#"{"prompt": "Implement #1: fix bug", "member": "superman"}"#;
        let req: StartLoopRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.prompt, "Implement #1: fix bug");
        assert_eq!(req.member, Some("superman".to_string()));
    }

    #[test]
    fn start_loop_request_deserialize_without_member() {
        let json = r#"{"prompt": "Implement #2: add feature"}"#;
        let req: StartLoopRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.prompt, "Implement #2: add feature");
        assert_eq!(req.member, None);
    }

    #[test]
    fn start_loop_response_serialize_success() {
        let resp = StartLoopResponse {
            ok: true,
            loop_id: Some("loop-1234".to_string()),
            pid: Some(1234),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["loop_id"], "loop-1234");
        assert_eq!(json["pid"], 1234);
        assert!(json["error"].is_null());
    }

    #[test]
    fn start_loop_response_serialize_error() {
        let resp = StartLoopResponse {
            ok: false,
            loop_id: None,
            pid: None,
            error: Some("no workspace found".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], false);
        assert!(json["loop_id"].is_null());
        assert!(json["pid"].is_null());
        assert_eq!(json["error"], "no workspace found");
    }

    #[test]
    fn start_loop_response_deserializes_for_client() {
        let json = serde_json::json!({
            "ok": true,
            "loop_id": "loop-5678",
            "pid": 5678,
            "error": null
        });
        let resp: StartLoopResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.loop_id, Some("loop-5678".to_string()));
        assert_eq!(resp.pid, Some(5678));
        assert!(resp.error.is_none());
    }

    #[test]
    fn start_loop_request_serializes_for_client() {
        let req = StartLoopRequest {
            prompt: "Fix the tests".to_string(),
            member: Some("alice".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["prompt"], "Fix the tests");
        assert_eq!(parsed["member"], "alice");
    }
}
