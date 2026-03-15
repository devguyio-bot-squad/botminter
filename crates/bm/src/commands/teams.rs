use std::fs;

use anyhow::{Context, Result};
use comfy_table::{ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};
use serde::Deserialize;

use crate::bridge;
use crate::commands::init::run_git;
use crate::config;
use crate::profile;
use crate::workspace;

/// Minimal manifest for reading project count.
#[derive(Debug, Deserialize)]
struct TeamManifest {
    #[serde(default)]
    projects: Vec<profile::ProjectDef>,
}

/// Counts member directories under `team_repo/members/`.
fn count_members(team_repo: &std::path::Path) -> usize {
    let members_dir = team_repo.join("members");
    if !members_dir.is_dir() {
        return 0;
    }
    fs::read_dir(&members_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                        && !e
                            .file_name()
                            .to_string_lossy()
                            .starts_with('.')
                })
                .count()
        })
        .unwrap_or(0)
}

/// Reads project count from botminter.yml in the team repo.
fn read_projects(team_repo: &std::path::Path) -> Vec<profile::ProjectDef> {
    let manifest_path = team_repo.join("botminter.yml");
    if let Ok(contents) = fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_yml::from_str::<TeamManifest>(&contents) {
            return manifest.projects;
        }
    }
    Vec::new()
}

/// Handles `bm teams list` — displays a table of all registered teams.
pub fn list() -> Result<()> {
    let cfg = config::load()?;

    if cfg.teams.is_empty() {
        println!("No teams registered. Run `bm init` to create one.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Team", "Profile", "GitHub", "Members", "Projects", "Default"]);

    for team in &cfg.teams {
        let is_default = cfg.default_team.as_ref() == Some(&team.name);
        let default_marker = if is_default { "✔" } else { "" };
        let team_repo = team.path.join("team");
        let member_count = count_members(&team_repo);
        let project_count = read_projects(&team_repo).len();
        table.add_row(vec![
            team.name.as_str(),
            team.profile.as_str(),
            team.github_repo.as_str(),
            &member_count.to_string(),
            &project_count.to_string(),
            default_marker,
        ]);
    }

    println!("{table}");
    Ok(())
}

/// Handles `bm teams show [<name>] [-t team]` — displays detailed team info.
pub fn show(name: Option<&str>, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    // Resolve: positional name > -t flag > default
    let effective_flag = name.or(team_flag);
    let team = config::resolve_team(&cfg, effective_flag)?;
    let team_repo = team.path.join("team");
    let is_default = cfg.default_team.as_ref() == Some(&team.name);

    println!("Team: {}", team.name);
    println!("Profile: {}", team.profile);

    // Show profile source path (disk location)
    if let Ok(profiles_path) = profile::profiles_dir() {
        let profile_source = profiles_path.join(&team.profile);
        if profile_source.is_dir() {
            println!("Profile Source: {}", profile_source.display());
        }
    }

    // Show resolved coding agent
    let manifest_path = team_repo.join("botminter.yml");
    if let Ok(contents) = fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_yml::from_str::<profile::ProfileManifest>(&contents) {
            if let Ok(agent) = profile::resolve_coding_agent(team, &manifest) {
                println!("Coding Agent: {}", agent.display_name);
            }
        }
    }

    // Show bridge configuration
    if let Ok(Some(bridge_dir)) = bridge::discover(&team_repo, &team.name) {
        let state_path = bridge::state_path(&cfg.workzone, &team.name);
        if let Ok(b) = bridge::Bridge::new(bridge_dir, state_path, team.name.clone()) {
            println!("Bridge: {} [{}]", b.display_name(), b.bridge_type());
        }
    }

    if !team.github_repo.is_empty() {
        println!("GitHub: {}", team.github_repo);
    }
    if let Some(number) = team.project_number {
        let owner = team.github_repo.split('/').next().unwrap_or(&team.github_repo);
        println!("Board: https://github.com/orgs/{}/projects/{}", owner, number);
    }
    println!("Path: {}", team.path.display());
    println!("Default: {}", if is_default { "yes" } else { "no" });

    print!("{}", format_team_summary(&team_repo));

    Ok(())
}

/// Formats a summary of the team's members and projects, suitable for display.
///
/// Reads member directories from `team_repo/members/` and projects from `team_repo/botminter.yml`.
/// Returns a formatted string with tables (or "none" placeholders).
pub fn format_team_summary(team_repo: &std::path::Path) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    // Members section
    let members_dir = team_repo.join("members");
    let mut members: Vec<(String, String)> = Vec::new();
    if members_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&members_dir) {
            for entry in entries.flatten() {
                if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let role = read_member_role(&members_dir, &name);
                members.push((name, role));
            }
        }
    }
    members.sort_by(|a, b| a.0.cmp(&b.0));

    writeln!(out).unwrap();
    if members.is_empty() {
        writeln!(out, "Members: none").unwrap();
    } else {
        writeln!(out, "Members:").unwrap();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Name", "Role"]);
        for (name, role) in &members {
            table.add_row(vec![name.as_str(), role.as_str()]);
        }
        writeln!(out, "{table}").unwrap();
    }

    // Projects section
    let projects = read_projects(team_repo);
    writeln!(out).unwrap();
    if projects.is_empty() {
        writeln!(out, "Projects: none").unwrap();
    } else {
        writeln!(out, "Projects:").unwrap();
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec!["Name", "Fork URL"]);
        for proj in &projects {
            table.add_row(vec![proj.name.as_str(), proj.fork_url.as_str()]);
        }
        writeln!(out, "{table}").unwrap();
    }

    out
}

/// Reads the role from a member's botminter.yml, falling back to dir-name inference.
fn read_member_role(members_dir: &std::path::Path, member_dir_name: &str) -> String {
    let manifest_path = members_dir.join(member_dir_name).join("botminter.yml");
    if let Ok(contents) = fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_yml::from_str::<MemberManifest>(&contents) {
            if let Some(role) = manifest.role {
                return role;
            }
        }
    }
    member_dir_name
        .split('-')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Minimal member manifest for reading role.
#[derive(Debug, Deserialize)]
struct MemberManifest {
    #[serde(default)]
    role: Option<String>,
}

/// Handles `bm teams sync [--repos] [--bridge] [--all] [-v] [-t team]` — provisions and reconciles workspaces.
pub fn sync(repos: bool, bridge_flag: bool, verbose: bool, team_flag: Option<&str>) -> Result<()> {
    profile::ensure_profiles_initialized()?;
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Schema version guard + resolve coding agent
    let manifest_path = team_repo.join("botminter.yml");
    let manifest: profile::ProfileManifest = {
        let contents = fs::read_to_string(&manifest_path)
            .context("Failed to read team repo's botminter.yml")?;
        serde_yml::from_str(&contents).context("Failed to parse botminter.yml")?
    };
    profile::check_schema_version(&team.profile, &manifest.schema_version)?;
    let coding_agent = profile::resolve_coding_agent(team, &manifest)?;

    // Optional git push (--repos flag)
    if repos {
        run_git(&team_repo, &["push"])?;
    }

    // Bridge provisioning (--bridge flag)
    // NOTE: We discover bridge here for provisioning AND for RObot injection below.
    let bridge_dir = bridge::discover(&team_repo, &team.name)?;
    if bridge_flag {
        if let Some(ref bdir) = bridge_dir {
            // Discover members first so we can pass them to provision
            let members_dir_pb = team_repo.join("members");
            let mut bridge_members: Vec<bridge::BridgeMember> = Vec::new();
            if members_dir_pb.is_dir() {
                for entry in fs::read_dir(&members_dir_pb)? {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        bridge_members.push(bridge::BridgeMember {
                            name: entry.file_name().to_string_lossy().to_string(),
                            is_operator: false,
                        });
                    }
                }
            }
            // Add operator to bridge members (if configured and local bridge)
            if let Some(op) = manifest.operator.as_ref() {
                if !bridge_members.iter().any(|m| m.name == op.bridge_username) {
                    bridge_members.push(bridge::BridgeMember {
                        name: op.bridge_username.clone(),
                        is_operator: true,
                    });
                }
            }

            bridge_members.sort_by(|a, b| a.name.cmp(&b.name));

            let bstate_path = bridge::state_path(&cfg.workzone, &team.name);
            let mut b = bridge::Bridge::new(bdir.clone(), bstate_path.clone(), team.name.clone())?;

            // Set up credential store
            let cred_store = bridge::LocalCredentialStore::new(
                &team.name,
                b.bridge_name(),
                bstate_path,
            ).with_collection(cfg.keyring_collection.clone());

            // Auto-start local bridge if stopped (provisioning needs a running server)
            if b.is_local() && !b.is_running() {
                if which::which("just").is_err() {
                    eprintln!(
                        "Warning: 'just' not found. Cannot start bridge for provisioning. \
                         Install: https://just.systems/"
                    );
                } else {
                    println!("Starting bridge '{}' for provisioning...", b.bridge_name());
                    b.start()?;
                    b.save()?;
                }
            }

            println!("Provisioning bridge identities...");
            b.provision(&bridge_members, &cred_store)?;
            b.save()?;
        } else {
            println!("No bridge configured -- skipping bridge provisioning");
        }
    }

    // Discover hired members (scan team/members/ dir)
    let members_dir = team_repo.join("members");
    let mut members: Vec<String> = Vec::new();
    if members_dir.is_dir() {
        for entry in fs::read_dir(&members_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                members.push(name);
            }
        }
    }
    members.sort();

    if members.is_empty() {
        println!("No members hired. Run `bm hire <role>` first.");
        return Ok(());
    }

    let projects = &manifest.projects;
    let mut created = 0u32;
    let mut updated = 0u32;
    let mut failures: Vec<String> = Vec::new();

    let gh = if team.github_repo.is_empty() {
        None
    } else {
        Some(team.github_repo.as_str())
    };
    let gh_token = team.credentials.gh_token.as_deref();

    // Build project list for workspace repo creation
    let project_refs: Vec<(&str, &str)> = projects
        .iter()
        .map(|p| (p.name.as_str(), p.fork_url.as_str()))
        .collect();

    // Set up credential store and bridge for RObot injection (only if bridge is configured)
    let (robot_cred_store, robot_bridge) = if let Some(ref bdir) = bridge_dir {
        let bstate_path = bridge::state_path(&cfg.workzone, &team.name);
        let b = bridge::Bridge::new(bdir.clone(), bstate_path.clone(), team.name.clone())?;
        let store = bridge::LocalCredentialStore::new(
            &team.name,
            b.bridge_name(),
            bstate_path,
        ).with_collection(cfg.keyring_collection.clone());
        (Some(store), Some(b))
    } else {
        (None, None)
    };

    for member_dir_name in &members {
        // One workspace per member (submodule model)
        let ws = team.path.join(member_dir_name);

        if ws.join(".botminter.workspace").exists() {
            // Existing workspace — sync it
            if verbose {
                println!("Syncing workspace: {}", member_dir_name);
            }
            workspace::sync_workspace(
                &ws,
                member_dir_name,
                coding_agent,
                verbose,
                repos,
            )?;
            updated += 1;
        } else {
            // New workspace — create with submodule model
            let ws_params = workspace::WorkspaceRepoParams {
                team_repo_path: &team_repo,
                workspace_base: &team.path,
                member_dir_name,
                team_name: &team.name,
                projects: &project_refs,
                github_repo: gh,
                push: repos,
                gh_token,
                coding_agent,
            };
            match workspace::create_workspace_repo(&ws_params) {
                Ok(()) => created += 1,
                Err(e) => {
                    eprintln!("Error: {}: {:#}", member_dir_name, e);
                    failures.push(member_dir_name.clone());
                    continue;
                }
            }
        }

        // Inject RObot config into ralph.yml (only when bridge is configured)
        if let Some(ref cred_store) = robot_cred_store {
            let ralph_yml = ws.join("ralph.yml");
            if ralph_yml.exists() {
                let has_cred = bridge::resolve_credential_from_store(
                    member_dir_name,
                    cred_store,
                )?
                .is_some();

                // Build bridge config from bridge state for RC bridges
                let bridge_config = if let Some(ref b) = robot_bridge {
                    let bname = b.bridge_name();
                    if bname == "rocketchat" || bname == "tuwunel" {
                        let bot_user_id = b
                            .member_user_id(member_dir_name)
                            .unwrap_or_default();
                        let room_id = b
                            .default_room_id()
                            .unwrap_or_default()
                            .to_string();
                        let server_url = b
                            .service_url()
                            .unwrap_or_default()
                            .to_string();

                        let operator_id = manifest
                            .operator
                            .as_ref()
                            .and_then(|op| b.member_user_id(&op.bridge_username));

                        Some(workspace::RobotBridgeConfig {
                            bot_user_id,
                            room_id,
                            server_url,
                            operator_id,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                let bridge_type_name = robot_bridge.as_ref().map(|b| b.bridge_name().to_string());
                workspace::inject_robot_config(
                    &ralph_yml,
                    has_cred,
                    bridge_type_name.as_deref(),
                    bridge_config.as_ref(),
                )?;
                if verbose {
                    println!(
                        "  RObot.enabled = {} for {}",
                        has_cred, member_dir_name
                    );
                }
            }
        }
    }

    let total = created + updated;
    println!(
        "Synced {} workspace{} ({} created, {} updated)",
        total,
        if total == 1 { "" } else { "s" },
        created,
        updated,
    );

    if !failures.is_empty() {
        anyhow::bail!(
            "{} workspace(s) failed to sync:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }

    Ok(())
}
