use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::bridge;
use crate::config::TeamEntry;
use crate::session;
use crate::topology;

use super::FormationConfig;

/// Result of running a formation manager.
pub struct FormationManagerResult {
    pub formation_name: String,
}

/// Runs a non-local formation manager (one-shot Ralph session).
///
/// Returns a structured result on success. The caller is responsible
/// for any user-facing output (eprintln/println).
pub fn run_formation_manager(
    team: &TeamEntry,
    team_repo: &Path,
    formation_cfg: &FormationConfig,
    workzone: &Path,
) -> Result<FormationManagerResult> {
    let mgr = formation_cfg.manager.as_ref().with_context(|| {
        format!(
            "Formation '{}' has no manager configuration",
            formation_cfg.name
        )
    })?;

    let formation_dir = super::formations_dir(team_repo).join(&formation_cfg.name);
    let prompt_path = formation_dir.join(&mgr.prompt);
    let ralph_yml_path = formation_dir.join(&mgr.ralph_yml);

    // Prepare env vars
    let mut env_vars = Vec::new();
    if let Some(token) = &team.credentials.gh_token {
        env_vars.push(("GH_TOKEN".to_string(), token.clone()));
    }
    // Legacy fallback: formation manager gets team-wide token.
    // TODO: Formation manager should resolve per-member credentials via CredentialStore
    // when non-local formations support bridge integration.
    if let Some(token) = &team.credentials.telegram_bot_token {
        // Determine bridge type for correct env var dispatch
        let team_bridge_type = bridge::discover(team_repo, &team.name)
            .ok()
            .flatten()
            .and_then(|dir| bridge::load_manifest(&dir).ok())
            .map(|m| m.metadata.name.clone());

        match team_bridge_type.as_deref() {
            Some("rocketchat") => {
                env_vars.push(("RALPH_ROCKETCHAT_AUTH_TOKEN".to_string(), token.clone()));
            }
            Some("tuwunel") => {
                env_vars.push(("RALPH_MATRIX_ACCESS_TOKEN".to_string(), token.clone()));
            }
            _ => {
                env_vars.push(("RALPH_TELEGRAM_BOT_TOKEN".to_string(), token.clone()));
            }
        }
    }
    // Pass workzone and team info to formation manager
    env_vars.push(("BM_WORKZONE".to_string(), workzone.display().to_string()));
    env_vars.push(("BM_TEAM_NAME".to_string(), team.name.clone()));
    env_vars.push(("BM_TEAM_REPO".to_string(), team_repo.display().to_string()));

    let status = session::oneshot_ralph_session(
        &formation_dir,
        &prompt_path,
        &ralph_yml_path,
        &env_vars,
    )?;

    if !status.success() {
        bail!(
            "Formation manager '{}' failed (exit code: {:?})",
            formation_cfg.name,
            status.code()
        );
    }

    // Verify topology file was written
    let topo_path = topology::topology_path(workzone, &team.name);
    if !topo_path.exists() {
        bail!(
            "Formation manager completed but no topology file was written at {}",
            topo_path.display()
        );
    }

    Ok(FormationManagerResult {
        formation_name: formation_cfg.name.clone(),
    })
}
