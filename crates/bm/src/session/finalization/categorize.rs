use std::path::{Path, PathBuf};

/// Classification of a workspace file for finalization purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    /// Commit this file and push the branch (project non-default branch, or team repo knowledge paths).
    CommitAndPush,
    /// Branch already has committed work that needs to be pushed.
    PushOnly,
    /// Never commit — credential files and auth tokens.
    NeverCommit,
    /// Leave in workspace for retention (logs, locks, runtime state, diagnostics).
    LeaveInPlace,
}

/// What kind of repo a workspace repo is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoKind {
    Project,
    Team,
}

/// Push/fetch relationship between local branch and remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteStatus {
    Ahead,
    Behind,
    Diverged,
    UpToDate,
    NoRemote,
}

/// Context about a repo needed to categorize files within it.
#[derive(Debug, Clone)]
pub struct RepoContext {
    pub repo_path: PathBuf,
    pub repo_kind: RepoKind,
    pub current_branch: String,
    pub default_branch: String,
    pub remote_status: RemoteStatus,
    /// Relative paths of files with uncommitted changes in this repo.
    pub uncommitted_files: Vec<PathBuf>,
}

/// Classify `file_path` (relative to repo root) according to finalization categorization rules.
pub fn categorize(_file_path: &Path, _context: &RepoContext) -> Category {
    todo!("categorize not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_ctx(
        branch: &str,
        default: &str,
        status: RemoteStatus,
        uncommitted: Vec<PathBuf>,
    ) -> RepoContext {
        RepoContext {
            repo_path: PathBuf::from("/workspace/projects/myapp"),
            repo_kind: RepoKind::Project,
            current_branch: branch.to_string(),
            default_branch: default.to_string(),
            remote_status: status,
            uncommitted_files: uncommitted,
        }
    }

    fn team_ctx(
        branch: &str,
        default: &str,
        status: RemoteStatus,
        uncommitted: Vec<PathBuf>,
    ) -> RepoContext {
        RepoContext {
            repo_path: PathBuf::from("/workspace/team"),
            repo_kind: RepoKind::Team,
            current_branch: branch.to_string(),
            default_branch: default.to_string(),
            remote_status: status,
            uncommitted_files: uncommitted,
        }
    }

    #[test]
    fn project_repo_uncommitted_on_non_default_branch_is_commit_and_push() {
        let file = PathBuf::from("src/main.rs");
        let ctx = project_ctx(
            "feature-branch",
            "main",
            RemoteStatus::Ahead,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx), Category::CommitAndPush);
    }

    #[test]
    fn team_repo_specs_file_is_commit_and_push() {
        let file = PathBuf::from("specs/botminter/design.md");
        let ctx = team_ctx(
            "main",
            "main",
            RemoteStatus::UpToDate,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx), Category::CommitAndPush);
    }

    #[test]
    fn team_repo_knowledge_file_is_commit_and_push() {
        let file = PathBuf::from("knowledge/patterns.md");
        let ctx = team_ctx(
            "main",
            "main",
            RemoteStatus::UpToDate,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx), Category::CommitAndPush);
    }

    #[test]
    fn team_repo_members_knowledge_file_is_commit_and_push() {
        let file = PathBuf::from("members/engineer-bob/knowledge/habits.md");
        let ctx = team_ctx(
            "main",
            "main",
            RemoteStatus::UpToDate,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx), Category::CommitAndPush);
    }

    #[test]
    fn push_only_for_committed_unpushed_branch() {
        let file = PathBuf::from("src/lib.rs");

        // Case A: uncommitted file on non-default branch with ahead commits → CommitAndPush
        let ctx_non_default = project_ctx(
            "feature-branch",
            "main",
            RemoteStatus::Ahead,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx_non_default), Category::CommitAndPush);

        // Case B: committed file (not in uncommitted_files), default branch, Ahead → PushOnly
        let ctx_default = project_ctx("main", "main", RemoteStatus::Ahead, vec![]);
        assert_eq!(categorize(&file, &ctx_default), Category::PushOnly);
    }

    #[test]
    fn credential_path_gh_config_is_never_commit() {
        let file = PathBuf::from(".config/gh/hosts.yml");
        let ctx = project_ctx(
            "main",
            "main",
            RemoteStatus::UpToDate,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx), Category::NeverCommit);
    }

    #[test]
    fn credential_path_env_file_is_never_commit() {
        let file = PathBuf::from(".env");
        let ctx = project_ctx(
            "main",
            "main",
            RemoteStatus::UpToDate,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx), Category::NeverCommit);
    }

    #[test]
    fn runtime_artifact_log_is_leave_in_place() {
        let file = PathBuf::from(".ralph/diagnostics/logs/ralph-2026.log");
        let ctx = project_ctx("main", "main", RemoteStatus::UpToDate, vec![]);
        assert_eq!(categorize(&file, &ctx), Category::LeaveInPlace);
    }

    #[test]
    fn runtime_artifact_tasks_jsonl_is_leave_in_place() {
        let file = PathBuf::from(".ralph/agent/tasks.jsonl");
        let ctx = project_ctx("main", "main", RemoteStatus::UpToDate, vec![]);
        assert_eq!(categorize(&file, &ctx), Category::LeaveInPlace);
    }

    #[test]
    fn never_commit_takes_precedence_over_commit_and_push() {
        // .env under members/*/knowledge/ — credential rule overrides team repo knowledge rule
        let file = PathBuf::from("members/engineer-bob/knowledge/.env");
        let ctx = team_ctx(
            "main",
            "main",
            RemoteStatus::UpToDate,
            vec![file.clone()],
        );
        assert_eq!(categorize(&file, &ctx), Category::NeverCommit);
    }
}
