use std::path::Path;

use anyhow::{bail, Result};

use crate::formation::{
    CredentialDomain, EnvironmentStatus, Formation, KeyValueCredentialStore, MemberHandle,
    MemberStatus, SetupParams, StartParams, StopParams,
};
use crate::formation::start_members::StartResult;
use crate::formation::stop_members::StopResult;

/// macOS local formation — not yet supported.
///
/// All methods return clear "not yet supported" errors. This exists to
/// provide a compile-time target for macOS platform detection.
pub struct MacosLocalFormation {
    _team_name: String,
}

impl MacosLocalFormation {
    pub fn new(team_name: &str) -> Self {
        Self {
            _team_name: team_name.to_string(),
        }
    }
}

impl Formation for MacosLocalFormation {
    fn name(&self) -> &str {
        "local"
    }

    fn setup(&self, _params: &SetupParams) -> Result<()> {
        bail!("macOS local formation is not yet supported")
    }

    fn check_environment(&self) -> Result<EnvironmentStatus> {
        bail!("macOS local formation is not yet supported")
    }

    fn check_prerequisites(&self) -> Result<()> {
        bail!("macOS local formation is not yet supported")
    }

    fn credential_store(
        &self,
        _domain: CredentialDomain,
    ) -> Result<Box<dyn KeyValueCredentialStore>> {
        bail!("macOS local formation is not yet supported")
    }

    fn setup_token_delivery(
        &self,
        _member: &str,
        _workspace: &Path,
        _bot_user: &str,
    ) -> Result<()> {
        bail!("macOS local formation is not yet supported")
    }

    fn refresh_token(&self, _member: &str, _workspace: &Path, _token: &str) -> Result<()> {
        bail!("macOS local formation is not yet supported")
    }

    fn start_members(&self, _params: &StartParams) -> Result<StartResult> {
        bail!("macOS local formation is not yet supported")
    }

    fn stop_members(&self, _params: &StopParams) -> Result<StopResult> {
        bail!("macOS local formation is not yet supported")
    }

    fn member_status(&self) -> Result<Vec<MemberStatus>> {
        bail!("macOS local formation is not yet supported")
    }

    fn exec_in(&self, _workspace: &Path, _cmd: &[&str]) -> Result<()> {
        bail!("macOS local formation is not yet supported")
    }

    fn shell(&self) -> Result<()> {
        bail!("macOS local formation is not yet supported")
    }

    fn write_topology(
        &self,
        _workzone: &Path,
        _team_name: &str,
        _members: &[(String, MemberHandle)],
    ) -> Result<()> {
        bail!("macOS local formation is not yet supported")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_formation_name_is_local() {
        let f = MacosLocalFormation::new("test-team");
        assert_eq!(f.name(), "local");
    }

    #[test]
    fn macos_formation_setup_returns_error() {
        let f = MacosLocalFormation::new("test-team");
        let params = SetupParams {
            coding_agent: "claude".to_string(),
            coding_agent_api_key: None,
        };
        let err = f.setup(&params).unwrap_err();
        assert!(err.to_string().contains("not yet supported"));
    }

    #[test]
    fn macos_formation_start_returns_error() {
        let f = MacosLocalFormation::new("test-team");
        // Can't construct StartParams without real refs, so test other methods
        let err = f.check_prerequisites().unwrap_err();
        assert!(err.to_string().contains("not yet supported"));
    }

    #[test]
    fn macos_formation_credential_store_returns_error() {
        let f = MacosLocalFormation::new("test-team");
        let domain = CredentialDomain::Bridge {
            team_name: "test-team".to_string(),
            bridge_name: "matrix".to_string(),
            state_path: std::path::PathBuf::from("/tmp/state.json"),
        };
        match f.credential_store(domain) {
            Ok(_) => panic!("expected error from credential_store"),
            Err(e) => assert!(e.to_string().contains("not yet supported")),
        }
    }

    #[test]
    fn macos_formation_is_object_safe() {
        let f: Box<dyn Formation> = Box::new(MacosLocalFormation::new("test-team"));
        assert_eq!(f.name(), "local");
    }
}
