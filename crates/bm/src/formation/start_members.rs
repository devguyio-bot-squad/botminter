use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::bridge::{self, BridgeStartResult};
use crate::config::{BotminterConfig, TeamEntry};
use crate::formation::{self, CredentialDomain};
use crate::git::app_auth;
use crate::git::manifest_flow::credential_keys;
use crate::state::{self, MemberRuntime};
use crate::workspace;

use super::MemberFailed;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Outcome of starting the local formation (members + optional bridge).
pub struct StartResult {
    pub launched: Vec<MemberLaunched>,
    pub skipped: Vec<MemberSkipped>,
    pub errors: Vec<MemberFailed>,
    pub stale_cleaned: Vec<String>,
    pub bridge: Option<BridgeAutoStartOutcome>,
}

pub struct MemberLaunched {
    pub name: String,
    pub pid: u32,
    pub brain_mode: bool,
}

pub struct MemberSkipped {
    pub name: String,
    pub pid: u32,
}

/// What happened when we tried to auto-start the bridge.
pub enum BridgeAutoStartOutcome {
    Started(String),
    Restarted(String),
    AlreadyRunning(String),
    External(String),
    JustNotFound,
}

// ---------------------------------------------------------------------------
// Start — launch all members of a local formation
// ---------------------------------------------------------------------------

/// Starts local formation members, optionally auto-starting the bridge first.
///
/// Handles bridge auto-start, prerequisite validation, credential resolution,
/// member discovery, stale state cleanup, process spawning, and topology writing.
pub fn start_local_members(
    team: &TeamEntry,
    cfg: &BotminterConfig,
    team_repo: &Path,
    member_filter: Option<&str>,
    no_bridge: bool,
    resolved_formation: Option<&str>,
) -> Result<StartResult> {
    let mut result = StartResult {
        launched: Vec::new(),
        skipped: Vec::new(),
        errors: Vec::new(),
        stale_cleaned: Vec::new(),
        bridge: None,
    };

    // Bridge auto-start (before members) — skip when starting a single member
    if !no_bridge && member_filter.is_none() && team.bridge_lifecycle.start_on_up {
        result.bridge = auto_start_bridge(team_repo, &team.name, &cfg.workzone);
    }

    // Prerequisite: ralph must be installed
    if which::which("ralph").is_err() {
        bail!("'ralph' not found in PATH. Install ralph-orchestrator first.");
    }

    // Per-member credential resolution via CredentialStore (system keyring)
    let bridge_creds = resolve_bridge_credentials(team_repo, team, cfg)?;

    // GitHub App credential store — used to check per-member App availability
    let local_formation = formation::local::create_local_formation(&team.name)?;
    let app_cred_store = local_formation.credential_store(CredentialDomain::GitHubApp {
        team_name: team.name.clone(),
        member_name: String::new(), // store is team-level, member is in the key
    })?;

    // Discover members
    let member_dirs = discover_members(team_repo, member_filter)?;

    // Load state, clean up stale entries
    let mut state = state::load()?;
    let stale = state::cleanup_stale(&mut state);
    if !stale.is_empty() {
        state::save(&state)?;
    }
    result.stale_cleaned = stale;

    // Launch each member
    let workzone = &cfg.workzone;
    let team_ws_base = workzone.join(&team.name);

    for member_dir_name in &member_dirs {
        let state_key = format!("{}/{}", team.name, member_dir_name);

        // Check if already running
        if let Some(rt) = state.members.get(&state_key) {
            if state::is_alive(rt.pid) {
                result.skipped.push(MemberSkipped {
                    name: member_dir_name.clone(),
                    pid: rt.pid,
                });
                continue;
            }
            // Stale — remove and re-launch
            state.members.remove(&state_key);
        }

        // Find workspace
        let ws = match workspace::find_workspace(&team_ws_base, member_dir_name) {
            Some(ws) => ws,
            None => {
                result.errors.push(MemberFailed {
                    name: member_dir_name.clone(),
                    error: "no workspace found. Run `bm teams sync` first.".to_string(),
                });
                continue;
            }
        };

        // Resolve per-member bridge credential
        let member_token = if let Some(ref store) = bridge_creds.credential_store {
            bridge::resolve_credential_from_store(member_dir_name, store)?
        } else {
            None
        };

        // Resolve per-member bridge user ID and room ID (for brain bridge adapter)
        let member_user_id = (bridge_creds.user_id_by_member)(member_dir_name);
        let member_room_id = (bridge_creds.room_id_by_member)(member_dir_name);

        // Diagnostic: credential exists but RObot.enabled is false
        let robot_mismatch = if member_token.is_some() {
            let ralph_yml = ws.join("ralph.yml");
            formation::check_robot_enabled_mismatch(&ralph_yml, true)
        } else {
            false
        };

        // Resolve GitHub App credentials for this member (on-demand — Req 8).
        // If App creds exist, do JWT→token exchange, setup delivery, and use GH_CONFIG_DIR.
        let gh_config_dir: Option<PathBuf> = match resolve_app_credentials_and_deliver(
            app_cred_store.as_ref(),
            local_formation.as_ref(),
            member_dir_name,
            &ws,
        ) {
            Ok(Some(dir)) => Some(dir),
            Ok(None) => None, // No App creds — fall back to GH_TOKEN
            Err(e) => {
                eprintln!(
                    "Warning: App credential setup failed for {}, falling back to GH_TOKEN: {}",
                    member_dir_name, e
                );
                None
            }
        };

        // Detect brain mode (chat-first member)
        let brain_mode = formation::is_brain_member(&ws);

        // Launch ralph or brain
        let launch_result = if brain_mode {
            let system_prompt_path = ws.join("brain-prompt.md");
            let brain_config = formation::BrainLaunchConfig {
                workspace: &ws,
                system_prompt_path: &system_prompt_path,
                member_token: member_token.as_deref(),
                bridge_type: bridge_creds.bridge_type_name.as_deref(),
                service_url: bridge_creds.service_url.as_deref(),
                room_id: member_room_id.as_deref(),
                user_id: member_user_id.as_deref(),
                operator_user_id: bridge_creds.operator_user_id.as_deref(),
                team_repo: Some(team_repo),
                gh_config_dir: gh_config_dir.as_deref(),
            };
            formation::launch_brain(&brain_config)
        } else {
            formation::launch_ralph(
                &ws,
                member_token.as_deref(),
                bridge_creds.bridge_type_name.as_deref(),
                bridge_creds.service_url.as_deref(),
                gh_config_dir.as_deref(),
            )
        };

        match launch_result {
            Ok(pid) => {
                let started_at = chrono::Utc::now().to_rfc3339();
                state.members.insert(
                    state_key.clone(),
                    MemberRuntime {
                        pid,
                        started_at,
                        workspace: ws,
                        brain_mode,
                    },
                );
                state::save(&state)?;

                // Verify alive after 2 seconds
                thread::sleep(Duration::from_secs(2));
                if state::is_alive(pid) {
                    result.launched.push(MemberLaunched {
                        name: member_dir_name.clone(),
                        pid,
                        brain_mode,
                    });
                } else {
                    state.members.remove(&state_key);
                    state::save(&state)?;
                    result.errors.push(MemberFailed {
                        name: member_dir_name.clone(),
                        error: format!(
                            "process exited immediately (PID {}). Check workspace logs.",
                            pid
                        ),
                    });
                }
            }
            Err(e) => {
                result.errors.push(MemberFailed {
                    name: member_dir_name.clone(),
                    error: format!("failed to launch — {}", e),
                });
            }
        }

        // Emit diagnostic warning about robot mismatch
        if robot_mismatch {
            result.errors.push(MemberFailed {
                name: member_dir_name.clone(),
                error: "has bridge credentials but RObot is disabled in ralph.yml. \
                        Run 'bm teams sync' to update."
                    .to_string(),
            });
        }
    }

    // Write topology file for v2 teams (when formations dir exists)
    if resolved_formation.is_some() && result.errors.is_empty() {
        formation::write_local_topology(&cfg.workzone, &team.name, &state)?;
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Auto-start the bridge if configured and available.
pub fn auto_start_bridge(
    team_repo: &Path,
    team_name: &str,
    workzone: &Path,
) -> Option<BridgeAutoStartOutcome> {
    let bridge_dir = match bridge::discover(team_repo, team_name) {
        Ok(Some(d)) => d,
        _ => return None,
    };
    let state_path = bridge::state_path(workzone, team_name);
    let mut b = match bridge::Bridge::new(bridge_dir, state_path, team_name.to_string()) {
        Ok(b) => b,
        Err(_) => return None,
    };

    if b.is_local() {
        if which::which("just").is_err() {
            return Some(BridgeAutoStartOutcome::JustNotFound);
        }
        let bridge_name = b.bridge_name().to_string();
        match b.start() {
            Ok(BridgeStartResult::AlreadyRunning) => {
                let _ = b.save();
                Some(BridgeAutoStartOutcome::AlreadyRunning(bridge_name))
            }
            Ok(BridgeStartResult::Restarted) => {
                let _ = b.save();
                Some(BridgeAutoStartOutcome::Restarted(bridge_name))
            }
            Ok(BridgeStartResult::Started) => {
                let _ = b.save();
                Some(BridgeAutoStartOutcome::Started(bridge_name))
            }
            Ok(BridgeStartResult::External) => None, // Can't happen for local bridge
            Err(_) => None,
        }
    } else if b.is_external() {
        Some(BridgeAutoStartOutcome::External(
            b.bridge_name().to_string(),
        ))
    } else {
        None
    }
}

/// Resolve bridge credential store and metadata for per-member token injection.
type MemberLookup = Box<dyn Fn(&str) -> Option<String>>;

/// Resolved bridge credentials and metadata for member launch.
struct BridgeCredentials {
    credential_store: Option<bridge::LocalCredentialStore>,
    bridge_type_name: Option<String>,
    service_url: Option<String>,
    user_id_by_member: MemberLookup,
    room_id_by_member: MemberLookup,
    operator_user_id: Option<String>,
}

fn resolve_bridge_credentials(
    team_repo: &Path,
    team: &TeamEntry,
    cfg: &BotminterConfig,
) -> Result<BridgeCredentials> {
    if let Some(ref dir) = bridge::discover(team_repo, &team.name)? {
        let bstate_path = bridge::state_path(&cfg.workzone, &team.name);
        let b = bridge::Bridge::new(dir.clone(), bstate_path.clone(), team.name.clone())?;
        let store = bridge::LocalCredentialStore::new(&team.name, b.bridge_name(), bstate_path.clone())
            .with_collection(cfg.keyring_collection.clone());
        let bname = Some(b.bridge_name().to_string());
        let surl = b.service_url().map(|s| s.to_string());

        // Pre-compute per-member room lookup (bridge is moved into user_id closure)
        let member_rooms: std::collections::HashMap<String, String> = b
            .rooms()
            .iter()
            .filter_map(|r| {
                let member = r.member.as_ref()?;
                let rid = r.room_id.as_ref()?;
                Some((member.clone(), rid.clone()))
            })
            .collect();

        // Resolve operator user ID for DM discovery security
        let op_user_id = b.admin_user_id().map(|s| s.to_string());

        // Capture bridge for per-member user_id lookup
        Ok(BridgeCredentials {
            credential_store: Some(store),
            bridge_type_name: bname,
            service_url: surl,
            user_id_by_member: Box::new(move |member_name: &str| {
                b.member_user_id(member_name)
            }),
            room_id_by_member: Box::new(move |member_name: &str| {
                member_rooms.get(member_name).cloned()
            }),
            operator_user_id: op_user_id,
        })
    } else {
        Ok(BridgeCredentials {
            credential_store: None,
            bridge_type_name: None,
            service_url: None,
            user_id_by_member: Box::new(|_| None),
            room_id_by_member: Box::new(|_| None),
            operator_user_id: None,
        })
    }
}

/// Cached GitHub App credentials for a member, used by the daemon refresh loop.
#[derive(Clone)]
pub struct AppCredentialsCached {
    pub member_name: String,
    pub client_id: String,
    pub private_key: String,
    pub installation_id: u64,
    pub workspace: PathBuf,
}

/// Resolves App credentials from keyring, exchanges JWT for installation token,
/// sets up token delivery, and writes the initial token. Returns the GH_CONFIG_DIR
/// path if App credentials were found, or None if the member has no App creds.
pub(crate) fn resolve_app_credentials_and_deliver(
    store: &dyn formation::KeyValueCredentialStore,
    formation: &dyn formation::Formation,
    member_name: &str,
    workspace: &Path,
) -> Result<Option<PathBuf>> {

    // Check if this member has App credentials
    let client_id = match store.retrieve(&credential_keys::client_id(member_name))? {
        Some(v) => v,
        None => return Ok(None), // No App creds — legacy path
    };
    let private_key = match store.retrieve(&credential_keys::private_key(member_name))? {
        Some(v) => v,
        None => return Ok(None),
    };
    let installation_id_str = match store.retrieve(&credential_keys::installation_id(member_name))? {
        Some(v) => v,
        None => return Ok(None),
    };
    let installation_id: u64 = installation_id_str
        .parse()
        .context("Invalid installation ID in credential store")?;

    // Generate JWT and exchange for installation token
    let jwt = app_auth::generate_jwt(&client_id, &private_key)
        .context("Failed to generate JWT for App authentication")?;
    let inst_token = app_auth::exchange_for_installation_token(&jwt, installation_id)
        .context("Failed to exchange JWT for installation token")?;

    // Derive bot user from numeric App ID (convention: {app-id}[bot]).
    // GitHub's canonical format is {app-slug}[bot], but the slug isn't stored
    // in the credential store. The numeric ID works for gh auth and commits;
    // the user field in hosts.yml is cosmetic — auth is driven by oauth_token.
    let app_id = store
        .retrieve(&credential_keys::app_id(member_name))?
        .unwrap_or_default();
    let bot_user = format!("{}[bot]", app_id);

    // Setup token delivery (creates GH_CONFIG_DIR + git credential helper)
    formation.setup_token_delivery(member_name, workspace, &bot_user)?;

    // Write the initial token
    formation.refresh_token(member_name, workspace, &inst_token.token)?;

    let gh_config_dir = workspace.join(".config").join("gh");
    Ok(Some(gh_config_dir))
}

/// Discover and filter member directories in the team repo.
fn discover_members(team_repo: &Path, member_filter: Option<&str>) -> Result<Vec<String>> {
    let members_dir = team_repo.join("members");
    if !members_dir.is_dir() {
        bail!("No members hired. Run `bm hire <role>` first.");
    }

    let all_member_dirs = workspace::list_member_dirs(&members_dir)?;
    if all_member_dirs.is_empty() {
        bail!("No members hired. Run `bm hire <role>` first.");
    }

    if let Some(target) = member_filter {
        if !all_member_dirs.iter().any(|d| d == target) {
            bail!(
                "Member '{}' not found. Available: {}",
                target,
                all_member_dirs.join(", ")
            );
        }
        Ok(vec![target.to_string()])
    } else {
        Ok(all_member_dirs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_result_tracks_all_outcomes() {
        let result = StartResult {
            launched: vec![MemberLaunched {
                name: "alice".to_string(),
                pid: 1234,
                brain_mode: false,
            }],
            skipped: vec![MemberSkipped {
                name: "bob".to_string(),
                pid: 5678,
            }],
            errors: vec![MemberFailed {
                name: "charlie".to_string(),
                error: "no workspace".to_string(),
            }],
            stale_cleaned: vec!["team/old-member".to_string()],
            bridge: Some(BridgeAutoStartOutcome::Started("tuwunel".to_string())),
        };

        assert_eq!(result.launched.len(), 1);
        assert_eq!(result.launched[0].name, "alice");
        assert_eq!(result.launched[0].pid, 1234);

        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].name, "bob");
        assert_eq!(result.skipped[0].pid, 5678);

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].name, "charlie");

        assert_eq!(result.stale_cleaned.len(), 1);
        assert!(result.bridge.is_some());
    }

    #[test]
    fn discover_members_filters_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        let members_dir = tmp.path().join("members");
        std::fs::create_dir_all(members_dir.join("alice")).unwrap();
        std::fs::create_dir_all(members_dir.join("bob")).unwrap();

        let result = discover_members(tmp.path(), Some("alice")).unwrap();
        assert_eq!(result, vec!["alice"]);
    }

    #[test]
    fn discover_members_returns_all_when_no_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let members_dir = tmp.path().join("members");
        std::fs::create_dir_all(members_dir.join("alice")).unwrap();
        std::fs::create_dir_all(members_dir.join("bob")).unwrap();

        let result = discover_members(tmp.path(), None).unwrap();
        assert_eq!(result, vec!["alice", "bob"]);
    }

    #[test]
    fn discover_members_errors_on_unknown_name() {
        let tmp = tempfile::tempdir().unwrap();
        let members_dir = tmp.path().join("members");
        std::fs::create_dir_all(members_dir.join("alice")).unwrap();

        let err = discover_members(tmp.path(), Some("nonexistent")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nonexistent"));
        assert!(msg.contains("alice"));
    }

    #[test]
    fn discover_members_errors_when_no_members_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let err = discover_members(tmp.path(), None).unwrap_err();
        assert!(err.to_string().contains("No members hired"));
    }
}
