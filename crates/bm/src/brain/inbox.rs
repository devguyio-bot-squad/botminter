use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

/// Workspace marker file that indicates a BotMinter workspace root.
const WORKSPACE_MARKER: &str = ".botminter.workspace";

/// A single inbox message with attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    /// ISO 8601 timestamp.
    pub ts: String,
    /// Sender identity (e.g., "brain").
    pub from: String,
    /// Message content.
    pub message: String,
}

/// Result of a read operation.
pub struct InboxReadResult {
    pub messages: Vec<InboxMessage>,
    pub consumed: bool,
}

/// Returns the inbox file path for a workspace root.
///
/// The inbox lives at `<root>/.ralph/loop-inbox.jsonl`.
pub fn inbox_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".ralph/loop-inbox.jsonl")
}

/// Write a message to the inbox file.
///
/// Creates parent directories if needed. Acquires an exclusive file lock
/// for concurrency safety. Rejects empty or whitespace-only messages.
pub fn write_message(path: &Path, from: &str, message: &str) -> anyhow::Result<()> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Message cannot be empty");
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    file.lock_exclusive()?;
    let result = write_line(&file, from, trimmed);
    file.unlock()?;

    result
}

fn write_line(mut file: &File, from: &str, message: &str) -> anyhow::Result<()> {
    let msg = InboxMessage {
        ts: chrono::Utc::now().to_rfc3339(),
        from: from.to_string(),
        message: message.to_string(),
    };
    let json = serde_json::to_string(&msg)?;
    writeln!(file, "{json}")?;
    Ok(())
}

/// Read messages from the inbox file.
///
/// If `consume` is true, the file is truncated after reading (best-effort consumption).
/// Malformed JSONL lines are silently skipped.
/// Returns an empty result (not an error) if the file is missing or empty.
pub fn read_messages(path: &Path, consume: bool) -> anyhow::Result<InboxReadResult> {
    let file = match OpenOptions::new().read(true).write(consume).open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(InboxReadResult {
                messages: Vec::new(),
                consumed: consume,
            });
        }
        Err(e) => return Err(e.into()),
    };

    file.lock_exclusive()?;
    let messages = parse_lines(&file);

    if consume {
        // Truncate the file to zero length
        file.set_len(0)?;
    }

    file.unlock()?;

    Ok(InboxReadResult {
        messages,
        consumed: consume,
    })
}

fn parse_lines(file: &File) -> Vec<InboxMessage> {
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<InboxMessage>(trimmed) {
            Ok(msg) => messages.push(msg),
            Err(_) => {
                // Skip malformed lines gracefully (same pattern as event_watcher)
            }
        }
    }

    messages
}

/// Walk up from `start` looking for the `.botminter.workspace` marker file.
///
/// Returns `Some(root)` if found, `None` if the filesystem root is reached
/// without finding the marker.
pub fn discover_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(WORKSPACE_MARKER).exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Format inbox messages as a Claude Code hook response with `additionalContext`.
///
/// Returns `None` if there are no messages, or `Some(json_string)` with the
/// formatted hook response.
pub fn format_hook_response(messages: &[InboxMessage]) -> Option<String> {
    if messages.is_empty() {
        return None;
    }

    let mut context = String::from(
        "## Brain Feedback\n\n\
         You have received feedback from your brain \
         — the consciousness that monitors your work and relays human directives.\n\n\
         **Messages:**\n",
    );

    for msg in messages {
        context.push_str(&format!("\n[{}] ({}): {}\n", msg.ts, msg.from, msg.message));
    }

    context.push_str(
        "\n**Instructions:** Brain feedback takes priority over your current subtask. \
         Acknowledge by adjusting your approach. If this feedback conflicts with your \
         current task, comply with the feedback.",
    );

    let response = serde_json::json!({
        "additionalContext": context,
    });

    Some(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- Path construction ---

    #[test]
    fn inbox_path_construction() {
        let root = PathBuf::from("/some/workspace");
        let path = inbox_path(&root);
        assert_eq!(path, PathBuf::from("/some/workspace/.ralph/loop-inbox.jsonl"));
    }

    // --- write_message ---

    #[test]
    fn write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());

        write_message(&path, "brain", "msg1").unwrap();
        write_message(&path, "brain", "msg2").unwrap();
        write_message(&path, "human", "msg3").unwrap();

        let result = read_messages(&path, false).unwrap();
        assert_eq!(result.messages.len(), 3);
        assert_eq!(result.messages[0].from, "brain");
        assert_eq!(result.messages[0].message, "msg1");
        assert_eq!(result.messages[1].message, "msg2");
        assert_eq!(result.messages[2].from, "human");
        assert_eq!(result.messages[2].message, "msg3");

        // Verify timestamps are valid ISO 8601
        for msg in &result.messages {
            assert!(
                chrono::DateTime::parse_from_rfc3339(&msg.ts).is_ok(),
                "Invalid timestamp: {}",
                msg.ts
            );
        }
    }

    #[test]
    fn write_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        // Don't create .ralph/ — write_message should do it
        let path = tmp.path().join(".ralph/loop-inbox.jsonl");

        write_message(&path, "brain", "hello").unwrap();

        assert!(path.exists());
        let result = read_messages(&path, false).unwrap();
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].message, "hello");
    }

    #[test]
    fn write_rejects_empty_message() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());

        let err = write_message(&path, "brain", "").unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("empty"),
            "Error should mention 'empty', got: {err}"
        );
    }

    #[test]
    fn write_rejects_whitespace_only_message() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());

        let err = write_message(&path, "brain", "   \n\t  ").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("empty"));
    }

    // --- read_messages ---

    #[test]
    fn read_consume_truncates_file() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());

        write_message(&path, "brain", "one").unwrap();
        write_message(&path, "brain", "two").unwrap();

        let result = read_messages(&path, true).unwrap();
        assert_eq!(result.messages.len(), 2);
        assert!(result.consumed);

        // File should now be empty
        let result2 = read_messages(&path, false).unwrap();
        assert_eq!(result2.messages.len(), 0);
    }

    #[test]
    fn read_preserve_keeps_messages() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());

        write_message(&path, "brain", "one").unwrap();
        write_message(&path, "brain", "two").unwrap();

        let result = read_messages(&path, false).unwrap();
        assert_eq!(result.messages.len(), 2);
        assert!(!result.consumed);

        // Messages should still be there
        let result2 = read_messages(&path, false).unwrap();
        assert_eq!(result2.messages.len(), 2);
    }

    #[test]
    fn read_missing_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent/inbox.jsonl");

        let result = read_messages(&path, false).unwrap();
        assert_eq!(result.messages.len(), 0);
    }

    #[test]
    fn read_empty_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(&path).unwrap();

        let result = read_messages(&path, false).unwrap();
        assert_eq!(result.messages.len(), 0);
    }

    #[test]
    fn read_skips_malformed_lines() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        let valid1 = serde_json::json!({"ts": "2026-03-21T10:00:00Z", "from": "brain", "message": "valid1"});
        let valid2 = serde_json::json!({"ts": "2026-03-21T10:01:00Z", "from": "brain", "message": "valid2"});

        let content = format!("{}\nnot json at all\n{}\n", valid1, valid2);
        fs::write(&path, content).unwrap();

        let result = read_messages(&path, false).unwrap();
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.messages[0].message, "valid1");
        assert_eq!(result.messages[1].message, "valid2");
    }

    // --- format_hook_response ---

    #[test]
    fn format_hook_response_with_messages() {
        let messages = vec![
            InboxMessage {
                ts: "2026-03-21T14:30:00Z".to_string(),
                from: "brain".to_string(),
                message: "Fix CI".to_string(),
            },
            InboxMessage {
                ts: "2026-03-21T14:31:00Z".to_string(),
                from: "human".to_string(),
                message: "Stop refactoring".to_string(),
            },
        ];

        let response = format_hook_response(&messages);
        assert!(response.is_some());

        let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
        let ctx = json["additionalContext"].as_str().unwrap();
        assert!(ctx.contains("Brain Feedback"));
        assert!(ctx.contains("Fix CI"));
        assert!(ctx.contains("Stop refactoring"));
    }

    #[test]
    fn format_hook_response_empty() {
        let result = format_hook_response(&[]);
        assert!(result.is_none());
    }

    // --- discover_workspace_root ---

    #[test]
    fn discover_workspace_root_in_current_dir() {
        let tmp = TempDir::new().unwrap();
        File::create(tmp.path().join(WORKSPACE_MARKER)).unwrap();

        let result = discover_workspace_root(tmp.path());
        assert_eq!(result, Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn discover_workspace_root_in_parent() {
        let tmp = TempDir::new().unwrap();
        File::create(tmp.path().join(WORKSPACE_MARKER)).unwrap();
        let child = tmp.path().join("child");
        fs::create_dir_all(&child).unwrap();

        let result = discover_workspace_root(&child);
        assert_eq!(result, Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn discover_workspace_root_in_grandparent() {
        let tmp = TempDir::new().unwrap();
        File::create(tmp.path().join(WORKSPACE_MARKER)).unwrap();
        let deep = tmp.path().join("a/b");
        fs::create_dir_all(&deep).unwrap();

        let result = discover_workspace_root(&deep);
        assert_eq!(result, Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn discover_workspace_root_not_found() {
        let tmp = TempDir::new().unwrap();
        // No marker file

        let result = discover_workspace_root(tmp.path());
        assert!(result.is_none());
    }

    // --- Concurrency ---

    #[test]
    fn concurrent_writes_are_safe() {
        let tmp = TempDir::new().unwrap();
        let path = inbox_path(tmp.path());

        let path_arc = std::sync::Arc::new(path.clone());
        let mut handles = Vec::new();

        for i in 0..8 {
            let p = std::sync::Arc::clone(&path_arc);
            handles.push(std::thread::spawn(move || {
                write_message(&p, &format!("writer-{i}"), &format!("message-{i}")).unwrap();
            }));
        }

        for h in handles {
            h.join().expect("Thread should not panic");
        }

        let result = read_messages(&path, false).unwrap();
        assert_eq!(result.messages.len(), 8);

        // Verify all 8 unique senders are present
        let mut senders: Vec<String> = result.messages.iter().map(|m| m.from.clone()).collect();
        senders.sort();
        let expected: Vec<String> = (0..8).map(|i| format!("writer-{i}")).collect();
        assert_eq!(senders, expected);
    }
}
