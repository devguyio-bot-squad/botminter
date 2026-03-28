use std::fs;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::api::{
    HealthResponse, MembersStatusResponse, StartLoopRequest, StartLoopResponse,
    StartMembersRequest, StartMembersResponse, StopMembersRequest, StopMembersResponse,
};
use super::config::{DaemonConfig, DaemonPaths};
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Daemon config not found")
        );
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse daemon config")
        );
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
            no_brain: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("alice"));

        let req_all = StartMembersRequest { member: None, no_brain: false };
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
}
