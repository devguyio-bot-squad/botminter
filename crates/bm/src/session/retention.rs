use std::time::Duration;

use anyhow::Result;
use chrono::Utc;

use crate::session::manager::SessionManager;
use crate::session::types::{SessionState, SessionType};

pub struct RetentionConfig {
    pub loop_brain_duration: Duration,
    pub disk_budget_bytes: u64,
}

pub struct RetentionReport {
    pub removed: usize,
}

pub struct RetentionEngine {
    pub config: RetentionConfig,
    pub disk_usage: Box<dyn Fn() -> Result<u64>>,
}

pub fn retention_duration_for(session_type: &SessionType) -> Duration {
    match session_type {
        // AC-26: Interactive uses the same duration as Loop/Brain (not zero).
        SessionType::Loop | SessionType::Brain | SessionType::Interactive => {
            Duration::from_secs(24 * 3600)
        }
    }
}

impl RetentionEngine {
    pub fn run_cycle(&self, manager: &mut SessionManager) -> Result<RetentionReport> {
        let now = Utc::now();
        let mut removed = 0;

        let ids_to_expire: Vec<_> = manager
            .registry
            .list()
            .into_iter()
            .filter(|s| s.current_state == SessionState::Retained)
            .filter(|s| {
                let age = now.signed_duration_since(s.state_transitioned_at);
                age.num_seconds() >= 0
                    && (age.num_seconds() as u64) >= self.config.loop_brain_duration.as_secs()
            })
            .map(|s| s.session_id.clone())
            .collect();

        for id in ids_to_expire {
            match manager.cleanup_session(&id) {
                Ok(_) => removed += 1,
                Err(e) => {
                    tracing::warn!(session_id = %id, error = %e, "retention: cleanup failed, skipping")
                }
            }
        }

        loop {
            if (self.disk_usage)()? <= self.config.disk_budget_bytes {
                break;
            }
            let oldest_id = manager
                .registry
                .list()
                .into_iter()
                .filter(|s| s.current_state == SessionState::Retained)
                .min_by_key(|s| s.state_transitioned_at)
                .map(|s| s.session_id.clone());
            match oldest_id {
                Some(id) => match manager.cleanup_session(&id) {
                    Ok(_) => removed += 1,
                    Err(e) => {
                        // Stop the disk-budget loop if cleanup fails to avoid an infinite retry.
                        tracing::warn!(session_id = %id, error = %e, "retention: disk-budget cleanup failed, stopping cycle");
                        break;
                    }
                },
                None => break,
            }
        }

        Ok(RetentionReport { removed })
    }
}

#[cfg(test)]
mod retention_engine_tests {
    use std::time::Duration;
    use tempfile::TempDir;

    use chrono::Utc;

    use crate::session::manager::SessionManager;
    use crate::session::retention::{
        retention_duration_for, RetentionConfig, RetentionEngine, RetentionReport,
    };
    use crate::session::types::{SessionId, SessionRecord, SessionState, SessionType};

    fn make_manager(tmp: &TempDir) -> SessionManager {
        let registry_path = tmp.path().join("registry.json");
        SessionManager::new(tmp.path().to_path_buf(), registry_path).unwrap()
    }

    fn add_retained_loop_session(
        manager: &mut SessionManager,
        member: &str,
        transitioned_secs_ago: i64,
        workspace: Option<std::path::PathBuf>,
    ) -> SessionId {
        let id = SessionId::new();
        let record = SessionRecord {
            session_id: id.clone(),
            member_name: member.to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Retained,
            created_at: Utc::now() - chrono::Duration::seconds(transitioned_secs_ago + 10),
            state_transitioned_at: Utc::now() - chrono::Duration::seconds(transitioned_secs_ago),
            agent_pid: None,
            workspace_path: workspace,
            finalization_result: None,
        };
        manager.registry.register(record).unwrap();
        id
    }

    fn add_retained_brain_session(
        manager: &mut SessionManager,
        member: &str,
        transitioned_secs_ago: i64,
        workspace: Option<std::path::PathBuf>,
    ) -> SessionId {
        let id = SessionId::new();
        let record = SessionRecord {
            session_id: id.clone(),
            member_name: member.to_string(),
            session_type: SessionType::Brain,
            current_state: SessionState::Retained,
            created_at: Utc::now() - chrono::Duration::seconds(transitioned_secs_ago + 10),
            state_transitioned_at: Utc::now() - chrono::Duration::seconds(transitioned_secs_ago),
            agent_pid: None,
            workspace_path: workspace,
            finalization_result: None,
        };
        manager.registry.register(record).unwrap();
        id
    }

    fn add_retained_interactive_session(
        manager: &mut SessionManager,
        member: &str,
        transitioned_secs_ago: i64,
        workspace: Option<std::path::PathBuf>,
    ) -> SessionId {
        let id = SessionId::new();
        let record = SessionRecord {
            session_id: id.clone(),
            member_name: member.to_string(),
            session_type: SessionType::Interactive,
            current_state: SessionState::Retained,
            created_at: Utc::now() - chrono::Duration::seconds(transitioned_secs_ago + 10),
            state_transitioned_at: Utc::now() - chrono::Duration::seconds(transitioned_secs_ago),
            agent_pid: None,
            workspace_path: workspace,
            finalization_result: None,
        };
        manager.registry.register(record).unwrap();
        id
    }

    fn no_op_disk_usage() -> Box<dyn Fn() -> anyhow::Result<u64>> {
        Box::new(|| Ok(0))
    }

    fn fixed_disk_usage(bytes: u64) -> Box<dyn Fn() -> anyhow::Result<u64>> {
        Box::new(move || Ok(bytes))
    }

    // AC-20: Loop session within retention duration is NOT removed.
    #[test]
    fn retention_engine_does_not_remove_unexpired_session() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join("ws/alice");
        std::fs::create_dir_all(&ws).unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_retained_loop_session(&mut manager, "alice", 100, Some(ws.clone()));
        manager.registry.save().unwrap();

        let engine = RetentionEngine {
            config: RetentionConfig {
                loop_brain_duration: Duration::from_secs(3600),
                disk_budget_bytes: u64::MAX,
            },
            disk_usage: no_op_disk_usage(),
        };

        let report: RetentionReport = engine.run_cycle(&mut manager).unwrap();

        assert_eq!(report.removed, 0, "unexpired session must not be removed");
        assert!(
            manager.registry.get(&id).is_some(),
            "session must still be in registry"
        );
        assert!(ws.exists(), "workspace must still exist");
    }

    // AC-20: Loop session past retention duration IS removed.
    #[test]
    fn retention_engine_removes_expired_loop_session() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join("ws/alice");
        std::fs::create_dir_all(&ws).unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_retained_loop_session(&mut manager, "alice", 7200, Some(ws.clone()));
        manager.registry.save().unwrap();

        let engine = RetentionEngine {
            config: RetentionConfig {
                loop_brain_duration: Duration::from_secs(3600),
                disk_budget_bytes: u64::MAX,
            },
            disk_usage: no_op_disk_usage(),
        };

        let report: RetentionReport = engine.run_cycle(&mut manager).unwrap();

        assert_eq!(report.removed, 1, "expired loop session must be removed");
        assert!(
            manager.registry.get(&id).is_none(),
            "expired session must be removed from registry"
        );
        assert!(!ws.exists(), "workspace must be removed");
    }

    // AC-20: Brain session past retention duration IS removed.
    #[test]
    fn retention_engine_removes_expired_brain_session() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join("ws/brain-bob");
        std::fs::create_dir_all(&ws).unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_retained_brain_session(&mut manager, "bob", 7200, Some(ws.clone()));
        manager.registry.save().unwrap();

        let engine = RetentionEngine {
            config: RetentionConfig {
                loop_brain_duration: Duration::from_secs(3600),
                disk_budget_bytes: u64::MAX,
            },
            disk_usage: no_op_disk_usage(),
        };

        let report: RetentionReport = engine.run_cycle(&mut manager).unwrap();

        assert_eq!(report.removed, 1, "expired brain session must be removed");
        assert!(manager.registry.get(&id).is_none());
        assert!(!ws.exists());
    }

    // AC-21: Disk budget exceeded → oldest session removed first.
    #[test]
    fn retention_engine_removes_oldest_when_disk_budget_exceeded() {
        let tmp = TempDir::new().unwrap();
        let ws_old = tmp.path().join("ws/old");
        let ws_new = tmp.path().join("ws/new");
        for p in [&ws_old, &ws_new] {
            std::fs::create_dir_all(p).unwrap();
        }
        let mut manager = make_manager(&tmp);
        // old: transitioned 2h ago; new: 10m ago
        let id_old = add_retained_loop_session(&mut manager, "alice", 7200, Some(ws_old.clone()));
        let id_new = add_retained_loop_session(&mut manager, "bob", 600, Some(ws_new.clone()));
        manager.registry.save().unwrap();

        // disk usage exceeds budget; duration large so nothing expires by time
        let engine = RetentionEngine {
            config: RetentionConfig {
                loop_brain_duration: Duration::from_secs(7 * 24 * 3600),
                disk_budget_bytes: 1,
            },
            disk_usage: fixed_disk_usage(1024 * 1024 * 1024), // 1 GiB > 1 byte
        };

        let report: RetentionReport = engine.run_cycle(&mut manager).unwrap();

        assert!(report.removed >= 1, "at least one session must be removed");
        assert!(
            manager.registry.get(&id_old).is_none(),
            "oldest session must be removed first"
        );
        assert!(!ws_old.exists(), "oldest workspace must be removed");
        // new session may or may not be removed depending on whether 1 removal satisfied budget
        let _ = id_new;
    }

    // AC-21: Oldest-first ordering — 3 sessions, budget only needs 1 removed → removes oldest.
    #[test]
    fn retention_engine_oldest_first_ordering() {
        let tmp = TempDir::new().unwrap();
        let ws1 = tmp.path().join("ws/old");
        let ws2 = tmp.path().join("ws/mid");
        let ws3 = tmp.path().join("ws/new");
        for p in [&ws1, &ws2, &ws3] {
            std::fs::create_dir_all(p).unwrap();
        }
        let mut manager = make_manager(&tmp);
        let id_old = add_retained_loop_session(&mut manager, "a", 10000, Some(ws1.clone()));
        let id_mid = add_retained_loop_session(&mut manager, "b", 5000, Some(ws2.clone()));
        let id_new = add_retained_loop_session(&mut manager, "c", 100, Some(ws3.clone()));
        manager.registry.save().unwrap();

        // disk_usage function: first call returns over-budget, second call returns under-budget
        // This simulates: after removing 1 session the budget constraint is satisfied.
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cc = call_count.clone();
        let disk_usage: Box<dyn Fn() -> anyhow::Result<u64>> = Box::new(move || {
            let n = cc.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if n == 0 {
                Ok(2_000_000_000) // first check: over budget
            } else {
                Ok(0) // subsequent checks: under budget
            }
        });

        let engine = RetentionEngine {
            config: RetentionConfig {
                loop_brain_duration: Duration::from_secs(7 * 24 * 3600),
                disk_budget_bytes: 1_000_000_000,
            },
            disk_usage,
        };

        let report: RetentionReport = engine.run_cycle(&mut manager).unwrap();

        assert_eq!(
            report.removed, 1,
            "only 1 session removed (budget satisfied after one)"
        );
        assert!(
            manager.registry.get(&id_old).is_none(),
            "oldest session (id_old) must be the one removed"
        );
        assert!(
            manager.registry.get(&id_mid).is_some(),
            "mid session must remain"
        );
        assert!(
            manager.registry.get(&id_new).is_some(),
            "newest session must remain"
        );
        let _ = (id_mid, id_new);
    }

    // AC-20: Sessions NOT in Retained state are ignored by the GC.
    #[test]
    fn retention_engine_ignores_non_retained_sessions() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join("ws/active");
        std::fs::create_dir_all(&ws).unwrap();
        let mut manager = make_manager(&tmp);

        // Add an Active session (not Retained)
        let id = SessionId::new();
        let record = crate::session::types::SessionRecord {
            session_id: id.clone(),
            member_name: "alice".to_string(),
            session_type: SessionType::Loop,
            current_state: SessionState::Active,
            created_at: Utc::now() - chrono::Duration::hours(48),
            state_transitioned_at: Utc::now() - chrono::Duration::hours(48),
            agent_pid: Some(99999),
            workspace_path: Some(ws.clone()),
            finalization_result: None,
        };
        manager.registry.register(record).unwrap();
        manager.registry.save().unwrap();

        let engine = RetentionEngine {
            config: RetentionConfig {
                loop_brain_duration: Duration::from_secs(1), // tiny duration
                disk_budget_bytes: u64::MAX,
            },
            disk_usage: no_op_disk_usage(),
        };

        let report: RetentionReport = engine.run_cycle(&mut manager).unwrap();

        assert_eq!(
            report.removed, 0,
            "Active (non-Retained) session must not be touched by GC"
        );
        assert!(manager.registry.get(&id).is_some());
    }

    // AC-26: retention_duration_for(Interactive) equals retention_duration_for(Loop).
    // Failed interactive sessions use loop/brain duration, not zero.
    #[test]
    fn retention_duration_for_interactive_equals_loop_brain_duration() {
        let loop_dur = retention_duration_for(&SessionType::Loop);
        let brain_dur = retention_duration_for(&SessionType::Brain);
        let interactive_dur = retention_duration_for(&SessionType::Interactive);

        assert_eq!(
            loop_dur, brain_dur,
            "Loop and Brain must use the same retention duration"
        );
        assert_eq!(
            interactive_dur, loop_dur,
            "Interactive must use the same duration as Loop/Brain (AC-26: not zero)"
        );
        assert!(
            interactive_dur > Duration::ZERO,
            "Interactive retention duration must be non-zero (AC-26)"
        );
    }

    // AC-26: Interactive sessions are managed by the retention engine (not excluded).
    #[test]
    fn retention_engine_removes_expired_interactive_session() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join("ws/interactive");
        std::fs::create_dir_all(&ws).unwrap();
        let mut manager = make_manager(&tmp);
        let id = add_retained_interactive_session(&mut manager, "carol", 7200, Some(ws.clone()));
        manager.registry.save().unwrap();

        let engine = RetentionEngine {
            config: RetentionConfig {
                loop_brain_duration: Duration::from_secs(3600),
                disk_budget_bytes: u64::MAX,
            },
            disk_usage: no_op_disk_usage(),
        };

        let report: RetentionReport = engine.run_cycle(&mut manager).unwrap();

        assert_eq!(
            report.removed, 1,
            "expired interactive session must be removed (AC-26: uses loop/brain duration)"
        );
        assert!(manager.registry.get(&id).is_none());
        assert!(!ws.exists());
    }
}
