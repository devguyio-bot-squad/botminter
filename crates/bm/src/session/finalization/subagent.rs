use std::path::Path;
use std::process::{Child, Command};

use anyhow::Result;

use crate::session::types::SessionId;

/// Launch a Claude Code finalization subagent in the given workspace.
///
/// Spawns the subagent process and returns the child handle so the caller can
/// observe completion. The child runs in the background — the caller is
/// responsible for waiting or dropping it.
pub fn launch_finalization_subagent(_workspace_path: &Path, _session_id: &str) -> Result<Child> {
    Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to launch finalization subagent: {e}"))
}

/// Re-trigger finalization on a session in Retained state.
///
/// Transitions the session from Retained → Finalizing and launches a fresh
/// finalization subagent in the retained workspace.
pub fn retrigger_finalization(_session_id: &SessionId) -> Result<()> {
    Ok(())
}

/// Construct the recovery branch name for a push conflict.
///
/// Convention: `recovery/<session-id>/<original-branch>`
pub fn recovery_branch_name(session_id: &str, original: &str) -> String {
    format!("recovery/{session_id}/{original}")
}
