use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::acp::{AcpClient, AcpConfig, AcpError, AcpEvent};

use super::queue::PromptQueue;
use super::types::{BrainMessage, BridgeOutput};

/// Configuration for the brain multiplexer.
#[derive(Debug, Clone)]
pub struct MultiplexerConfig {
    /// ACP agent binary path.
    pub acp_binary: String,
    /// Working directory for the ACP session.
    pub cwd: PathBuf,
    /// System prompt for the brain.
    pub system_prompt: Option<String>,
    /// Environment variables for the ACP process.
    pub env_vars: Vec<(String, String)>,
}

/// The brain multiplexer merges input streams and routes them through
/// an ACP session, streaming responses back to the bridge.
///
/// # Architecture
///
/// ```text
/// Bridge messages ─────┐
/// Loop events ─────────┤──► PromptQueue ──► ACP session ──► BridgeOutput
/// Heartbeat timer ─────┘      (priority)
/// ```
///
/// Only one prompt is in-flight at a time. While a prompt is being processed,
/// incoming messages are queued. When the response completes, the queue is
/// drained by priority order — human messages first, then loop events, then
/// heartbeat.
pub struct Multiplexer {
    config: MultiplexerConfig,
    /// Receives messages from all input sources (bridge, event watcher, heartbeat).
    input_rx: mpsc::Receiver<BrainMessage>,
    /// Sends output events to the bridge.
    output_tx: mpsc::Sender<BridgeOutput>,
    /// Receives shutdown signal.
    shutdown_rx: mpsc::Receiver<()>,
}

/// Handle for sending messages into the multiplexer.
#[derive(Clone)]
pub struct MultiplexerInput {
    tx: mpsc::Sender<BrainMessage>,
}

impl MultiplexerInput {
    /// Send a message to the multiplexer.
    ///
    /// Returns an error if the multiplexer has shut down.
    pub async fn send(&self, message: BrainMessage) -> Result<(), MultiplexerError> {
        self.tx
            .send(message)
            .await
            .map_err(|_| MultiplexerError::Shutdown)
    }
}

/// Handle for receiving output from the multiplexer.
pub struct MultiplexerOutput {
    rx: mpsc::Receiver<BridgeOutput>,
}

impl MultiplexerOutput {
    /// Receive the next output event from the brain.
    ///
    /// Returns `None` when the multiplexer has shut down.
    pub async fn recv(&mut self) -> Option<BridgeOutput> {
        self.rx.recv().await
    }
}

/// Handle for shutting down the multiplexer.
pub struct MultiplexerShutdown {
    tx: mpsc::Sender<()>,
}

impl MultiplexerShutdown {
    /// Signal the multiplexer to shut down.
    pub async fn shutdown(&self) {
        let _ = self.tx.send(()).await;
    }
}

/// Errors from the multiplexer.
#[derive(Debug)]
pub enum MultiplexerError {
    /// The ACP client failed.
    Acp(AcpError),
    /// The multiplexer has shut down.
    Shutdown,
}

impl std::fmt::Display for MultiplexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiplexerError::Acp(e) => write!(f, "ACP error: {e}"),
            MultiplexerError::Shutdown => write!(f, "multiplexer shut down"),
        }
    }
}

impl std::error::Error for MultiplexerError {}

impl From<AcpError> for MultiplexerError {
    fn from(e: AcpError) -> Self {
        MultiplexerError::Acp(e)
    }
}

impl Multiplexer {
    /// Create a new multiplexer and its communication handles.
    ///
    /// Returns the multiplexer (to be run on a tokio task), plus handles for:
    /// - `MultiplexerInput`: send messages into the brain
    /// - `MultiplexerOutput`: receive streaming responses
    /// - `MultiplexerShutdown`: signal clean shutdown
    pub fn new(
        config: MultiplexerConfig,
    ) -> (Self, MultiplexerInput, MultiplexerOutput, MultiplexerShutdown) {
        let (input_tx, input_rx) = mpsc::channel(64);
        let (output_tx, output_rx) = mpsc::channel(256);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let mux = Multiplexer {
            config,
            input_rx,
            output_tx,
            shutdown_rx,
        };

        let input = MultiplexerInput { tx: input_tx };
        let output = MultiplexerOutput { rx: output_rx };
        let shutdown = MultiplexerShutdown { tx: shutdown_tx };

        (mux, input, output, shutdown)
    }

    /// Run the multiplexer event loop.
    ///
    /// This is the core async loop that:
    /// 1. Spawns the ACP client and creates a session
    /// 2. Waits for messages from input channels
    /// 3. Sends prompts to the ACP session one at a time
    /// 4. Streams responses back via the output channel
    /// 5. Drains queued messages by priority after each response completes
    ///
    /// Returns when a shutdown signal is received or the ACP session ends.
    pub async fn run(mut self) -> Result<(), MultiplexerError> {
        // Spawn ACP client
        let acp_config = AcpConfig {
            binary: self.config.acp_binary.clone(),
            cwd: self.config.cwd.clone(),
            system_prompt: self.config.system_prompt.clone(),
            env_vars: self.config.env_vars.clone(),
        };

        let client = AcpClient::spawn(acp_config).await?;

        // Create a session
        let session_id = client
            .create_session(&self.config.cwd, self.config.system_prompt.as_deref())
            .await?;

        tracing::info!(session_id = %session_id, "Brain multiplexer session started");

        let mut queue = PromptQueue::new();
        let mut prompt_in_flight = false;

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = self.shutdown_rx.recv() => {
                    tracing::info!("Brain multiplexer shutting down");
                    // Cancel any in-flight prompt
                    if prompt_in_flight {
                        let _ = client.cancel(&session_id).await;
                    }
                    client.shutdown().await?;
                    return Ok(());
                }

                // Receive new messages from input sources
                msg = self.input_rx.recv() => {
                    match msg {
                        Some(message) => {
                            if prompt_in_flight {
                                // Queue the message while a prompt is in-flight
                                tracing::debug!(
                                    priority = %message.priority,
                                    "Queuing message (prompt in-flight)"
                                );
                                queue.push(message);
                            } else {
                                // Send immediately
                                let prompt = message.to_prompt();
                                tracing::debug!(prompt = %prompt, "Sending prompt to ACP");
                                client.prompt(&session_id, &prompt).await?;
                                prompt_in_flight = true;
                            }
                        }
                        None => {
                            // All input senders dropped — shut down
                            tracing::info!("All input channels closed, shutting down");
                            if prompt_in_flight {
                                let _ = client.cancel(&session_id).await;
                            }
                            client.shutdown().await?;
                            return Ok(());
                        }
                    }
                }

                // Receive events from the ACP session
                event = client.recv_event(), if prompt_in_flight => {
                    match event {
                        Some(AcpEvent::Text(text)) => {
                            let _ = self.output_tx.send(BridgeOutput::Text(text)).await;
                        }
                        Some(AcpEvent::TurnComplete { .. }) => {
                            let _ = self.output_tx.send(BridgeOutput::TurnComplete).await;
                            prompt_in_flight = false;

                            // Drain the queue by priority
                            if let Some(next_msg) = queue.pop() {
                                let prompt = next_msg.to_prompt();
                                tracing::debug!(
                                    priority = %next_msg.priority,
                                    "Draining queue, sending next prompt"
                                );
                                client.prompt(&session_id, &prompt).await?;
                                prompt_in_flight = true;
                            }
                        }
                        Some(AcpEvent::PermissionRequest { .. }) => {
                            // Permission requests are handled by the ACP client's
                            // permission handler (auto-approve by default).
                            // Nothing to do here.
                        }
                        None => {
                            // ACP connection closed
                            tracing::warn!("ACP connection closed unexpectedly");
                            let _ = self.output_tx.send(BridgeOutput::Error(
                                "ACP connection closed".into()
                            )).await;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::types::BrainMessage;

    fn test_config() -> MultiplexerConfig {
        MultiplexerConfig {
            acp_binary: "echo".into(),
            cwd: PathBuf::from("/tmp"),
            system_prompt: Some("Test prompt".into()),
            env_vars: vec![],
        }
    }

    #[test]
    fn multiplexer_new_creates_handles() {
        let config = test_config();
        let (_mux, _input, _output, _shutdown) = Multiplexer::new(config);
    }

    #[tokio::test]
    async fn input_handle_is_cloneable() {
        let config = test_config();
        let (_mux, input, _output, _shutdown) = Multiplexer::new(config);
        let _input2 = input.clone();
    }

    #[tokio::test]
    async fn shutdown_signal_drops_cleanly() {
        let config = test_config();
        let (_mux, _input, _output, shutdown) = Multiplexer::new(config);
        // Dropping the multiplexer before signaling shutdown should be fine
        drop(_mux);
        shutdown.shutdown().await;
    }

    #[tokio::test]
    async fn input_send_after_mux_dropped_returns_error() {
        let config = test_config();
        let (mux, input, _output, _shutdown) = Multiplexer::new(config);
        drop(mux);
        let result = input.send(BrainMessage::human("test")).await;
        assert!(matches!(result, Err(MultiplexerError::Shutdown)));
    }

    #[tokio::test]
    async fn output_recv_after_mux_dropped_returns_none() {
        let config = test_config();
        let (mux, _input, mut output, _shutdown) = Multiplexer::new(config);
        drop(mux);
        // Once the output_tx is dropped (inside mux), recv returns None
        let result = output.recv().await;
        assert!(result.is_none());
    }

    #[test]
    fn multiplexer_error_display() {
        let err = MultiplexerError::Shutdown;
        assert_eq!(err.to_string(), "multiplexer shut down");

        let err = MultiplexerError::Acp(AcpError::ChannelClosed);
        assert!(err.to_string().contains("ACP"));
    }

    #[test]
    fn multiplexer_config_debug() {
        let config = test_config();
        let debug = format!("{config:?}");
        assert!(debug.contains("echo"));
        assert!(debug.contains("Test prompt"));
    }

    #[test]
    fn brain_message_queue_integration() {
        // Test that messages go through the queue correctly and come
        // out with proper prompt formatting
        let mut q = PromptQueue::new();

        q.push(BrainMessage::heartbeat());
        q.push(BrainMessage::loop_event("loop-1", "task.close", "done"));
        q.push(BrainMessage::human("hi"));

        let prompts: Vec<String> = std::iter::from_fn(|| q.pop())
            .map(|m| m.to_prompt())
            .collect();

        assert_eq!(prompts.len(), 3);
        assert!(prompts[0].starts_with("[Human on bridge]:"));
        assert!(prompts[1].starts_with("[Loop loop-1 event]:"));
        assert!(prompts[2].starts_with("[Heartbeat]:"));
    }
}
