use std::path::Path;
use std::process::Command;

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
}
