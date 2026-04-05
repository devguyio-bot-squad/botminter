use anyhow::Result;

use crate::config::{BotminterConfig, TeamEntry};
use crate::formation::start_members::StartResult;
use crate::formation::stop_members::StopResult;
use crate::formation::{Formation, StartParams, StopParams};

/// Operator-facing API boundary wrapping a team entry and its formation.
///
/// Commands resolve a Team and call its methods instead of calling formation
/// free functions directly. The Team delegates to the formation trait, which
/// delegates to platform-specific implementations.
///
/// Bridge lifecycle (auto-start on `bm start`, auto-stop on `bm stop`) is
/// NOT handled here — it's a command-layer concern per ADR-0008.
pub struct Team<'a> {
    entry: &'a TeamEntry,
    formation: Box<dyn Formation>,
}

impl<'a> Team<'a> {
    pub fn new(entry: &'a TeamEntry, formation: Box<dyn Formation>) -> Self {
        Self { entry, formation }
    }

    pub fn entry(&self) -> &TeamEntry {
        self.entry
    }

    pub fn formation(&self) -> &dyn Formation {
        &*self.formation
    }

    /// Start members via the formation.
    ///
    /// Bridge auto-start is NOT handled here — it's a command-layer concern.
    /// The formation starts members only (no bridge involvement).
    pub fn start(
        &self,
        config: &BotminterConfig,
        member_filter: Option<&str>,
    ) -> Result<StartResult> {
        let team_repo = self.entry.path.join("team");
        self.formation.start_members(&StartParams {
            team: self.entry,
            config,
            team_repo: &team_repo,
            member_filter,
        })
    }

    /// Stop members via the formation.
    pub fn stop(
        &self,
        config: &BotminterConfig,
        member_filter: Option<&str>,
        force: bool,
        bridge_flag: bool,
        stop_all: bool,
    ) -> Result<StopResult> {
        self.formation.stop_members(&StopParams {
            team: self.entry,
            config,
            member_filter,
            force,
            bridge_flag,
            stop_all,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    use crate::config::{BridgeLifecycle, Credentials};
    use crate::formation::{
        CredentialDomain, EnvironmentCheck, EnvironmentStatus, KeyValueCredentialStore,
        MemberHandle, MemberStatus, SetupParams,
    };
    use crate::formation::start_members::MemberLaunched;
    use crate::formation::stop_members::MemberStopped;

    /// Mock formation that returns canned results for testing delegation.
    struct MockFormation {
        start_result: std::sync::Mutex<Option<StartResult>>,
        stop_result: std::sync::Mutex<Option<StopResult>>,
    }

    impl MockFormation {
        fn new() -> Self {
            Self {
                start_result: std::sync::Mutex::new(Some(StartResult {
                    launched: vec![MemberLaunched {
                        name: "superman".to_string(),
                        pid: 42,
                        brain_mode: false,
                    }],
                    skipped: vec![],
                    errors: vec![],
                    stale_cleaned: vec![],
                    bridge: None,
                })),
                stop_result: std::sync::Mutex::new(Some(StopResult {
                    stopped: vec![MemberStopped {
                        name: "superman".to_string(),
                        forced: false,
                        already_exited: false,
                    }],
                    errors: vec![],
                    no_members_running: false,
                    topology_removed: false,
                })),
            }
        }

        fn erroring() -> Self {
            Self {
                start_result: std::sync::Mutex::new(None),
                stop_result: std::sync::Mutex::new(None),
            }
        }
    }

    impl Formation for MockFormation {
        fn name(&self) -> &str {
            "mock"
        }

        fn setup(&self, _params: &SetupParams) -> Result<()> {
            Ok(())
        }

        fn check_environment(&self) -> Result<EnvironmentStatus> {
            Ok(EnvironmentStatus {
                ready: true,
                checks: vec![EnvironmentCheck {
                    name: "mock".to_string(),
                    passed: true,
                    detail: "mock check".to_string(),
                }],
            })
        }

        fn check_prerequisites(&self) -> Result<()> {
            Ok(())
        }

        fn credential_store(
            &self,
            _domain: CredentialDomain,
        ) -> Result<Box<dyn KeyValueCredentialStore>> {
            Ok(Box::new(crate::formation::InMemoryKeyValueCredentialStore::new()))
        }

        fn setup_token_delivery(
            &self,
            _member: &str,
            _workspace: &Path,
            _bot_user: &str,
        ) -> Result<()> {
            Ok(())
        }

        fn refresh_token(&self, _member: &str, _workspace: &Path, _token: &str) -> Result<()> {
            Ok(())
        }

        fn start_members(&self, _params: &StartParams) -> Result<StartResult> {
            self.start_result
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| anyhow::anyhow!("mock start error"))
        }

        fn stop_members(&self, _params: &StopParams) -> Result<StopResult> {
            self.stop_result
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| anyhow::anyhow!("mock stop error"))
        }

        fn member_status(&self) -> Result<Vec<MemberStatus>> {
            Ok(vec![])
        }

        fn exec_in(&self, _workspace: &Path, _cmd: &[&str]) -> Result<()> {
            Ok(())
        }

        fn shell(&self) -> Result<()> {
            Ok(())
        }

        fn write_topology(
            &self,
            _workzone: &Path,
            _team_name: &str,
            _members: &[(String, MemberHandle)],
        ) -> Result<()> {
            Ok(())
        }
    }

    fn test_team_entry() -> TeamEntry {
        TeamEntry {
            name: "test-team".to_string(),
            path: PathBuf::from("/tmp/test-team"),
            profile: "scrum-compact".to_string(),
            github_repo: "org/repo".to_string(),
            credentials: Credentials::default(),
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: BridgeLifecycle::default(),
            vm: None,
        }
    }

    fn test_config() -> BotminterConfig {
        BotminterConfig {
            workzone: PathBuf::from("/tmp/workzone"),
            default_team: None,
            teams: vec![],
            vms: vec![],
            keyring_collection: None,
        }
    }

    #[test]
    fn team_wraps_entry_and_formation() {
        let entry = test_team_entry();
        let formation = MockFormation::new();
        let team = Team::new(&entry, Box::new(formation));

        assert_eq!(team.entry().name, "test-team");
        assert_eq!(team.formation().name(), "mock");
    }

    #[test]
    fn team_start_delegates_to_formation() {
        let entry = test_team_entry();
        let formation = MockFormation::new();
        let team = Team::new(&entry, Box::new(formation));
        let config = test_config();

        let result = team.start(&config, None).unwrap();

        assert_eq!(result.launched.len(), 1);
        assert_eq!(result.launched[0].name, "superman");
        assert_eq!(result.launched[0].pid, 42);
        assert!(result.bridge.is_none(), "Team.start() should not handle bridge");
    }

    #[test]
    fn team_start_with_member_filter() {
        let entry = test_team_entry();
        let formation = MockFormation::new();
        let team = Team::new(&entry, Box::new(formation));
        let config = test_config();

        let result = team.start(&config, Some("superman")).unwrap();
        assert_eq!(result.launched.len(), 1);
    }

    #[test]
    fn team_stop_delegates_to_formation() {
        let entry = test_team_entry();
        let formation = MockFormation::new();
        let team = Team::new(&entry, Box::new(formation));
        let config = test_config();

        let result = team.stop(&config, None, false, false, false).unwrap();

        assert_eq!(result.stopped.len(), 1);
        assert_eq!(result.stopped[0].name, "superman");
        assert!(!result.stopped[0].forced);
    }

    #[test]
    fn team_stop_with_force() {
        let entry = test_team_entry();
        let formation = MockFormation::new();
        let team = Team::new(&entry, Box::new(formation));
        let config = test_config();

        // Force flag is passed through to formation
        let result = team.stop(&config, None, true, false, false).unwrap();
        assert_eq!(result.stopped.len(), 1);
    }

    #[test]
    fn team_start_propagates_formation_error() {
        let entry = test_team_entry();
        let formation = MockFormation::erroring();
        let team = Team::new(&entry, Box::new(formation));
        let config = test_config();

        let result = team.start(&config, None);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("mock start error"));
    }

    #[test]
    fn team_stop_propagates_formation_error() {
        let entry = test_team_entry();
        let formation = MockFormation::erroring();
        let team = Team::new(&entry, Box::new(formation));
        let config = test_config();

        let result = team.stop(&config, None, false, false, false);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("mock stop error"));
    }

    #[test]
    fn team_is_generic_over_formation() {
        // Verify Team works with any Formation implementation
        let entry = test_team_entry();

        // With MockFormation
        let mock = MockFormation::new();
        let team = Team::new(&entry, Box::new(mock));
        assert_eq!(team.formation().name(), "mock");

        // With InMemory credential store formation (just checking type erasure works)
        let mock2 = MockFormation::new();
        let boxed: Box<dyn Formation> = Box::new(mock2);
        let team2 = Team::new(&entry, boxed);
        assert_eq!(team2.formation().name(), "mock");
    }
}
