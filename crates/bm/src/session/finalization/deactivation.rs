use std::path::Path;

use anyhow::Result;

use super::categorize::{self, Category, RepoContext, RepoKind, TrackingStatus};
use super::subagent;
use crate::session::dirty_state::RepoDirtyState;
use crate::session::types::SessionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalizationOutcome {
    Completed,
    CompletedDegraded,
    Failed(String),
    Skipped,
}

#[derive(Debug, Clone)]
pub struct FinalizationResult {
    pub outcome: FinalizationOutcome,
    pub recovery_branches: Vec<String>,
}

impl FinalizationResult {
    pub fn new(outcome: FinalizationOutcome) -> Self {
        Self {
            outcome,
            recovery_branches: Vec::new(),
        }
    }
}

/// Determines whether any uncommitted files in the dirty state warrant
/// finalization (i.e., categorize as CommitAndPush).
pub fn has_committable_files(workspace_path: &Path, dirty_state: &[RepoDirtyState]) -> bool {
    for repo in dirty_state {
        if repo.uncommitted_files.is_empty() {
            continue;
        }

        let context = build_repo_context(workspace_path, repo);
        for file in &repo.uncommitted_files {
            let file_path = extract_porcelain_path(file);
            if categorize::categorize(Path::new(file_path), &context) == Category::CommitAndPush {
                return true;
            }
        }
    }
    false
}

/// Re-trigger finalization for a retained session by launching the finalization subagent.
///
/// Returns the spawned child so callers can attach a watcher (e.g., `wait_and_transition`).
pub fn retrigger_finalization(
    session_id: &SessionId,
    workspace_path: &Path,
) -> Result<std::process::Child> {
    subagent::retrigger_finalization(workspace_path, session_id)
}

fn build_repo_context(workspace_path: &Path, repo: &RepoDirtyState) -> RepoContext {
    let repo_kind = if repo.repo_name == "team" {
        RepoKind::Team
    } else {
        RepoKind::Project
    };

    let repo_path = if repo.repo_name == "team" {
        workspace_path.join("team")
    } else {
        workspace_path.join("projects").join(&repo.repo_name)
    };

    let current_branch = get_current_branch(&repo_path).unwrap_or_else(|| "main".to_string());

    let tracking_status = if !repo.unpushed_branches.is_empty() {
        TrackingStatus::Ahead
    } else {
        TrackingStatus::Untracked
    };

    let uncommitted_files: Vec<String> = repo
        .uncommitted_files
        .iter()
        .map(|f| extract_porcelain_path(f).to_string())
        .collect();

    RepoContext {
        repo_kind,
        current_branch,
        default_branch: "main".to_string(),
        tracking_status,
        uncommitted_files,
    }
}

fn get_current_branch(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

/// Extract the file path from a `git status --porcelain` line.
///
/// Porcelain format: `XY filename` — two status characters followed by
/// a space and the path. Returns the input unchanged when it doesn't
/// match porcelain format.
fn extract_porcelain_path(line: &str) -> &str {
    if line.len() > 3 && line.as_bytes()[2] == b' ' {
        &line[3..]
    } else {
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::session::dirty_state::RepoDirtyState;
    use crate::session::types::SessionId;

    fn dirty_repo(name: &str, uncommitted: &[&str], unpushed: &[&str]) -> RepoDirtyState {
        RepoDirtyState {
            repo_name: name.to_string(),
            uncommitted_files: uncommitted.iter().map(|s| s.to_string()).collect(),
            unpushed_branches: unpushed.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn clean_repo(name: &str) -> RepoDirtyState {
        dirty_repo(name, &[], &[])
    }

    fn git(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn setup_project_workspace(tmp: &Path, repo_name: &str, branch: &str) -> PathBuf {
        let ws = tmp.join("workspace");
        let repo = ws.join("projects").join(repo_name);
        std::fs::create_dir_all(&repo).unwrap();

        git(&repo, &["init", "-b", "main"]);
        git(&repo, &["config", "user.email", "test@test.com"]);
        git(&repo, &["config", "user.name", "Test"]);
        git(&repo, &["config", "commit.gpgsign", "false"]);
        std::fs::write(repo.join("README.md"), "init").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "initial"]);

        if branch != "main" {
            git(&repo, &["checkout", "-b", branch]);
        }

        ws
    }

    // ---
    // has_committable_files — production caller for categorize()
    // ---

    #[test]
    fn committable_files_project_on_feature_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = setup_project_workspace(tmp.path(), "myproject", "feature/story-88");
        let dirty = vec![dirty_repo("myproject", &["src/lib.rs"], &[])];

        assert!(
            has_committable_files(&ws, &dirty),
            "uncommitted file in project repo on feature branch must be committable"
        );
    }

    #[test]
    fn committable_files_project_on_default_branch_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = setup_project_workspace(tmp.path(), "myproject", "main");
        let dirty = vec![dirty_repo("myproject", &["src/lib.rs"], &[])];

        assert!(
            !has_committable_files(&ws, &dirty),
            "uncommitted file in project repo on default branch must not be committable"
        );
    }

    #[test]
    fn committable_files_team_specs() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        let dirty = vec![dirty_repo(
            "team",
            &["specs/epic-85/design.md", "knowledge/patterns.md"],
            &[],
        )];

        assert!(
            has_committable_files(&ws, &dirty),
            "uncommitted team repo files under specs/ or knowledge/ must be committable"
        );
    }

    #[test]
    fn committable_files_clean_repos_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![clean_repo("myproject")];

        assert!(
            !has_committable_files(tmp.path(), &dirty),
            "clean repos must not have committable files"
        );
    }

    #[test]
    fn committable_files_credentials_not_committable() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = setup_project_workspace(tmp.path(), "myproject", "feature/story-88");
        let dirty = vec![dirty_repo("myproject", &[".env", "token.txt"], &[])];

        assert!(
            !has_committable_files(&ws, &dirty),
            "credential files must not be committable"
        );
    }

    #[test]
    fn committable_files_unpushed_only_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let dirty = vec![dirty_repo("myproject", &[], &["feature/story-88"])];

        assert!(
            !has_committable_files(tmp.path(), &dirty),
            "repos with only unpushed branches must not have committable files"
        );
    }

    #[test]
    fn committable_files_team_member_knowledge() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        let dirty = vec![dirty_repo(
            "team",
            &["members/bob/knowledge/notes.md"],
            &[],
        )];

        assert!(
            has_committable_files(&ws, &dirty),
            "uncommitted files under members/*/knowledge/ in team repo must be committable"
        );
    }

    #[test]
    fn committable_files_porcelain_format() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = setup_project_workspace(tmp.path(), "myproject", "feature/story-88");
        let dirty = vec![dirty_repo("myproject", &[" M src/lib.rs"], &[])];

        assert!(
            has_committable_files(&ws, &dirty),
            "porcelain-format file paths must be handled correctly"
        );
    }

    // ---
    // retrigger_finalization
    // ---

    #[test]
    fn retrigger_delegates_to_subagent() {
        let session_id = SessionId::from_raw("abc12345");
        let workspace = PathBuf::from("/nonexistent/deactivation-retrigger-test");

        let result = retrigger_finalization(&session_id, &workspace);

        assert!(
            result.is_err(),
            "retrigger_finalization with non-existent workspace must fail — \
             confirms delegation to subagent (not a stub)"
        );
    }

    // ---
    // extract_porcelain_path
    // ---

    #[test]
    fn extract_porcelain_modified_file() {
        assert_eq!(extract_porcelain_path(" M src/lib.rs"), "src/lib.rs");
    }

    #[test]
    fn extract_porcelain_untracked_file() {
        assert_eq!(extract_porcelain_path("?? new_file.rs"), "new_file.rs");
    }

    #[test]
    fn extract_porcelain_added_file() {
        assert_eq!(extract_porcelain_path("A  src/new.rs"), "src/new.rs");
    }

    #[test]
    fn extract_porcelain_clean_path_unchanged() {
        assert_eq!(extract_porcelain_path("src/lib.rs"), "src/lib.rs");
    }
}
