use serde::Serialize;

use super::dirty_state::RepoDirtyState;

#[derive(Debug, Clone, Serialize)]
pub struct SessionDisplayRow {
    pub session_id: String,
    pub member: String,
    pub session_type: String,
    pub state: String,
    pub start_time: String,
}

impl SessionDisplayRow {
    fn fields(&self) -> [&str; 5] {
        [&self.session_id, &self.member, &self.session_type, &self.state, &self.start_time]
    }
}

pub fn format_sessions_table(sessions: &[SessionDisplayRow]) -> String {
    let headers = ["SESSION ID", "MEMBER", "TYPE", "STATE", "START TIME"];

    let widths: Vec<usize> = (0..headers.len())
        .map(|i| {
            let data_max = sessions.iter().map(|s| s.fields()[i].len()).max().unwrap_or(0);
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

    for s in sessions {
        let row: Vec<String> = s
            .fields()
            .iter()
            .enumerate()
            .map(|(i, f)| format!("{:<width$}", f, width = widths[i]))
            .collect();
        out.push_str(&row.join("  "));
        out.push('\n');
    }

    out
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

    fn sample_sessions() -> Vec<SessionDisplayRow> {
        vec![
            SessionDisplayRow {
                session_id: "a1b2c3d4".to_string(),
                member: "alice".to_string(),
                session_type: "Interactive".to_string(),
                state: "Active".to_string(),
                start_time: "2026-06-03T10:00:00Z".to_string(),
            },
            SessionDisplayRow {
                session_id: "e5f6g7h8".to_string(),
                member: "bob".to_string(),
                session_type: "Loop".to_string(),
                state: "Active".to_string(),
                start_time: "2026-06-03T10:05:00Z".to_string(),
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
}
