use anyhow::{bail, Result};

/// Handles `bm teams sync [--repos] [--bridge] [--all] [-v] [-t team]`.
///
/// This command has been removed in favor of the ephemeral session model.
/// Sessions automatically use the latest committed state — no manual synchronization needed.
pub fn sync(_repos: bool, _bridge: bool, _verbose: bool, _team_flag: Option<&str>) -> Result<()> {
    bail!(
        "bm teams sync has been removed. Sessions automatically use the latest committed state — no manual synchronization needed.\n\
         \n\
         To migrate existing workspaces:\n\
           1. Run 'bm minty -t <team>' to migrate permanent workspaces to shared clones\n\
           2. Run 'bm start <member>' to create a new ephemeral session\n\
         \n\
         The migration preserves your permanent workspaces as a rollback fallback."
    );
}
