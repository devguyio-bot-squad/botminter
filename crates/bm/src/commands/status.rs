use std::io::Write;

use anyhow::Result;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, ContentArrangement, Table,
};

use crate::config;
use crate::daemon::SessionsListResponse;
use crate::state::{self, MemberStatus};

/// Handles `bm status [-t team] [-v] [--json]`.
pub fn run(team_flag: Option<&str>, verbose: bool, _json: bool) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;

    let info = state::gather_status(team, &cfg, verbose)?;

    // Header
    println!("Team: {}", team.name);
    if let Some(f) = &info.formation {
        println!("Formation: {}", f);
    }
    println!("Profile: {}", team.profile);
    if !team.github_repo.is_empty() {
        println!("GitHub: {}", team.github_repo);
    }
    if !info.project_names.is_empty() {
        println!("Projects: {}", info.project_names.join(", "));
    }
    if let Some(d) = &info.daemon {
        match d.mode.as_str() {
            "webhook" => println!(
                "Daemon: running (PID {}, webhook mode, port {})",
                d.pid, d.port
            ),
            "poll" => println!(
                "Daemon: running (PID {}, poll mode, interval {}s)",
                d.pid, d.interval_secs
            ),
            _ => println!("Daemon: running (PID {})", d.pid),
        }
        if d.port > 0 {
            println!("Console: http://localhost:{}", d.port);
        }
    }
    println!();

    // Members
    if !info.has_members {
        println!("No members hired yet.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec![
            "Member", "Role", "Status", "Enabled", "Branch", "Started", "PID",
        ]);

    for m in &info.members {
        let (label, started, pid_str) = match &m.status {
            MemberStatus::Running {
                pid,
                started_at,
                brain_mode,
            } => {
                let status = if *brain_mode { "brain" } else { "running" };
                (status, format_timestamp(started_at), pid.to_string())
            }
            MemberStatus::Crashed { pid, started_at } => {
                ("crashed", format_timestamp(started_at), pid.to_string())
            }
            MemberStatus::Stopped => ("stopped", "—".to_string(), "—".to_string()),
        };
        let enabled = if m.enabled { "yes" } else { "-" };
        table.add_row(vec![
            m.name.as_str(),
            &m.role,
            label,
            enabled,
            &m.branch,
            &started,
            &pid_str,
        ]);
    }
    println!("{table}");
    println!();
    println!("Enabled = daemon will auto-start this member when GitHub activity is detected (poll/webhook).");
    println!("Change with `bm enable <member>` / `bm disable <member>`.");

    // Bridge
    if let Some(b) = &info.bridge {
        println!();
        println!("Bridge: {} ({})", b.name, b.bridge_type);
        println!("Status: {}", b.status);
        if let Some(url) = &b.url {
            println!("URL: {}", url);
        }
        if !b.identities.is_empty() {
            println!();
            let mut bt = Table::new();
            bt.load_preset(UTF8_FULL_CONDENSED)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_content_arrangement(ContentArrangement::DynamicFullWidth)
                .set_header(vec!["Member", "Bridge User", "User ID"]);
            for id in &b.identities {
                bt.add_row(vec![&id.member, &id.bridge_user, &id.user_id]);
            }
            println!("{bt}");
        }
    }

    // Verbose
    if let Some(v) = &info.verbose {
        for ws in &v.workspaces {
            println!("\n── {} workspace ──", ws.member);
            println!("  Submodules:");
            for s in &ws.submodules {
                println!("    {}: {}", s.name, s.status_label);
            }
        }
        for ri in &v.ralph_sections {
            println!("\n── {} (PID {}) ──", ri.member, ri.pid);
            for (label, output) in &ri.sections {
                println!("\n  {}:", label);
                for line in output.lines() {
                    println!("    {}", line);
                }
            }
        }
    }

    Ok(())
}

/// Fetches sessions via `session_fetcher` and writes the session section to `writer`.
///
/// When `json` is true, writes only a JSON `{"sessions":[...]}` object and
/// suppresses all other output. When the daemon is not reachable, writes
/// "Sessions: none (daemon not running)" in text mode or `{"sessions":[]}` in JSON mode.
///
/// GREEN phase will implement this; the stub currently writes nothing so all
/// behavioural tests fail (RED phase intent).
pub(crate) fn fetch_and_display_sessions<W: Write>(
    _team_name: &str,
    _json: bool,
    _writer: &mut W,
    _session_fetcher: &dyn Fn() -> Result<SessionsListResponse>,
) -> Result<()> {
    Ok(())
}

/// Formats an ISO 8601 timestamp for display, stripping sub-seconds.
fn format_timestamp(ts: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        ts.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── format_timestamp ──────────────────────────────────────────

    #[test]
    fn format_timestamp_rfc3339() {
        let result = format_timestamp("2026-02-21T10:30:00+00:00");
        assert_eq!(result, "2026-02-21 10:30:00");
    }

    #[test]
    fn format_timestamp_with_offset() {
        let result = format_timestamp("2026-02-21T12:30:00+02:00");
        assert_eq!(result, "2026-02-21 12:30:00");
    }

    #[test]
    fn format_timestamp_unparseable_passthrough() {
        let result = format_timestamp("not-a-timestamp");
        assert_eq!(result, "not-a-timestamp");
    }

    #[test]
    fn format_timestamp_empty_passthrough() {
        let result = format_timestamp("");
        assert_eq!(result, "");
    }
}

#[cfg(test)]
mod session_display_tests {
    use super::*;
    use crate::daemon::{SessionInfo, SessionsListResponse};

    fn make_session(id: &str, member: &str, stype: &str, state: &str, start: &str) -> SessionInfo {
        SessionInfo {
            session_id: id.to_string(),
            owning_member: member.to_string(),
            session_type: stype.to_string(),
            current_state: state.to_string(),
            start_time: start.to_string(),
            workspace_path: None,
        }
    }

    // AC-10: bm status shows sessions table with truncated ID, member, type, state, started columns
    #[test]
    fn status_shows_sessions_table_with_truncated_id() {
        let sessions = vec![make_session(
            "abc12345xyz9999",
            "alice",
            "loop",
            "Active",
            "2026-05-31T00:00:00Z",
        )];
        let fetcher = || -> Result<SessionsListResponse> {
            Ok(SessionsListResponse {
                sessions: sessions.clone(),
            })
        };
        let mut buf = Vec::new();
        fetch_and_display_sessions("test-team", false, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // session_id must be truncated to 8 chars + '…'
        assert!(
            output.contains("abc12345"),
            "output must contain first 8 chars of session_id; got: {output}"
        );
        assert!(
            output.contains("alice"),
            "output must contain member name; got: {output}"
        );
        assert!(
            output.contains("loop"),
            "output must contain session_type; got: {output}"
        );
        assert!(
            output.contains("Active"),
            "output must contain state; got: {output}"
        );
    }

    // AC-10: bm status shows 'Sessions: none (daemon not running)' when daemon is offline
    #[test]
    fn status_shows_none_message_when_daemon_not_running() {
        let fetcher = || -> Result<SessionsListResponse> {
            Err(anyhow::anyhow!(
                "Daemon for team 'test-team' is not running (stale PID 99999)"
            ))
        };
        let mut buf = Vec::new();
        fetch_and_display_sessions("test-team", false, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("none (daemon not running)"),
            "output must say 'Sessions: none (daemon not running)'; got: {output}"
        );
    }

    // AC-10: bm status shows 'Sessions: none' when daemon is running but has no sessions
    #[test]
    fn status_shows_none_when_no_sessions() {
        let fetcher = || -> Result<SessionsListResponse> {
            Ok(SessionsListResponse { sessions: vec![] })
        };
        let mut buf = Vec::new();
        fetch_and_display_sessions("test-team", false, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Sessions: none"),
            "output must say 'Sessions: none' when no sessions; got: {output}"
        );
    }

    // AC-10: --json flag serializes full session list as JSON, suppresses all other output
    #[test]
    fn status_json_flag_outputs_json_sessions() {
        let sessions = vec![make_session(
            "abc12345xyz9999",
            "alice",
            "loop",
            "Active",
            "2026-05-31T00:00:00Z",
        )];
        let fetcher = || -> Result<SessionsListResponse> {
            Ok(SessionsListResponse {
                sessions: sessions.clone(),
            })
        };
        let mut buf = Vec::new();
        fetch_and_display_sessions("test-team", true, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("--json output must be valid JSON");
        let sessions_arr = parsed["sessions"]
            .as_array()
            .expect("--json output must have 'sessions' array");
        assert!(!sessions_arr.is_empty(), "--json output must include sessions");
        assert_eq!(
            sessions_arr[0]["session_id"].as_str().unwrap(),
            "abc12345xyz9999",
            "--json must include full session_id (not truncated)"
        );
    }

    // AC-10: --json with daemon not running outputs {{"sessions":[]}} with exit 0 (no error)
    #[test]
    fn status_json_daemon_not_running_outputs_empty_sessions() {
        let fetcher = || -> Result<SessionsListResponse> {
            Err(anyhow::anyhow!(
                "Daemon for team 'test-team' is not running"
            ))
        };
        let mut buf = Vec::new();
        fetch_and_display_sessions("test-team", true, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .expect("--json output must be valid JSON even when daemon not running");
        let sessions_arr = parsed["sessions"]
            .as_array()
            .expect("must have 'sessions' array");
        assert!(
            sessions_arr.is_empty(),
            "--json with daemon not running must output {{\"sessions\":[]}}; got: {output}"
        );
    }
}
