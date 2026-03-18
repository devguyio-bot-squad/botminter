//! Bootstrap utilities and case functions for `bm teams bootstrap` E2E tests.
//!
//! These cases are registered by the operator journey scenario, which already
//! runs `bm init` — so a team exists when bootstrap runs. Cases skip gracefully
//! when Lima is not available (like bridge cases skip without podman).
//!
//! VM cleanup is handled by TestEnv's Drop via the `lima_vm_name` export.

use std::process::Command;

use super::test_env::TestEnv;

/// VM name used by bootstrap e2e cases.
pub const VM_NAME: &str = "bm-e2e-bootstrap";

/// Checks if Lima can create VMs on this system.
///
/// Requires both `limactl` and `qemu-img` (used by Lima to manage disk images).
/// Some environments (containers, CI) have limactl installed but lack the
/// full QEMU toolchain needed for VM creation.
pub fn lima_vm_capable() -> bool {
    let limactl_ok = Command::new("limactl")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let qemu_ok = Command::new("qemu-img")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    limactl_ok && qemu_ok
}

/// Bootstrap creates a VM for the team.
pub fn bootstrap_vm_fn(
    team_name: &'static str,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        if !lima_vm_capable() {
            eprintln!("SKIP: Lima VM support not available (need limactl + qemu-img)");
            return;
        }

        // Pre-clean: ensure no leftover VM from a previous failed run
        let _ = env
            .command("limactl")
            .args(["delete", "--force", VM_NAME])
            .output();

        // Register VM for TestEnv cleanup
        env.export("lima_vm_name", VM_NAME);

        let output = env
            .command("bm")
            .args([
                "teams",
                "bootstrap",
                "--non-interactive",
                "--name",
                VM_NAME,
                "--cpus",
                "2",
                "--memory",
                "4GiB",
                "--disk",
                "20GiB",
                "-t",
                team_name,
            ])
            .output();
        assert!(
            output.status.success(),
            "bm teams bootstrap failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify VM exists and is running
        let list_out = env
            .command("limactl")
            .args(["list", "--json"])
            .output();
        let stdout = String::from_utf8_lossy(&list_out.stdout);
        let found_running = stdout.lines().any(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .map(|v| {
                    v.get("name").and_then(|n| n.as_str()) == Some(VM_NAME)
                        && v.get("status").and_then(|s| s.as_str()) == Some("Running")
                })
                .unwrap_or(false)
        });
        assert!(
            found_running,
            "VM '{}' should be running after bootstrap",
            VM_NAME
        );

        // Verify config contains the VM
        let config_path = env.home().join(".botminter").join("config.yml");
        let config_content =
            std::fs::read_to_string(&config_path).expect("config.yml should exist");
        assert!(
            config_content.contains(VM_NAME),
            "config.yml should contain VM name '{}'",
            VM_NAME
        );
    }
}

/// Idempotent re-run of bootstrap should succeed without errors.
pub fn bootstrap_idempotent_fn(
    team_name: &'static str,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        if env.get_export("lima_vm_name").is_none() {
            eprintln!("SKIP: no VM created (Lima not available)");
            return;
        }

        let output = env
            .command("bm")
            .args([
                "teams",
                "bootstrap",
                "--non-interactive",
                "--name",
                VM_NAME,
                "--cpus",
                "2",
                "--memory",
                "4GiB",
                "--disk",
                "20GiB",
                "-t",
                team_name,
            ])
            .output();
        assert!(
            output.status.success(),
            "idempotent re-run should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Verify required tools are available inside the VM.
pub fn bootstrap_tools_fn(
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        if env.get_export("lima_vm_name").is_none() {
            eprintln!("SKIP: no VM created (Lima not available)");
            return;
        }

        for tool in &["bm", "ralph", "claude", "gh", "git", "just"] {
            let which_out = env
                .command("limactl")
                .args(["shell", VM_NAME, "--", "which", tool])
                .output();
            assert!(
                which_out.status.success(),
                "Tool '{}' not found in VM. stderr: {}",
                tool,
                String::from_utf8_lossy(&which_out.stderr)
            );
        }
    }
}

/// Explicit teardown — delete the VM and verify it's gone.
pub fn bootstrap_teardown_fn(
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        if env.get_export("lima_vm_name").is_none() {
            eprintln!("SKIP: no VM created (Lima not available)");
            return;
        }

        let del_out = env
            .command("limactl")
            .args(["delete", "--force", VM_NAME])
            .output();
        assert!(
            del_out.status.success(),
            "VM deletion should succeed: {}",
            String::from_utf8_lossy(&del_out.stderr)
        );

        // Verify VM is gone
        let list_after = env
            .command("limactl")
            .args(["list", "--json"])
            .output();
        let after_stdout = String::from_utf8_lossy(&list_after.stdout);
        let still_exists = after_stdout.lines().any(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .map(|v| v.get("name").and_then(|n| n.as_str()) == Some(VM_NAME))
                .unwrap_or(false)
        });
        assert!(!still_exists, "VM should be gone after delete");

        // Clear the export so TestEnv Drop doesn't try to delete again
        env.remove_export("lima_vm_name");
    }
}
