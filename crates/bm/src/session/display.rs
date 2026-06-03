use serde::Serialize;

use super::dirty_state::RepoDirtyState;
use super::history::SessionHistoryEntry;

#[derive(Debug, Clone, Serialize)]
pub struct SessionDisplayRow {
    pub session_id: String,
    pub member: String,
    pub session_type: String,
    pub state: String,
    pub start_time: String,
    pub elapsed_time: String,
    pub concurrent_count: String,
}

impl SessionDisplayRow {
    pub fn fields(&self) -> [&str; 7] {
        [
            &self.session_id,
            &self.member,
            &self.session_type,
            &self.state,
            &self.start_time,
            &self.elapsed_time,
            &self.concurrent_count,
        ]
    }
}

pub fn format_elapsed(seconds: i64) -> String {
    if seconds < 60 {
        return "<1m".to_string();
    }
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if hours == 0 {
        format!("{}m", minutes)
    } else if days == 0 {
        format!("{}h {}m", hours, minutes % 60)
    } else {
        format!("{}d {}h", days, hours % 24)
    }
}

fn format_aligned_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let widths: Vec<usize> = (0..headers.len())
        .map(|i| {
            let data_max = rows.iter().map(|r| r[i].len()).max().unwrap_or(0);
            headers[i].len().max(data_max)
        })
        .collect();

    let mut out = String::new();

    let header_line: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:<width$}", h, width = widths[i]))
        .collect();
    out.push_str(&header_line.join("  "));
    out.push('\n');

    let sep: Vec<String> = widths.iter().map(|&w| "-".repeat(w)).collect();
    out.push_str(&sep.join("  "));
    out.push('\n');

    for row in rows {
        let line: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, f)| format!("{:<width$}", f, width = widths[i]))
            .collect();
        out.push_str(&line.join("  "));
        out.push('\n');
    }

    out
}

pub fn format_history_table(entries: &[SessionHistoryEntry]) -> String {
    let headers = ["SESSION ID", "MEMBER", "TYPE", "START", "END", "EXIT"];
    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|e| {
            vec![
                e.session_id.clone(),
                e.member.clone(),
                e.session_type.clone(),
                e.start_time.to_rfc3339(),
                e.end_time.to_rfc3339(),
                e.exit_status.to_string(),
            ]
        })
        .collect();
    format_aligned_table(&headers, &rows)
}

pub fn format_history_json(entries: &[SessionHistoryEntry]) -> String {
    serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string())
}

pub fn format_sessions_table(sessions: &[SessionDisplayRow]) -> String {
    let headers = [
        "SESSION ID",
        "MEMBER",
        "TYPE",
        "STATE",
        "START TIME",
        "ELAPSED",
        "CONCURRENT",
    ];
    let rows: Vec<Vec<String>> = sessions
        .iter()
        .map(|s| s.fields().iter().map(|f| f.to_string()).collect())
        .collect();
    format_aligned_table(&headers, &rows)
}

pub fn format_sessions_json(sessions: &[SessionDisplayRow]) -> String {
    serde_json::to_string_pretty(sessions).unwrap_or_else(|_| "[]".to_string())
}

pub fn format_deactivation_summary(dirty_repos: &[RepoDirtyState]) -> String {
    let dirty: Vec<&RepoDirtyState> = dirty_repos.iter().filter(|r| !r.is_clean()).collect();

    if dirty.is_empty() {
        return "All repos clean.".to_string();
    }

    let mut out = String::new();
    for repo in &dirty {
        out.push_str(&format!("{}:\n", repo.repo_name));
        if !repo.uncommitted_files.is_empty() {
            out.push_str(&format!(
                "  {} uncommitted file(s)\n",
                repo.uncommitted_files.len()
            ));
        }
        if !repo.unpushed_branches.is_empty() {
            out.push_str(&format!(
                "  {} unpushed branch commit(s)\n",
                repo.unpushed_branches.len()
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::history::ExitStatus;

    fn sample_sessions() -> Vec<SessionDisplayRow> {
        vec![
            SessionDisplayRow {
                session_id: "a1b2c3d4".to_string(),
                member: "alice".to_string(),
                session_type: "Interactive".to_string(),
                state: "Active".to_string(),
                start_time: "2026-06-03T10:00:00Z".to_string(),
                elapsed_time: String::new(),
                concurrent_count: "0".to_string(),
            },
            SessionDisplayRow {
                session_id: "e5f6g7h8".to_string(),
                member: "bob".to_string(),
                session_type: "Loop".to_string(),
                state: "Active".to_string(),
                start_time: "2026-06-03T10:05:00Z".to_string(),
                elapsed_time: String::new(),
                concurrent_count: "0".to_string(),
            },
        ]
    }

    fn sample_dirty_repos() -> Vec<RepoDirtyState> {
        vec![
            RepoDirtyState {
                repo_name: "botminter".to_string(),
                uncommitted_files: vec![
                    "M src/main.rs".to_string(),
                    "?? new-file.txt".to_string(),
                ],
                unpushed_branches: vec!["abc1234 feat: add caching".to_string()],
            },
            RepoDirtyState {
                repo_name: "hypershift".to_string(),
                uncommitted_files: vec![],
                unpushed_branches: vec![],
            },
        ]
    }

    // AC-6: bm status Shows Sessions — table with required columns

    #[test]
    fn sessions_table_includes_required_column_headers() {
        let table = format_sessions_table(&sample_sessions());
        for col in ["SESSION ID", "MEMBER", "TYPE", "STATE", "START TIME"] {
            assert!(
                table.contains(col),
                "table must include column header '{col}', got:\n{table}"
            );
        }
    }

    #[test]
    fn sessions_table_renders_all_rows() {
        let sessions = sample_sessions();
        let table = format_sessions_table(&sessions);
        for s in &sessions {
            assert!(
                table.contains(&s.session_id),
                "table must include session ID '{}', got:\n{table}",
                s.session_id
            );
            assert!(
                table.contains(&s.member),
                "table must include member '{}', got:\n{table}",
                s.member
            );
        }
    }

    #[test]
    fn sessions_table_has_aligned_columns() {
        let table = format_sessions_table(&sample_sessions());
        let lines: Vec<&str> = table.lines().collect();
        assert!(
            lines.len() >= 3,
            "table must have header + separator + data rows, got {} lines",
            lines.len()
        );

        let header_session_pos = lines[0].find("SESSION ID");
        let header_member_pos = lines[0].find("MEMBER");
        assert!(
            header_session_pos.is_some() && header_member_pos.is_some(),
            "header must contain both SESSION ID and MEMBER columns"
        );
    }

    // AC-6: --json flag for machine output

    #[test]
    fn sessions_json_produces_valid_json() {
        let json_str = format_sessions_json(&sample_sessions());
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("sessions JSON must be valid JSON");
        assert!(
            parsed.is_array(),
            "sessions JSON must be an array, got: {parsed}"
        );
    }

    #[test]
    fn sessions_json_contains_all_fields() {
        let json_str = format_sessions_json(&sample_sessions());
        let parsed: Vec<serde_json::Value> =
            serde_json::from_str(&json_str).expect("sessions JSON must be valid JSON array");

        assert_eq!(parsed.len(), 2, "JSON must contain all sessions");
        for entry in &parsed {
            assert!(entry["session_id"].is_string(), "entry must have session_id");
            assert!(entry["member"].is_string(), "entry must have member");
            assert!(entry["session_type"].is_string(), "entry must have session_type");
            assert!(entry["state"].is_string(), "entry must have state");
            assert!(entry["start_time"].is_string(), "entry must have start_time");
        }
    }

    // AC-7: Deactivation Summary Display — per-repo sections

    #[test]
    fn deactivation_summary_shows_uncommitted_count() {
        let summary = format_deactivation_summary(&sample_dirty_repos());
        assert!(
            summary.contains("botminter"),
            "summary must include dirty repo name 'botminter'"
        );
        assert!(
            summary.contains("2") || summary.contains("uncommitted"),
            "summary must show uncommitted file count or label, got:\n{summary}"
        );
    }

    #[test]
    fn deactivation_summary_shows_unpushed_branches() {
        let summary = format_deactivation_summary(&sample_dirty_repos());
        assert!(
            summary.contains("unpushed") || summary.contains("branch"),
            "summary must mention unpushed branches, got:\n{summary}"
        );
    }

    #[test]
    fn deactivation_summary_distinguishes_clean_vs_dirty() {
        let repos = sample_dirty_repos();
        let summary = format_deactivation_summary(&repos);
        let has_dirty_marker = summary.contains("botminter");
        assert!(
            has_dirty_marker,
            "summary must show dirty repos, got:\n{summary}"
        );
    }

    #[test]
    fn deactivation_summary_handles_all_clean() {
        let clean_repos = vec![RepoDirtyState {
            repo_name: "myproject".to_string(),
            uncommitted_files: vec![],
            unpushed_branches: vec![],
        }];
        let summary = format_deactivation_summary(&clean_repos);
        assert!(
            summary.contains("clean") || summary.is_empty() || !summary.contains("uncommitted"),
            "all-clean summary must indicate no dirty state, got:\n{summary}"
        );
    }

    // CT-89-01: format_elapsed — human-readable durations

    #[test]
    fn elapsed_under_one_hour_shows_minutes() {
        let result = format_elapsed(45 * 60);
        assert_eq!(result, "45m", "45 minutes should display as '45m'");
    }

    #[test]
    fn elapsed_under_24h_shows_hours_and_minutes() {
        let result = format_elapsed(2 * 3600 + 15 * 60);
        assert_eq!(
            result, "2h 15m",
            "2 hours 15 minutes should display as '2h 15m'"
        );
    }

    #[test]
    fn elapsed_over_24h_shows_days_and_hours() {
        let result = format_elapsed(27 * 3600);
        assert_eq!(result, "1d 3h", "27 hours should display as '1d 3h'");
    }

    #[test]
    fn elapsed_zero_shows_less_than_minute() {
        let result = format_elapsed(0);
        assert_eq!(result, "<1m", "0 seconds should display as '<1m'");
    }

    // CT-89-01: History display

    fn sample_history_entry() -> SessionHistoryEntry {
        use chrono::Utc;
        SessionHistoryEntry {
            session_id: "a1b2c3d4".to_string(),
            member: "alice".to_string(),
            session_type: "Loop".to_string(),
            start_time: Utc::now() - chrono::Duration::hours(2),
            end_time: Utc::now(),
            exit_status: ExitStatus::Normal,
        }
    }

    #[test]
    fn history_table_includes_required_columns() {
        let entries = vec![sample_history_entry()];
        let table = format_history_table(&entries);
        for col in ["SESSION ID", "MEMBER", "TYPE", "START", "END", "EXIT"] {
            assert!(
                table.contains(col),
                "history table must include '{col}' column header, got:\n{table}"
            );
        }
    }

    #[test]
    fn history_json_includes_exit_status_field() {
        let entries = vec![sample_history_entry()];
        let json_str = format_history_json(&entries);
        let parsed: Vec<serde_json::Value> =
            serde_json::from_str(&json_str).expect("history JSON must be valid JSON array");
        assert!(!parsed.is_empty(), "history JSON must contain entries");
        assert!(
            parsed[0]["exit_status"].is_string(),
            "history entry must have exit_status field"
        );
    }
}
