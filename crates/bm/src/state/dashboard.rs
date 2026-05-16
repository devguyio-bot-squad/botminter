use std::path::Path;

use anyhow::Result;

use crate::bridge;
use crate::config::{BotminterConfig, TeamEntry};
use crate::daemon;
use crate::profile;
use crate::topology;
use crate::workspace;

use super::MemberStatus;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Aggregated status data for the team dashboard display.
pub struct StatusInfo {
    pub formation: Option<String>,
    pub project_names: Vec<String>,
    pub daemon: Option<DaemonDisplay>,
    pub members: Vec<MemberRow>,
    pub has_members: bool,
    pub crashed_cleaned: usize,
    pub bridge: Option<BridgeDisplay>,
    pub verbose: Option<VerboseDisplay>,
    pub tmux: Option<TmuxStatusInfo>,
}

/// Tmux session info for the status dashboard.
pub struct TmuxStatusInfo {
    pub session_name: String,
    pub socket_name: String,
    pub window_count: usize,
    pub attach_command: String,
    pub raw_attach_command: String,
}

/// Daemon status info for display.
pub struct DaemonDisplay {
    pub pid: u32,
    pub mode: String,
    pub port: u16,
    pub interval_secs: u64,
}

/// A single member's status row.
pub struct MemberRow {
    pub name: String,
    pub role: String,
    pub status: MemberStatus,
    pub branch: String,
    pub enabled: bool,
}

/// Bridge status info for display.
pub struct BridgeDisplay {
    pub name: String,
    pub bridge_type: String,
    pub status: String,
    pub url: Option<String>,
    pub identities: Vec<BridgeIdentityRow>,
}

/// A single bridge identity mapping.
pub struct BridgeIdentityRow {
    pub member: String,
    pub bridge_user: String,
    pub user_id: String,
}

/// Verbose display info (workspace submodules + ralph CLI output).
pub struct VerboseDisplay {
    pub workspaces: Vec<WorkspaceVerbose>,
    pub ralph_sections: Vec<RalphMemberInfo>,
}

/// Workspace submodule info for a single member.
pub struct WorkspaceVerbose {
    pub member: String,
    pub submodules: Vec<SubmoduleRow>,
}

/// A submodule status row.
pub struct SubmoduleRow {
    pub name: String,
    pub status_label: String,
}

/// Ralph runtime info for a running member.
pub struct RalphMemberInfo {
    pub member: String,
    pub pid: u32,
    pub sections: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Gather
// ---------------------------------------------------------------------------

/// Gathers all status dashboard data. Cleans up crashed entries as a side effect.
pub fn gather_status(
    team: &TeamEntry,
    cfg: &BotminterConfig,
    verbose: bool,
) -> Result<StatusInfo> {
    let team_repo = team.path.join("team");
    let team_name = &team.name;

    // Formation
    let topo_path = topology::topology_path(&cfg.workzone, team_name);
    let topo = topology::load(&topo_path)?;
    let formation = topo.as_ref().map(|t| t.formation.clone());

    // Projects
    let project_names = match profile::read_team_repo_manifest(&team_repo) {
        Ok(m) => m.projects.iter().map(|p| p.name.clone()).collect(),
        Err(_) => Vec::new(),
    };

    // Daemon
    let daemon = gather_daemon_info(team_name);

    // Members
    let members_dir = team_repo.join("members");
    let member_dirs = profile::discover_member_dirs(&team_repo);
    let has_members = !member_dirs.is_empty();
    let mut runtime_state = super::load()?;

    let mut members = Vec::new();
    let mut crashed_keys: Vec<String> = Vec::new();

    for name in &member_dirs {
        let role = profile::read_member_role(&members_dir, name);
        let status = super::resolve_member_status(&runtime_state, team_name, name);
        let ws_path = team.path.join(name);
        let branch = if ws_path.join(".botminter.workspace").exists() {
            workspace::workspace_git_branch(&ws_path)
        } else {
            "—".to_string()
        };
        if matches!(&status, MemberStatus::Crashed { .. }) {
            crashed_keys.push(format!("{}/{}", team_name, name));
        }
        let key = format!("{}/{}", team_name, name);
        let enabled = super::is_enabled(&runtime_state, &key);
        members.push(MemberRow {
            name: name.clone(),
            role,
            status,
            branch,
            enabled,
        });
    }

    // Clean crashed
    let crashed_cleaned = crashed_keys.len();
    if !crashed_keys.is_empty() {
        for key in &crashed_keys {
            runtime_state.members.remove(key);
        }
        super::save(&runtime_state)?;
    }

    // Bridge
    let bridge_display = gather_bridge_info(&team_repo, team_name, cfg);

    // Verbose
    let verbose_display = if verbose {
        Some(gather_verbose(
            &member_dirs,
            &team.path,
            team_name,
        )?)
    } else {
        None
    };

    // Tmux
    let tmux = gather_tmux_info(team_name);

    Ok(StatusInfo {
        formation,
        project_names,
        daemon,
        members,
        has_members,
        crashed_cleaned,
        bridge: bridge_display,
        verbose: verbose_display,
        tmux,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn gather_tmux_info(_team_name: &str) -> Option<TmuxStatusInfo> {
    None
}

fn gather_daemon_info(team_name: &str) -> Option<DaemonDisplay> {
    match daemon::query_status(team_name) {
        Ok(daemon::DaemonStatusInfo::Running { pid, config }) => Some(DaemonDisplay {
            pid,
            mode: config
                .as_ref()
                .map(|c| c.mode.clone())
                .unwrap_or_default(),
            port: config.as_ref().map(|c| c.port).unwrap_or(0),
            interval_secs: config.as_ref().map(|c| c.interval_secs).unwrap_or(0),
        }),
        _ => None,
    }
}

fn gather_bridge_info(
    team_repo: &Path,
    team_name: &str,
    cfg: &BotminterConfig,
) -> Option<BridgeDisplay> {
    let bridge_dir = match bridge::discover(team_repo, team_name) {
        Ok(Some(dir)) => dir,
        _ => return None,
    };
    let state_path = bridge::state_path(&cfg.workzone, team_name);
    let b = match bridge::Bridge::new(bridge_dir, state_path, team_name.to_string()) {
        Ok(b) if b.is_active() => b,
        _ => return None,
    };

    let mut identities = Vec::new();
    let mut entries: Vec<_> = b.identities().iter().collect();
    entries.sort_by_key(|(k, _)| (*k).clone());
    for (username, identity) in entries {
        identities.push(BridgeIdentityRow {
            member: username.clone(),
            bridge_user: identity.username.clone(),
            user_id: identity.user_id.clone(),
        });
    }

    Some(BridgeDisplay {
        name: b.bridge_name().to_string(),
        bridge_type: b.bridge_type().to_string(),
        status: b.status().to_string(),
        url: b.service_url().map(|s| s.to_string()),
        identities,
    })
}

fn gather_verbose(
    member_dirs: &[String],
    team_path: &Path,
    team_name: &str,
) -> Result<VerboseDisplay> {
    let mut workspaces = Vec::new();
    for name in member_dirs {
        let ws_path = team_path.join(name);
        if !ws_path.join(".botminter.workspace").exists() {
            continue;
        }
        let submodules = workspace::workspace_submodule_status(&ws_path);
        if !submodules.is_empty() {
            workspaces.push(WorkspaceVerbose {
                member: name.clone(),
                submodules: submodules
                    .iter()
                    .map(|s| SubmoduleRow {
                        name: s.name.clone(),
                        status_label: s.status.label().to_string(),
                    })
                    .collect(),
            });
        }
    }

    let runtime_state = super::load()?;
    let team_prefix = format!("{}/", team_name);
    let mut ralph_sections = Vec::new();

    for (key, rt) in &runtime_state.members {
        if !key.starts_with(&team_prefix) {
            continue;
        }
        if !super::is_alive(rt.pid) {
            continue;
        }
        let member_name = key.strip_prefix(&team_prefix).unwrap_or(key).to_string();
        let mut sections = Vec::new();
        for (label, args) in &[
            ("Hats", vec!["hats"]),
            ("Loops", vec!["loops", "list"]),
            ("Events", vec!["events"]),
            ("Bot", vec!["bot", "status"]),
        ] {
            if let Ok(output) = crate::session::run_ralph_cmd(&rt.workspace, args) {
                sections.push((label.to_string(), output));
            }
        }
        ralph_sections.push(RalphMemberInfo {
            member: member_name,
            pid: rt.pid,
            sections,
        });
    }

    Ok(VerboseDisplay {
        workspaces,
        ralph_sections,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formation::local::tmux::{TmuxSession, tmux_cmd};

    struct TmuxGuard {
        session_name: String,
    }

    impl TmuxGuard {
        fn new(session_name: &str) -> Self {
            Self {
                session_name: session_name.to_string(),
            }
        }
    }

    impl Drop for TmuxGuard {
        fn drop(&mut self) {
            let _ = tmux_cmd()
                .args(["-L", "botminter", "kill-session", "-t", &self.session_name])
                .output();
        }
    }

    // ── CT-03 (Story #8): AC1 — Tmux info in status output ──────────

    #[test]
    fn tmux_status_info_includes_session_name_windows_and_attach() {
        let session = TmuxSession::new("ct03s8-info").unwrap();
        let _guard = TmuxGuard::new(session.session_name());
        session.create().expect("session create should succeed");

        let cwd = std::env::temp_dir();
        session
            .create_window("bob", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'bob'");
        session
            .create_window("cos", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'cos'");
        session
            .create_window("sentinel", &["sleep", "300"], &cwd, &[])
            .expect("create_window 'sentinel'");

        let info = gather_tmux_info("ct03s8-info");

        let tmux = info.expect("tmux info must be Some when session exists");
        assert_eq!(tmux.session_name, "bm-ct03s8-info");
        assert!(
            tmux.window_count >= 3,
            "window_count must include created windows, got: {}",
            tmux.window_count
        );
        assert_eq!(tmux.attach_command, "bm attach");
    }

    // ── CT-03 (Story #8): AC2 — Raw attach command shown ────────────

    #[test]
    fn tmux_status_info_includes_raw_attach_command() {
        let session = TmuxSession::new("ct03s8-raw").unwrap();
        let _guard = TmuxGuard::new(session.session_name());
        session.create().expect("session create should succeed");

        let info = gather_tmux_info("ct03s8-raw");

        let tmux = info.expect("tmux info must be Some when session exists");
        assert_eq!(
            tmux.raw_attach_command,
            "tmux -L botminter attach -t bm-ct03s8-raw",
            "raw_attach_command must be the full tmux attach command"
        );
    }

    // ── CT-03 (Story #8): AC3 — No tmux info when session absent ────

    #[test]
    fn no_tmux_info_when_session_does_not_exist() {
        let info = gather_tmux_info("ct03s8-nosession");

        assert!(
            info.is_none(),
            "tmux info must be None when no session exists"
        );
    }

    // ── CT-03 (Story #8): AC4 — Graceful degradation on tmux error ──

    #[test]
    fn tmux_error_returns_none_without_panic() {
        let info = gather_tmux_info("ct03s8-\x00invalid");

        assert!(
            info.is_none(),
            "tmux info must be None on error, not panic"
        );
    }

    // ── CT-03 (Story #8): AC5 — Display format integration ─────────

    #[test]
    fn status_display_includes_tmux_section_when_present() {
        let tmux = TmuxStatusInfo {
            session_name: "bm-test-team".to_string(),
            socket_name: "botminter".to_string(),
            window_count: 3,
            attach_command: "bm attach".to_string(),
            raw_attach_command: "tmux -L botminter attach -t bm-test-team".to_string(),
        };

        let mut output = Vec::new();
        use std::io::Write;
        writeln!(
            &mut output,
            "tmux: {} ({} windows)",
            tmux.session_name, tmux.window_count
        )
        .unwrap();
        writeln!(
            &mut output,
            "attach: {}  (or: {})",
            tmux.attach_command, tmux.raw_attach_command
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(
            text.contains("bm-test-team"),
            "output must contain session name"
        );
        assert!(
            text.contains("3 windows"),
            "output must contain window count"
        );
        assert!(
            text.contains("bm attach"),
            "output must contain friendly attach command"
        );
        assert!(
            text.contains("tmux -L botminter attach -t bm-test-team"),
            "output must contain raw tmux attach command"
        );
    }
}
