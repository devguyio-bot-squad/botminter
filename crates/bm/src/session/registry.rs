use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a session — short random alphanumeric string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Generate a new unique session ID.
    pub fn new() -> Self {
        todo!("generate short random alphanumeric session ID")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session type — affects metadata and retention policy, not lifecycle state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    Interactive,
    Loop,
    Brain,
}

/// Session lifecycle states per the state machine in the design doc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Creating,
    Active,
    Finalizing,
    Completed,
    Failed,
    Killed,
    Retained,
}

impl SessionState {
    /// Returns true if transitioning from `self` to `next` is valid per the lifecycle state machine.
    pub fn can_transition_to(&self, next: &SessionState) -> bool {
        todo!("implement session lifecycle state machine transition table")
    }
}

/// A persistent record of a tracked session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: SessionId,
    pub member_name: String,
    pub session_type: SessionType,
    pub current_state: SessionState,
    pub created_at: DateTime<Utc>,
    pub state_transitioned_at: DateTime<Utc>,
    pub agent_pid: Option<u32>,
    pub workspace_path: Option<PathBuf>,
}

/// Persistent registry of sessions backed by an atomic JSON file on disk.
pub struct SessionRegistry {
    path: PathBuf,
    sessions: HashMap<SessionId, SessionRecord>,
}

impl SessionRegistry {
    /// Create a new empty registry that will persist to `path`.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            sessions: HashMap::new(),
        }
    }

    /// Load a registry from disk, returning an empty registry if the file does not exist.
    pub fn load(path: PathBuf) -> Result<Self> {
        todo!("deserialize registry from JSON file; return empty registry when file is absent")
    }

    /// Atomically persist the registry to disk (write to temp file, then rename).
    pub fn save(&self) -> Result<()> {
        todo!("serialize registry to a temp file beside `self.path`, then rename to `self.path`")
    }

    /// Register a new session. The record's `current_state` must be `Creating`.
    pub fn register(&mut self, record: SessionRecord) -> Result<()> {
        todo!("insert record; reject if a session with the same ID already exists")
    }

    /// Look up a session by ID.
    pub fn get(&self, id: &SessionId) -> Option<&SessionRecord> {
        todo!("return reference to session record if present")
    }

    /// Return all tracked sessions.
    pub fn list(&self) -> Vec<&SessionRecord> {
        todo!("collect all session records into a Vec")
    }

    /// Validate and apply a state transition, recording the timestamp.
    pub fn update_state(&mut self, id: &SessionId, new_state: SessionState) -> Result<()> {
        todo!("validate transition via SessionState::can_transition_to; update state and state_transitioned_at; return clear error for invalid transitions")
    }

    /// Remove a session record from the registry.
    pub fn remove(&mut self, id: &SessionId) -> Result<()> {
        todo!("remove session; return error if not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn make_record(member: &str, session_type: SessionType) -> SessionRecord {
        SessionRecord {
            session_id: SessionId::new(),
            member_name: member.to_string(),
            session_type,
            current_state: SessionState::Creating,
            created_at: Utc::now(),
            state_transitioned_at: Utc::now(),
            agent_pid: None,
            workspace_path: None,
        }
    }

    // AC-1: Session Identity — unique ID, correct initial fields
    #[test]
    fn new_session_assigns_unique_id_and_creating_state() {
        let r1 = make_record("alice", SessionType::Interactive);
        let r2 = make_record("alice", SessionType::Interactive);

        assert_ne!(r1.session_id, r2.session_id, "session IDs must be globally unique");
        assert_eq!(r1.current_state, SessionState::Creating);
        assert_eq!(r1.member_name, "alice");
    }

    #[test]
    fn session_id_is_short_alphanumeric() {
        let id = SessionId::new();
        let s = id.as_str();
        assert!(!s.is_empty(), "SessionId must not be empty");
        assert!(
            s.len() <= 16,
            "SessionId must be short (≤16 chars), got {} chars: {s}",
            s.len()
        );
        assert!(
            s.chars().all(|c| c.is_alphanumeric()),
            "SessionId must be alphanumeric, got: {s}"
        );
    }

    // AC-2: Valid State Transition — Creating → Active succeeds with timestamp recorded
    #[test]
    fn creating_can_transition_to_active() {
        assert!(
            SessionState::Creating.can_transition_to(&SessionState::Active),
            "Creating -> Active must be a valid transition"
        );
    }

    #[test]
    fn update_state_to_active_records_transition_timestamp() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("registry.json");
        let mut reg = SessionRegistry::new(path);

        let before = Utc::now();
        let record = make_record("alice", SessionType::Interactive);
        let id = record.session_id.clone();
        reg.register(record).unwrap();

        reg.update_state(&id, SessionState::Active).unwrap();

        let updated = reg.get(&id).expect("session must exist after update_state");
        assert_eq!(updated.current_state, SessionState::Active);
        assert!(
            updated.state_transitioned_at >= before,
            "state_transitioned_at must be updated on transition"
        );
    }

    // AC-3: Invalid State Transitions Rejected — Completed → Active returns error
    #[test]
    fn completed_cannot_transition_to_active() {
        assert!(
            !SessionState::Completed.can_transition_to(&SessionState::Active),
            "Completed -> Active must be an invalid transition"
        );
    }

    #[test]
    fn update_state_rejects_invalid_transition_with_descriptive_error() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("registry.json");
        let mut reg = SessionRegistry::new(path);

        let record = make_record("alice", SessionType::Interactive);
        let id = record.session_id.clone();
        reg.register(record).unwrap();

        // Walk to Completed via valid transitions
        reg.update_state(&id, SessionState::Active).unwrap();
        reg.update_state(&id, SessionState::Completed).unwrap();

        // Attempt invalid back-transition
        let err = reg
            .update_state(&id, SessionState::Active)
            .expect_err("Completed -> Active must be rejected");
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("invalid") || msg.contains("transition") || msg.contains("completed"),
            "error must describe the invalid transition, got: {msg}"
        );
    }

    // AC-4: Persistence Survives Restart — reload returns sessions with correct state and timestamps
    #[test]
    fn registry_persists_sessions_across_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("registry.json");

        let id;
        {
            let mut reg = SessionRegistry::new(path.clone());
            let record = make_record("bob", SessionType::Loop);
            id = record.session_id.clone();
            reg.register(record).unwrap();
            reg.update_state(&id, SessionState::Active).unwrap();
            reg.save().unwrap();
        }

        let reg2 = SessionRegistry::load(path).unwrap();
        let reloaded = reg2.get(&id).expect("session must survive a registry reload");
        assert_eq!(reloaded.current_state, SessionState::Active);
        assert_eq!(reloaded.member_name, "bob");
        assert_eq!(reloaded.session_type, SessionType::Loop);
        assert!(
            reloaded.state_transitioned_at >= reloaded.created_at,
            "state_transitioned_at ({:?}) must be >= created_at ({:?}) after reload",
            reloaded.state_transitioned_at,
            reloaded.created_at
        );
    }

    // AC-5: Concurrent Read Safety — two parallel loads both return consistent data
    #[test]
    fn concurrent_reads_return_consistent_data() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("registry.json");

        let mut reg = SessionRegistry::new(path.clone());
        for i in 0..5 {
            let record = make_record(&format!("member-{i}"), SessionType::Loop);
            reg.register(record).unwrap();
        }
        reg.save().unwrap();

        let p1 = path.clone();
        let p2 = path.clone();

        let t1 = thread::spawn(move || SessionRegistry::load(p1).unwrap().list().len());
        let t2 = thread::spawn(move || SessionRegistry::load(p2).unwrap().list().len());

        let count1 = t1.join().unwrap();
        let count2 = t2.join().unwrap();

        assert_eq!(count1, 5, "first concurrent read returned {count1}, expected 5");
        assert_eq!(count2, 5, "second concurrent read returned {count2}, expected 5");
    }

    // AC-6: Unified State Machine — Interactive, Loop, and Brain types all follow the same lifecycle
    #[test]
    fn all_session_types_follow_same_state_machine() {
        for session_type in [SessionType::Interactive, SessionType::Loop, SessionType::Brain] {
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("registry.json");
            let mut reg = SessionRegistry::new(path);

            let record = make_record("member", session_type.clone());
            let id = record.session_id.clone();
            reg.register(record).unwrap();

            reg.update_state(&id, SessionState::Active).unwrap_or_else(|e| {
                panic!("{session_type:?}: Creating -> Active failed: {e}")
            });

            reg.update_state(&id, SessionState::Completed)
                .unwrap_or_else(|e| {
                    panic!("{session_type:?}: Active -> Completed failed: {e}")
                });
        }
    }
}
