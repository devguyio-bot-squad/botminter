use std::path::Path;
use std::process::{Child, Command};

use anyhow::Result;

use crate::session::types::SessionId;

/// Build the Command for the finalization subagent without spawning.
/// Exposed for unit testing so callers can inspect program, CWD, and env vars.
pub(crate) fn build_finalization_command(workspace_path: &Path, session_id: &str) -> Command {
    let prompt = format!(
        "Finalize session {session_id}: inspect all repos, commit and push relevant work, handle push conflicts with D-10 recovery."
    );
    let mut cmd = Command::new("claude");
    cmd.args([
        "--dangerously-skip-permissions",
        "--agent",
        "finalization",
        "-p",
        &prompt,
    ])
    .current_dir(workspace_path)
    .env("BM_SESSION_ID", session_id)
    .env_remove("CLAUDECODE");
    cmd
}

/// Launch a Claude Code finalization subagent in the given workspace.
///
/// Spawns the subagent process and returns the child handle so the caller can
/// observe completion. The child runs in the background — the caller is
/// responsible for waiting or dropping it.
pub fn launch_finalization_subagent(workspace_path: &Path, session_id: &str) -> Result<Child> {
    build_finalization_command(workspace_path, session_id)
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to launch finalization subagent: {e}"))
}

/// Re-trigger finalization on a session in Retained state.
///
/// Launches a fresh finalization subagent in the retained workspace.
pub fn retrigger_finalization(workspace_path: &Path, session_id: &SessionId) -> Result<()> {
    launch_finalization_subagent(workspace_path, session_id.as_str()).map(drop)
}

/// Construct the recovery branch name for a push conflict.
///
/// Convention: `recovery/<session-id>/<original-branch>`
pub fn recovery_branch_name(session_id: &str, original: &str) -> String {
    format!("recovery/{session_id}/{original}")
}

#[cfg(test)]
mod finalization_subagent_tests {
    use std::ffi::OsStr;
    use std::path::PathBuf;

    use crate::session::types::SessionId;

    use super::*;

    #[test]
    fn launch_finalization_subagent_spawns_claude_binary() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let cmd = build_finalization_command(&workspace, "sess-01");
        assert_eq!(
            cmd.get_program(),
            OsStr::new("claude"),
            "launch_finalization_subagent must spawn 'claude', got '{:?}'",
            cmd.get_program()
        );
    }

    #[test]
    fn launch_finalization_subagent_sets_workspace_as_cwd() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let cmd = build_finalization_command(&workspace, "sess-01");
        assert_eq!(
            cmd.get_current_dir(),
            Some(workspace.as_path()),
            "command current_dir must be workspace_path"
        );
    }

    #[test]
    fn launch_finalization_subagent_passes_session_id_in_env() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let session_id = "sess-abc-123";
        let cmd = build_finalization_command(&workspace, session_id);
        let value = cmd
            .get_envs()
            .find(|(k, _)| *k == OsStr::new("BM_SESSION_ID"))
            .and_then(|(_, v)| v)
            .map(|v| v.to_string_lossy().into_owned());
        assert_eq!(
            value.as_deref(),
            Some(session_id),
            "BM_SESSION_ID env var must be set to session_id"
        );
    }

    #[test]
    fn finalization_agent_file_exists_in_agentic_sdlc_planning_profile() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let agent_file = manifest_dir
            .join("../../profiles/agentic-sdlc-planning/coding-agent/agents/finalization.md");
        assert!(
            agent_file.exists(),
            "profiles/agentic-sdlc-planning/coding-agent/agents/finalization.md must exist"
        );
    }

    #[test]
    fn retrigger_finalization_is_not_a_stub() {
        let workspace = PathBuf::from("/nonexistent/workspace-retrigger-test");
        let session_id = SessionId::from_string("sess-retrigger-stub-check".to_string());
        let result = retrigger_finalization(&workspace, &session_id);
        assert!(
            result.is_err(),
            "retrigger_finalization must attempt to spawn a process, not return Ok(()) as a stub; got Ok(())"
        );
    }
}
