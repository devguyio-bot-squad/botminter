use std::path::Path;
use std::process::Child;

use anyhow::Result;

use crate::session::types::SessionId;

/// Launch a Claude Code finalization subagent in the given workspace.
///
/// Returns the child process so Session Management can observe completion.
pub fn launch_finalization_subagent(_workspace_path: &Path, _session_id: &str) -> Result<Child> {
    anyhow::bail!("launch_finalization_subagent: not yet implemented")
}

/// Re-trigger finalization on a session in Retained state.
///
/// Transitions the session from Retained → Finalizing and launches a fresh
/// finalization subagent in the retained workspace.
pub fn retrigger_finalization(_session_id: &SessionId) -> Result<()> {
    anyhow::bail!("retrigger_finalization: not yet implemented")
}

/// Construct the recovery branch name for a push conflict.
///
/// Convention: `recovery/<session-id>/<original-branch>`
pub fn recovery_branch_name(_session_id: &str, _original: &str) -> String {
    String::new()
}
