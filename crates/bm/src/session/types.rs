use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a session — short random alphanumeric string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Generate a new unique session ID.
    pub fn new() -> Self {
        let full = Uuid::new_v4().simple().to_string();
        Self(full[..8].to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_raw(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
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

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SessionType::Interactive => "Interactive",
            SessionType::Loop => "Loop",
            SessionType::Brain => "Brain",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for SessionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Interactive" => Ok(SessionType::Interactive),
            "Loop" => Ok(SessionType::Loop),
            "Brain" => Ok(SessionType::Brain),
            _ => Err(anyhow::anyhow!("Unknown session type: {s}")),
        }
    }
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
        matches!(
            (self, next),
            (SessionState::Creating, SessionState::Active)
                | (SessionState::Active, SessionState::Finalizing)
                | (SessionState::Active, SessionState::Completed)
                | (SessionState::Active, SessionState::Failed)
                | (SessionState::Active, SessionState::Killed)
                | (SessionState::Finalizing, SessionState::Completed)
                | (SessionState::Finalizing, SessionState::Failed)
                | (SessionState::Finalizing, SessionState::Killed)
                | (SessionState::Completed, SessionState::Retained)
                | (SessionState::Failed, SessionState::Retained)
                | (SessionState::Killed, SessionState::Retained)
                | (SessionState::Retained, SessionState::Finalizing)
        )
    }
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SessionState::Creating => "Creating",
            SessionState::Active => "Active",
            SessionState::Finalizing => "Finalizing",
            SessionState::Completed => "Completed",
            SessionState::Failed => "Failed",
            SessionState::Killed => "Killed",
            SessionState::Retained => "Retained",
        };
        write!(f, "{s}")
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

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_ne!(
            r1.session_id, r2.session_id,
            "session IDs must be globally unique"
        );
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

    // AC-2: Valid State Transition — Creating → Active
    #[test]
    fn creating_can_transition_to_active() {
        assert!(
            SessionState::Creating.can_transition_to(&SessionState::Active),
            "Creating -> Active must be a valid transition"
        );
    }

    // AC-3: Invalid State Transitions Rejected
    #[test]
    fn completed_cannot_transition_to_active() {
        assert!(
            !SessionState::Completed.can_transition_to(&SessionState::Active),
            "Completed -> Active must be an invalid transition"
        );
    }

    #[test]
    fn retained_cannot_transition_to_most_states() {
        let invalid_targets = [
            SessionState::Creating,
            SessionState::Active,
            SessionState::Completed,
            SessionState::Failed,
            SessionState::Killed,
        ];
        for target in &invalid_targets {
            assert!(
                !SessionState::Retained.can_transition_to(target),
                "Retained -> {target} must be an invalid transition"
            );
        }
    }

    #[test]
    fn all_valid_transitions_accepted() {
        let valid = [
            (SessionState::Creating, SessionState::Active),
            (SessionState::Active, SessionState::Finalizing),
            (SessionState::Active, SessionState::Completed),
            (SessionState::Active, SessionState::Failed),
            (SessionState::Active, SessionState::Killed),
            (SessionState::Finalizing, SessionState::Completed),
            (SessionState::Finalizing, SessionState::Failed),
            (SessionState::Finalizing, SessionState::Killed),
            (SessionState::Completed, SessionState::Retained),
            (SessionState::Failed, SessionState::Retained),
            (SessionState::Killed, SessionState::Retained),
            (SessionState::Retained, SessionState::Finalizing),
        ];
        for (from, to) in &valid {
            assert!(
                from.can_transition_to(to),
                "{from} -> {to} must be a valid transition"
            );
        }
    }
}
