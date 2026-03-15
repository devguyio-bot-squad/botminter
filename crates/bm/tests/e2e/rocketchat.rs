//! Rocket.Chat Podman Pod cleanup guard for E2E tests.
//!
//! Unlike TgMock which starts an external container, RcPodGuard is a cleanup
//! safety net only. The RC pod is created by `bm bridge start` (local bridge
//! whose lifecycle is managed by the Justfile). This guard wraps an already-running
//! pod and force-removes it on Drop, protecting against test panics that occur
//! between `bm bridge start` and `bm bridge stop`.

use std::process::Command;

/// Safety-net guard for a Podman Pod created by `bm bridge start`.
///
/// Created AFTER `bm bridge start` succeeds. The test calls `bm bridge stop`
/// during normal execution. If the test panics before reaching `bm bridge stop`,
/// the Drop implementation force-removes the pod to avoid leaking infrastructure.
pub struct RcPodGuard {
    pod_name: String,
    port: u16,
}

impl RcPodGuard {
    /// Wraps an already-running pod (created by `bm bridge start`).
    pub fn new(pod_name: String, port: u16) -> Self {
        eprintln!(
            "RcPodGuard created: pod={} port={}",
            pod_name, port
        );
        RcPodGuard { pod_name, port }
    }

    /// Alias for `new` -- used by progressive mode to reconnect to a pod
    /// from a previous run.
    pub fn from_existing(pod_name: String, port: u16) -> Self {
        eprintln!(
            "RcPodGuard from_existing: pod={} port={}",
            pod_name, port
        );
        RcPodGuard { pod_name, port }
    }

    /// Returns the host port the RC pod is mapped to.
    #[allow(dead_code)]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the pod name.
    #[allow(dead_code)]
    pub fn pod_name(&self) -> &str {
        &self.pod_name
    }

    /// Consumes self WITHOUT triggering Drop. Returns (pod_name, port).
    /// Used after `bm bridge stop` succeeds to prevent double-cleanup.
    pub fn into_parts(self) -> (String, u16) {
        let name = self.pod_name.clone();
        let port = self.port;
        std::mem::forget(self);
        (name, port)
    }
}

impl Drop for RcPodGuard {
    fn drop(&mut self) {
        eprintln!(
            "RcPodGuard dropping: force-removing pod {}",
            self.pod_name
        );
        let _ = Command::new("podman")
            .args(["pod", "rm", "-f", &self.pod_name])
            .output();
    }
}
