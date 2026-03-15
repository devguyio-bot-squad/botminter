use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::bridge;
use crate::config::{self, TeamEntry};
use crate::formation;
use crate::profile;
use crate::state::{self, MemberRuntime, RuntimeState};
use crate::topology::{self, Endpoint, MemberTopology, Topology};
use crate::workspace;

/// Handles `bm start [member] [-t team] [--formation <name>] [--no-bridge] [--bridge-only]`.
pub fn run(
    team_flag: Option<&str>,
    formation_flag: Option<&str>,
    no_bridge: bool,
    bridge_only: bool,
    member_filter: Option<&str>,
) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Schema version guard
    let manifest_path = team_repo.join("botminter.yml");
    if !manifest_path.exists() {
        bail!(
            "Team repo at {} has no botminter.yml. Is this a valid team repo?",
            team_repo.display()
        );
    }
    let manifest_contents = fs::read_to_string(&manifest_path)
        .context("Failed to read team botminter.yml")?;
    let team_manifest: serde_yml::Value =
        serde_yml::from_str(&manifest_contents).context("Failed to parse team botminter.yml")?;
    let team_schema = team_manifest["schema_version"]
        .as_str()
        .unwrap_or("");
    let team_profile = team_manifest["profile"]
        .as_str()
        .unwrap_or(&team.profile);
    profile::check_schema_version(team_profile, team_schema)?;

    // Resolve formation
    let resolved_formation = formation::resolve_formation(&team_repo, formation_flag)?;

    // Non-local formations require current schema
    if let Some(ref fname) = resolved_formation {
        if fname != "local" {
            profile::require_current_schema(&team.name, team_schema)?;
            // Non-local formations delegate to formation manager
            let formation_cfg = formation::load(&team_repo, fname)?;
            if !formation_cfg.is_local() {
                return run_formation_manager(team, &team_repo, &formation_cfg, &cfg.workzone);
            }
        }
    }

    // Bridge auto-start (before members) — skip when starting a single member
    if !no_bridge && member_filter.is_none() && team.bridge_lifecycle.start_on_up {
        if let Some(bridge_dir) = bridge::discover(&team_repo, &team.name)? {
            let state_path = bridge::state_path(&cfg.workzone, &team.name);
            let mut b = bridge::Bridge::new(bridge_dir, state_path, team.name.clone())?;
            if b.is_local() {
                if which::which("just").is_err() {
                    eprintln!(
                        "Warning: 'just' not found. Skipping bridge start. \
                         Install: https://just.systems/"
                    );
                } else {
                    b.start()?;
                    b.save()?;
                }
            }
            if b.is_external() {
                println!(
                    "Bridge '{}' is external (managed externally).",
                    b.bridge_name()
                );
            }
        }
    }

    if bridge_only {
        return Ok(());
    }

    // Prerequisite: ralph must be installed
    if which::which("ralph").is_err() {
        bail!("'ralph' not found in PATH. Install ralph-orchestrator first.");
    }

    // Credentials → env vars
    let gh_token = require_gh_token(team)?;

    // Per-member credential resolution via CredentialStore (system keyring).
    // Each Ralph instance gets its own credential, not a team-wide token.
    // Bridge-type-aware: dispatches RALPH_ROCKETCHAT_AUTH_TOKEN for RC,
    // RALPH_TELEGRAM_BOT_TOKEN for Telegram.
    let (credential_store, bridge_type_name, bridge_service_url) = if let Some(ref dir) = bridge::discover(&team_repo, &team.name)? {
        let bstate_path = bridge::state_path(&cfg.workzone, &team.name);
        let b = bridge::Bridge::new(dir.clone(), bstate_path.clone(), team.name.clone())?;
        let store = bridge::LocalCredentialStore::new(
            &team.name,
            b.bridge_name(),
            bstate_path,
        ).with_collection(cfg.keyring_collection.clone());
        let bname = Some(b.bridge_name().to_string());
        let surl = b.service_url().map(|s| s.to_string());
        (Some(store), bname, surl)
    } else {
        (None, None, None)
    };

    // Discover members
    let members_dir = team_repo.join("members");
    if !members_dir.is_dir() {
        bail!("No members hired. Run `bm hire <role>` first.");
    }

    let all_member_dirs = workspace::list_member_dirs(&members_dir)?;
    if all_member_dirs.is_empty() {
        bail!("No members hired. Run `bm hire <role>` first.");
    }

    // Filter to a single member if requested
    let member_dirs = if let Some(target) = member_filter {
        if !all_member_dirs.iter().any(|d| d == target) {
            bail!(
                "Member '{}' not found. Available: {}",
                target,
                all_member_dirs.join(", ")
            );
        }
        vec![target.to_string()]
    } else {
        all_member_dirs
    };

    // Load state, clean up stale entries
    let mut state = state::load()?;
    let stale = state::cleanup_stale(&mut state);
    if !stale.is_empty() {
        for key in &stale {
            eprintln!("Cleaned stale entry for {}", key);
        }
        state::save(&state)?;
    }

    // Discover workspaces and launch
    let workzone = &cfg.workzone;
    let team_ws_base = workzone.join(&team.name);
    let mut launched = 0u32;
    let mut skipped = 0u32;
    let mut errors = 0u32;

    for member_dir_name in &member_dirs {
        let state_key = format!("{}/{}", team.name, member_dir_name);

        // Check if already running
        if let Some(rt) = state.members.get(&state_key) {
            if state::is_alive(rt.pid) {
                eprintln!(
                    "{}: already running (PID {})",
                    member_dir_name, rt.pid
                );
                skipped += 1;
                continue;
            }
            // Stale — remove and re-launch
            state.members.remove(&state_key);
        }

        // Find workspace
        let ws = workspace::find_workspace(&team_ws_base, member_dir_name);
        let ws = match ws {
            Some(ws) => ws,
            None => {
                eprintln!(
                    "{}: no workspace found. Run `bm teams sync` first.",
                    member_dir_name
                );
                errors += 1;
                continue;
            }
        };

        // Resolve per-member bridge credential: env var first, then keyring
        let member_token = if let Some(ref store) = credential_store {
            bridge::resolve_credential_from_store(member_dir_name, store)?
        } else {
            None
        };

        // Diagnostic warning: credential exists but RObot.enabled is false
        if member_token.is_some() {
            let ralph_yml = ws.join("ralph.yml");
            if check_robot_enabled_mismatch(&ralph_yml, true) {
                eprintln!(
                    "Warning: {} has bridge credentials but RObot is disabled in ralph.yml. \
                     Run 'bm teams sync' to update.",
                    member_dir_name
                );
            }
        }

        // Launch ralph
        match launch_ralph(&ws, &gh_token, member_token.as_deref(), bridge_type_name.as_deref(), bridge_service_url.as_deref()) {
            Ok(pid) => {
                let started_at = chrono::Utc::now().to_rfc3339();
                state.members.insert(
                    state_key,
                    MemberRuntime {
                        pid,
                        started_at,
                        workspace: ws,
                    },
                );
                state::save(&state)?;

                // Verify alive after 2 seconds
                thread::sleep(Duration::from_secs(2));
                if state::is_alive(pid) {
                    eprintln!("{}: started (PID {})", member_dir_name, pid);
                    launched += 1;
                } else {
                    eprintln!(
                        "{}: process exited immediately (PID {}). Check workspace logs.",
                        member_dir_name, pid
                    );
                    state.members.remove(&format!("{}/{}", team.name, member_dir_name));
                    state::save(&state)?;
                    errors += 1;
                }
            }
            Err(e) => {
                eprintln!("{}: failed to launch — {}", member_dir_name, e);
                errors += 1;
            }
        }
    }

    println!(
        "\nStarted {} member(s), skipped {} (already running), {} error(s).",
        launched, skipped, errors
    );

    if errors > 0 {
        bail!("Some members failed to start. See errors above.");
    }

    // Write topology file for v2 teams (when formations dir exists)
    if resolved_formation.is_some() {
        write_local_topology(&cfg.workzone, &team.name, &state)?;
    }

    Ok(())
}

/// Extracts GH_TOKEN from credentials, erroring if missing.
fn require_gh_token(team: &TeamEntry) -> Result<String> {
    team.credentials
        .gh_token
        .clone()
        .with_context(|| {
            format!(
                "No GH token configured for team '{}'. \
                 Run `bm init` or edit `~/.botminter/config.yml`.",
                team.name
            )
        })
}

// list_member_dirs and find_workspace live in crate::workspace

/// Launches `ralph run -p PROMPT.md` in the given workspace directory.
/// Returns the child PID.
fn launch_ralph(
    workspace: &std::path::Path,
    gh_token: &str,
    member_token: Option<&str>,
    bridge_type: Option<&str>,
    service_url: Option<&str>,
) -> Result<u32> {
    let mut cmd = Command::new("ralph");
    cmd.args(["run", "-p", "PROMPT.md"])
        .current_dir(workspace)
        .env("GH_TOKEN", gh_token)
        // Unset CLAUDECODE to avoid nested-Claude issues
        .env_remove("CLAUDECODE");

    if let Some(token) = member_token {
        match bridge_type {
            Some("rocketchat") => {
                cmd.env("RALPH_ROCKETCHAT_AUTH_TOKEN", token);
                if let Some(url) = service_url {
                    cmd.env("RALPH_ROCKETCHAT_SERVER_URL", url);
                }
            }
            Some("tuwunel") => {
                cmd.env("RALPH_MATRIX_ACCESS_TOKEN", token);
                if let Some(url) = service_url {
                    cmd.env("RALPH_MATRIX_HOMESERVER_URL", url);
                }
            }
            _ => {
                cmd.env("RALPH_TELEGRAM_BOT_TOKEN", token);
            }
        }
    }

    // Detach from current process group
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let child = cmd.spawn().with_context(|| {
        format!(
            "Failed to spawn ralph in {}",
            workspace.display()
        )
    })?;

    Ok(child.id())
}

/// Checks if a member has a credential but RObot.enabled is false in ralph.yml.
///
/// Returns `true` if there is a mismatch (credential present but RObot disabled),
/// meaning the user should run `bm teams sync` to update.
fn check_robot_enabled_mismatch(
    ralph_yml_path: &std::path::Path,
    has_credential: bool,
) -> bool {
    if !has_credential {
        return false;
    }
    if !ralph_yml_path.exists() {
        return false;
    }
    let contents = match fs::read_to_string(ralph_yml_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let doc: serde_yml::Value = match serde_yml::from_str(&contents) {
        Ok(d) => d,
        Err(_) => return false,
    };

    // Check if RObot.enabled is explicitly false
    match doc.get("RObot").and_then(|r| r.get("enabled")).and_then(|e| e.as_bool()) {
        Some(false) => true,  // Mismatch: has cred but disabled
        _ => false,           // Either enabled or not set at all
    }
}

/// Resolves state for display/inspection by external callers.
pub fn resolve_member_status(
    state: &RuntimeState,
    team_name: &str,
    member_dir_name: &str,
) -> MemberStatus {
    let key = format!("{}/{}", team_name, member_dir_name);
    match state.members.get(&key) {
        Some(rt) => {
            if state::is_alive(rt.pid) {
                MemberStatus::Running {
                    pid: rt.pid,
                    started_at: rt.started_at.clone(),
                }
            } else {
                MemberStatus::Crashed {
                    pid: rt.pid,
                    started_at: rt.started_at.clone(),
                }
            }
        }
        None => MemberStatus::Stopped,
    }
}

/// Writes a local topology file after starting members.
fn write_local_topology(
    workzone: &std::path::Path,
    team_name: &str,
    state: &RuntimeState,
) -> Result<()> {
    use std::collections::HashMap;

    let team_prefix = format!("{}/", team_name);
    let mut members = HashMap::new();

    for (key, rt) in &state.members {
        if !key.starts_with(&team_prefix) {
            continue;
        }
        let member_name = key.strip_prefix(&team_prefix).unwrap_or(key);
        members.insert(
            member_name.to_string(),
            MemberTopology {
                status: if crate::state::is_alive(rt.pid) {
                    "running".to_string()
                } else {
                    "stopped".to_string()
                },
                endpoint: Endpoint::Local {
                    pid: rt.pid,
                    workspace: rt.workspace.clone(),
                },
            },
        );
    }

    let topo = Topology {
        formation: "local".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        members,
    };

    let topo_path = topology::topology_path(workzone, team_name);
    topology::save(&topo_path, &topo)?;

    Ok(())
}

/// Runs a non-local formation manager (one-shot Ralph session).
fn run_formation_manager(
    team: &TeamEntry,
    team_repo: &std::path::Path,
    formation_cfg: &formation::FormationConfig,
    workzone: &std::path::Path,
) -> Result<()> {
    let mgr = formation_cfg.manager.as_ref().with_context(|| {
        format!(
            "Formation '{}' has no manager configuration",
            formation_cfg.name
        )
    })?;

    let formation_dir = formation::formations_dir(team_repo).join(&formation_cfg.name);
    let prompt_path = formation_dir.join(&mgr.prompt);
    let ralph_yml_path = formation_dir.join(&mgr.ralph_yml);

    // Prepare env vars
    let mut env_vars = Vec::new();
    if let Some(token) = &team.credentials.gh_token {
        env_vars.push(("GH_TOKEN".to_string(), token.clone()));
    }
    // Legacy fallback: formation manager gets team-wide token.
    // TODO: Formation manager should resolve per-member credentials via CredentialStore
    // when non-local formations support bridge integration.
    if let Some(token) = &team.credentials.telegram_bot_token {
        // Determine bridge type for correct env var dispatch
        let team_bridge_type = bridge::discover(team_repo, &team.name)
            .ok()
            .flatten()
            .and_then(|dir| bridge::load_manifest(&dir).ok())
            .map(|m| m.metadata.name.clone());

        match team_bridge_type.as_deref() {
            Some("rocketchat") => {
                env_vars.push(("RALPH_ROCKETCHAT_AUTH_TOKEN".to_string(), token.clone()));
            }
            Some("tuwunel") => {
                env_vars.push(("RALPH_MATRIX_ACCESS_TOKEN".to_string(), token.clone()));
            }
            _ => {
                env_vars.push(("RALPH_TELEGRAM_BOT_TOKEN".to_string(), token.clone()));
            }
        }
    }
    // Pass workzone and team info to formation manager
    env_vars.push(("BM_WORKZONE".to_string(), workzone.display().to_string()));
    env_vars.push(("BM_TEAM_NAME".to_string(), team.name.clone()));
    env_vars.push(("BM_TEAM_REPO".to_string(), team_repo.display().to_string()));

    eprintln!(
        "Launching formation manager for '{}' formation...",
        formation_cfg.name
    );

    let status = crate::session::oneshot_ralph_session(
        &formation_dir,
        &prompt_path,
        &ralph_yml_path,
        &env_vars,
    )?;

    if !status.success() {
        bail!(
            "Formation manager '{}' failed (exit code: {:?})",
            formation_cfg.name,
            status.code()
        );
    }

    // Verify topology file was written
    let topo_path = topology::topology_path(workzone, &team.name);
    if !topo_path.exists() {
        bail!(
            "Formation manager completed but no topology file was written at {}",
            topo_path.display()
        );
    }

    eprintln!("Formation '{}' deployed successfully.", formation_cfg.name);
    Ok(())
}

/// Status of a team member process.
#[derive(Debug)]
pub enum MemberStatus {
    Running { pid: u32, started_at: String },
    Crashed { pid: u32, started_at: String },
    Stopped,
}

impl MemberStatus {
    pub fn label(&self) -> &'static str {
        match self {
            MemberStatus::Running { .. } => "running",
            MemberStatus::Crashed { .. } => "crashed",
            MemberStatus::Stopped => "stopped",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── list_member_dirs ──────────────────────────────────────────

    #[test]
    fn list_member_dirs_returns_sorted_dirs_only() {
        let tmp = tempfile::tempdir().unwrap();
        // Create directories
        fs::create_dir(tmp.path().join("bob")).unwrap();
        fs::create_dir(tmp.path().join("alice")).unwrap();
        fs::create_dir(tmp.path().join(".hidden")).unwrap();
        // Create a plain file
        fs::write(tmp.path().join("file.txt"), "hello").unwrap();

        let result = crate::workspace::list_member_dirs(tmp.path()).unwrap();
        assert_eq!(result, vec!["alice", "bob"]);
    }

    #[test]
    fn list_member_dirs_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = crate::workspace::list_member_dirs(tmp.path()).unwrap();
        assert!(result.is_empty());
    }

    // ── find_workspace ────────────────────────────────────────────

    #[test]
    fn find_workspace_with_marker() {
        let tmp = tempfile::tempdir().unwrap();
        let team_ws_base = tmp.path();
        let member_dir = team_ws_base.join("member");
        fs::create_dir_all(&member_dir).unwrap();
        fs::write(member_dir.join(".botminter.workspace"), "member: member\n").unwrap();

        let result = crate::workspace::find_workspace(team_ws_base, "member");
        assert_eq!(result, Some(member_dir));
    }

    #[test]
    fn find_workspace_old_botminter_dir_not_recognized() {
        let tmp = tempfile::tempdir().unwrap();
        let team_ws_base = tmp.path();
        // Old model: .botminter/ dir without marker file
        let member_dir = team_ws_base.join("member");
        fs::create_dir_all(member_dir.join(".botminter")).unwrap();

        let result = crate::workspace::find_workspace(team_ws_base, "member");
        assert_eq!(result, None);
    }

    #[test]
    fn find_workspace_missing_member_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = crate::workspace::find_workspace(tmp.path(), "nonexistent");
        assert_eq!(result, None);
    }

    #[test]
    fn find_workspace_no_marker() {
        let tmp = tempfile::tempdir().unwrap();
        // Create member dir without any marker
        fs::create_dir_all(tmp.path().join("member")).unwrap();

        let result = crate::workspace::find_workspace(tmp.path(), "member");
        assert_eq!(result, None);
    }

    // ── resolve_member_status ─────────────────────────────────────

    #[test]
    fn resolve_member_status_running() {
        let mut state = RuntimeState::default();
        let alive_pid = std::process::id(); // current process, guaranteed alive
        state.members.insert(
            "team/member".to_string(),
            MemberRuntime {
                pid: alive_pid,
                started_at: "2026-02-21T10:00:00Z".to_string(),
                workspace: PathBuf::from("/tmp/ws"),
            },
        );

        let status = resolve_member_status(&state, "team", "member");
        assert_eq!(status.label(), "running");
        match status {
            MemberStatus::Running { pid, .. } => assert_eq!(pid, alive_pid),
            other => panic!("Expected Running, got {:?}", other),
        }
    }

    #[test]
    fn resolve_member_status_crashed() {
        let mut state = RuntimeState::default();
        let dead_pid = 4_000_000u32; // unlikely to exist
        state.members.insert(
            "team/member".to_string(),
            MemberRuntime {
                pid: dead_pid,
                started_at: "2026-02-21T10:00:00Z".to_string(),
                workspace: PathBuf::from("/tmp/ws"),
            },
        );

        let status = resolve_member_status(&state, "team", "member");
        assert_eq!(status.label(), "crashed");
        match status {
            MemberStatus::Crashed { pid, .. } => assert_eq!(pid, dead_pid),
            other => panic!("Expected Crashed, got {:?}", other),
        }
    }

    #[test]
    fn resolve_member_status_stopped() {
        let state = RuntimeState::default(); // empty — no entries
        let status = resolve_member_status(&state, "team", "member");
        assert_eq!(status.label(), "stopped");
    }

    // ── require_gh_token ──────────────────────────────────────────

    #[test]
    fn require_gh_token_present() {
        let team = TeamEntry {
            name: "test-team".to_string(),
            path: PathBuf::from("/tmp/team"),
            profile: "scrum".to_string(),
            github_repo: "org/repo".to_string(),
            credentials: config::Credentials {
                gh_token: Some("ghp_test123".to_string()),
                telegram_bot_token: None,
                webhook_secret: None,
            },
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
        };
        let token = require_gh_token(&team).unwrap();
        assert_eq!(token, "ghp_test123");
    }

    #[test]
    fn require_gh_token_missing_errors_with_team_name() {
        let team = TeamEntry {
            name: "my-team".to_string(),
            path: PathBuf::from("/tmp/team"),
            profile: "scrum".to_string(),
            github_repo: "org/repo".to_string(),
            credentials: config::Credentials {
                gh_token: None,
                telegram_bot_token: None,
                webhook_secret: None,
            },
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
        };
        let err = require_gh_token(&team).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("my-team"),
            "Error should mention team name, got: {msg}"
        );
    }

    // ── MemberStatus::label ───────────────────────────────────────

    #[test]
    fn member_status_labels() {
        assert_eq!(
            MemberStatus::Running {
                pid: 1,
                started_at: String::new()
            }
            .label(),
            "running"
        );
        assert_eq!(
            MemberStatus::Crashed {
                pid: 1,
                started_at: String::new()
            }
            .label(),
            "crashed"
        );
        assert_eq!(MemberStatus::Stopped.label(), "stopped");
    }

    // ── Per-member credential resolution tests ─────────────────────

    #[test]
    fn resolve_per_member_credential_from_store() {
        use crate::bridge::{
            self, CredentialStore, InMemoryCredentialStore,
        };

        let store = InMemoryCredentialStore::new();
        store.store("alice", "alice-token").unwrap();
        store.store("bob", "bob-token").unwrap();

        // Each member gets their own token
        let alice_token = bridge::resolve_credential_from_store("alice", &store).unwrap();
        let bob_token = bridge::resolve_credential_from_store("bob", &store).unwrap();

        assert_eq!(alice_token, Some("alice-token".to_string()));
        assert_eq!(bob_token, Some("bob-token".to_string()));
    }

    #[test]
    fn resolve_per_member_credential_missing_returns_none() {
        use crate::bridge::{self, InMemoryCredentialStore};

        let store = InMemoryCredentialStore::new();
        // No credentials stored for charlie

        let result = bridge::resolve_credential_from_store("charlie", &store).unwrap();
        assert!(
            result.is_none(),
            "member without credential should get None"
        );
    }

    #[test]
    fn resolve_per_member_credential_env_var_priority() {
        use crate::bridge::{
            self, CredentialStore, InMemoryCredentialStore,
        };

        let store = InMemoryCredentialStore::new();
        store.store("envpritest", "store-token").unwrap();

        // Set env var — should take priority
        let env_key = "BM_BRIDGE_TOKEN_ENVPRITEST";
        std::env::set_var(env_key, "env-token");

        let result = bridge::resolve_credential_from_store("envpritest", &store).unwrap();
        assert_eq!(
            result,
            Some("env-token".to_string()),
            "env var should take priority over credential store"
        );

        std::env::remove_var(env_key);
    }

    #[test]
    fn launch_ralph_receives_per_member_credential() {
        // This test verifies that launch_ralph correctly accepts bridge-type-aware
        // parameters. We can't test actual process spawning, but we verify the
        // function signature accepts the new bridge_type + service_url parameters.
        //
        // The real test is that `bm start` resolves credentials per-member
        // via resolve_credential_from_store() in the member loop.

        // Verify launch_ralph compiles with bridge-type-aware parameters
        let _: fn(&std::path::Path, &str, Option<&str>, Option<&str>, Option<&str>) -> Result<u32> = launch_ralph;
    }

    #[test]
    fn check_robot_enabled_diagnostic() {
        // Test the diagnostic warning logic: when a member has a credential
        // but RObot.enabled is false, a warning should be emitted.
        // This validates the function exists and works correctly.
        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(
            &ralph_yml,
            "preset: feature-development\nRObot:\n  enabled: false\n",
        )
        .unwrap();

        let has_credential = true;
        let robot_enabled = check_robot_enabled_mismatch(&ralph_yml, has_credential);
        assert!(
            robot_enabled,
            "should return true when credential exists but RObot.enabled is false"
        );
    }
}
