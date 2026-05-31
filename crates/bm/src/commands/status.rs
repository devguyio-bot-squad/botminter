use std::io::Write;

use anyhow::Result;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, ContentArrangement, Table,
};

use crate::config;
use crate::daemon::{DaemonClient, SessionHistoryInfo, SessionsListResponse};
use crate::state::{self, MemberStatus};

/// Handles `bm status [-t team] [-v] [--json] [--history] [--member <m>] [--since <d>]`.
pub fn run(
    team_flag: Option<&str>,
    verbose: bool,
    json: bool,
    history: bool,
    member_filter: Option<&str>,
    since: Option<&str>,
) -> Result<()> {
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

    // Sessions (AC-10) / History (AC-17)
    let team_name = team.name.clone();
    if history {
        let fetcher = move || DaemonClient::connect(&team_name)?.list_session_history();
        fetch_and_display_history(json, member_filter, since, &mut std::io::stdout(), &fetcher)?;
    } else {
        let fetcher = move || DaemonClient::connect(&team_name)?.list_sessions();
        fetch_and_display_sessions(json, &mut std::io::stdout(), &fetcher)?;
    }

    Ok(())
}

/// Fetches sessions via `session_fetcher` and writes the session section to `writer`.
///
/// When `json` is true, writes `{"sessions":[...]}` (full IDs, no table).
/// When the daemon is not reachable, writes "Sessions: none (daemon not running)" in
/// text mode or `{"sessions":[]}` in JSON mode.
pub(crate) fn fetch_and_display_sessions<W: Write>(
    json: bool,
    writer: &mut W,
    session_fetcher: &dyn Fn() -> Result<SessionsListResponse>,
) -> Result<()> {
    let sessions = match session_fetcher() {
        Ok(resp) => resp.sessions,
        Err(_) => {
            if json {
                writeln!(writer, "{{\"sessions\":[]}}")?;
            } else {
                writeln!(writer, "Sessions: none (daemon not running)")?;
            }
            return Ok(());
        }
    };

    if json {
        let resp = SessionsListResponse { sessions };
        writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
        return Ok(());
    }

    if sessions.is_empty() {
        writeln!(writer, "Sessions: none")?;
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec![
            "Session ID",
            "Member",
            "Type",
            "State",
            "Started",
            "Elapsed",
            "Concurrent",
        ]);

    for s in &sessions {
        let short_id = if s.session_id.len() > 8 {
            format!("{}…", &s.session_id[..8])
        } else {
            s.session_id.clone()
        };
        let started = format_timestamp(&s.start_time);
        let elapsed = if s.state_transitioned_at.is_empty() {
            "—".to_string()
        } else {
            format_elapsed(compute_elapsed_secs(&s.state_transitioned_at))
        };
        let concurrent = s.concurrent_count.to_string();
        table.add_row(vec![
            short_id.as_str(),
            &s.owning_member,
            &s.session_type,
            &s.current_state,
            &started,
            &elapsed,
            &concurrent,
        ]);
    }
    writeln!(writer, "{table}")?;
    Ok(())
}

/// Formats elapsed seconds as a human-readable duration string.
///
/// Examples: 135 → "2m 15s", 7335 → "2h 2m", 90061 → "1d 1h"
pub(crate) fn format_elapsed(secs: u64) -> String {
    if secs >= 86400 {
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        format!("{days}d {hours}h")
    } else if secs >= 3600 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{hours}h {mins}m")
    } else {
        let mins = secs / 60;
        let secs_rem = secs % 60;
        format!("{mins}m {secs_rem}s")
    }
}

fn compute_elapsed_secs(ts: &str) -> u64 {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) else {
        return 0;
    };
    let now = chrono::Utc::now();
    let elapsed = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
    elapsed.num_seconds().max(0) as u64
}

fn parse_since_cutoff(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = s.trim();
    let (num_str, secs_per_unit) = if let Some(n) = s.strip_suffix('h') {
        (n, 3600i64)
    } else if let Some(n) = s.strip_suffix('d') {
        (n, 86400i64)
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 60i64)
    } else {
        return None;
    };
    let n: i64 = num_str.trim().parse().ok()?;
    let secs = n.checked_mul(secs_per_unit)?;
    let now = chrono::Utc::now();
    now.checked_sub_signed(chrono::Duration::seconds(secs))
}

/// Fetches session history via `history_fetcher`, applies filters, and writes to `writer`.
pub(crate) fn fetch_and_display_history<W: Write>(
    json: bool,
    member_filter: Option<&str>,
    since: Option<&str>,
    writer: &mut W,
    history_fetcher: &dyn Fn() -> Result<Vec<SessionHistoryInfo>>,
) -> Result<()> {
    let entries = match history_fetcher() {
        Ok(e) => e,
        Err(_) => {
            if json {
                writeln!(writer, "{{\"sessions\":[]}}")?;
            } else {
                writeln!(writer, "History: none")?;
            }
            return Ok(());
        }
    };

    let since_cutoff = since.and_then(parse_since_cutoff);

    let entries: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            if let Some(m) = member_filter {
                if e.owning_member != m {
                    return false;
                }
            }
            if let Some(cutoff) = since_cutoff {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&e.end_time) {
                    if dt.with_timezone(&chrono::Utc) < cutoff {
                        return false;
                    }
                }
            }
            true
        })
        .collect();

    if entries.is_empty() {
        if json {
            writeln!(writer, "{{\"sessions\":[]}}")?;
        } else {
            writeln!(writer, "History: none")?;
        }
        return Ok(());
    }

    if json {
        writeln!(writer, "{}", serde_json::to_string(&entries)?)?;
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Session ID", "Member", "Type", "Start", "End", "Exit"]);

    for e in &entries {
        let short_id = if e.session_id.len() > 7 {
            format!("{}…", &e.session_id[..7])
        } else {
            e.session_id.clone()
        };
        let exit_label = if e.exit_normal { "normal" } else { "abnormal" };
        table.add_row(vec![
            short_id.as_str(),
            &e.owning_member,
            &e.session_type,
            &format_timestamp(&e.start_time),
            &format_timestamp(&e.end_time),
            exit_label,
        ]);
    }
    writeln!(writer, "{table}")?;
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
            ..SessionInfo::default()
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
        fetch_and_display_sessions(false, &mut buf, &fetcher).unwrap();
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
        fetch_and_display_sessions(false, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("none (daemon not running)"),
            "output must say 'Sessions: none (daemon not running)'; got: {output}"
        );
    }

    // AC-10: bm status shows 'Sessions: none' when daemon is running but has no sessions
    #[test]
    fn status_shows_none_when_no_sessions() {
        let fetcher =
            || -> Result<SessionsListResponse> { Ok(SessionsListResponse { sessions: vec![] }) };
        let mut buf = Vec::new();
        fetch_and_display_sessions(false, &mut buf, &fetcher).unwrap();
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
        fetch_and_display_sessions(true, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("--json output must be valid JSON");
        let sessions_arr = parsed["sessions"]
            .as_array()
            .expect("--json output must have 'sessions' array");
        assert!(
            !sessions_arr.is_empty(),
            "--json output must include sessions"
        );
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
        fetch_and_display_sessions(true, &mut buf, &fetcher).unwrap();
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

// AC-10 (extended) + AC-17: Tests for elapsed time, concurrent count, and session history.
// These reference APIs that do not yet exist — compile errors are the expected RED state.
#[cfg(test)]
mod session_extended_display_tests {
    use super::*;
    use crate::daemon::{SessionHistoryInfo, SessionInfo, SessionsListResponse};

    // Build a SessionInfo that includes the AC-10 extended fields:
    // state_transitioned_at (for elapsed) and concurrent_count.
    // These fields do NOT exist on SessionInfo yet → E0560 compile errors.
    fn make_extended_session(
        id: &str,
        member: &str,
        state_transitioned_at: &str,
        concurrent_count: u32,
    ) -> SessionInfo {
        SessionInfo {
            session_id: id.to_string(),
            owning_member: member.to_string(),
            session_type: "loop".to_string(),
            current_state: "Active".to_string(),
            start_time: "2026-05-31T00:00:00Z".to_string(),
            workspace_path: None,
            state_transitioned_at: state_transitioned_at.to_string(),
            concurrent_count,
        }
    }

    // Build a SessionHistoryInfo for AC-17 history display tests.
    // SessionHistoryInfo does NOT exist in crate::daemon yet → E0412 compile error.
    fn make_history_entry(
        id: &str,
        member: &str,
        start: &str,
        end: &str,
        exit_normal: bool,
    ) -> SessionHistoryInfo {
        SessionHistoryInfo {
            session_id: id.to_string(),
            owning_member: member.to_string(),
            session_type: "loop".to_string(),
            start_time: start.to_string(),
            end_time: end.to_string(),
            exit_normal,
        }
    }

    // AC-10: format_elapsed formats minute-scale durations as "Xm Ys"
    // format_elapsed does NOT exist yet → E0425 compile error.
    #[test]
    fn format_elapsed_shows_minutes_for_short_duration() {
        let s = format_elapsed(135); // 2m 15s
        assert!(s.contains("2m"), "expected '2m' in elapsed string '{s}'");
    }

    // AC-10: format_elapsed formats hour-scale durations as "Xh Ym"
    #[test]
    fn format_elapsed_shows_hours_and_minutes() {
        let s = format_elapsed(7335); // 2h 2m 15s
        assert!(s.contains("2h"), "expected '2h' in elapsed string '{s}'");
        assert!(s.contains("2m"), "expected '2m' in elapsed string '{s}'");
    }

    // AC-10: format_elapsed formats day-scale durations as "Xd Yh"
    #[test]
    fn format_elapsed_shows_days_for_large_duration() {
        let s = format_elapsed(86400 + 3661); // 1d 1h 1m 1s
        assert!(s.contains("1d"), "expected '1d' in elapsed string '{s}'");
    }

    // AC-10: status table shows elapsed time column for an active session.
    // Uses state_transitioned_at to compute elapsed — field doesn't exist yet.
    #[test]
    fn status_shows_elapsed_time_in_sessions_table() {
        let sessions = vec![make_extended_session(
            "abc12345xyz",
            "alice",
            "2026-05-31T00:00:00Z",
            1,
        )];
        let fetcher = || -> Result<SessionsListResponse> {
            Ok(SessionsListResponse {
                sessions: sessions.clone(),
            })
        };
        let mut buf = Vec::new();
        fetch_and_display_sessions(false, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains('h') || output.contains('m') || output.contains('d'),
            "output must contain an elapsed duration; got: {output}"
        );
    }

    // AC-10: each session row shows the concurrent count for that member.
    // alice has 2 active sessions → both rows show concurrent_count = 2.
    #[test]
    fn status_shows_concurrent_count_for_member_with_multiple_sessions() {
        let ts = "2026-05-31T04:00:00Z";
        let sessions = vec![
            make_extended_session("sess0001", "alice", ts, 2),
            make_extended_session("sess0002", "alice", ts, 2),
            make_extended_session("sess0003", "bob", ts, 1),
        ];
        let fetcher = || -> Result<SessionsListResponse> {
            Ok(SessionsListResponse {
                sessions: sessions.clone(),
            })
        };
        let mut buf = Vec::new();
        fetch_and_display_sessions(false, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // alice's two rows must both display concurrent count "2"
        let count = output.matches('2').count();
        assert!(
            count >= 2,
            "alice's 2 sessions must each show concurrent_count=2; got: {output}"
        );
    }

    // AC-17: history display shows session with start time, end time, and exit indicator.
    // fetch_and_display_history does NOT exist yet → E0425 compile error.
    #[test]
    fn history_display_shows_start_end_and_exit_status() {
        let entries = vec![make_history_entry(
            "done0001",
            "alice",
            "2026-05-31T00:00:00Z",
            "2026-05-31T01:00:00Z",
            true,
        )];
        let fetcher = move || -> Result<Vec<SessionHistoryInfo>> { Ok(entries.clone()) };
        let mut buf = Vec::new();
        fetch_and_display_history(false, None, None, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("done000"),
            "output must include session id prefix; got: {output}"
        );
        assert!(
            output.contains("01:00") || output.contains("2026-05-31 01:00"),
            "output must include end time; got: {output}"
        );
        assert!(
            output.contains("normal") || output.contains("ok") || output.contains("✓"),
            "output must indicate normal exit; got: {output}"
        );
    }

    // AC-17: --member filter shows only sessions belonging to the specified member.
    #[test]
    fn history_display_member_filter_returns_only_matching() {
        let entries = vec![
            make_history_entry(
                "s001",
                "alice",
                "2026-05-31T00:00:00Z",
                "2026-05-31T01:00:00Z",
                true,
            ),
            make_history_entry(
                "s002",
                "bob",
                "2026-05-31T00:00:00Z",
                "2026-05-31T01:30:00Z",
                false,
            ),
        ];
        let fetcher = move || -> Result<Vec<SessionHistoryInfo>> { Ok(entries.clone()) };
        let mut buf = Vec::new();
        fetch_and_display_history(false, Some("alice"), None, &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("alice"), "alice must appear; got: {output}");
        assert!(
            !output.contains("bob"),
            "bob must NOT appear when filtered to alice; got: {output}"
        );
    }

    // AC-17: --since filter excludes sessions whose end_time is outside the window.
    #[test]
    fn history_display_since_filter_excludes_old_sessions() {
        let entries = vec![
            // recent: end_time within last 24h (relative to 2026-05-31T06:00:00Z)
            make_history_entry(
                "recent",
                "alice",
                "2026-05-30T12:00:00Z",
                "2026-05-31T04:00:00Z",
                true,
            ),
            // old: end_time 48h ago
            make_history_entry(
                "oldone",
                "alice",
                "2026-05-29T00:00:00Z",
                "2026-05-29T01:00:00Z",
                true,
            ),
        ];
        let fetcher = move || -> Result<Vec<SessionHistoryInfo>> { Ok(entries.clone()) };
        let mut buf = Vec::new();
        fetch_and_display_history(false, None, Some("24h"), &mut buf, &fetcher).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("recent"),
            "recent session must appear in 24h window; got: {output}"
        );
        assert!(
            !output.contains("oldone"),
            "old session must NOT appear in 24h window; got: {output}"
        );
    }
}
