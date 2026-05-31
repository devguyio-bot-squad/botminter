use anyhow::{bail, Result};

use crate::config;
use crate::daemon::{
    ForceStopResponse, RetriggerFinalizationResponse, SessionsListResponse, StopSessionResponse,
};
use crate::formation;
use crate::team::Team;

/// Summary returned immediately by all stop variants — deactivation proceeds asynchronously.
#[derive(Debug, Default)]
pub struct StopSummary {
    /// Number of sessions that entered deactivation (graceful or force).
    pub sessions_deactivating: usize,
    /// Number of those sessions that will launch a finalization subagent.
    pub entering_finalization: usize,
    /// Number of interactive sessions skipped (bare stop only).
    pub interactive_skipped: usize,
}

/// Stop all sessions belonging to `member` via graceful deactivation.
///
/// Calls `list_fn` to enumerate sessions, then calls `stop_fn` for each session
/// owned by `member`. Returns immediately with a summary — finalization proceeds
/// asynchronously within the daemon.
pub fn stop_member_sessions<F, G>(member: &str, list_fn: &F, stop_fn: &G) -> Result<StopSummary>
where
    F: Fn() -> Result<SessionsListResponse>,
    G: Fn(&str) -> Result<StopSessionResponse>,
{
    let sessions = list_fn()?.sessions;
    let mut summary = StopSummary::default();

    for session in sessions.iter().filter(|s| s.owning_member == member) {
        let resp = stop_fn(&session.session_id)?;
        summary.sessions_deactivating += 1;
        if resp.dirty_repos.iter().any(|r| r.is_dirty()) {
            summary.entering_finalization += 1;
        }
    }

    Ok(summary)
}

/// Stop a specific session by ID via graceful deactivation.
///
/// Returns immediately with a summary — finalization proceeds asynchronously.
pub fn stop_session_by_id<F>(session_id: &str, stop_fn: &F) -> Result<StopSummary>
where
    F: Fn(&str) -> Result<StopSessionResponse>,
{
    let resp = stop_fn(session_id)?;
    let entering_finalization = usize::from(resp.dirty_repos.iter().any(|r| r.is_dirty()));
    Ok(StopSummary {
        sessions_deactivating: 1,
        entering_finalization,
        interactive_skipped: 0,
    })
}

/// Bare stop: stop all autonomous sessions (loop/brain), skip interactive.
///
/// Calls `list_fn` to enumerate sessions, stops only those with session_type
/// "loop" or "brain". Interactive sessions are untouched and counted in
/// `StopSummary.interactive_skipped`. Returns immediately.
pub fn stop_autonomous_sessions<F, G>(list_fn: &F, stop_fn: &G) -> Result<StopSummary>
where
    F: Fn() -> Result<SessionsListResponse>,
    G: Fn(&str) -> Result<StopSessionResponse>,
{
    let sessions = list_fn()?.sessions;
    let mut summary = StopSummary::default();

    for session in &sessions {
        if session.session_type == "loop" || session.session_type == "brain" {
            let resp = stop_fn(&session.session_id)?;
            summary.sessions_deactivating += 1;
            if resp.dirty_repos.iter().any(|r| r.is_dirty()) {
                summary.entering_finalization += 1;
            }
        } else {
            summary.interactive_skipped += 1;
        }
    }

    Ok(summary)
}

/// Force-stop a session: kill immediately, no finalization, session → Killed.
///
/// For Active sessions: kills the agent and transitions to Killed.
/// For Finalizing sessions: kills the finalization subagent and transitions to Killed.
/// Returns immediately with a summary. Workspace is retained (re-trigger available).
pub fn force_stop_session_cmd<F>(session_id: &str, force_fn: &F) -> Result<StopSummary>
where
    F: Fn(&str) -> Result<ForceStopResponse>,
{
    let resp = force_fn(session_id)?;
    Ok(StopSummary {
        sessions_deactivating: 1,
        entering_finalization: usize::from(resp.finalization_launched),
        interactive_skipped: 0,
    })
}

/// Re-trigger finalization on a Retained session: Retained → Finalizing.
///
/// Calls `retrigger_fn` which asks the daemon to launch a fresh finalization
/// subagent in the retained workspace.
pub fn retrigger_finalization_cmd<F>(session_id: &str, retrigger_fn: &F) -> Result<()>
where
    F: Fn(&str) -> Result<RetriggerFinalizationResponse>,
{
    let resp = retrigger_fn(session_id)?;
    if !resp.ok {
        anyhow::bail!(
            "retrigger finalization failed for session {}: {}",
            session_id,
            resp.error.unwrap_or_default()
        );
    }
    Ok(())
}

/// Handles `bm stop [member] [-t team] [--force] [--bridge] [--all]`.
pub fn run(
    team_flag: Option<&str>,
    force: bool,
    member_filter: Option<&str>,
    bridge_flag: bool,
    stop_all: bool,
) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;

    let local_formation = formation::create_local_formation(&team.name)?;
    let team_api = Team::new(team, local_formation);

    let result = team_api.stop(&cfg, member_filter, force, bridge_flag, stop_all)?;

    // Display: specific member not running
    if let (true, Some(target)) = (result.members.no_members_running, member_filter) {
        println!(
            "Member '{}' is not running for team '{}'.",
            target, team.name
        );
        return Ok(());
    }

    // Display: no members running at all
    if result.members.no_members_running {
        println!("No members running for team '{}'.", team.name);
    } else {
        for m in &result.members.stopped {
            if m.already_exited {
                eprintln!("{}... already exited", m.name);
            } else if m.forced {
                eprintln!("Stopping {} (force)... done", m.name);
            } else {
                eprintln!("Stopping {}... done", m.name);
            }
        }
        for m in &result.members.errors {
            eprintln!("Stopping {}... failed: {}", m.name, m.error);
        }

        println!(
            "\nStopped {} member(s), {} error(s).",
            result.members.stopped.len(),
            result.members.errors.len()
        );

        if !result.members.errors.is_empty() {
            bail!(
                "Some members could not be stopped gracefully. \
                 Use `bm stop -f` to force-kill."
            );
        }
    }

    // Display: bridge outcome
    match &result.bridge {
        Some(formation::BridgeStopOutcome::Stopped(name)) => {
            println!("Bridge '{}' stopped.", name);
        }
        Some(formation::BridgeStopOutcome::LeftRunning(name)) => {
            println!(
                "Bridge '{}' left running. Use `bm stop --bridge` to stop it.",
                name
            );
        }
        None => {}
    }

    // Display: daemon lifecycle
    if result.daemon_stopped {
        eprintln!("Daemon stopped.");
    } else if result.daemon_events_active {
        eprintln!(
            "\nNote: Daemon is running with polling enabled. \
             Enabled members may restart on GitHub events.\n\
             Use `bm disable <member>` to prevent auto-restart, \
             or `bm stop --all` to stop the daemon."
        );
    }

    Ok(())
}

#[cfg(test)]
mod session_cli_stop_tests {
    use super::*;
    use crate::daemon::{DirtyRepoInfo, SessionInfo, SessionsListResponse, StopSessionResponse};
    use std::cell::RefCell;

    fn make_session(id: &str, member: &str, stype: &str, state: &str) -> SessionInfo {
        SessionInfo {
            session_id: id.to_string(),
            owning_member: member.to_string(),
            session_type: stype.to_string(),
            current_state: state.to_string(),
            start_time: "2026-05-31T00:00:00Z".to_string(),
            workspace_path: None,
            ..SessionInfo::default()
        }
    }

    // AC-15: bm stop <member> — all sessions for that member enter deactivation, others untouched
    #[test]
    fn stop_member_triggers_graceful_deactivation_for_all_member_sessions() {
        let sessions = vec![
            make_session("s1", "alice", "loop", "Active"),
            make_session("s2", "alice", "brain", "Active"),
            make_session("s3", "bob", "loop", "Active"),
        ];
        let stopped_ids = RefCell::new(vec![]);
        let list_fn = || {
            Ok(SessionsListResponse {
                sessions: sessions.clone(),
            })
        };
        let stop_fn = |id: &str| {
            stopped_ids.borrow_mut().push(id.to_string());
            Ok(StopSessionResponse {
                ok: true,
                dirty_repos: vec![],
                error: None,
            })
        };

        let summary = stop_member_sessions("alice", &list_fn, &stop_fn).unwrap();

        let ids = stopped_ids.borrow();
        assert!(
            ids.contains(&"s1".to_string()),
            "alice's loop session must be stopped; stopped: {ids:?}"
        );
        assert!(
            ids.contains(&"s2".to_string()),
            "alice's brain session must be stopped; stopped: {ids:?}"
        );
        assert!(
            !ids.contains(&"s3".to_string()),
            "bob's session must NOT be stopped; stopped: {ids:?}"
        );
        assert_eq!(
            summary.sessions_deactivating, 2,
            "2 alice sessions must be deactivating"
        );
    }

    // AC-15: bm stop --session <id> — only that specific session is stopped, others untouched
    #[test]
    fn stop_session_specific_only_stops_that_session() {
        let stopped_ids = RefCell::new(vec![]);
        let stop_fn = |id: &str| {
            stopped_ids.borrow_mut().push(id.to_string());
            Ok(StopSessionResponse {
                ok: true,
                dirty_repos: vec![],
                error: None,
            })
        };

        let summary = stop_session_by_id("target-session", &stop_fn).unwrap();

        let ids = stopped_ids.borrow();
        assert_eq!(
            ids.len(),
            1,
            "exactly one session must be stopped; stopped: {ids:?}"
        );
        assert_eq!(
            ids[0], "target-session",
            "only the target session must be stopped"
        );
        assert_eq!(
            summary.sessions_deactivating, 1,
            "1 session must be deactivating"
        );
    }

    // AC-15: bare bm stop — autonomous (loop/brain) stopped, interactive sessions skipped
    #[test]
    fn bare_stop_stops_autonomous_skips_interactive() {
        let sessions = vec![
            make_session("s1", "alice", "loop", "Active"),
            make_session("s2", "bob", "brain", "Active"),
            make_session("s3", "carol", "interactive", "Active"),
        ];
        let stopped_ids = RefCell::new(vec![]);
        let list_fn = || {
            Ok(SessionsListResponse {
                sessions: sessions.clone(),
            })
        };
        let stop_fn = |id: &str| {
            stopped_ids.borrow_mut().push(id.to_string());
            Ok(StopSessionResponse {
                ok: true,
                dirty_repos: vec![],
                error: None,
            })
        };

        let summary = stop_autonomous_sessions(&list_fn, &stop_fn).unwrap();

        let ids = stopped_ids.borrow();
        assert!(
            ids.contains(&"s1".to_string()),
            "loop session must be stopped; stopped: {ids:?}"
        );
        assert!(
            ids.contains(&"s2".to_string()),
            "brain session must be stopped; stopped: {ids:?}"
        );
        assert!(
            !ids.contains(&"s3".to_string()),
            "interactive session must NOT be stopped; stopped: {ids:?}"
        );
        assert_eq!(
            summary.interactive_skipped, 1,
            "1 interactive session must be reported as skipped"
        );
        assert_eq!(
            summary.sessions_deactivating, 2,
            "2 autonomous sessions must be deactivating"
        );
    }

    // AC-15: force stop Active session → Killed immediately, no finalization launched
    #[test]
    fn force_stop_active_session_kills_immediately_no_finalization() {
        let force_fn = |id: &str| {
            Ok(ForceStopResponse {
                ok: true,
                session_id: id.to_string(),
                new_state: "Killed".to_string(),
                finalization_launched: false,
                error: None,
            })
        };

        let summary = force_stop_session_cmd("active-session-id", &force_fn).unwrap();

        assert_eq!(
            summary.sessions_deactivating, 1,
            "1 session must be deactivating"
        );
        assert_eq!(
            summary.entering_finalization, 0,
            "force stop must NOT launch finalization"
        );
    }

    // AC-15: force stop Finalizing session → finalization subagent terminated, Killed, workspace retained
    #[test]
    fn force_stop_finalizing_session_terminates_subagent_retained() {
        let force_fn = |id: &str| {
            Ok(ForceStopResponse {
                ok: true,
                session_id: id.to_string(),
                new_state: "Killed".to_string(),
                finalization_launched: false,
                error: None,
            })
        };

        let summary = force_stop_session_cmd("finalizing-session-id", &force_fn).unwrap();

        assert_eq!(
            summary.sessions_deactivating, 1,
            "1 session must be deactivating"
        );
        assert_eq!(
            summary.entering_finalization, 0,
            "force stop on Finalizing must not launch new finalization"
        );
    }

    // AC-15: retained session can be re-triggered → fresh finalization subagent launched (Finalizing)
    #[test]
    fn retrigger_after_force_stop_launches_fresh_finalization() {
        let retrigger_called = RefCell::new(false);
        let retrigger_fn = |id: &str| {
            *retrigger_called.borrow_mut() = true;
            Ok(RetriggerFinalizationResponse {
                ok: true,
                session_id: id.to_string(),
                new_state: "Finalizing".to_string(),
                error: None,
            })
        };

        retrigger_finalization_cmd("retained-session-id", &retrigger_fn).unwrap();

        assert!(
            *retrigger_called.borrow(),
            "retrigger function must be called"
        );
    }

    // AC-15: bm stop returns immediately with a summary — does not block waiting for finalization
    #[test]
    fn stop_returns_immediately_with_summary() {
        let sessions = vec![
            make_session("s1", "alice", "loop", "Active"),
            make_session("s2", "alice", "loop", "Active"),
        ];
        // stop_fn returns dirty repos (finalization needed) — CLI must still return immediately
        let list_fn = || {
            Ok(SessionsListResponse {
                sessions: sessions.clone(),
            })
        };
        let stop_fn = |_id: &str| {
            Ok(StopSessionResponse {
                ok: true,
                dirty_repos: vec![DirtyRepoInfo {
                    name: "my-project".to_string(),
                    has_uncommitted: true,
                    unpushed_branches: vec![],
                }],
                error: None,
            })
        };

        let summary = stop_member_sessions("alice", &list_fn, &stop_fn).unwrap();

        assert_eq!(
            summary.sessions_deactivating, 2,
            "both sessions must be deactivating"
        );
        assert_eq!(
            summary.entering_finalization, 2,
            "both dirty sessions must be entering finalization"
        );
        // Reaching here proves the function returned immediately without blocking
    }
}
