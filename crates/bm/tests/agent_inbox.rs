//! Integration tests for the `bm-agent` CLI binary.
//!
//! Tests exercise the full CLI lifecycle: inbox write/peek/read and
//! Claude hook post-tool-use. Each test creates an isolated tempdir
//! with workspace markers — no TestEnv needed (no keyring, no GitHub, no dbus).

use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// Set up a temporary workspace with `.botminter.workspace` marker and `.ralph/` dir.
fn setup_workspace() -> TempDir {
    let tmp = TempDir::new().expect("create tempdir");
    fs::write(tmp.path().join(".botminter.workspace"), "").expect("create marker");
    fs::create_dir_all(tmp.path().join(".ralph")).expect("create .ralph dir");
    tmp
}

/// Build a `bm-agent` command configured to run inside the given workspace.
fn agent_cmd(workspace: &TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bm-agent"));
    cmd.current_dir(workspace.path());
    // Override HOME to prevent touching real user directories (test-path-isolation)
    cmd.env("HOME", workspace.path());
    cmd
}

// --- Test 1: write + peek lifecycle ---

#[test]
fn write_and_peek_shows_message() {
    let ws = setup_workspace();

    // Write a message
    let out = agent_cmd(&ws)
        .args(["inbox", "write", "fix CI please"])
        .output()
        .expect("run write");
    assert!(out.status.success(), "write should succeed: {}", String::from_utf8_lossy(&out.stderr));

    // Peek should show the message
    let out = agent_cmd(&ws)
        .args(["inbox", "peek"])
        .output()
        .expect("run peek");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("fix CI please"), "peek should contain message, got: {stdout}");
    assert!(stdout.contains("brain"), "peek should show sender, got: {stdout}");
}

// --- Test 2: read + consume ---

#[test]
fn read_json_consumes_messages() {
    let ws = setup_workspace();

    // Write a message
    agent_cmd(&ws)
        .args(["inbox", "write", "test msg"])
        .output()
        .expect("write");

    // Read as JSON (consumes)
    let out = agent_cmd(&ws)
        .args(["inbox", "read", "--format", "json"])
        .output()
        .expect("read json");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    let arr = parsed.as_array().expect("should be array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["message"].as_str().unwrap(), "test msg");

    // Peek should now show empty
    let out = agent_cmd(&ws)
        .args(["inbox", "peek"])
        .output()
        .expect("peek");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("No pending messages"), "inbox should be empty after read, got: {stdout}");
}

// --- Test 3: read --format hook ---

#[test]
fn read_hook_format_returns_additional_context() {
    let ws = setup_workspace();

    agent_cmd(&ws)
        .args(["inbox", "write", "redirect to API"])
        .output()
        .expect("write");

    let out = agent_cmd(&ws)
        .args(["inbox", "read", "--format", "hook"])
        .output()
        .expect("read hook");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert!(parsed["additionalContext"].is_string(), "should have additionalContext key");
    let ctx = parsed["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("redirect to API"), "additionalContext should contain message");
}

// --- Test 4: empty write rejected ---

#[test]
fn empty_write_rejected() {
    let ws = setup_workspace();

    let out = agent_cmd(&ws)
        .args(["inbox", "write", ""])
        .output()
        .expect("write empty");
    assert!(!out.status.success(), "empty write should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.to_lowercase().contains("empty"), "should mention empty, got: {stderr}");
}

// --- Test 5: whitespace-only write rejected ---

#[test]
fn whitespace_only_write_rejected() {
    let ws = setup_workspace();

    let out = agent_cmd(&ws)
        .args(["inbox", "write", "   "])
        .output()
        .expect("write whitespace");
    assert!(!out.status.success(), "whitespace-only write should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.to_lowercase().contains("empty"), "should mention empty, got: {stderr}");
}

// --- Test 6: outside workspace rejected ---

#[test]
fn inbox_write_outside_workspace_rejected() {
    let tmp = TempDir::new().expect("create tempdir");
    // No .botminter.workspace marker

    let out = Command::new(env!("CARGO_BIN_EXE_bm-agent"))
        .current_dir(tmp.path())
        .env("HOME", tmp.path())
        .args(["inbox", "write", "test"])
        .output()
        .expect("write outside workspace");
    assert!(!out.status.success(), "write outside workspace should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("workspace") || stderr.contains("botminter"),
        "should mention workspace, got: {stderr}"
    );
}

// --- Test 7: hook graceful outside workspace ---

#[test]
fn hook_graceful_outside_workspace() {
    let tmp = TempDir::new().expect("create tempdir");
    // No .botminter.workspace marker

    let out = Command::new(env!("CARGO_BIN_EXE_bm-agent"))
        .current_dir(tmp.path())
        .env("HOME", tmp.path())
        .args(["claude", "hook", "post-tool-use"])
        .output()
        .expect("hook outside workspace");
    assert!(out.status.success(), "hook should always exit 0");
    assert!(out.stdout.is_empty(), "hook should produce no output outside workspace");
}

// --- Test 8: hook empty inbox ---

#[test]
fn hook_empty_inbox_nudge() {
    let ws = setup_workspace();

    let out = agent_cmd(&ws)
        .args(["claude", "hook", "post-tool-use"])
        .output()
        .expect("hook empty inbox");
    assert!(out.status.success(), "hook should exit 0");
    // Even with no inbox messages, the hook outputs a response nudge
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).expect("should be valid JSON");
    assert!(
        json["additionalContext"].as_str().unwrap().contains("respond"),
        "nudge should mention responding to user"
    );
}

// --- Test 9: hook delivery + consumption ---

#[test]
fn hook_delivers_and_consumes_messages() {
    let ws = setup_workspace();

    // Write a message
    agent_cmd(&ws)
        .args(["inbox", "write", "focus on tests"])
        .output()
        .expect("write");

    // Hook should return additionalContext
    let out = agent_cmd(&ws)
        .args(["claude", "hook", "post-tool-use"])
        .output()
        .expect("hook delivery");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    let ctx = parsed["additionalContext"].as_str().expect("additionalContext key");
    assert!(ctx.contains("focus on tests"), "should contain message");

    // Peek should show empty (consumed)
    let out = agent_cmd(&ws)
        .args(["inbox", "peek"])
        .output()
        .expect("peek after hook");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("No pending messages"), "inbox should be empty after hook, got: {stdout}");
}

// --- Test 10: hook corrupted file ---

#[test]
fn hook_graceful_with_corrupted_file() {
    let ws = setup_workspace();

    // Write garbage to the inbox file
    let inbox_path = ws.path().join(".ralph/loop-inbox.jsonl");
    fs::write(&inbox_path, "not valid json\n{broken\n\x00\x01\x02\n").expect("write garbage");

    let out = agent_cmd(&ws)
        .args(["claude", "hook", "post-tool-use"])
        .output()
        .expect("hook with corrupted file");
    assert!(out.status.success(), "hook should always exit 0 even with corrupted file");
}
