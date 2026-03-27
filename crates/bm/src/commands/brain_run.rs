use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::brain::{
    EventWatcher, EventWatcherConfig, Heartbeat, HeartbeatConfig, Multiplexer, MultiplexerConfig,
};

/// Runs the brain multiplexer event loop.
///
/// This is the internal handler for `bm brain-run`, spawned as a background
/// process by `bm start` for chat-first members. It creates a tokio runtime
/// and runs the multiplexer with event watcher and heartbeat components.
pub fn run(workspace: &str, system_prompt: &str, acp_binary: &str) -> Result<()> {
    let workspace_path = PathBuf::from(workspace);
    let prompt_content = std::fs::read_to_string(system_prompt)
        .with_context(|| format!("Failed to read brain system prompt at {system_prompt}"))?;

    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    rt.block_on(async {
        run_brain(workspace_path, prompt_content, acp_binary.to_string()).await
    })
}

async fn run_brain(
    workspace: PathBuf,
    system_prompt: String,
    acp_binary: String,
) -> Result<()> {
    let config = MultiplexerConfig {
        acp_binary,
        cwd: workspace.clone(),
        system_prompt: Some(system_prompt),
        env_vars: collect_env_vars(),
    };

    let (mux, input, _output, shutdown) = Multiplexer::new(config);

    // Get raw senders for event watcher and heartbeat
    let event_sender = input.sender();
    let heartbeat_sender = input.sender();

    // Spawn event watcher
    let event_config = EventWatcherConfig {
        workspace_root: workspace.clone(),
        poll_interval: std::time::Duration::from_secs(1),
    };
    let event_watcher = EventWatcher::new(event_config, event_sender);
    let (event_shutdown_tx, event_shutdown_rx) = tokio::sync::mpsc::channel(1);
    let event_handle = tokio::spawn(async move {
        event_watcher.run(event_shutdown_rx).await;
    });

    // Spawn heartbeat
    let heartbeat_config = HeartbeatConfig::default();
    let (heartbeat, heartbeat_shutdown, _pending) =
        Heartbeat::new(heartbeat_config, heartbeat_sender);
    let heartbeat_handle = tokio::spawn(async move {
        if let Err(e) = heartbeat.run().await {
            tracing::error!("Heartbeat error: {e}");
        }
    });

    // Handle SIGTERM for graceful shutdown
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
                tracing::info!("Received SIGTERM, shutting down brain");
                shutdown_clone.shutdown().await;
            }
            Err(e) => {
                tracing::error!("Failed to register SIGTERM handler: {e}");
            }
        }
    });

    // Run the multiplexer (blocks until shutdown)
    let result = mux.run().await;

    // Clean up
    heartbeat_shutdown.shutdown().await;
    let _ = event_shutdown_tx.send(()).await;
    let _ = event_handle.await;
    let _ = heartbeat_handle.await;

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            tracing::error!("Brain multiplexer error: {e}");
            anyhow::bail!("Brain multiplexer failed: {e}")
        }
    }
}

/// Collect relevant environment variables for the ACP process.
fn collect_env_vars() -> Vec<(String, String)> {
    let mut vars = Vec::new();

    for key in &[
        "GH_TOKEN",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_MODEL",
        "PATH",
        "HOME",
    ] {
        if let Ok(val) = std::env::var(key) {
            vars.push((key.to_string(), val));
        }
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_env_vars_includes_path() {
        let vars = collect_env_vars();
        assert!(
            vars.iter().any(|(k, _)| k == "PATH"),
            "PATH should be collected"
        );
    }

    #[test]
    fn collect_env_vars_skips_missing() {
        let vars = collect_env_vars();
        assert!(
            !vars.iter().any(|(k, _)| k == "NONEXISTENT_VAR_12345"),
            "Missing vars should not appear"
        );
    }
}
