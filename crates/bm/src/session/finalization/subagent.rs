use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::session::types::SessionId;

pub fn build_finalization_command(
    workspace_path: &Path,
    session_id: &SessionId,
) -> Command {
    let prompt = format!(
        "Finalize session {}: inspect all repos, commit and push relevant work, \
         handle push conflicts with D-10 recovery.",
        session_id.as_str()
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
    .env("BM_SESSION_ID", session_id.as_str())
    .env_remove("CLAUDECODE");
    cmd
}

pub fn retrigger_finalization(
    workspace_path: &Path,
    session_id: &SessionId,
) -> Result<()> {
    build_finalization_command(workspace_path, session_id)
        .spawn()
        .map(drop)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    fn finalization_command() -> Command {
        let workspace = PathBuf::from("/tmp/workspace");
        let session_id = SessionId::from_raw("abc12345");
        build_finalization_command(&workspace, &session_id)
    }

    #[test]
    fn launch_finalization_subagent_spawns_claude_binary() {
        let cmd = finalization_command();

        assert_eq!(
            cmd.get_program(),
            OsStr::new("claude"),
            "finalization command must use 'claude' as executable"
        );
    }

    #[test]
    fn launch_finalization_subagent_sets_workspace_as_cwd() {
        let cmd = finalization_command();

        assert_eq!(
            cmd.get_current_dir(),
            Some(Path::new("/tmp/workspace")),
            "finalization command must set current_dir to workspace_path"
        );
    }

    #[test]
    fn launch_finalization_subagent_passes_session_id_in_env() {
        let cmd = finalization_command();

        let bm_session_id = cmd
            .get_envs()
            .find(|(k, _)| *k == OsStr::new("BM_SESSION_ID"))
            .map(|(_, v)| v);

        assert_eq!(
            bm_session_id,
            Some(Some(OsStr::new("abc12345"))),
            "finalization command must set BM_SESSION_ID env var to session ID"
        );
    }

    #[test]
    fn finalization_agent_file_exists_in_agentic_sdlc_planning_profile() {
        let profiles = crate::profile::embedded::embedded_profiles();
        let path = "agentic-sdlc-planning/coding-agent/agents/finalization.md";

        let file = profiles.get_file(path);

        assert!(
            file.is_some(),
            "finalization.md must exist at profiles/{}", path
        );
    }

    #[test]
    fn retrigger_finalization_is_not_a_stub() {
        let workspace = PathBuf::from("/nonexistent/workspace-retrigger-test");
        let session_id = SessionId::from_raw("sess-retrigger-stub-check");

        let result = retrigger_finalization(&workspace, &session_id);

        assert!(
            result.is_err(),
            "retrigger_finalization with non-existent workspace must fail — a stub returning Ok is not a real implementation"
        );
    }

    #[test]
    fn retrigger_finalization_uses_workspace_as_cwd() {
        let workspace = PathBuf::from("/tmp/workspace-retrigger");
        let session_id = SessionId::from_raw("sess-retrigger-cwd");

        let cmd = build_finalization_command(&workspace, &session_id);

        assert_eq!(
            cmd.get_current_dir(),
            Some(workspace.as_path()),
            "finalization command must set current_dir to workspace_path for retrigger"
        );
    }
}
