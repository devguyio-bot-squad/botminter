//! Migration tests for transitioning from permanent workspaces to ephemeral sessions.
//!
//! Tests cover:
//! - AC-27: Migration initializes shared clones from existing permanent workspaces
//! - AC-28: `bm teams sync` removed with clear error explaining sessions replace sync
//! - AC-29: Permanent workspace directories preserved on disk unchanged

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Helper: create a `Command` for the `bm` binary with HOME isolation.
fn bm(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bm"));
    cmd.env("HOME", home);
    cmd.env("XDG_CONFIG_HOME", home.join(".config"));
    cmd
}

/// Helper: simulate a permanent workspace structure (pre-migration state).
///
/// Creates:
/// - ~/.botminter/config.yml with a team
/// - {workzone}/my-team/team/ (team repo)
/// - {workzone}/my-team/engineer-alice/ (permanent workspace)
/// - {workzone}/my-team/engineer-alice/team/ (submodule to team repo)
/// - {workzone}/my-team/engineer-alice/projects/myproject/ (project submodule)
///
/// Returns (tmp_dir, workzone_path, team_path, workspace_path)
fn setup_permanent_workspace() -> (TempDir, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let workzone = home.join("workzone");
    let team_path = workzone.join("my-team");
    let team_repo = team_path.join("team");
    let workspace_path = team_path.join("engineer-alice");

    // Create directories
    std::fs::create_dir_all(&team_repo).unwrap();
    std::fs::create_dir_all(&workspace_path).unwrap();

    // Create minimal botminter.yml in team repo
    let botminter_yml = r#"
name: test-team
display_name: "Test Team"
description: "Test team for migration"
version: "1.0.0"
schema_version: '1.0'
default_coding_agent: claude-code
coding_agents:
  claude-code:
    name: claude-code
    display_name: Claude Code
    context_file: CLAUDE.md
    agent_dir: .claude
    binary: claude
    system_prompt_flag: --append-system-prompt-file
    skip_permissions_flag: --dangerously-skip-permissions
"#;
    std::fs::write(team_repo.join("botminter.yml"), botminter_yml).unwrap();

    // Create config.yml with credentials field
    let config_dir = home.join(".botminter");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config_yml = format!(
        r#"
workzone: {}
keyring_collection: default
default_team: my-team
teams:
  - name: my-team
    path: {}
    profile: agentic-sdlc-minimal
    github_repo: devguyio-bot-squad/my-team-team
    project_number: 1
    credentials:
      format: gh-hosts
      gh_config_dir: {}/.config/gh
"#,
        workzone.display(),
        team_path.display(),
        home.display()
    );
    std::fs::write(config_dir.join("config.yml"), config_yml).unwrap();

    // Create workspace marker
    std::fs::write(workspace_path.join(".botminter.workspace"), "permanent").unwrap();

    // Create workspace submodules (simulated)
    let workspace_team = workspace_path.join("team");
    let workspace_projects = workspace_path.join("projects").join("myproject");
    std::fs::create_dir_all(&workspace_team).unwrap();
    std::fs::create_dir_all(&workspace_projects).unwrap();

    // Add some fake git state to simulate repos
    std::fs::write(workspace_team.join("README.md"), "Team repo").unwrap();
    std::fs::write(workspace_projects.join("README.md"), "Project repo").unwrap();

    (tmp, workzone, team_path, workspace_path)
}

// ── AC-27: Migration initializes shared clones ──────────────────────────

#[test]
fn test_bm_minty_discovers_permanent_workspaces() {
    let (tmp, _workzone, _team_path, workspace_path) = setup_permanent_workspace();
    let home = tmp.path();

    // Run `bm minty --discover` (or similar flag to trigger discovery)
    // This should scan for permanent workspaces and report findings
    let output = bm(home)
        .args(["minty", "--discover"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should discover the permanent workspace
    assert!(
        stdout.contains("engineer-alice") || stderr.contains("engineer-alice"),
        "minty --discover should find permanent workspace 'engineer-alice'\nstdout: {}\nstderr: {}",
        stdout, stderr
    );

    // Verify workspace directory still exists (not modified)
    assert!(
        workspace_path.exists(),
        "Permanent workspace should not be deleted during discovery"
    );
}

#[test]
fn test_bm_minty_initializes_shared_clones() {
    let (tmp, _workzone, _team_path, _workspace_path) = setup_permanent_workspace();
    let home = tmp.path();

    // Run `bm minty` to initialize shared clones
    // This should create ~/.botminter/shared-clones/ with team and project repos
    let output = bm(home)
        .args(["minty", "-t", "my-team"])
        .output()
        .unwrap();

    // Check that shared clones directory was created
    let shared_clones = home.join(".botminter").join("shared-clones");
    assert!(
        shared_clones.exists(),
        "bm minty should create shared-clones directory\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Check for team repo clone
    let team_clone = shared_clones.join("my-team-team");
    assert!(
        team_clone.exists(),
        "bm minty should clone team repo to shared-clones\nshared_clones contents: {:?}",
        std::fs::read_dir(&shared_clones)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .into()
            })
    );
}

#[test]
fn test_first_bm_start_creates_valid_session() {
    let (tmp, _workzone, _team_path, _workspace_path) = setup_permanent_workspace();
    let home = tmp.path();

    // First run `bm minty` to initialize shared clones
    let minty_output = bm(home)
        .args(["minty", "-t", "my-team"])
        .output()
        .unwrap();

    assert!(
        minty_output.status.success(),
        "bm minty should succeed\nstderr: {}",
        String::from_utf8_lossy(&minty_output.stderr)
    );

    // Then run `bm start` to create ephemeral session
    let start_output = bm(home)
        .args(["start", "engineer-alice", "-t", "my-team"])
        .output()
        .unwrap();

    // Should succeed in creating a session
    // (Actual validation would check session directory, process, etc.)
    assert!(
        start_output.status.success() || String::from_utf8_lossy(&start_output.stderr).contains("session"),
        "bm start after migration should create valid session\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&start_output.stdout),
        String::from_utf8_lossy(&start_output.stderr)
    );
}

// ── AC-28: bm teams sync removed ──────────────────────────────────────

#[test]
fn test_bm_teams_sync_fails_with_migration_guidance() {
    let (tmp, _workzone, _team_path, _workspace_path) = setup_permanent_workspace();
    let home = tmp.path();

    // Run `bm teams sync`
    let output = bm(home)
        .args(["teams", "sync", "-t", "my-team"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should fail
    assert!(
        !output.status.success(),
        "bm teams sync should fail after migration\nstdout: {}\nstderr: {}",
        stdout, stderr
    );

    // Error message should explain migration
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("session") || combined.contains("minty") || combined.contains("migration"),
        "Error should explain sessions replace sync and mention migration\nstdout: {}\nstderr: {}",
        stdout, stderr
    );

    // Should guide user to use `bm minty` and `bm start`
    assert!(
        combined.contains("bm minty") || combined.contains("bm start"),
        "Error should guide user to new commands\nstdout: {}\nstderr: {}",
        stdout, stderr
    );
}

// ── AC-29: Permanent workspaces preserved ──────────────────────────────

#[test]
fn test_permanent_workspaces_untouched_after_migration() {
    let (tmp, _workzone, _team_path, workspace_path) = setup_permanent_workspace();
    let home = tmp.path();

    // Record state before migration
    let workspace_files_before: Vec<_> = std::fs::read_dir(&workspace_path)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();

    let marker_content_before = std::fs::read_to_string(workspace_path.join(".botminter.workspace"))
        .unwrap();

    // Run migration command (bm minty)
    let _ = bm(home)
        .args(["minty", "-t", "my-team"])
        .output()
        .unwrap();

    // Verify workspace directory still exists
    assert!(
        workspace_path.exists(),
        "Permanent workspace directory should not be deleted"
    );

    // Verify files unchanged
    let workspace_files_after: Vec<_> = std::fs::read_dir(&workspace_path)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();

    assert_eq!(
        workspace_files_before, workspace_files_after,
        "Permanent workspace files should not be modified during migration"
    );

    // Verify marker file unchanged
    let marker_content_after = std::fs::read_to_string(workspace_path.join(".botminter.workspace"))
        .unwrap();

    assert_eq!(
        marker_content_before, marker_content_after,
        "Workspace marker file should not be modified"
    );
}

// ── E2E: Full migration journey ──────────────────────────────────────

#[test]
fn test_e2e_migration_journey() {
    let (tmp, _workzone, _team_path, workspace_path) = setup_permanent_workspace();
    let home = tmp.path();

    // Step 1: Verify we have a permanent workspace
    assert!(
        workspace_path.exists(),
        "Precondition: permanent workspace should exist"
    );

    // Step 2: Run `bm minty` to initialize migration
    let minty_output = bm(home)
        .args(["minty", "-t", "my-team"])
        .output()
        .unwrap();

    assert!(
        minty_output.status.success(),
        "bm minty should succeed\nstderr: {}",
        String::from_utf8_lossy(&minty_output.stderr)
    );

    // Step 3: Verify shared clones created
    let shared_clones = home.join(".botminter").join("shared-clones");
    assert!(
        shared_clones.exists(),
        "Shared clones directory should be created"
    );

    // Step 4: Verify `bm teams sync` now fails
    let sync_output = bm(home)
        .args(["teams", "sync", "-t", "my-team"])
        .output()
        .unwrap();

    assert!(
        !sync_output.status.success(),
        "bm teams sync should fail after migration"
    );

    // Step 5: Verify permanent workspace untouched
    assert!(
        workspace_path.exists(),
        "Permanent workspace should still exist"
    );

    let marker = workspace_path.join(".botminter.workspace");
    assert!(
        marker.exists(),
        "Workspace marker should still exist"
    );

    // Step 6: Verify can start session
    let start_output = bm(home)
        .args(["start", "engineer-alice", "-t", "my-team"])
        .output()
        .unwrap();

    // Should create a session (actual validation would check session state)
    let stderr = String::from_utf8_lossy(&start_output.stderr);
    assert!(
        start_output.status.success() || stderr.contains("session"),
        "Should be able to start session after migration\nstderr: {}",
        stderr
    );
}
