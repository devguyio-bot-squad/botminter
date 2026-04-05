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
            content: concat!(
                "Periodic check. If you have pending background tasks, ",
                "check status and report. If nothing to report, leave bm-chat empty.",
            )
            .into(),
            source: None,
        }
    }

    /// Serialize this message using the default built-in envelope.
    pub fn to_prompt(&self) -> String {
        let envelopes = MessageEnvelopes::default();
        self.to_prompt_with_envelope(&envelopes)
    }

    /// Serialize using pre-compiled envelopes (prefix/suffix pairs per priority).
    pub fn to_prompt_with_envelope(&self, envelopes: &MessageEnvelopes) -> String {
        let content = match self.priority {
            Priority::Human => self.content.clone(),
            Priority::LoopEvent => {
                let loop_id = self.source.as_deref().unwrap_or("unknown");
                format!("Loop {loop_id}: {}", self.content)
            }
            Priority::Heartbeat => self.content.clone(),
        };

        let (prefix, suffix) = envelopes.for_priority(self.priority);
        format!("{prefix}{content}{suffix}")
    }
}

/// Pre-compiled envelope templates, split into prefix/suffix pairs per message type.
#[derive(Debug, Clone)]
pub struct MessageEnvelopes {
    pub human: (String, String),
    pub loop_event: (String, String),
    pub heartbeat: (String, String),
}

impl MessageEnvelopes {
    pub fn from_template(template: &str) -> Self {
        Self {
            human: Self::compile(template, "human", concat!(
                "Your operator just sent you a new message. They are waiting for a reply.\n",
                "IMMEDIATELY respond with a <bm-chat> acknowledgement before doing any tool calls.\n",
                "Then continue working and send another <bm-chat> with the full result when ready.",
            )),
            loop_event: Self::compile(template, "loop-event",
                "A loop event just fired. Report it to the operator if relevant.",
            ),
            heartbeat: Self::compile(template, "heartbeat",
                "This is a periodic check. Only respond if you have something to report.",
            ),
        }
    }

    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(template) => {
                tracing::info!(path = %path.display(), "Loaded brain envelope template");
                Self::from_template(&template)
            }
            Err(_) => {
                tracing::debug!(path = %path.display(), "No envelope template found, using default");
                Self::default()
            }
        }
    }

    fn compile(template: &str, msg_type: &str, urgency: &str) -> (String, String) {
        let resolved = template
            .replace("{{type}}", msg_type)
            .replace("{{urgency}}", urgency);

        match resolved.split_once("{{content}}") {
            Some((prefix, suffix)) => (prefix.to_string(), suffix.to_string()),
            None => {
                tracing::warn!("Envelope template missing {{{{content}}}} placeholder, appending content at end");
                (resolved, String::new())
            }
        }
    }

    fn for_priority(&self, priority: Priority) -> (&str, &str) {
        match priority {
            Priority::Human => (&self.human.0, &self.human.1),
            Priority::LoopEvent => (&self.loop_event.0, &self.loop_event.1),
            Priority::Heartbeat => (&self.heartbeat.0, &self.heartbeat.1),
        }
    }
}

impl Default for MessageEnvelopes {
    fn default() -> Self {
        Self::from_template(DEFAULT_ENVELOPE_TEMPLATE)
    }
}

const DEFAULT_ENVELOPE_TEMPLATE: &str = "\
<bm-context type=\"{{type}}\" channel=\"matrix\">\n\
<bm-message>\n\
{{content}}\n\
</bm-message>\n\
</bm-context>\n\
\n\
{{urgency}}\n\
\n\
Your response will be delivered to the operator on Matrix.\n\
Wrap operator-facing text in <bm-chat> tags:\n\
\n\
```\n\
<bm-response>\n\
<bm-chat>\n\
Your message to the operator.\n\
</bm-chat>\n\
</bm-response>\n\
```\n\
\n\
Only <bm-chat> content reaches the operator. Everything else is internal.";

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
        let prompt = msg.to_prompt();
        assert!(prompt.contains("<bm-context type=\"human\" channel=\"matrix\">"));
        assert!(prompt.contains("<bm-message>\nhello world\n</bm-message>"));
        assert!(prompt.contains("<bm-chat>"));
    }

    #[test]
    fn human_message_with_source() {
        let msg = BrainMessage::human_from("hello", "alice");
        let prompt = msg.to_prompt();
        assert!(prompt.contains("<bm-context type=\"human\""));
        assert!(prompt.contains("<bm-message>\nhello\n</bm-message>"));
    }

    #[test]
    fn loop_event_prompt() {
        let msg = BrainMessage::loop_event("loop-abc", "build.completed", "tests pass");
        let prompt = msg.to_prompt();
        assert!(prompt.contains("<bm-context type=\"loop-event\" channel=\"matrix\">"));
        assert!(prompt.contains("Loop loop-abc: build.completed"));
    }

    #[test]
    fn heartbeat_prompt() {
        let msg = BrainMessage::heartbeat();
        let prompt = msg.to_prompt();
        assert!(prompt.contains("<bm-context type=\"heartbeat\" channel=\"matrix\">"));
        assert!(prompt.contains("Periodic check"));
        assert!(prompt.contains("<bm-chat>"));
    }

    #[test]
    fn envelopes_default_produces_valid_output() {
        let env = MessageEnvelopes::default();
        let msg = BrainMessage::human("test input");
        let prompt = msg.to_prompt_with_envelope(&env);
        assert!(prompt.contains("test input"));
        assert!(prompt.contains("<bm-context type=\"human\""));
        assert!(prompt.contains("<bm-chat>"));
    }

    #[test]
    fn envelopes_from_custom_template() {
        let template = "PREFIX {{type}} | {{urgency}} | {{content}} | SUFFIX";
        let env = MessageEnvelopes::from_template(template);
        let msg = BrainMessage::human("hello");
        let prompt = msg.to_prompt_with_envelope(&env);
        assert!(prompt.starts_with("PREFIX human | "));
        assert!(prompt.contains("hello"));
        assert!(prompt.ends_with(" | SUFFIX"));
    }

    #[test]
    fn envelopes_content_not_substituted_at_runtime() {
        let env = MessageEnvelopes::default();
        let msg = BrainMessage::human("my text has {{type}} in it");
        let prompt = msg.to_prompt_with_envelope(&env);
        assert!(prompt.contains("my text has {{type}} in it"));
    }

    #[test]
    fn envelopes_load_nonexistent_file_uses_default() {
        let env = MessageEnvelopes::load(std::path::Path::new("/nonexistent/envelope.md"));
        let msg = BrainMessage::human("test");
        let prompt = msg.to_prompt_with_envelope(&env);
        assert!(prompt.contains("<bm-context type=\"human\""));
    }

    #[test]
    fn envelopes_load_from_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "CUSTOM: {{type}} says {{content}} end").unwrap();
        let env = MessageEnvelopes::load(tmp.path());
        let msg = BrainMessage::human("hi");
        let prompt = msg.to_prompt_with_envelope(&env);
        assert!(prompt.contains("CUSTOM: human says hi end"));
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
