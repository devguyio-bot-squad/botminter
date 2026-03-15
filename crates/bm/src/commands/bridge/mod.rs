mod identity;
mod lifecycle;
mod room;

use anyhow::Result;

use crate::bridge::{self, Bridge, LocalCredentialStore};
use crate::config;

pub use identity::{identity_add, identity_list, identity_remove, identity_rotate, identity_show};
pub use lifecycle::{start, status, stop};
pub use room::{room_create, room_list};

/// Common setup: load config, resolve team, check `just` is installed, discover bridge.
pub(super) struct BridgeContext {
    pub team_name: String,
    pub bridge_dir: std::path::PathBuf,
    pub workzone: std::path::PathBuf,
    pub keyring_collection: Option<String>,
}

pub(super) fn resolve_bridge(team_flag: Option<&str>) -> Result<Option<BridgeContext>> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");
    let team_name = team.name.clone();
    let workzone = cfg.workzone.clone();
    let keyring_collection = cfg.keyring_collection.clone();

    if which::which("just").is_err() {
        anyhow::bail!(
            "Bridge commands require 'just'. Install it: https://just.systems/"
        );
    }

    match bridge::discover(&team_repo, &team_name)? {
        Some(bridge_dir) => Ok(Some(BridgeContext {
            team_name,
            bridge_dir,
            workzone,
            keyring_collection,
        })),
        None => {
            println!("No bridge configured for team '{}'.", team_name);
            Ok(None)
        }
    }
}

pub(super) fn make_bridge(ctx: &BridgeContext) -> Result<Bridge> {
    let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
    Bridge::new(ctx.bridge_dir.clone(), state_path, ctx.team_name.clone())
}

pub(super) fn make_credential_store(ctx: &BridgeContext, bridge_name: &str) -> LocalCredentialStore {
    let state_path = bridge::state_path(&ctx.workzone, &ctx.team_name);
    LocalCredentialStore::new(&ctx.team_name, bridge_name, state_path)
        .with_collection(ctx.keyring_collection.clone())
}
