use std::fs;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::api::{
    HealthResponse, MembersStatusResponse, StartLoopRequest, StartLoopResponse,
    StartMembersRequest, StartMembersResponse, StopMembersRequest, StopMembersResponse,
};
use super::config::{DaemonConfig, DaemonPaths};
use super::session_api::{
    BulkCleanupResponse, CleanupSessionResponse, ForceStopResponse, InspectSessionResponse,
    RetriggerFinalizationResponse, SessionHistoryInfo, SessionInfo, SessionsListResponse,
    StartSessionRequest, StartSessionResponse, StopSessionResponse,
};
use crate::state;

/// HTTP client for communicating with a running daemon.
///
/// Created via [`DaemonClient::connect`], which discovers the daemon's
/// address from its config file and verifies the process is alive.
pub struct DaemonClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl DaemonClient {
    /// Connects to a running daemon for the given team.
    ///
    /// Reads `~/.botminter/daemon-<team>.json` for the port, verifies the
    /// PID is alive, and returns a client ready to make API calls.
    pub fn connect(team_name: &str) -> Result<Self> {
        let paths = DaemonPaths::new(team_name)?;
        let cfg = load_daemon_config(&paths)?;

        if !state::is_alive(cfg.pid) {
            // Clean up stale files
            let _ = fs::remove_file(paths.pid());
            let _ = fs::remove_file(paths.config());
            bail!(
                "Daemon for team '{}' is not running (stale PID {})",
                team_name,
                cfg.pid
            );
        }

        let base_url = format!("http://127.0.0.1:{}", cfg.port);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { base_url, client })
    }

    /// Returns the base URL this client is connected to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// POST /api/members/start — launch team members.
    pub fn start_members(&self, req: &StartMembersRequest) -> Result<StartMembersResponse> {
        let url = format!("{}/api/members/start", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(req)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {}", url))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for start: {}", status, body);
        }

        resp.json::<StartMembersResponse>()
            .context("Failed to parse start response")
    }

    /// POST /api/members/stop — stop team members.
    pub fn stop_members(&self, req: &StopMembersRequest) -> Result<StopMembersResponse> {
        let url = format!("{}/api/members/stop", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(req)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {}", url))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for stop: {}", status, body);
        }

        resp.json::<StopMembersResponse>()
            .context("Failed to parse stop response")
    }

    /// GET /api/members — list member status.
    pub fn list_members(&self) -> Result<MembersStatusResponse> {
        let url = format!("{}/api/members", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {}", url))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for members list: {}", status, body);
        }

        resp.json::<MembersStatusResponse>()
            .context("Failed to parse members response")
    }

    /// POST /api/loops/start — start a Ralph loop in a member's workspace.
    pub fn start_loop(&self, req: &StartLoopRequest) -> Result<StartLoopResponse> {
        let url = format!("{}/api/loops/start", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(req)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {}", url))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for start loop: {}", status, body);
        }

        resp.json::<StartLoopResponse>()
            .context("Failed to parse start loop response")
    }

    /// GET /api/health — daemon health check.
    pub fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/api/health", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {}", url))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for health: {}", status, body);
        }

        resp.json::<HealthResponse>()
            .context("Failed to parse health response")
    }

    /// POST /api/sessions/start — create and register a new session for `member`.
    pub fn start_session(&self, member: &str, session_type: &str) -> Result<StartSessionResponse> {
        let url = format!("{}/api/sessions/start", self.base_url);
        let req = StartSessionRequest {
            member: member.to_string(),
            session_type: session_type.to_string(),
        };
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for session start: {}", status, body);
        }

        resp.json::<StartSessionResponse>()
            .context("Failed to parse session start response")
    }

    /// POST /api/sessions/{id}/stop — stop the agent and deactivate the session.
    pub fn stop_session(&self, session_id: &str) -> Result<StopSessionResponse> {
        let url = format!("{}/api/sessions/{}/stop", self.base_url, session_id);
        let resp = self
            .client
            .post(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for session stop: {}", status, body);
        }

        resp.json::<StopSessionResponse>()
            .context("Failed to parse session stop response")
    }

    /// GET /api/sessions — list all active sessions.
    pub fn list_sessions(&self) -> Result<SessionsListResponse> {
        let url = format!("{}/api/sessions", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for session list: {}", status, body);
        }

        resp.json::<SessionsListResponse>()
            .context("Failed to parse session list response")
    }

    /// GET /api/sessions/history — list completed/terminated sessions.
    pub fn list_session_history(&self) -> Result<Vec<SessionHistoryInfo>> {
        let url = format!("{}/api/sessions/history", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for session history: {}", status, body);
        }

        resp.json::<Vec<SessionHistoryInfo>>()
            .context("Failed to parse session history response")
    }

    /// GET /api/sessions/{id} — get a single session by ID.
    pub fn get_session(&self, session_id: &str) -> Result<SessionInfo> {
        let url = format!("{}/api/sessions/{}", self.base_url, session_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for session get: {}", status, body);
        }

        resp.json::<SessionInfo>()
            .context("Failed to parse session get response")
    }

    /// DELETE /api/sessions/{id}?force=true — force-stop a session immediately.
    ///
    /// Transitions Active → Killed or Finalizing → Killed without finalization.
    pub fn force_stop_session(&self, session_id: &str) -> Result<ForceStopResponse> {
        let url = format!("{}/api/sessions/{}?force=true", self.base_url, session_id);
        let resp = self
            .client
            .delete(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for force stop: {}", status, body);
        }

        resp.json::<ForceStopResponse>()
            .context("Failed to parse force stop response")
    }

    /// POST /api/sessions/{id}/finalize — re-trigger finalization on a Retained session.
    pub fn retrigger_finalization(
        &self,
        session_id: &str,
    ) -> Result<RetriggerFinalizationResponse> {
        let url = format!("{}/api/sessions/{}/finalize", self.base_url, session_id);
        let resp = self
            .client
            .post(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!(
                "Daemon returned {} for retrigger finalization: {}",
                status,
                body
            );
        }

        resp.json::<RetriggerFinalizationResponse>()
            .context("Failed to parse retrigger finalization response")
    }

    /// GET /api/sessions/{id}/inspect — return structured summary of a session.
    pub fn inspect_session(&self, session_id: &str) -> Result<InspectSessionResponse> {
        let url = format!("{}/api/sessions/{}/inspect", self.base_url, session_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for session inspect: {}", status, body);
        }

        resp.json::<InspectSessionResponse>()
            .context("Failed to parse session inspect response")
    }

    /// DELETE /api/sessions/{id}/cleanup — remove a session's workspace and registry entry.
    pub fn cleanup_session(&self, session_id: &str) -> Result<CleanupSessionResponse> {
        let url = format!("{}/api/sessions/{}/cleanup", self.base_url, session_id);
        let resp = self
            .client
            .delete(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for session cleanup: {}", status, body);
        }

        resp.json::<CleanupSessionResponse>()
            .context("Failed to parse session cleanup response")
    }

    /// DELETE /api/sessions/cleanup — bulk cleanup of retained sessions.
    pub fn bulk_cleanup_sessions(
        &self,
        all: bool,
        member: Option<&str>,
        older_than: Option<&str>,
    ) -> Result<BulkCleanupResponse> {
        let mut url = format!("{}/api/sessions/cleanup", self.base_url);
        let mut sep = '?';
        if all {
            url.push(sep);
            url.push_str("all=true");
            sep = '&';
        }
        if let Some(m) = member {
            url.push(sep);
            url.push_str(&format!("member={m}"));
            sep = '&';
        }
        if let Some(d) = older_than {
            url.push(sep);
            url.push_str(&format!("older_than={d}"));
        }
        let resp = self
            .client
            .delete(&url)
            .send()
            .with_context(|| format!("Failed to connect to daemon at {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Daemon returned {} for bulk cleanup: {}", status, body);
        }

        resp.json::<BulkCleanupResponse>()
            .context("Failed to parse bulk cleanup response")
    }
}

/// Reads the daemon config file for a team.
fn load_daemon_config(paths: &DaemonPaths) -> Result<DaemonConfig> {
    let cfg_path = paths.config();
    if !cfg_path.exists() {
        anyhow::bail!(
            "Daemon config not found at {}. Is the daemon running?",
            cfg_path.display()
        );
    }
    let contents = fs::read_to_string(&cfg_path)
        .with_context(|| format!("Failed to read daemon config at {}", cfg_path.display()))?;
    serde_json::from_str::<DaemonConfig>(&contents)
        .with_context(|| format!("Failed to parse daemon config at {}", cfg_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::api::{
        MemberLaunchedInfo, MemberSkippedInfo, MemberStatusInfo, MemberStoppedInfo,
    };

    #[test]
    fn load_daemon_config_missing_file() {
        let paths = DaemonPaths::new_with_dir("test-team", "/tmp/nonexistent-dir-12345");
        let result = load_daemon_config(&paths);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Daemon config not found"));
    }

    #[test]
    fn load_daemon_config_valid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::new_with_dir("my-team", tmp.path().to_str().unwrap());

        let cfg = DaemonConfig {
            team: "my-team".to_string(),
            mode: "poll".to_string(),
            port: 9090,
            interval_secs: 30,
            pid: 99999,
            started_at: "2026-03-24T10:00:00Z".to_string(),
        };
        let contents = serde_json::to_string_pretty(&cfg).unwrap();
        fs::write(paths.config(), contents).unwrap();

        let loaded = load_daemon_config(&paths).unwrap();
        assert_eq!(loaded.team, "my-team");
        assert_eq!(loaded.port, 9090);
        assert_eq!(loaded.pid, 99999);
    }

    #[test]
    fn load_daemon_config_corrupt_file() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::new_with_dir("my-team", tmp.path().to_str().unwrap());

        fs::write(paths.config(), "not valid json!!!").unwrap();

        let result = load_daemon_config(&paths);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse daemon config"));
    }

    #[test]
    fn connect_no_config_file() {
        // DaemonClient::connect with a team that has no config file
        let result = DaemonClient::connect("nonexistent-team-xyz-12345");
        assert!(result.is_err());
    }

    #[test]
    fn connect_stale_pid() {
        let tmp = tempfile::tempdir().unwrap();
        // Write a config with a PID that definitely doesn't exist
        let cfg = DaemonConfig {
            team: "stale-team".to_string(),
            mode: "poll".to_string(),
            port: 19999,
            interval_secs: 30,
            pid: 4294967, // Very unlikely to be a real PID
            started_at: "2026-03-24T10:00:00Z".to_string(),
        };
        let cfg_path = tmp.path().join("daemon-stale-team.json");
        let pid_path = tmp.path().join("daemon-stale-team.pid");
        fs::write(&cfg_path, serde_json::to_string_pretty(&cfg).unwrap()).unwrap();
        fs::write(&pid_path, "4294967").unwrap();

        // Can't easily test with DaemonClient::connect since it uses DaemonPaths::new
        // which reads from the real config dir. But the load_daemon_config + is_alive
        // logic is tested above. This test verifies the error path shape.
        let paths = DaemonPaths::new_with_dir("stale-team", tmp.path().to_str().unwrap());
        let loaded = load_daemon_config(&paths).unwrap();
        assert!(!state::is_alive(loaded.pid));
    }

    #[test]
    fn client_base_url_format() {
        // Verify the base_url construction logic
        let port: u16 = 8484;
        let base_url = format!("http://127.0.0.1:{}", port);
        assert_eq!(base_url, "http://127.0.0.1:8484");
    }

    #[test]
    fn start_request_serializes_for_client() {
        let req = StartMembersRequest {
            member: Some("alice".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("alice"));

        let req_all = StartMembersRequest { member: None };
        let json = serde_json::to_string(&req_all).unwrap();
        // member: null should be present or absent depending on serde behavior
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["member"].is_null());
    }

    #[test]
    fn stop_request_serializes_for_client() {
        let req = StopMembersRequest {
            member: Some("bob".to_string()),
            force: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["member"], "bob");
        assert_eq!(parsed["force"], true);
    }

    #[test]
    fn start_response_deserializes_for_client() {
        let json = serde_json::json!({
            "ok": true,
            "launched": [{"name": "alice", "pid": 1234, "brain_mode": false}],
            "skipped": [{"name": "bob", "pid": 5678}],
            "errors": []
        });
        let resp: StartMembersResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.launched.len(), 1);
        assert_eq!(resp.launched[0].name, "alice");
        assert_eq!(resp.launched[0].pid, 1234);
        assert_eq!(resp.skipped.len(), 1);
        assert!(resp.errors.is_empty());
    }

    #[test]
    fn stop_response_deserializes_for_client() {
        let json = serde_json::json!({
            "ok": true,
            "stopped": [{"name": "alice", "already_exited": false, "forced": true}],
            "errors": []
        });
        let resp: StopMembersResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.stopped.len(), 1);
        assert!(resp.stopped[0].forced);
    }

    #[test]
    fn members_status_response_deserializes_for_client() {
        let json = serde_json::json!({
            "members": [{
                "name": "alice",
                "status": "running",
                "pid": 1234,
                "workspace": "/tmp/ws/alice",
                "brain_mode": false,
                "started_at": "2026-03-24T10:00:00Z"
            }]
        });
        let resp: MembersStatusResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.members.len(), 1);
        assert_eq!(resp.members[0].name, "alice");
        assert_eq!(resp.members[0].pid, Some(1234));
    }

    #[test]
    fn health_response_deserializes_for_client() {
        let json = serde_json::json!({
            "ok": true,
            "version": "0.2.0",
            "team": "my-team",
            "daemon_mode": "poll",
            "member_count": 2,
            "uptime_secs": 300
        });
        let resp: HealthResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.team, "my-team");
        assert_eq!(resp.member_count, 2);
        assert_eq!(resp.uptime_secs, Some(300));
    }

    #[test]
    fn start_loop_request_serializes_for_client() {
        let req = StartLoopRequest {
            prompt: "Implement issue #5: add caching".to_string(),
            member: Some("superman".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["prompt"], "Implement issue #5: add caching");
        assert_eq!(parsed["member"], "superman");
    }

    #[test]
    fn start_loop_response_deserializes_for_client() {
        let json = serde_json::json!({
            "ok": true,
            "loop_id": "loop-9999",
            "pid": 9999,
            "error": null
        });
        let resp: StartLoopResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.loop_id, Some("loop-9999".to_string()));
        assert_eq!(resp.pid, Some(9999));
        assert!(resp.error.is_none());
    }

    #[test]
    fn start_loop_response_deserializes_error_for_client() {
        let json = serde_json::json!({
            "ok": false,
            "loop_id": null,
            "pid": null,
            "error": "no workspace found"
        });
        let resp: StartLoopResponse = serde_json::from_value(json).unwrap();
        assert!(!resp.ok);
        assert!(resp.loop_id.is_none());
        assert!(resp.pid.is_none());
        assert_eq!(resp.error, Some("no workspace found".to_string()));
    }

    // ── Session API type serde tests ─────────────────────────────────────────

    // AC-1/8: StartSessionRequest serializes member and session_type fields
    #[test]
    fn session_start_request_serializes_member_and_type() {
        let req = StartSessionRequest {
            member: "alice".to_string(),
            session_type: "interactive".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["member"], "alice");
        assert_eq!(parsed["session_type"], "interactive");
    }

    // AC-1: StartSessionResponse deserializes ok+session fields
    #[test]
    fn session_start_response_ok_deserializes_with_session_info() {
        let json = serde_json::json!({
            "ok": true,
            "session": {
                "session_id": "abc12345",
                "owning_member": "alice",
                "session_type": "interactive",
                "current_state": "Active",
                "start_time": "2026-05-31T00:00:00Z",
                "workspace_path": "/tmp/ws/alice"
            },
            "error": null
        });
        let resp: StartSessionResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        let session = resp.session.expect("session must be present on ok=true");
        assert_eq!(session.session_id, "abc12345");
        assert_eq!(session.owning_member, "alice");
        assert_eq!(session.current_state, "Active");
    }

    // AC-6: SessionsListResponse deserializes sessions array
    #[test]
    fn session_list_response_deserializes_sessions_array() {
        let json = serde_json::json!({
            "sessions": [
                {
                    "session_id": "abc12345",
                    "owning_member": "alice",
                    "session_type": "interactive",
                    "current_state": "Active",
                    "start_time": "2026-05-31T00:00:00Z",
                    "workspace_path": null
                },
                {
                    "session_id": "def67890",
                    "owning_member": "bob",
                    "session_type": "loop",
                    "current_state": "Active",
                    "start_time": "2026-05-31T01:00:00Z",
                    "workspace_path": null
                }
            ]
        });
        let resp: SessionsListResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.sessions.len(), 2);
        assert_eq!(resp.sessions[0].session_id, "abc12345");
        assert_eq!(resp.sessions[1].owning_member, "bob");
    }

    // AC-7: StopSessionResponse deserializes with structured dirty_repos
    #[test]
    fn session_stop_response_deserializes_structured_dirty_repos() {
        let json = serde_json::json!({
            "ok": true,
            "dirty_repos": [
                {
                    "name": "my-project",
                    "has_uncommitted": true,
                    "unpushed_branches": ["feature/x", "hotfix/z"]
                }
            ],
            "error": null
        });
        let resp: StopSessionResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.dirty_repos.len(), 1);
        assert_eq!(resp.dirty_repos[0].name, "my-project");
        assert!(resp.dirty_repos[0].has_uncommitted);
        assert_eq!(
            resp.dirty_repos[0].unpushed_branches,
            vec!["feature/x", "hotfix/z"]
        );
    }

    // AC-7: StopSessionResponse with empty dirty_repos (clean workspace)
    #[test]
    fn session_stop_response_empty_dirty_repos_for_clean_workspace() {
        let json = serde_json::json!({
            "ok": true,
            "dirty_repos": [],
            "error": null
        });
        let resp: StopSessionResponse = serde_json::from_value(json).unwrap();
        assert!(resp.ok);
        assert!(resp.dirty_repos.is_empty());
    }

    // ── Session client behavioral tests ─────────────────────────────────────────
    //
    // These tests start a lightweight in-process HTTP server with stub session
    // routes and verify that DaemonClient methods produce correct requests and
    // parse responses correctly.

    static SESSION_SERVER_BASE_URL: std::sync::OnceLock<String> = std::sync::OnceLock::new();

    /// Starts a stub axum server with session API routes and returns its base URL.
    /// The server starts once per test process (OnceLock) on a random port.
    fn session_server_base_url() -> String {
        SESSION_SERVER_BASE_URL
            .get_or_init(|| {
                use axum::extract::Path;
                use axum::routing::{delete, get, post};
                use axum::Json;

                async fn stub_start_session(
                    Json(req): Json<serde_json::Value>,
                ) -> Json<serde_json::Value> {
                    let member = req["member"].as_str().unwrap_or("unknown").to_string();
                    let session_type = req["session_type"].as_str().unwrap_or("loop").to_string();
                    Json(serde_json::json!({
                        "ok": true,
                        "session": {
                            "session_id": format!("stub-{member}-{session_type}"),
                            "owning_member": member,
                            "session_type": session_type,
                            "current_state": "Active",
                            "start_time": "2026-05-31T00:00:00Z",
                            "workspace_path": null
                        },
                        "error": null
                    }))
                }

                async fn stub_list_sessions() -> Json<serde_json::Value> {
                    Json(serde_json::json!({ "sessions": [] }))
                }

                async fn stub_stop_session(Path(_id): Path<String>) -> Json<serde_json::Value> {
                    Json(serde_json::json!({
                        "ok": true,
                        "dirty_repos": [],
                        "error": null
                    }))
                }

                async fn stub_get_session(Path(id): Path<String>) -> Json<serde_json::Value> {
                    Json(serde_json::json!({
                        "session_id": id,
                        "owning_member": "stub-member",
                        "session_type": "loop",
                        "current_state": "Active",
                        "start_time": "2026-05-31T00:00:00Z",
                        "workspace_path": null
                    }))
                }

                async fn stub_inspect_session(Path(id): Path<String>) -> Json<serde_json::Value> {
                    Json(serde_json::json!({
                        "ok": true,
                        "session_id": id,
                        "member_name": "stub-member",
                        "session_type": "loop",
                        "current_state": "Retained",
                        "workspace_path": null,
                        "created_at": "2026-05-31T00:00:00Z",
                        "state_transitioned_at": "2026-05-31T00:00:00Z",
                        "finalization_results": null,
                        "git_state": null,
                    }))
                }

                async fn stub_cleanup_session(Path(id): Path<String>) -> Json<serde_json::Value> {
                    Json(serde_json::json!({
                        "ok": true,
                        "session_id": id,
                        "error": null,
                    }))
                }

                async fn stub_bulk_cleanup() -> Json<serde_json::Value> {
                    Json(serde_json::json!({
                        "ok": true,
                        "removed": 0,
                        "error": null,
                    }))
                }

                let router = axum::Router::new()
                    .route("/api/sessions/start", post(stub_start_session))
                    .route("/api/sessions/cleanup", delete(stub_bulk_cleanup))
                    .route("/api/sessions", get(stub_list_sessions))
                    .route("/api/sessions/{id}/stop", post(stub_stop_session))
                    .route("/api/sessions/{id}/inspect", get(stub_inspect_session))
                    .route("/api/sessions/{id}/cleanup", delete(stub_cleanup_session))
                    .route("/api/sessions/{id}", get(stub_get_session));

                let (tx, rx) = std::sync::mpsc::channel::<String>();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                        let port = listener.local_addr().unwrap().port();
                        tx.send(format!("http://127.0.0.1:{port}")).unwrap();
                        axum::serve(listener, router).await.unwrap();
                    });
                });

                rx.recv().unwrap()
            })
            .clone()
    }

    // AC-1: DaemonClient::start_session sends correct request and returns SessionInfo
    #[test]
    fn client_start_session_returns_session_info() {
        let client = DaemonClient {
            base_url: session_server_base_url(),
            client: reqwest::blocking::Client::new(),
        };
        let result = client.start_session("alice", "interactive");
        let resp = result.unwrap();
        assert!(resp.ok, "start_session must return ok=true on success");
        assert!(
            resp.session.is_some(),
            "start_session must return session info"
        );
    }

    // AC-6: DaemonClient::list_sessions returns sessions array
    #[test]
    fn client_list_sessions_returns_sessions_array() {
        let client = DaemonClient {
            base_url: session_server_base_url(),
            client: reqwest::blocking::Client::new(),
        };
        let result = client.list_sessions();
        let resp = result.unwrap();
        // sessions may be empty if none active; field must exist
        let _ = resp.sessions;
    }

    // AC-5: DaemonClient::stop_session returns deactivation result (idempotent)
    #[test]
    fn client_stop_session_returns_deactivation_result() {
        let client = DaemonClient {
            base_url: session_server_base_url(),
            client: reqwest::blocking::Client::new(),
        };
        // stop is idempotent — returns ok=true even for unknown session IDs
        let result = client.stop_session("sess-abc12345");
        let resp = result.unwrap();
        assert!(
            resp.ok,
            "stop_session must return ok=true for known session"
        );
    }

    // AC-8: DaemonClient::get_session returns session info by ID
    #[test]
    fn client_get_session_returns_session_info_by_id() {
        let client = DaemonClient {
            base_url: session_server_base_url(),
            client: reqwest::blocking::Client::new(),
        };
        let result = client.get_session("sess-abc12345");
        let info = result.unwrap();
        assert_eq!(info.session_id, "sess-abc12345");
    }

    // AC-18: DaemonClient::inspect_session — CT-89-06 RED

    #[test]
    fn client_inspect_session_returns_inspection_response() {
        let client = DaemonClient {
            base_url: session_server_base_url(),
            client: reqwest::blocking::Client::new(),
        };
        // E0599: method `inspect_session` not found on `DaemonClient` until added
        let result = client.inspect_session("sess-abc12345");
        let resp = result.unwrap();
        assert!(resp.ok, "inspect_session must return ok=true on success");
        assert_eq!(resp.session_id, "sess-abc12345");
    }

    #[test]
    fn client_cleanup_session_returns_ok() {
        let client = DaemonClient {
            base_url: session_server_base_url(),
            client: reqwest::blocking::Client::new(),
        };
        // E0599: method `cleanup_session` not found on `DaemonClient` until added
        let result = client.cleanup_session("sess-abc12345");
        let resp = result.unwrap();
        assert!(resp.ok, "cleanup_session must return ok=true on success");
        assert_eq!(resp.session_id, "sess-abc12345");
    }

    #[test]
    fn client_bulk_cleanup_sessions_returns_removed_count() {
        let client = DaemonClient {
            base_url: session_server_base_url(),
            client: reqwest::blocking::Client::new(),
        };
        // E0599: method `bulk_cleanup_sessions` not found on `DaemonClient` until added
        let result = client.bulk_cleanup_sessions(true, None, None);
        let resp = result.unwrap();
        assert!(
            resp.ok,
            "bulk_cleanup_sessions must return ok=true on success"
        );
    }
}
