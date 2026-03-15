//! Tuwunel (Matrix) container cleanup guard for E2E tests.
//!
//! Like RcPodGuard, TuwunelGuard is a cleanup safety net only. The Tuwunel
//! container is created by `bm bridge start` (local bridge whose lifecycle is
//! managed by the Justfile). This guard wraps an already-running container and
//! force-removes it (plus its volume) on Drop, protecting against test panics
//! that occur between `bm bridge start` and `bm bridge stop`.

use std::process::Command;

/// Safety-net guard for a Tuwunel Podman container created by `bm bridge start`.
///
/// Created AFTER `bm bridge start` succeeds. The test calls `bm bridge stop`
/// during normal execution. If the test panics before reaching `bm bridge stop`,
/// the Drop implementation force-removes the container and volume.
pub struct TuwunelGuard {
    container_name: String,
    port: u16,
}

impl TuwunelGuard {
    /// Wraps an already-running container (created by `bm bridge start`).
    pub fn new(container_name: String, port: u16) -> Self {
        eprintln!(
            "TuwunelGuard created: container={} port={}",
            container_name, port
        );
        TuwunelGuard {
            container_name,
            port,
        }
    }

    /// Alias for `new` -- used by progressive mode to reconnect.
    pub fn from_existing(container_name: String, port: u16) -> Self {
        eprintln!(
            "TuwunelGuard from_existing: container={} port={}",
            container_name, port
        );
        TuwunelGuard {
            container_name,
            port,
        }
    }

    /// Consumes self WITHOUT triggering Drop. Returns (container_name, port).
    /// Used after `bm bridge stop` succeeds to prevent double-cleanup.
    pub fn into_parts(self) -> (String, u16) {
        let name = self.container_name.clone();
        let port = self.port;
        std::mem::forget(self);
        (name, port)
    }
}

impl Drop for TuwunelGuard {
    fn drop(&mut self) {
        eprintln!(
            "TuwunelGuard dropping: force-removing container {} and volume",
            self.container_name
        );
        let _ = Command::new("podman")
            .args(["rm", "-f", &self.container_name])
            .output();
        let volume_name = format!("{}-data", self.container_name);
        let _ = Command::new("podman")
            .args(["volume", "rm", "-f", &volume_name])
            .output();
    }
}
