use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::brain;
use crate::bridge::{self, Bridge, LocalCredentialStore};
use crate::profile::{self, CodingAgentDef, ProfileManifest};
use crate::workspace;
use crate::workspace::GhRemoteOps;

// ── Sync parameters ─────────────────────────────────────────────────

/// All parameters needed for `bm teams sync`.
pub struct TeamSyncParams<'a> {
    pub team_repo: &'a Path,
    pub team_path: &'a Path,
    pub team_name: &'a str,
    pub manifest: &'a ProfileManifest,
    pub coding_agent: &'a CodingAgentDef,
    pub github_repo: Option<&'a str>,
    pub project_number: Option<u64>,
    pub repos: bool,
    pub verbose: bool,
    pub bridge_flag: bool,
    pub workzone: &'a Path,
    pub keyring_collection: Option<String>,
}

// ── Sync result ─────────────────────────────────────────────────────

/// Result of syncing all team workspaces.
pub struct TeamSyncResult {
    pub created: u32,
    pub updated: u32,
    pub failures: Vec<String>,
    pub events: Vec<TeamSyncEvent>,
}

/// Events emitted during team sync, for the command layer to display.
pub enum TeamSyncEvent {
    NoMembers,
    GitPush,
    NoBridge,
    BridgeAutoStart { name: String, result: bridge::BridgeStartResult },
    BridgeAutoStartSkipped { reason: String },
    BridgeProvisionMember { name: String, result: bridge::ProvisionMemberResult },
    BridgeRoomCreated(String),
    BridgeSaved,
    WorkspaceCreated(String),
    WorkspaceSynced { name: String, events: Vec<workspace::SyncEvent> },
    WorkspaceCreateFailed { name: String, error: String },
    WorkspaceSyncFailed { name: String, error: String },
    RobotInjected { member: String, enabled: bool },
    BrainPromptSurfaced { member: String },
}

// ── Sync orchestration ──────────────────────────────────────────────

/// Orchestrates the full `bm teams sync` operation: git push, bridge
/// provisioning, workspace creation/sync, and RObot config injection.
///
/// Returns a structured result with counts and events for display.
pub fn sync_team_workspaces(params: &TeamSyncParams) -> Result<TeamSyncResult> {
    let mut events = Vec::new();

    // Optional git push (--repos flag)
    if params.repos {
        crate::git::run_git(params.team_repo, &["push"])?;
        events.push(TeamSyncEvent::GitPush);
    }

    // Bridge provisioning (--bridge flag)
    let bridge_dir = bridge::discover(params.team_repo, params.team_name)?;
    if params.bridge_flag {
        provision_bridge(params, &bridge_dir, &mut events)?;
    }

    // Discover hired members
    let members = profile::discover_member_dirs(params.team_repo);
    if members.is_empty() {
        events.push(TeamSyncEvent::NoMembers);
        return Ok(TeamSyncResult {
            created: 0,
            updated: 0,
            failures: Vec::new(),
            events,
        });
    }

    // Build project list for workspace repo creation
    let project_refs: Vec<(&str, &str)> = params
        .manifest
        .projects
        .iter()
        .map(|p| (p.name.as_str(), p.fork_url.as_str()))
        .collect();

    // Set up bridge context for RObot injection
    let robot_context = build_robot_context(params, &bridge_dir)?;

    // Build remote ops for push mode
    let gh_ops = if params.repos {
        crate::git::detect_token().map(|token| GhRemoteOps {
            gh_token: token,
        })
    } else {
        None
    };

    let mut created = 0u32;
    let mut updated = 0u32;
    let mut failures: Vec<String> = Vec::new();

    for member_dir_name in &members {
        let ws = params.team_path.join(member_dir_name);

        // Clean up stale local dir without a workspace marker
        if ws.exists() && !ws.join(".botminter.workspace").exists() {
            fs::remove_dir_all(&ws).ok();
        }

        if ws.join(".botminter.workspace").exists() {
            // Existing workspace — sync it
            let sync_result = workspace::sync_workspace(
                &ws,
                member_dir_name,
                params.coding_agent,
                params.verbose,
                params.repos,
                params.project_number,
                params.github_repo,
                &project_refs,
            )?;
            events.push(TeamSyncEvent::WorkspaceSynced {
                name: member_dir_name.clone(),
                events: sync_result.events,
            });
            updated += 1;
        } else {
            // New workspace — create
            let ws_params = workspace::WorkspaceRepoParams {
                team_repo_path: params.team_repo,
                workspace_base: params.team_path,
                member_dir_name,
                team_name: params.team_name,
                projects: &project_refs,
                github_repo: params.github_repo,
                project_number: params.project_number,
                push: params.repos,
                coding_agent: params.coding_agent,
                remote_ops: gh_ops.as_ref().map(|o| o as &dyn workspace::RemoteRepoOps),
                team_submodule_url: None,
            };
            match workspace::create_workspace_repo(&ws_params) {
                Ok(()) => {
                    events.push(TeamSyncEvent::WorkspaceCreated(member_dir_name.clone()));
                    created += 1;
                }
                Err(e) => {
                    events.push(TeamSyncEvent::WorkspaceCreateFailed {
                        name: member_dir_name.clone(),
                        error: format!("{:#}", e),
                    });
                    failures.push(member_dir_name.clone());
                    continue;
                }
            }
        }

        // Inject RObot config
        if let Some(ref ctx) = robot_context {
            inject_robot_for_member(&ws, member_dir_name, ctx, params, &mut events)?;
        }

        // Surface brain prompt (rendered from profile template)
        surface_brain_prompt_for_member(
            params.team_repo,
            &ws,
            member_dir_name,
            params.team_name,
            params.github_repo,
            params.verbose,
            &mut events,
        );
    }

    Ok(TeamSyncResult {
        created,
        updated,
        failures,
        events,
    })
}

// ── Bridge provisioning ─────────────────────────────────────────────

fn provision_bridge(
    params: &TeamSyncParams,
    bridge_dir: &Option<PathBuf>,
    events: &mut Vec<TeamSyncEvent>,
) -> Result<()> {
    let bdir = match bridge_dir {
        Some(d) => d,
        None => {
            events.push(TeamSyncEvent::NoBridge);
            return Ok(());
        }
    };

    // Discover members for bridge provisioning
    let members_dir = params.team_repo.join("members");
    let mut bridge_members: Vec<bridge::BridgeMember> = Vec::new();
    if members_dir.is_dir() {
        for entry in fs::read_dir(&members_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                bridge_members.push(bridge::BridgeMember {
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_operator: false,
                });
            }
        }
    }

    // Add operator to bridge members
    if let Some(op) = params.manifest.operator.as_ref() {
        if !bridge_members.iter().any(|m| m.name == op.bridge_username) {
            bridge_members.push(bridge::BridgeMember {
                name: op.bridge_username.clone(),
                is_operator: true,
            });
        }
    }
    bridge_members.sort_by(|a, b| a.name.cmp(&b.name));

    let bstate_path = bridge::state_path(params.workzone, params.team_name);
    let mut b = Bridge::new(bdir.clone(), bstate_path.clone(), params.team_name.to_string())?;

    let cred_store = LocalCredentialStore::new(
        params.team_name,
        b.bridge_name(),
        bstate_path,
    )
    .with_collection(params.keyring_collection.clone());

    // Ensure local bridge is running before provisioning.
    // Always call start() — it's idempotent: health-checks first and
    // returns AlreadyRunning if healthy, or restarts if the container
    // died (e.g., after VM reboot while state still says "running").
    if b.is_local() {
        if which::which("just").is_err() {
            events.push(TeamSyncEvent::BridgeAutoStartSkipped {
                reason: "'just' not found. Cannot start bridge for provisioning. \
                         Install: https://just.systems/"
                    .to_string(),
            });
        } else {
            let result = b.start()?;
            events.push(TeamSyncEvent::BridgeAutoStart {
                name: b.bridge_name().to_string(),
                result,
            });
            b.save()?;
        }
    }

    // Provision identities
    let provision_result = b.provision(&bridge_members, &cred_store)?;
    for (name, member_result) in &provision_result.members {
        events.push(TeamSyncEvent::BridgeProvisionMember {
            name: name.clone(),
            result: member_result.clone(),
        });
    }
    if let Some(room_name) = &provision_result.room_created {
        events.push(TeamSyncEvent::BridgeRoomCreated(room_name.clone()));
    }
    b.save()?;
    events.push(TeamSyncEvent::BridgeSaved);

    Ok(())
}

// ── RObot injection ─────────────────────────────────────────────────

/// Pre-computed bridge context for RObot injection.
struct RobotContext {
    cred_store: LocalCredentialStore,
    bridge: Bridge,
}

fn build_robot_context(
    params: &TeamSyncParams,
    bridge_dir: &Option<PathBuf>,
) -> Result<Option<RobotContext>> {
    let bdir = match bridge_dir {
        Some(d) => d,
        None => return Ok(None),
    };
    let bstate_path = bridge::state_path(params.workzone, params.team_name);
    let b = Bridge::new(bdir.clone(), bstate_path.clone(), params.team_name.to_string())?;
    let store = LocalCredentialStore::new(params.team_name, b.bridge_name(), bstate_path)
        .with_collection(params.keyring_collection.clone());
    Ok(Some(RobotContext {
        cred_store: store,
        bridge: b,
    }))
}

fn surface_brain_prompt_for_member(
    team_repo: &Path,
    ws: &Path,
    member_dir_name: &str,
    team_name: &str,
    github_repo: Option<&str>,
    verbose: bool,
    events: &mut Vec<TeamSyncEvent>,
) {
    let (gh_org, gh_repo_name) = github_repo
        .and_then(brain::parse_github_repo)
        .map(|(org, repo)| (org.to_string(), repo.to_string()))
        .unwrap_or_default();

    let role = brain::read_member_role(team_repo, member_dir_name)
        .unwrap_or_default();
    let member_name = brain::read_member_name(team_repo, member_dir_name);

    // Read the template from the team repo root (extracted from profile)
    let vars = brain::BrainPromptVars {
        member_name,
        team_name: team_name.to_string(),
        role,
        gh_org,
        gh_repo: gh_repo_name,
    };

    match brain::surface_brain_prompt(team_repo, ws, &vars) {
        Ok(true) => {
            if verbose {
                events.push(TeamSyncEvent::BrainPromptSurfaced {
                    member: member_dir_name.to_string(),
                });
            }
        }
        Ok(false) => {} // No template in profile — skip silently
        Err(e) => {
            tracing::warn!(
                member = member_dir_name,
                error = %e,
                "Failed to surface brain prompt"
            );
        }
    }
}

fn inject_robot_for_member(
    ws: &Path,
    member_dir_name: &str,
    ctx: &RobotContext,
    params: &TeamSyncParams,
    events: &mut Vec<TeamSyncEvent>,
) -> Result<()> {
    let ralph_yml = ws.join("ralph.yml");
    if !ralph_yml.exists() {
        return Ok(());
    }

    let has_cred = bridge::resolve_credential_from_store(member_dir_name, &ctx.cred_store)?
        .is_some();

    // Build bridge config for RC/tuwunel bridges
    let bridge_config = {
        let bname = ctx.bridge.bridge_name();
        if bname == "rocketchat" || bname == "tuwunel" {
            let bot_user_id = ctx
                .bridge
                .member_user_id(member_dir_name)
                .unwrap_or_default();
            let room_id = ctx
                .bridge
                .default_room_id()
                .unwrap_or_default()
                .to_string();
            let server_url = ctx
                .bridge
                .service_url()
                .unwrap_or_default()
                .to_string();
            let operator_id = params
                .manifest
                .operator
                .as_ref()
                .and_then(|op| ctx.bridge.member_user_id(&op.bridge_username));

            Some(workspace::RobotBridgeConfig {
                bot_user_id,
                room_id,
                server_url,
                operator_id,
            })
        } else {
            None
        }
    };

    let bridge_type_name = Some(ctx.bridge.bridge_name().to_string());
    workspace::inject_robot_config(
        &ralph_yml,
        has_cred,
        bridge_type_name.as_deref(),
        bridge_config.as_ref(),
    )?;

    if params.verbose {
        events.push(TeamSyncEvent::RobotInjected {
            member: member_dir_name.to_string(),
            enabled: has_cred,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::git_cmd;
    use crate::workspace::repo::tests::{claude_code_agent, test_ws_params};
    use crate::workspace::repo::create_workspace_repo;
    use std::collections::HashMap;

    fn setup_team_repo_two_members(tmp: &Path) -> PathBuf {
        let team_repo = tmp.join("team_repo");
        for m in &["arch-01", "arch-02"] {
            let member_cfg = team_repo.join("members").join(m);
            fs::create_dir_all(&member_cfg).unwrap();
            fs::write(member_cfg.join("PROMPT.md"), "# P").unwrap();
            fs::write(member_cfg.join("CLAUDE.md"), "# C").unwrap();
            fs::write(member_cfg.join("ralph.yml"), "v: 1").unwrap();
            fs::create_dir_all(member_cfg.join("coding-agent/agents")).unwrap();
        }
        fs::create_dir_all(team_repo.join("coding-agent/agents")).unwrap();

        git_cmd(&team_repo, &["init", "-b", "main"]).unwrap();
        git_cmd(&team_repo, &["config", "user.email", "test@test"]).unwrap();
        git_cmd(&team_repo, &["config", "user.name", "Test"]).unwrap();
        git_cmd(&team_repo, &["add", "-f", "-A"]).unwrap();
        git_cmd(&team_repo, &["commit", "-m", "init"]).unwrap();

        team_repo
    }

    fn empty_manifest() -> profile::ProfileManifest {
        profile::ProfileManifest {
            name: "test".into(),
            display_name: "Test".into(),
            description: "test".into(),
            version: "1.0".into(),
            schema_version: "1.0".into(),
            roles: Vec::new(),
            labels: Vec::new(),
            statuses: Vec::new(),
            projects: Vec::new(),
            views: Vec::new(),
            coding_agents: HashMap::new(),
            default_coding_agent: String::new(),
            bridges: Vec::new(),
            bridge: None,
            operator: None,
            meetings: Vec::new(),
        }
    }

    #[test]
    fn sync_all_continues_after_workspace_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_two_members(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();
        let agent = claude_code_agent();

        for member in &["arch-01", "arch-02"] {
            let params = test_ws_params(&team_repo, &workspace_base, member, &[], &agent);
            create_workspace_repo(&params).unwrap();
        }

        // Sabotage arch-01: corrupt git state so sync_workspace fails at git add
        let ws1 = workspace_base.join("arch-01");
        fs::remove_file(ws1.join(".git/HEAD")).unwrap();

        let manifest = empty_manifest();
        let params = TeamSyncParams {
            team_repo: &team_repo,
            team_path: &workspace_base,
            team_name: "my-team",
            manifest: &manifest,
            coding_agent: &agent,
            github_repo: None,
            project_number: None,
            repos: false,
            verbose: false,
            bridge_flag: false,
            workzone: tmp.path(),
            keyring_collection: None,
        };

        // AC-05: sync should succeed overall despite one workspace failure
        let result = sync_team_workspaces(&params).unwrap();

        // AC-05: second workspace should still be synced
        assert!(
            result.updated >= 1,
            "at least one workspace should be synced, got updated={}",
            result.updated
        );

        // AC-05: failed workspace should appear in failures list
        assert!(
            result.failures.contains(&"arch-01".to_string()),
            "arch-01 should appear in failures: {:?}",
            result.failures
        );

        // AC-06: WorkspaceSyncFailed event emitted with member name and error
        let sync_failed = result.events.iter().find(|e| {
            matches!(e, TeamSyncEvent::WorkspaceSyncFailed { name, .. } if name == "arch-01")
        });
        assert!(
            sync_failed.is_some(),
            "WorkspaceSyncFailed event should be emitted for arch-01"
        );
    }

    #[test]
    fn sync_all_push_failure_non_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        let team_repo = setup_team_repo_two_members(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();
        let agent = claude_code_agent();

        for member in &["arch-01", "arch-02"] {
            let params = test_ws_params(&team_repo, &workspace_base, member, &[], &agent);
            create_workspace_repo(&params).unwrap();
        }

        let ws1 = workspace_base.join("arch-01");
        let ws2 = workspace_base.join("arch-02");

        // Set up bare remote for team repo (needed for repos: true)
        let bare_team = tmp.path().join("bare-team.git");
        fs::create_dir_all(&bare_team).unwrap();
        git_cmd(&bare_team, &["init", "--bare"]).unwrap();
        git_cmd(&team_repo, &["remote", "add", "origin", bare_team.to_str().unwrap()]).unwrap();
        git_cmd(&team_repo, &["push", "-u", "origin", "main"]).unwrap();

        // Set up valid bare remote for workspace 2
        let bare_ws2 = tmp.path().join("bare-ws2.git");
        fs::create_dir_all(&bare_ws2).unwrap();
        git_cmd(&bare_ws2, &["init", "--bare"]).unwrap();
        git_cmd(&ws2, &["remote", "add", "origin", bare_ws2.to_str().unwrap()]).unwrap();
        git_cmd(&ws2, &["push", "-u", "origin", "main"]).unwrap();

        // Set workspace 1's remote to unreachable path (push will fail).
        // Configure upstream tracking so `git push` attempts the remote
        // rather than failing with "no upstream branch" error.
        git_cmd(&ws1, &["remote", "add", "origin", "/nonexistent/bad/path"]).unwrap();
        git_cmd(&ws1, &["config", "branch.main.remote", "origin"]).unwrap();
        git_cmd(&ws1, &["config", "branch.main.merge", "refs/heads/main"]).unwrap();

        // Modify team repo so sync detects changes and triggers commit+push
        fs::write(team_repo.join("members/arch-01/ralph.yml"), "v: 2").unwrap();
        fs::write(team_repo.join("members/arch-02/ralph.yml"), "v: 2").unwrap();
        git_cmd(&team_repo, &["add", "-A"]).unwrap();
        git_cmd(&team_repo, &["commit", "-m", "update configs"]).unwrap();
        git_cmd(&team_repo, &["push"]).unwrap();

        let manifest = empty_manifest();
        let params = TeamSyncParams {
            team_repo: &team_repo,
            team_path: &workspace_base,
            team_name: "my-team",
            manifest: &manifest,
            coding_agent: &agent,
            github_repo: None,
            project_number: None,
            repos: true,
            verbose: false,
            bridge_flag: false,
            workzone: tmp.path(),
            keyring_collection: None,
        };

        // AC-05: sync should succeed overall despite push failure on one workspace
        let result = sync_team_workspaces(&params).unwrap();

        // AC-05: second workspace should be synced
        assert!(
            result.updated >= 1,
            "at least one workspace should be synced, got updated={}",
            result.updated
        );

        // AC-05: workspace with push failure should be in failures list
        assert!(
            result.failures.contains(&"arch-01".to_string()),
            "arch-01 should appear in failures: {:?}",
            result.failures
        );

        // AC-06: WorkspaceSyncFailed event emitted with member name
        let sync_failed = result.events.iter().find(|e| {
            matches!(e, TeamSyncEvent::WorkspaceSyncFailed { name, .. } if name == "arch-01")
        });
        assert!(
            sync_failed.is_some(),
            "WorkspaceSyncFailed event should be emitted for arch-01"
        );
    }
}
