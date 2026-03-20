use anyhow::{bail, Result};

use crate::config;
use crate::formation;
use crate::profile;

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
    let manifest = profile::validate_team_manifest(&team_repo, &team.profile)?;

    // Resolve formation
    let resolved_formation = formation::resolve_formation(&team_repo, formation_flag)?;

    // Non-local formations require current schema
    if let Some(ref fname) = resolved_formation {
        if fname != "local" {
            profile::require_current_schema(&team.name, &manifest.schema_version)?;
            let formation_cfg = formation::load(&team_repo, fname)?;
            if !formation_cfg.is_local() {
                eprintln!(
                    "Launching formation manager for '{}' formation...",
                    formation_cfg.name
                );
                let result = formation::run_formation_manager(
                    team, &team_repo, &formation_cfg, &cfg.workzone,
                )?;
                eprintln!(
                    "Formation '{}' deployed successfully.",
                    result.formation_name
                );
                return Ok(());
            }
        }
    }

    // Bridge-only mode: start bridge, skip members
    if bridge_only {
        if !no_bridge && team.bridge_lifecycle.start_on_up {
            if let Some(outcome) =
                formation::auto_start_bridge(&team_repo, &team.name, &cfg.workzone)
            {
                display_bridge_outcome(&outcome);
            }
        }
        return Ok(());
    }

    // Start local formation members
    let result = formation::start_local_members(
        team,
        &cfg,
        &team_repo,
        member_filter,
        no_bridge,
        resolved_formation.as_deref(),
    )?;

    // Display results
    if let Some(ref bridge_outcome) = result.bridge {
        display_bridge_outcome(bridge_outcome);
    }

    for s in &result.stale_cleaned {
        eprintln!("Cleaned stale entry for {}", s);
    }
    for m in &result.skipped {
        eprintln!("{}: already running (PID {})", m.name, m.pid);
    }
    for m in &result.launched {
        if m.brain_mode {
            eprintln!("{}: started brain (PID {})", m.name, m.pid);
        } else {
            eprintln!("{}: started (PID {})", m.name, m.pid);
        }
    }
    for m in &result.errors {
        eprintln!("{}: {}", m.name, m.error);
    }

    println!(
        "\nStarted {} member(s), skipped {} (already running), {} error(s).",
        result.launched.len(),
        result.skipped.len(),
        result.errors.len()
    );

    if !result.errors.is_empty() {
        bail!("Some members failed to start. See errors above.");
    }

    Ok(())
}

fn display_bridge_outcome(outcome: &formation::BridgeAutoStartOutcome) {
    match outcome {
        formation::BridgeAutoStartOutcome::Started(name) => {
            println!("Bridge '{}' started.", name);
        }
        formation::BridgeAutoStartOutcome::Restarted(name) => {
            println!("Bridge '{}' health check failed, restarted.", name);
        }
        formation::BridgeAutoStartOutcome::AlreadyRunning(name) => {
            println!("Bridge '{}' already running.", name);
        }
        formation::BridgeAutoStartOutcome::External(name) => {
            println!("Bridge '{}' is external (managed externally).", name);
        }
        formation::BridgeAutoStartOutcome::JustNotFound => {
            eprintln!(
                "Warning: 'just' not found. Skipping bridge start. \
                 Install: https://just.systems/"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::config::{self, Credentials, TeamEntry};

    // -- list_member_dirs --

    #[test]
    fn list_member_dirs_returns_sorted_dirs_only() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("bob")).unwrap();
        fs::create_dir(tmp.path().join("alice")).unwrap();
        fs::create_dir(tmp.path().join(".hidden")).unwrap();
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

    // -- find_workspace --

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
        fs::create_dir_all(tmp.path().join("member")).unwrap();

        let result = crate::workspace::find_workspace(tmp.path(), "member");
        assert_eq!(result, None);
    }

    // -- require_gh_token --

    #[test]
    fn require_gh_token_present() {
        let team = TeamEntry {
            name: "test-team".to_string(),
            path: PathBuf::from("/tmp/team"),
            profile: "scrum".to_string(),
            github_repo: "org/repo".to_string(),
            credentials: Credentials {
                gh_token: Some("ghp_test123".to_string()),
                telegram_bot_token: None,
                webhook_secret: None,
            },
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
            vm: None,
        };
        let token = config::require_gh_token(&team).unwrap();
        assert_eq!(token, "ghp_test123");
    }

    #[test]
    fn require_gh_token_missing_errors_with_team_name() {
        let team = TeamEntry {
            name: "my-team".to_string(),
            path: PathBuf::from("/tmp/team"),
            profile: "scrum".to_string(),
            github_repo: "org/repo".to_string(),
            credentials: Credentials {
                gh_token: None,
                telegram_bot_token: None,
                webhook_secret: None,
            },
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
            vm: None,
        };
        let err = config::require_gh_token(&team).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("my-team"),
            "Error should mention team name, got: {msg}"
        );
    }

    // -- Per-member credential resolution tests --

    #[test]
    fn resolve_per_member_credential_from_store() {
        use crate::bridge::{self, CredentialStore, InMemoryCredentialStore};

        let store = InMemoryCredentialStore::new();
        store.store("alice", "alice-token").unwrap();
        store.store("bob", "bob-token").unwrap();

        let alice_token = bridge::resolve_credential_from_store("alice", &store).unwrap();
        let bob_token = bridge::resolve_credential_from_store("bob", &store).unwrap();

        assert_eq!(alice_token, Some("alice-token".to_string()));
        assert_eq!(bob_token, Some("bob-token".to_string()));
    }

    #[test]
    fn resolve_per_member_credential_missing_returns_none() {
        use crate::bridge::{self, InMemoryCredentialStore};

        let store = InMemoryCredentialStore::new();

        let result = bridge::resolve_credential_from_store("charlie", &store).unwrap();
        assert!(
            result.is_none(),
            "member without credential should get None"
        );
    }

    #[test]
    fn resolve_per_member_credential_env_var_priority() {
        use crate::bridge::{self, CredentialStore, InMemoryCredentialStore};

        let store = InMemoryCredentialStore::new();
        store.store("envpritest", "store-token").unwrap();

        // Set env var -- should take priority
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
        use crate::formation;
        use anyhow::Result;

        let _: fn(&std::path::Path, &str, Option<&str>, Option<&str>, Option<&str>) -> Result<u32> =
            formation::launch_ralph;
    }

    #[test]
    fn check_robot_enabled_diagnostic() {
        use crate::formation;

        let tmp = tempfile::tempdir().unwrap();
        let ralph_yml = tmp.path().join("ralph.yml");
        fs::write(
            &ralph_yml,
            "preset: feature-development\nRObot:\n  enabled: false\n",
        )
        .unwrap();

        let has_credential = true;
        let robot_enabled = formation::check_robot_enabled_mismatch(&ralph_yml, has_credential);
        assert!(
            robot_enabled,
            "should return true when credential exists but RObot.enabled is false"
        );
    }
}
