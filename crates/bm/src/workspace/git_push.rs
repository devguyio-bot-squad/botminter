use std::path::Path;
use std::process::Command;

use anyhow::{bail, Result};

pub const DEFAULT_MAX_RETRIES: usize = 3;

/// Attempt to push `branch` to `remote` from `repo`, retrying with a
/// fetch+rebase cycle whenever the push is rejected due to non-fast-forward.
/// Returns `Err` if the push fails for any other reason, or after
/// `max_retries` rebase+retry cycles all fail.
pub fn push_with_rebase_retry(
    repo: &Path,
    remote: &str,
    branch: &str,
    max_retries: usize,
) -> Result<()> {
    for attempt in 0..=max_retries {
        let out = Command::new("git")
            .args(["push", remote, branch])
            .current_dir(repo)
            .output()?;

        if out.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&out.stderr);
        let is_nff = stderr.contains("non-fast-forward") || stderr.contains("[rejected]");

        if !is_nff {
            bail!(
                "git push {remote}/{branch} failed (non-retryable): {}",
                stderr.trim()
            );
        }

        if attempt == max_retries {
            break;
        }

        let fetch = Command::new("git")
            .args(["fetch", remote])
            .current_dir(repo)
            .output()?;
        if !fetch.status.success() {
            bail!(
                "git fetch {remote} failed (attempt {attempt}): {}",
                String::from_utf8_lossy(&fetch.stderr).trim()
            );
        }

        let rebase_target = format!("{remote}/{branch}");
        let rebase = Command::new("git")
            .args(["rebase", &rebase_target])
            .current_dir(repo)
            .output()?;
        if !rebase.status.success() {
            bail!(
                "git rebase {rebase_target} failed during push retry (attempt {attempt}): {}",
                String::from_utf8_lossy(&rebase.stderr).trim()
            );
        }
    }

    bail!("Push failed after {max_retries} rebase+retry attempts on branch '{branch}'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    // ── fixtures ─────────────────────────────────────────────────────────────

    /// Create a bare repo seeded with one commit. Returns path to the bare repo.
    fn init_bare_repo(tmp: &TempDir, name: &str) -> PathBuf {
        let bare = tmp.path().join(format!("{name}.git"));
        fs::create_dir_all(&bare).unwrap();
        Command::new("git")
            .args(["init", "--bare", "-b", "main"])
            .arg(&bare)
            .output()
            .unwrap();

        let seed = tmp.path().join(format!("{name}-seed"));
        Command::new("git")
            .args(["clone", bare.to_str().unwrap(), seed.to_str().unwrap()])
            .output()
            .unwrap();
        fs::write(seed.join("file.txt"), "initial\n").unwrap();
        git_commit_all(&seed, "init");
        Command::new("git")
            .args(["-C", seed.to_str().unwrap(), "push"])
            .output()
            .unwrap();
        bare
    }

    /// Clone `remote_url` into `dest` under `tmp`.
    fn git_clone(tmp: &TempDir, remote_url: &str, dest: &str) -> PathBuf {
        let path = tmp.path().join(dest);
        Command::new("git")
            .args(["clone", remote_url, path.to_str().unwrap()])
            .output()
            .unwrap();
        path
    }

    /// Stage all changes and commit with a fixed author identity.
    fn git_commit_all(repo: &Path, msg: &str) {
        Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap(),
                "-c",
                "user.email=test@test.com",
                "-c",
                "user.name=Test",
                "commit",
                "-m",
                msg,
            ])
            .output()
            .unwrap();
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn push_succeeds_on_first_try() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let clone_a = git_clone(&tmp, bare.to_str().unwrap(), "clone-a");

        fs::write(clone_a.join("file.txt"), "modified by A\n").unwrap();
        git_commit_all(&clone_a, "change from A");

        let result = push_with_rebase_retry(&clone_a, "origin", "main", 3);
        assert!(
            result.is_ok(),
            "push should succeed on first try: {:?}",
            result
        );
    }

    #[test]
    fn push_rejected_nff_then_retry_succeeds() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let clone_a = git_clone(&tmp, bare.to_str().unwrap(), "clone-a");
        let clone_b = git_clone(&tmp, bare.to_str().unwrap(), "clone-b");

        // clone_a advances main and pushes — bare is now ahead of clone_b
        fs::write(clone_a.join("file.txt"), "clone_a version\n").unwrap();
        git_commit_all(&clone_a, "clone_a commit");
        Command::new("git")
            .args(["-C", clone_a.to_str().unwrap(), "push"])
            .output()
            .unwrap();

        // clone_b makes a non-conflicting commit on its old base (will be rejected)
        fs::write(clone_b.join("other.txt"), "from clone_b\n").unwrap();
        git_commit_all(&clone_b, "clone_b commit");

        // should detect rejection, fetch+rebase onto origin/main, retry → Ok
        let result = push_with_rebase_retry(&clone_b, "origin", "main", 3);
        assert!(
            result.is_ok(),
            "should succeed after rebase+retry: {:?}",
            result
        );
    }

    #[test]
    fn push_exceeds_max_retries_returns_err() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let clone_a = git_clone(&tmp, bare.to_str().unwrap(), "clone-a");
        let clone_b = git_clone(&tmp, bare.to_str().unwrap(), "clone-b");

        // advance bare past clone_b
        fs::write(clone_a.join("file.txt"), "clone_a ahead\n").unwrap();
        git_commit_all(&clone_a, "clone_a ahead");
        Command::new("git")
            .args(["-C", clone_a.to_str().unwrap(), "push"])
            .output()
            .unwrap();

        fs::write(clone_b.join("other.txt"), "clone_b work\n").unwrap();
        git_commit_all(&clone_b, "clone_b work");

        // max_retries=0: first rejection → no rebase cycles allowed → immediate Err
        let result = push_with_rebase_retry(&clone_b, "origin", "main", 0);
        let err = result.expect_err("expected Err when max_retries=0 and push rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("rebase+retry"),
            "error must mention rebase+retry attempts, got: {msg}"
        );
    }

    #[test]
    fn push_fails_with_non_rejection_error_returns_err_immediately() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let clone_a = git_clone(&tmp, bare.to_str().unwrap(), "clone-a");

        Command::new("git")
            .args([
                "-C",
                clone_a.to_str().unwrap(),
                "remote",
                "add",
                "bad-remote",
                "file:///nonexistent/missing.git",
            ])
            .output()
            .unwrap();

        fs::write(clone_a.join("file.txt"), "changed\n").unwrap();
        git_commit_all(&clone_a, "change");

        // push to a nonexistent remote: error must not be non-fast-forward
        let result = push_with_rebase_retry(&clone_a, "bad-remote", "main", 3);
        let err = result.expect_err("expected Err for nonexistent remote");
        let msg = err.to_string();
        assert!(
            !msg.contains("not implemented"),
            "must be a real git error, not the stub: {msg}"
        );
    }

    #[test]
    fn push_rebase_conflict_fails_immediately() {
        let tmp = TempDir::new().unwrap();
        let bare = init_bare_repo(&tmp, "repo");
        let clone_a = git_clone(&tmp, bare.to_str().unwrap(), "clone-a");
        let clone_b = git_clone(&tmp, bare.to_str().unwrap(), "clone-b");

        // clone_a: overwrite the same line → push → advances bare
        fs::write(clone_a.join("file.txt"), "version from A\n").unwrap();
        git_commit_all(&clone_a, "A modifies file.txt");
        Command::new("git")
            .args(["-C", clone_a.to_str().unwrap(), "push"])
            .output()
            .unwrap();

        // clone_b: overwrite the same line differently → will conflict on rebase
        fs::write(clone_b.join("file.txt"), "version from B\n").unwrap();
        git_commit_all(&clone_b, "B modifies file.txt");

        // push rejected → fetch → rebase FAILS (conflict) → Err immediately (not retry loop)
        let result = push_with_rebase_retry(&clone_b, "origin", "main", 3);
        let err = result.expect_err("expected Err when rebase conflicts");
        let msg = err.to_string();
        assert!(
            !msg.contains("not implemented"),
            "must be a real git error, not the stub: {msg}"
        );
        assert!(
            !msg.contains("rebase+retry attempts"),
            "conflict should fail immediately, not exhaust retries: {msg}"
        );
    }
}
