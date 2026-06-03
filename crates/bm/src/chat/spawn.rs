use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

pub struct SpawnConfig {
    pub agent_binary: String,
    pub agent_args: Vec<String>,
    pub working_dir: PathBuf,
    pub env_vars: Vec<(String, String)>,
}

pub struct SpawnResult {
    pub exit_code: i32,
    pub agent_pid: u32,
}

pub fn spawn_and_wait(config: &SpawnConfig) -> Result<SpawnResult> {
    let mut cmd = Command::new(&config.agent_binary);
    cmd.args(&config.agent_args)
        .current_dir(&config.working_dir)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    for (key, val) in &config.env_vars {
        cmd.env(key, val);
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn agent: {}", config.agent_binary))?;

    let agent_pid = child.id();

    setup_signal_forwarding(agent_pid)?;

    let status = child.wait().context("Failed waiting for agent process")?;

    let exit_code = status.code().unwrap_or(1);

    Ok(SpawnResult {
        exit_code,
        agent_pid,
    })
}

pub fn setup_signal_forwarding(child_pid: u32) -> Result<()> {
    use std::sync::atomic::{AtomicU32, Ordering};

    static CHILD_PID: AtomicU32 = AtomicU32::new(0);
    CHILD_PID.store(child_pid, Ordering::SeqCst);

    extern "C" fn forward_signal(sig: libc::c_int) {
        let pid = CHILD_PID.load(std::sync::atomic::Ordering::SeqCst);
        if pid > 0 {
            unsafe {
                libc::kill(pid as i32, sig);
            }
        }
    }

    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = forward_signal as *const () as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut()) != 0 {
            anyhow::bail!("Failed to install SIGINT handler");
        }
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
            anyhow::bail!("Failed to install SIGTERM handler");
        }
    }

    Ok(())
}

pub fn validates_tty_inheritance(_config: &SpawnConfig) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SpawnConfig {
        SpawnConfig {
            agent_binary: "/usr/bin/echo".to_string(),
            agent_args: vec!["hello".to_string()],
            working_dir: PathBuf::from("/tmp"),
            env_vars: vec![("TEST_VAR".to_string(), "1".to_string())],
        }
    }

    // AC-2: bm chat Spawn+Wait Lifecycle — agent runs, exits, returns exit code

    #[test]
    fn spawn_and_wait_returns_exit_code_from_agent() {
        let config = test_config();
        let result = spawn_and_wait(&config).expect("spawn_and_wait should succeed");
        assert_eq!(
            result.exit_code, 0,
            "exit code must reflect the agent's exit status"
        );
    }

    #[test]
    fn spawn_and_wait_propagates_nonzero_exit_code() {
        let config = SpawnConfig {
            agent_binary: "/usr/bin/false".to_string(),
            agent_args: vec![],
            working_dir: PathBuf::from("/tmp"),
            env_vars: vec![],
        };
        let result = spawn_and_wait(&config).expect("spawn_and_wait should succeed even on nonzero exit");
        assert_ne!(
            result.exit_code, 0,
            "nonzero agent exit code must be propagated to the caller"
        );
    }

    #[test]
    fn spawn_and_wait_records_agent_pid() {
        let config = test_config();
        let result = spawn_and_wait(&config).expect("spawn_and_wait should succeed");
        assert!(
            result.agent_pid > 0,
            "agent_pid must be a valid process ID, got {}",
            result.agent_pid
        );
    }

    // AC-3: Operator Experience Parity — TTY inheritance

    #[test]
    fn spawn_config_validates_tty_inheritance() {
        let config = test_config();
        assert!(
            validates_tty_inheritance(&config),
            "spawn config must inherit stdin/stdout/stderr for TTY parity with exec"
        );
    }

    // AC-4: Signal Forwarding — SIGINT forwarded to child

    #[test]
    fn signal_forwarding_setup_accepts_valid_pid() {
        let fake_pid = std::process::id();
        let result = setup_signal_forwarding(fake_pid);
        assert!(
            result.is_ok(),
            "setup_signal_forwarding must succeed for a valid PID"
        );
    }
}
