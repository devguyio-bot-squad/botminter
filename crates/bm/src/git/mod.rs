pub mod app_auth;
pub mod manifest_flow;
mod github;
mod project;

pub use github::{
    bootstrap_labels, clone_repo, create_github_label, create_project, create_repo_and_push,
    derive_project_name, detect_token, detect_token_non_interactive, find_project_number,
    get_user_login, list_projects, list_repos, list_user_orgs, mask_token, repo_exists,
    sync_project_status_field, validate_token, verify_fork_url, TokenInfo,
};
pub use project::{add_project, sync_project_board, ProjectSyncResult, ViewDisplay};

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Runs a git command in the given directory. Returns an error if the command fails.
pub fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("Failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_git_succeeds_on_valid_command() {
        let tmp = tempfile::tempdir().unwrap();
        run_git(tmp.path(), &["init", "-b", "main"]).unwrap();
        assert!(tmp.path().join(".git").exists());
    }

    #[test]
    fn run_git_fails_on_invalid_command() {
        let tmp = tempfile::tempdir().unwrap();
        let err = run_git(tmp.path(), &["checkout", "nonexistent-branch"]).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("git checkout nonexistent-branch failed"),
            "Error should describe the failed command, got: {msg}"
        );
    }
}
