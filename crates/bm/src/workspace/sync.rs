use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::profile::CodingAgentDef;
use super::repo::assemble_agent_dir_submodule;
use super::util::{copy_if_newer_verbose, git_cmd, git_cmd_output, git_submodule_add};

/// Events emitted during workspace sync for the caller to display.
#[derive(Debug)]
pub enum SyncEvent {
    UpdatingSubmodule(String),
    FileCopied(String),
    FileSkipped(String),
    AgentDirRebuilt,
    ChangesCommitted,
    PushedToRemote,
    NoChanges,
    BranchMigrated(String),
    ProjectProvisioned(String),
}

/// Result of a workspace sync operation.
#[derive(Debug, Default)]
pub struct SyncResult {
    /// Events that occurred during sync (for verbose display by the caller).
    pub events: Vec<SyncEvent>,
}

/// Lists member directory names from a members/ directory.
/// Returns sorted directory names, skipping hidden entries.
pub fn list_member_dirs(members_dir: &Path) -> Result<Vec<String>> {
    let mut dirs = Vec::new();
    if !members_dir.is_dir() {
        return Ok(dirs);
    }
    for entry in fs::read_dir(members_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with('.') {
            dirs.push(name);
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// Finds the workspace path for a member.
/// Returns Some if the member workspace dir exists and has the `.botminter.workspace` marker.
pub fn find_workspace(team_ws_base: &Path, member_dir_name: &str) -> Option<PathBuf> {
    let member_ws = team_ws_base.join(member_dir_name);
    if member_ws.is_dir() && member_ws.join(".botminter.workspace").exists() {
        Some(member_ws)
    } else {
        None
    }
}

/// Writes the `.botminter.workspace` marker file with workspace metadata.
pub(super) fn write_workspace_marker(ws_root: &Path, member_dir_name: &str) -> Result<()> {
    let content = format!(
        "# BotMinter workspace marker — do not delete\nmember: {}\n",
        member_dir_name,
    );
    fs::write(ws_root.join(".botminter.workspace"), content)
        .context("Failed to write .botminter.workspace marker")
}

/// Syncs an existing workspace by updating submodules, re-copying context files,
/// re-assembling agent directory, and committing+pushing any changes.
///
/// Uses the `team/` submodule model. Updates submodules to latest remote content,
/// migrates legacy member branches to main, re-copies context files when newer,
/// and rebuilds agent dir symlinks idempotently.
///
/// Returns a `SyncResult` with events describing what happened. The caller
/// decides whether and how to display these events (e.g., only in verbose mode).
pub fn sync_workspace(
    ws_root: &Path,
    member_dir_name: &str,
    coding_agent: &CodingAgentDef,
    verbose: bool,
    push: bool,
    project_number: Option<u64>,
    github_repo: Option<&str>,
    projects: &[(&str, &str)],
) -> Result<SyncResult> {
    let mut result = SyncResult::default();
    let team_dir = ws_root.join("team");

    // Update submodules to latest remote content
    if team_dir.is_dir() {
        if verbose {
            result.events.push(SyncEvent::UpdatingSubmodule("team/".to_string()));
        }
        // Fetch and update to latest remote tracking branch
        if let Err(e) = git_cmd(ws_root, &[
            "-c", "protocol.file.allow=always",
            "submodule", "update", "--remote", "--merge", "team",
        ]) {
            eprintln!("Warning: failed to update team submodule: {:#}", e);
        }

        // Migrate from member branch to main if needed
        migrate_to_main(&team_dir, &mut result)?;
    }

    // Update project submodules
    let projects_dir = ws_root.join("projects");
    if projects_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let project_name = entry.file_name().to_string_lossy().to_string();
                    let project_path = format!("projects/{}", project_name);
                    if verbose {
                        result.events.push(SyncEvent::UpdatingSubmodule(project_path.clone()));
                    }
                    if let Err(e) = git_cmd(ws_root, &[
                        "-c", "protocol.file.allow=always",
                        "submodule", "update", "--remote", "--merge", &project_path,
                    ]) {
                        eprintln!("Warning: failed to update project submodule '{}': {:#}", project_name, e);
                    }

                    // Migrate from member branch to main if needed
                    migrate_to_main(&entry.path(), &mut result)?;
                }
            }
        }
    }

    // Provision new project submodules from manifest
    if !projects.is_empty() {
        fs::create_dir_all(&projects_dir)
            .context("Failed to create projects directory")?;

        let mut provision_errors: Vec<String> = Vec::new();
        for &(project_name, fork_url) in projects {
            let project_path = projects_dir.join(project_name);
            if project_path.is_dir() {
                continue;
            }
            let submodule_path = format!("projects/{}", project_name);
            match git_submodule_add(ws_root, fork_url, &submodule_path) {
                Ok(()) => {
                    result.events.push(SyncEvent::ProjectProvisioned(project_name.to_string()));
                }
                Err(e) => {
                    let msg = format!(
                        "Failed to provision project '{}' from {}: {:#}",
                        project_name, fork_url, e
                    );
                    eprintln!("{}", msg);
                    provision_errors.push(msg);
                }
            }
        }
        if !provision_errors.is_empty() && provision_errors.len() == projects.len() {
            anyhow::bail!("All project provisioning failed:\n{}", provision_errors.join("\n"));
        }
    }

    // Re-copy context files from team/members/<member>/
    let member_src = team_dir.join("members").join(member_dir_name);
    let files_to_sync = [
        (member_src.join("ralph.yml"), ws_root.join("ralph.yml"), "ralph.yml"),
        (
            member_src.join(&coding_agent.context_file),
            ws_root.join(&coding_agent.context_file),
            coding_agent.context_file.as_str(),
        ),
        (member_src.join("PROMPT.md"), ws_root.join("PROMPT.md"), "PROMPT.md"),
    ];

    for (src, dst, name) in &files_to_sync {
        let copied = copy_if_newer_verbose(src, dst)?;
        if verbose {
            if copied {
                result.events.push(SyncEvent::FileCopied(name.to_string()));
            } else if src.exists() {
                result.events.push(SyncEvent::FileSkipped(name.to_string()));
            }
        }
    }

    // Re-copy settings.local.json if source is newer (member-level)
    let settings_src = member_src
        .join("coding-agent")
        .join("settings.local.json");
    let settings_dst = ws_root
        .join(&coding_agent.agent_dir)
        .join("settings.local.json");
    let settings_copied = copy_if_newer_verbose(&settings_src, &settings_dst)?;
    if verbose && settings_src.exists() {
        if settings_copied {
            result.events.push(SyncEvent::FileCopied("settings.local.json".to_string()));
        } else {
            result.events.push(SyncEvent::FileSkipped("settings.local.json".to_string()));
        }
    }

    // NOTE: settings.json (team-level) is copied unconditionally by
    // assemble_agent_dir_submodule below — no need to copy_if_newer here.

    // Discover project names from projects/ submodules
    let project_names: Vec<String> = if projects_dir.is_dir() {
        fs::read_dir(&projects_dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let project_name_refs: Vec<&str> = project_names.iter().map(|s| s.as_str()).collect();

    // Inject workspace context (unconditional — decoupled from copy_if_newer)
    super::context::inject_workspace_context(
        ws_root,
        member_dir_name,
        &coding_agent.context_file,
        github_repo,
        project_number,
        &project_name_refs,
    )?;

    // Inject project-aware sections into ralph.yml and context file
    super::context::inject_project_skill_dirs(
        &ws_root.join("ralph.yml"),
        &project_name_refs,
    )?;
    super::context::inject_project_sections(
        &ws_root.join(&coding_agent.context_file),
        member_dir_name,
        &project_name_refs,
    )?;

    // Re-assemble agent dir from team/ submodule paths (idempotent)
    assemble_agent_dir_submodule(ws_root, member_dir_name, &project_name_refs, coding_agent)?;
    if verbose {
        result.events.push(SyncEvent::AgentDirRebuilt);
    }

    // Commit changes if any, then push
    git_cmd(ws_root, &["add", "-A"])?;
    let has_changes = git_cmd(ws_root, &["diff", "--cached", "--quiet"]).is_err();
    if has_changes {
        git_cmd(ws_root, &["commit", "-m", "Sync workspace with team repo"])?;
        if verbose {
            result.events.push(SyncEvent::ChangesCommitted);
        }
        if push {
            git_cmd(ws_root, &["push"]).with_context(|| {
                "Failed to push workspace changes. \
                 Ensure the workspace repo has a remote configured."
            })?;
            if verbose {
                result.events.push(SyncEvent::PushedToRemote);
            }
        }
    } else if verbose {
        result.events.push(SyncEvent::NoChanges);
    }

    Ok(result)
}

/// Migrates a submodule from any non-main branch to main.
/// For legacy workspaces that used per-member branches.
fn migrate_to_main(sub_dir: &Path, result: &mut SyncResult) -> Result<()> {
    let current = git_cmd_output(sub_dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_default();
    let current = current.trim();

    if current == "main" {
        return Ok(());
    }

    let old_branch = if current != "HEAD" {
        Some(current.to_string())
    } else {
        None
    };

    if git_cmd(sub_dir, &["checkout", "main"]).is_err() {
        if git_cmd(sub_dir, &["checkout", "-b", "main", "origin/main"]).is_err() {
            eprintln!("Warning: could not checkout main in {}", sub_dir.display());
            return Ok(());
        }
    }

    if let Some(old_branch) = old_branch {
        result.events.push(SyncEvent::BranchMigrated(old_branch.clone()));
        if git_cmd(sub_dir, &["branch", "-d", &old_branch]).is_err() {
            eprintln!(
                "Warning: branch '{}' has unmerged commits in {} — preserved",
                old_branch,
                sub_dir.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::repo::tests::{
        claude_code_agent, setup_team_repo_for_ws, test_ws_params,
    };
    use crate::workspace::repo::create_workspace_repo;

    /// Helper: create a workspace using the submodule model for sync tests.
    fn setup_syncable_workspace(tmp: &Path) -> (PathBuf, String, CodingAgentDef) {
        let member = "arch-01"; // Must match setup_team_repo_for_ws member
        let team_repo = setup_team_repo_for_ws(tmp);
        let workspace_base = tmp.join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();
        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, member, &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join(member);
        (ws, member.to_string(), agent)
    }

    #[test]
    fn sync_recopies_changed_ralph_yml() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Verify initial content
        assert_eq!(
            fs::read_to_string(ws.join("ralph.yml")).unwrap(),
            "v: 1"
        );

        // Modify ralph.yml in team/ submodule (simulating upstream change)
        let source = ws.join("team/members").join(&member).join("ralph.yml");
        fs::write(&source, "updated: true").unwrap();

        // Ensure source is newer
        let now = filetime::FileTime::from_unix_time(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                + 2,
            0,
        );
        filetime::set_file_mtime(&source, now).unwrap();

        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        assert_eq!(
            fs::read_to_string(ws.join("ralph.yml")).unwrap(),
            "updated: true",
            "Sync should re-copy the updated ralph.yml"
        );
    }

    #[test]
    fn sync_reassembles_agent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        let agents_dir = ws.join(".claude/agents");
        let initial_count = fs::read_dir(&agents_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .count();

        // Add a new agent file in team/ submodule
        let member_agents = ws
            .join("team/members")
            .join(&member)
            .join("coding-agent/agents");
        fs::create_dir_all(&member_agents).unwrap();
        fs::write(member_agents.join("new-agent.md"), "# New Agent").unwrap();

        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        let new_count = fs::read_dir(&agents_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .count();
        assert!(
            new_count > initial_count,
            "Agent count should increase after sync: {} > {}",
            new_count,
            initial_count
        );
        assert!(
            agents_dir.join("new-agent.md").exists(),
            "new-agent.md should be symlinked after sync"
        );
    }

    #[test]
    fn sync_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Run sync twice
        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();
        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        // Verify workspace is still correct
        assert!(ws.join("PROMPT.md").exists());
        assert!(ws.join("CLAUDE.md").exists());
        assert!(ws.join("ralph.yml").exists());
        assert!(ws.join(".claude/agents").is_dir());
        assert_eq!(fs::read_to_string(ws.join("PROMPT.md")).unwrap(), "# P");
    }

    #[test]
    fn sync_commits_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Modify a context file in the team submodule and commit it
        // (simulating an upstream change that arrives via submodule update)
        let team_sub = ws.join("team");
        let source = team_sub.join("members").join(&member).join("ralph.yml");
        fs::write(&source, "updated: true").unwrap();
        git_cmd(&team_sub, &["add", "-A"]).unwrap();
        git_cmd(&team_sub, &["commit", "-m", "upstream change"]).unwrap();

        // Ensure source is newer than workspace copy
        let now = filetime::FileTime::from_unix_time(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                + 2,
            0,
        );
        filetime::set_file_mtime(&source, now).unwrap();

        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        // The workspace ralph.yml should have the updated content
        assert_eq!(
            fs::read_to_string(ws.join("ralph.yml")).unwrap(),
            "updated: true",
            "Sync should re-copy the updated ralph.yml"
        );

        // Working tree should be clean after sync (changes committed)
        let status = git_cmd_output(&ws, &["status", "--porcelain"]).unwrap();
        assert!(
            status.trim().is_empty(),
            "Working tree should be clean after sync, got: {}",
            status
        );
    }

    #[test]
    fn sync_skips_unchanged_files() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Sync once — no changes expected
        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        // Count commits before and after a second sync
        let log_before = git_cmd_output(&ws, &["rev-list", "--count", "HEAD"]).unwrap();

        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        let log_after = git_cmd_output(&ws, &["rev-list", "--count", "HEAD"]).unwrap();
        assert_eq!(
            log_before.trim(),
            log_after.trim(),
            "No new commits should be created when nothing changed"
        );
    }

    #[test]
    fn sync_team_submodule_on_main() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, _member, agent) = setup_syncable_workspace(tmp.path());

        // Sync — team/ submodule should end up on main (no member branches)
        sync_workspace(&ws, "arch-01", &agent, false, false, None, None, &[]).unwrap();

        let team_sub = ws.join("team");
        let branch = git_cmd_output(&team_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch.trim(),
            "main",
            "AC-01: team/ submodule should be on main after sync, not a member branch"
        );
    }

    /// Helper: create a workspace with team-level settings.json for sync tests.
    fn setup_syncable_workspace_with_settings(tmp: &Path) -> (PathBuf, String, CodingAgentDef) {
        let member = "arch-01";
        let team_repo = tmp.join("team_repo");
        let member_cfg = team_repo.join("members/arch-01");
        fs::create_dir_all(&member_cfg).unwrap();
        fs::write(member_cfg.join("PROMPT.md"), "# P").unwrap();
        fs::write(member_cfg.join("CLAUDE.md"), "# C").unwrap();
        fs::write(member_cfg.join("ralph.yml"), "v: 1").unwrap();
        fs::create_dir_all(member_cfg.join("coding-agent/agents")).unwrap();

        let team_coding_agent = team_repo.join("coding-agent");
        fs::create_dir_all(team_coding_agent.join("agents")).unwrap();
        fs::write(
            team_coding_agent.join("settings.json"),
            r#"{"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"bm-agent claude hook post-tool-use"}]}]}}"#,
        ).unwrap();

        git_cmd(&team_repo, &["init", "-b", "main"]).unwrap();
        git_cmd(&team_repo, &["config", "user.email", "test@test"]).unwrap();
        git_cmd(&team_repo, &["config", "user.name", "Test"]).unwrap();
        git_cmd(&team_repo, &["add", "-f", "-A"]).unwrap();
        git_cmd(&team_repo, &["commit", "-m", "init"]).unwrap();

        let workspace_base = tmp.join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();
        let agent = claude_code_agent();
        let params = test_ws_params(&team_repo, &workspace_base, member, &[], &agent);
        create_workspace_repo(&params).unwrap();

        let ws = workspace_base.join(member);
        (ws, member.to_string(), agent)
    }

    #[test]
    fn sync_copies_team_settings_json() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace_with_settings(tmp.path());

        // settings.json should already exist from initial creation
        assert!(ws.join(".claude/settings.json").exists());

        // Delete it and verify sync restores it
        fs::remove_file(ws.join(".claude/settings.json")).unwrap();
        assert!(!ws.join(".claude/settings.json").exists());

        // assemble_agent_dir_submodule always copies settings.json unconditionally
        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        assert!(
            ws.join(".claude/settings.json").exists(),
            "Sync should restore settings.json"
        );
        let content = fs::read_to_string(ws.join(".claude/settings.json")).unwrap();
        assert!(
            content.contains("bm-agent claude hook post-tool-use"),
            "Restored settings.json should contain hook command"
        );
    }

    #[test]
    fn sync_skips_unchanged_settings_json() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace_with_settings(tmp.path());

        // Count commits before and after sync (settings.json already up-to-date)
        let log_before = git_cmd_output(&ws, &["rev-list", "--count", "HEAD"]).unwrap();

        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();
        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        let log_after = git_cmd_output(&ws, &["rev-list", "--count", "HEAD"]).unwrap();
        assert_eq!(
            log_before.trim(),
            log_after.trim(),
            "No new commits should be created when settings.json is unchanged"
        );
    }

    #[test]
    fn sync_preserves_inbox_messages() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace_with_settings(tmp.path());

        // Create inbox file with a pending message
        let ralph_dir = ws.join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let inbox_content = r#"{"ts":"2026-03-22T12:00:00Z","from":"brain","message":"test message"}"#;
        fs::write(ralph_dir.join("loop-inbox.jsonl"), inbox_content).unwrap();

        sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        let inbox_after = fs::read_to_string(ralph_dir.join("loop-inbox.jsonl")).unwrap();
        assert_eq!(
            inbox_after.trim(),
            inbox_content,
            "Sync should not touch .ralph/loop-inbox.jsonl"
        );
    }

    #[test]
    fn sync_verbose_runs_without_error() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Verbose mode should complete without error
        sync_workspace(&ws, &member, &agent, true, false, None, None, &[]).unwrap();

        // Workspace should still be valid
        assert!(ws.join("PROMPT.md").exists());
        assert!(ws.join("CLAUDE.md").exists());
        assert!(ws.join("ralph.yml").exists());
    }

    #[test]
    fn sync_injects_workspace_context() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Add botminter.yml to the team submodule member dir
        let member_dir = ws.join("team/members").join(&member);
        fs::write(
            member_dir.join("botminter.yml"),
            "name: Arch One\nrole: architect\n",
        )
        .unwrap();

        sync_workspace(
            &ws,
            &member,
            &agent,
            false,
            false,
            Some(99),
            Some("testorg/test-team"),
            &[],
        )
        .unwrap();

        // Verify CLAUDE.md contains injected context
        let claude_content = fs::read_to_string(ws.join("CLAUDE.md")).unwrap();
        assert!(
            claude_content.contains("<!-- BM:WORKSPACE_CONTEXT -->"),
            "CLAUDE.md should contain context start marker after sync"
        );
        assert!(
            claude_content.contains("| Team repo | `testorg/test-team` |"),
            "CLAUDE.md should contain team repo after sync"
        );
        assert!(
            claude_content.contains("| Project number | `99` |"),
            "CLAUDE.md should contain project number after sync"
        );
        assert!(
            claude_content.contains("| Member | `Arch One` |"),
            "CLAUDE.md should contain member name after sync"
        );

        // Verify .botminter.workspace has KV pairs
        let marker = fs::read_to_string(ws.join(".botminter.workspace")).unwrap();
        assert!(
            marker.contains("team_repo: testorg/test-team"),
            ".botminter.workspace should contain team_repo after sync"
        );
        assert!(
            marker.contains("project_number: 99"),
            ".botminter.workspace should contain project_number after sync"
        );
    }

    #[test]
    fn sync_context_injection_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Run sync twice with context params
        sync_workspace(&ws, &member, &agent, false, false, Some(42), Some("org/repo"), &[]).unwrap();
        let claude_1 = fs::read_to_string(ws.join("CLAUDE.md")).unwrap();

        sync_workspace(&ws, &member, &agent, false, false, Some(42), Some("org/repo"), &[]).unwrap();
        let claude_2 = fs::read_to_string(ws.join("CLAUDE.md")).unwrap();

        assert_eq!(
            claude_1, claude_2,
            "Context injection should be idempotent across syncs"
        );
        // Only one set of markers
        assert_eq!(
            claude_2.matches("<!-- BM:WORKSPACE_CONTEXT -->").count(),
            1,
            "Should have exactly one start marker after multiple syncs"
        );
    }

    // ── New-project provisioning tests (Issue #33) ──────────────────

    fn setup_syncable_workspace_with_fork(tmp: &Path) -> (PathBuf, String, CodingAgentDef, PathBuf) {
        let (ws, member, agent) = setup_syncable_workspace(tmp);
        let fork = crate::workspace::repo::tests::setup_fork_repo(tmp, "new-project-fork");
        (ws, member, agent, fork)
    }

    #[test]
    fn sync_provisions_new_project_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent, fork) = setup_syncable_workspace_with_fork(tmp.path());
        let fork_url = fork.to_str().unwrap();

        sync_workspace(
            &ws, &member, &agent, false, false, None, None,
            &[("new-project", fork_url)],
        ).unwrap();

        let project_dir = ws.join("projects/new-project");
        assert!(
            project_dir.is_dir(),
            "AC-01: new project should be provisioned as submodule at projects/new-project/"
        );
        assert!(
            project_dir.join("README.md").exists(),
            "AC-01: provisioned project should contain fork content"
        );
    }

    #[test]
    fn sync_provisions_project_on_main() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent, fork) = setup_syncable_workspace_with_fork(tmp.path());
        let fork_url = fork.to_str().unwrap();

        sync_workspace(
            &ws, &member, &agent, false, false, None, None,
            &[("new-project", fork_url)],
        ).unwrap();

        let project_dir = ws.join("projects/new-project");
        assert!(
            project_dir.is_dir(),
            "project must exist before checking branch"
        );
        let branch = git_cmd_output(&project_dir, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch.trim(), "main",
            "AC-01: provisioned project submodule should be on main, not a member branch"
        );
    }

    #[test]
    fn sync_existing_projects_unaffected_by_empty_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let fork = crate::workspace::repo::tests::setup_fork_repo(tmp.path(), "existing-proj");
        let fork_url_str = fork.to_str().unwrap().to_string();
        let member = "arch-01";
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();
        let agent = claude_code_agent();
        let projects = [("existing-proj", fork_url_str.as_str())];
        let params = test_ws_params(
            &team_repo, &workspace_base, member,
            &projects, &agent,
        );
        create_workspace_repo(&params).unwrap();
        let ws = workspace_base.join(member);

        // Sync with no new projects — existing should remain intact
        sync_workspace(
            &ws, member, &agent, false, false, None, None, &[],
        ).unwrap();

        assert!(
            ws.join("projects/existing-proj").is_dir(),
            "AC-03: existing project submodule should not be affected"
        );
        assert!(
            ws.join("projects/existing-proj/README.md").exists(),
            "AC-03: existing project content should be intact"
        );
    }

    #[test]
    fn sync_project_provisioning_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent, fork) = setup_syncable_workspace_with_fork(tmp.path());
        let fork_url = fork.to_str().unwrap();

        let projects = &[("new-project", fork_url)];

        // First sync provisions it
        sync_workspace(&ws, &member, &agent, false, false, None, None, projects).unwrap();
        assert!(ws.join("projects/new-project").is_dir());

        // Second sync should not error or duplicate
        sync_workspace(&ws, &member, &agent, false, false, None, None, projects).unwrap();
        assert!(
            ws.join("projects/new-project").is_dir(),
            "AC-04: project should still exist after idempotent re-sync"
        );
    }

    #[test]
    fn sync_emits_project_provisioned_event() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent, fork) = setup_syncable_workspace_with_fork(tmp.path());
        let fork_url = fork.to_str().unwrap();

        let result = sync_workspace(
            &ws, &member, &agent, false, false, None, None,
            &[("new-project", fork_url)],
        ).unwrap();

        let provisioned = result.events.iter().any(|e| matches!(e, SyncEvent::ProjectProvisioned(name) if name == "new-project"));
        assert!(
            provisioned,
            "AC-06: SyncResult events should contain ProjectProvisioned for new-project"
        );
    }

    #[test]
    fn sync_partial_failure_first_project_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent, fork) = setup_syncable_workspace_with_fork(tmp.path());
        let fork_url = fork.to_str().unwrap();

        let result = sync_workspace(
            &ws, &member, &agent, false, false, None, None,
            &[("good-project", fork_url), ("bad-project", "/nonexistent/repo")],
        );

        // The function should not hard-fail — it should provision what it can
        assert!(
            result.is_ok(),
            "AC-07: sync should not hard-fail when one project fails; got: {:?}",
            result.err()
        );
        assert!(
            ws.join("projects/good-project").is_dir(),
            "AC-07: first project should be provisioned even when second fails"
        );
    }

    #[test]
    fn sync_provisioning_error_includes_project_name_and_url() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent, _fork) = setup_syncable_workspace_with_fork(tmp.path());

        let result = sync_workspace(
            &ws, &member, &agent, false, false, None, None,
            &[("bad-project", "/nonexistent/repo")],
        );

        // Whether the function returns Ok or Err, the error message should be actionable
        match result {
            Ok(r) => {
                // If Ok, check events for error info
                let has_error_info = format!("{:?}", r.events);
                assert!(
                    has_error_info.contains("bad-project") || has_error_info.contains("/nonexistent/repo"),
                    "AC-08: error info should include project name or URL in events; got: {}",
                    has_error_info
                );
            }
            Err(e) => {
                let msg = format!("{:#}", e);
                assert!(
                    msg.contains("bad-project"),
                    "AC-08: error should include project name 'bad-project'; got: {}",
                    msg
                );
                assert!(
                    msg.contains("/nonexistent/repo"),
                    "AC-08: error should include fork URL '/nonexistent/repo'; got: {}",
                    msg
                );
            }
        }
    }

    // ── Member branch migration tests (Issue #13) ─────────────────

    #[test]
    fn sync_migrates_member_branch_to_main() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Simulate a legacy workspace that has a member branch
        let team_sub = ws.join("team");
        git_cmd(&team_sub, &["checkout", "-b", &member]).unwrap();
        let branch_before = git_cmd_output(&team_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch_before.trim(), &member,
            "precondition: team/ should start on member branch"
        );

        // Sync should migrate to main
        let result = sync_workspace(&ws, &member, &agent, false, false, None, None, &[]).unwrap();

        // After migration, team submodule should be on main
        let branch_after = git_cmd_output(&team_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch_after.trim(), "main",
            "AC-07: team/ should be migrated to main after sync"
        );

        // Old member branch should be deleted
        let branch_list = git_cmd_output(&team_sub, &["branch", "--list", &member]).unwrap();
        assert!(
            branch_list.trim().is_empty(),
            "AC-07: old member branch '{}' should be deleted after migration",
            member
        );

        // BranchMigrated event should be emitted (unconditionally — visible without -v)
        let migrated = result.events.iter().any(|e| matches!(e, SyncEvent::BranchMigrated(name) if name == &member));
        assert!(
            migrated,
            "AC-07: SyncResult events should contain BranchMigrated for '{}'",
            member
        );
    }

    #[test]
    fn sync_migrates_project_submodule_branch_to_main() {
        let tmp = tempfile::tempdir().unwrap();
        let fork = crate::workspace::repo::tests::setup_fork_repo(tmp.path(), "migr-proj");
        let fork_url_str = fork.to_str().unwrap().to_string();
        let member = "arch-01";
        let team_repo = setup_team_repo_for_ws(tmp.path());
        let workspace_base = tmp.path().join("workzone");
        fs::create_dir_all(&workspace_base).unwrap();
        let agent = claude_code_agent();
        let projects = [("migr-proj", fork_url_str.as_str())];
        let params = test_ws_params(&team_repo, &workspace_base, member, &projects, &agent);
        create_workspace_repo(&params).unwrap();
        let ws = workspace_base.join(member);

        // Simulate a legacy workspace with member branch in project submodule
        let proj_sub = ws.join("projects/migr-proj");
        git_cmd(&proj_sub, &["checkout", "-b", member]).unwrap();
        let branch_before = git_cmd_output(&proj_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch_before.trim(), member,
            "precondition: project submodule should start on member branch"
        );

        // Sync should migrate project submodule to main
        sync_workspace(&ws, member, &agent, false, false, None, None, &[]).unwrap();

        let branch_after = git_cmd_output(&proj_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch_after.trim(), "main",
            "AC-07: project submodule should be migrated to main after sync"
        );

        // Old member branch should be deleted
        let branch_list = git_cmd_output(&proj_sub, &["branch", "--list", member]).unwrap();
        assert!(
            branch_list.trim().is_empty(),
            "AC-07: old member branch should be deleted from project submodule"
        );
    }

    #[test]
    fn sync_migration_logs_warning_on_unmerged_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let (ws, member, agent) = setup_syncable_workspace(tmp.path());

        // Simulate a legacy workspace with member branch, then create divergent commits
        let team_sub = ws.join("team");
        git_cmd(&team_sub, &["checkout", "-b", &member]).unwrap();
        fs::write(team_sub.join("divergent.txt"), "unmerged work").unwrap();
        git_cmd(&team_sub, &["add", "divergent.txt"]).unwrap();
        git_cmd(&team_sub, &["commit", "-m", "divergent commit on member branch"]).unwrap();

        // Switch to main and make a different commit so branches diverge
        git_cmd(&team_sub, &["checkout", "main"]).unwrap();
        fs::write(team_sub.join("main-only.txt"), "main work").unwrap();
        git_cmd(&team_sub, &["add", "main-only.txt"]).unwrap();
        git_cmd(&team_sub, &["commit", "-m", "main diverges"]).unwrap();

        // Switch back to member branch so sync finds it
        git_cmd(&team_sub, &["checkout", &member]).unwrap();

        // Sync should succeed (not error out) even with unmerged branch
        let result = sync_workspace(&ws, &member, &agent, false, false, None, None, &[]);
        assert!(
            result.is_ok(),
            "AC-07: sync should not fail when member branch has unmerged commits; got: {:?}",
            result.err()
        );

        // The member branch should be preserved (not deleted) since it has unmerged work
        let branch_list = git_cmd_output(&team_sub, &["branch", "--list", &member]).unwrap();
        assert!(
            !branch_list.trim().is_empty(),
            "AC-07: member branch with unmerged commits should be preserved (not forcefully deleted)"
        );

        // Submodule should still be on main after migration attempt
        let branch_after = git_cmd_output(&team_sub, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(
            branch_after.trim(), "main",
            "AC-07: team submodule should be on main even when old branch can't be deleted"
        );
    }
}
