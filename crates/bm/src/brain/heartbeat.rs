use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::time::{self, Duration, MissedTickBehavior};

use super::types::BrainMessage;

/// Configuration for the heartbeat timer.
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between heartbeat prompts. Set to `Duration::ZERO` to disable.
    pub interval: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
        }
    }
}

impl HeartbeatConfig {
    /// Create a heartbeat config from seconds. 0 means disabled.
    pub fn from_secs(secs: u64) -> Self {
        Self {
            interval: Duration::from_secs(secs),
        }
    }

    /// Whether the heartbeat is disabled (interval is zero).
    pub fn is_disabled(&self) -> bool {
        self.interval.is_zero()
    }
}

/// Errors from the heartbeat timer.
#[derive(Debug)]
pub enum HeartbeatError {
    /// The multiplexer input channel is closed.
    ChannelClosed,
}

impl std::fmt::Display for HeartbeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeartbeatError::ChannelClosed => write!(f, "multiplexer channel closed"),
        }
    }
}

impl std::error::Error for HeartbeatError {}

/// A flag shared between the heartbeat timer and the multiplexer to indicate
/// whether a heartbeat is pending in the queue.
///
/// The heartbeat sets this to `true` when it sends a heartbeat message.
/// The multiplexer sets it to `false` when it processes (sends to ACP) a
/// heartbeat-priority message. This prevents heartbeat messages from
/// accumulating in the queue while the brain is busy.
#[derive(Debug, Clone)]
pub struct HeartbeatPending(Arc<AtomicBool>);

impl HeartbeatPending {
    /// Create a new pending flag (initially not pending).
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Mark a heartbeat as pending (sent but not yet processed).
    pub fn set(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Clear the pending flag (heartbeat was processed).
    pub fn clear(&self) {
        self.0.store(false, Ordering::Release);
    }

    /// Check whether a heartbeat is currently pending.
    pub fn is_pending(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

impl Default for HeartbeatPending {
    fn default() -> Self {
        Self::new()
    }
}

/// Periodic heartbeat timer that sends low-priority prompts to the brain
/// when it has been idle.
///
/// The heartbeat fires at a configurable interval, pushing
/// `BrainMessage::heartbeat()` into the multiplexer at P2 (lowest) priority.
/// It skips firing when a previous heartbeat is still pending in the queue,
/// preventing accumulation during busy periods.
pub struct Heartbeat {
    config: HeartbeatConfig,
    /// Channel to send heartbeat messages to the multiplexer.
    input_tx: mpsc::Sender<BrainMessage>,
    /// Shared flag to track whether a heartbeat is pending.
    pending: HeartbeatPending,
    /// Shutdown signal receiver.
    shutdown_rx: mpsc::Receiver<()>,
}

/// Handle for shutting down the heartbeat timer.
pub struct HeartbeatShutdown {
    tx: mpsc::Sender<()>,
}

impl HeartbeatShutdown {
    /// Signal the heartbeat timer to stop.
    pub async fn shutdown(&self) {
        let _ = self.tx.send(()).await;
    }
}

impl Heartbeat {
    /// Create a new heartbeat timer.
    ///
    /// Returns the heartbeat (to be run on a tokio task), its shutdown handle,
    /// and the shared pending flag that the multiplexer should use to clear
    /// after processing heartbeat messages.
    pub fn new(
        config: HeartbeatConfig,
        input_tx: mpsc::Sender<BrainMessage>,
    ) -> (Self, HeartbeatShutdown, HeartbeatPending) {
        let pending = HeartbeatPending::new();
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let heartbeat = Self {
            config,
            input_tx,
            pending: pending.clone(),
            shutdown_rx,
        };

        let shutdown = HeartbeatShutdown { tx: shutdown_tx };

        (heartbeat, shutdown, pending)
    }

    /// Run the heartbeat timer loop.
    ///
    /// Fires at the configured interval, sending `BrainMessage::heartbeat()`
    /// to the multiplexer. Skips ticks when a previous heartbeat is still
    /// pending. Returns when shutdown is signaled or the multiplexer channel
    /// closes.
    ///
    /// Returns `Ok(())` on clean shutdown, or `Err(HeartbeatError::ChannelClosed)`
    /// if the multiplexer dropped its receiver.
    pub async fn run(mut self) -> Result<(), HeartbeatError> {
        if self.config.is_disabled() {
            tracing::info!("Heartbeat timer disabled (interval=0)");
            // Wait for shutdown signal, then exit cleanly
            let _ = self.shutdown_rx.recv().await;
            return Ok(());
        }

        let mut interval = time::interval(self.config.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        // Skip the first immediate tick — we don't want a heartbeat at t=0
        interval.tick().await;

        tracing::info!(
            interval_secs = self.config.interval.as_secs(),
            "Heartbeat timer started"
        );

        loop {
            tokio::select! {
                _ = self.shutdown_rx.recv() => {
                    tracing::info!("Heartbeat timer shutting down");
                    return Ok(());
                }
                _ = interval.tick() => {
                    if self.pending.is_pending() {
                        tracing::debug!("Skipping heartbeat — previous still pending");
                        continue;
                    }

                    let msg = BrainMessage::heartbeat();
                    match self.input_tx.send(msg).await {
                        Ok(()) => {
                            self.pending.set();
                            tracing::debug!("Heartbeat sent to multiplexer");
                        }
                        Err(_) => {
                            tracing::warn!("Multiplexer channel closed, heartbeat stopping");
                            return Err(HeartbeatError::ChannelClosed);
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

    #[test]
    fn config_default_is_60s() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval, Duration::from_secs(60));
        assert!(!config.is_disabled());
    }

    #[test]
    fn config_from_secs_zero_is_disabled() {
        let config = HeartbeatConfig::from_secs(0);
        assert!(config.is_disabled());
    }

    #[test]
    fn config_from_secs_nonzero() {
        let config = HeartbeatConfig::from_secs(30);
        assert_eq!(config.interval, Duration::from_secs(30));
        assert!(!config.is_disabled());
    }

    #[test]
    fn pending_flag_default_not_pending() {
        let pending = HeartbeatPending::new();
        assert!(!pending.is_pending());
    }

    #[test]
    fn pending_flag_set_and_clear() {
        let pending = HeartbeatPending::new();
        pending.set();
        assert!(pending.is_pending());
        pending.clear();
        assert!(!pending.is_pending());
    }

    #[test]
    fn pending_flag_clone_shares_state() {
        let a = HeartbeatPending::new();
        let b = a.clone();
        a.set();
        assert!(b.is_pending());
        b.clear();
        assert!(!a.is_pending());
    }

    #[test]
    fn heartbeat_error_display() {
        let err = HeartbeatError::ChannelClosed;
        assert_eq!(err.to_string(), "multiplexer channel closed");
    }

    #[tokio::test]
    async fn disabled_heartbeat_exits_on_shutdown() {
        let (tx, _rx) = mpsc::channel(16);
        let (heartbeat, shutdown, _pending) =
            Heartbeat::new(HeartbeatConfig::from_secs(0), tx);

        let handle = tokio::spawn(heartbeat.run());
        shutdown.shutdown().await;

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn heartbeat_fires_at_interval() {
        let (tx, mut rx) = mpsc::channel(16);
        let config = HeartbeatConfig {
            interval: Duration::from_millis(50),
        };
        let (heartbeat, shutdown, _pending) = Heartbeat::new(config, tx);

        let handle = tokio::spawn(heartbeat.run());

        // Wait for at least one heartbeat
        let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("should receive heartbeat within timeout")
            .expect("channel should not be closed");

        assert_eq!(msg.priority, super::super::types::Priority::Heartbeat);
        assert!(
            msg.to_prompt().contains("<bm-context type=\"heartbeat\" channel=\"matrix\">"),
            "heartbeat prompt should use envelope format"
        );

        shutdown.shutdown().await;
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn heartbeat_skips_when_pending() {
        let (tx, mut rx) = mpsc::channel(16);
        let config = HeartbeatConfig {
            interval: Duration::from_millis(30),
        };
        let (heartbeat, shutdown, pending) = Heartbeat::new(config, tx);

        // Pre-set pending flag before starting
        pending.set();

        let handle = tokio::spawn(heartbeat.run());

        // Wait long enough for multiple ticks to pass
        tokio::time::sleep(Duration::from_millis(120)).await;

        // No heartbeat should have been sent (all skipped)
        assert!(rx.try_recv().is_err());

        // Clear the pending flag — next tick should fire
        pending.clear();

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should receive heartbeat after clearing pending")
            .expect("channel should not be closed");

        assert_eq!(msg.priority, super::super::types::Priority::Heartbeat);

        shutdown.shutdown().await;
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn heartbeat_stops_when_channel_closes() {
        let (tx, rx) = mpsc::channel(16);
        let config = HeartbeatConfig {
            interval: Duration::from_millis(30),
        };
        let (heartbeat, _shutdown, _pending) = Heartbeat::new(config, tx);

        // Drop the receiver to close the channel
        drop(rx);

        let result = heartbeat.run().await;
        assert!(matches!(result, Err(HeartbeatError::ChannelClosed)));
    }

    #[tokio::test]
    async fn heartbeat_sets_pending_on_send() {
        let (tx, mut rx) = mpsc::channel(16);
        let config = HeartbeatConfig {
            interval: Duration::from_millis(30),
        };
        let (heartbeat, shutdown, pending) = Heartbeat::new(config, tx);

        assert!(!pending.is_pending());

        let handle = tokio::spawn(heartbeat.run());

        // Wait for first heartbeat
        let _ = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should receive heartbeat");

        // Pending should be set
        assert!(pending.is_pending());

        shutdown.shutdown().await;
        handle.await.unwrap().unwrap();
    }

    #[test]
    fn new_returns_all_handles() {
        let (tx, _rx) = mpsc::channel(16);
        let (_heartbeat, _shutdown, _pending) =
            Heartbeat::new(HeartbeatConfig::default(), tx);
        // All three components created successfully
    }

    #[test]
    fn config_debug_format() {
        let config = HeartbeatConfig::from_secs(45);
        let debug = format!("{config:?}");
        assert!(debug.contains("45"));
    }
}
