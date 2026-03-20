use std::fmt;

/// Priority levels for messages entering the multiplexer.
/// Lower numeric value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// Human messages from the bridge chat (highest priority).
    Human = 0,
    /// Events from running Ralph loops (medium priority).
    LoopEvent = 1,
    /// Periodic heartbeat prompts (lowest priority).
    Heartbeat = 2,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Human => write!(f, "human"),
            Priority::LoopEvent => write!(f, "loop_event"),
            Priority::Heartbeat => write!(f, "heartbeat"),
        }
    }
}

/// A message entering the multiplexer from any input source.
#[derive(Debug, Clone)]
pub struct BrainMessage {
    /// Priority determines queue ordering.
    pub priority: Priority,
    /// The raw content of the message.
    pub content: String,
    /// Optional source identifier (e.g., loop ID, user name).
    pub source: Option<String>,
}

impl BrainMessage {
    /// Create a human message from the bridge.
    pub fn human(content: impl Into<String>) -> Self {
        Self {
            priority: Priority::Human,
            content: content.into(),
            source: None,
        }
    }

    /// Create a human message with a source identifier.
    pub fn human_from(content: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            priority: Priority::Human,
            content: content.into(),
            source: Some(source.into()),
        }
    }

    /// Create a loop event message.
    pub fn loop_event(
        loop_id: impl Into<String>,
        event_type: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        let event_type = event_type.into();
        let summary = summary.into();
        Self {
            priority: Priority::LoopEvent,
            content: format!("{event_type} — {summary}"),
            source: Some(loop_id.into()),
        }
    }

    /// Create a heartbeat message.
    pub fn heartbeat() -> Self {
        Self {
            priority: Priority::Heartbeat,
            content: "Check your loops. Check the board. Pick up new work if idle.".into(),
            source: None,
        }
    }

    /// Serialize this message into a prompt string with the appropriate context prefix.
    pub fn to_prompt(&self) -> String {
        match self.priority {
            Priority::Human => {
                let source = self.source.as_deref().unwrap_or("bridge");
                format!("[Human on {source}]: {}", self.content)
            }
            Priority::LoopEvent => {
                let loop_id = self.source.as_deref().unwrap_or("unknown");
                format!("[Loop {loop_id} event]: {}", self.content)
            }
            Priority::Heartbeat => {
                format!("[Heartbeat]: {}", self.content)
            }
        }
    }
}

/// Output events from the brain to the bridge.
#[derive(Debug, Clone)]
pub enum BridgeOutput {
    /// A text chunk from the ACP agent's streaming response.
    Text(String),
    /// The agent's turn completed.
    TurnComplete,
    /// The brain encountered an error.
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_ordering() {
        assert!(Priority::Human < Priority::LoopEvent);
        assert!(Priority::LoopEvent < Priority::Heartbeat);
        assert!(Priority::Human < Priority::Heartbeat);
    }

    #[test]
    fn human_message_prompt() {
        let msg = BrainMessage::human("hello world");
        assert_eq!(msg.to_prompt(), "[Human on bridge]: hello world");
    }

    #[test]
    fn human_message_with_source() {
        let msg = BrainMessage::human_from("hello", "alice");
        assert_eq!(msg.to_prompt(), "[Human on alice]: hello");
    }

    #[test]
    fn loop_event_prompt() {
        let msg = BrainMessage::loop_event("loop-abc", "build.completed", "tests pass");
        assert_eq!(
            msg.to_prompt(),
            "[Loop loop-abc event]: build.completed — tests pass"
        );
    }

    #[test]
    fn heartbeat_prompt() {
        let msg = BrainMessage::heartbeat();
        assert_eq!(
            msg.to_prompt(),
            "[Heartbeat]: Check your loops. Check the board. Pick up new work if idle."
        );
    }

    #[test]
    fn priority_display() {
        assert_eq!(Priority::Human.to_string(), "human");
        assert_eq!(Priority::LoopEvent.to_string(), "loop_event");
        assert_eq!(Priority::Heartbeat.to_string(), "heartbeat");
    }

    #[test]
    fn bridge_output_variants() {
        let text = BridgeOutput::Text("chunk".into());
        assert!(matches!(text, BridgeOutput::Text(s) if s == "chunk"));

        let done = BridgeOutput::TurnComplete;
        assert!(matches!(done, BridgeOutput::TurnComplete));

        let err = BridgeOutput::Error("oops".into());
        assert!(matches!(err, BridgeOutput::Error(s) if s == "oops"));
    }
}
