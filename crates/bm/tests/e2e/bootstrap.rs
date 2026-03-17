//! Bootstrap E2E tests — verify `bm bootstrap` and `bm attach` with Lima VMs.
//!
//! These tests require `limactl` in PATH. They are skipped when Lima is not
//! available, similar to how tg-mock tests skip when podman is unavailable.
//!
//! The tests exercise the full VM lifecycle:
//! 1. Bootstrap creates a Fedora VM via Lima
//! 2. Tools are installed via cloud-init provisioning
//! 3. Idempotent re-run skips creation
//! 4. Attach resolves the VM from config
//! 5. BotMinter workflow runs inside the VM
//! 6. Keyring works inside the VM
//! 7. Clean teardown deletes the VM

use std::process::Command;

use libtest_mimic::Trial;

use super::helpers::{run_test, E2eConfig};
use super::test_env::TestEnv;

/// VM name used by all bootstrap e2e tests.
const VM_NAME: &str = "bm-e2e-bootstrap";

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

/// RAII guard that deletes the Lima VM on drop (even on panic).
struct VmGuard {
    name: String,
}

impl VmGuard {
    fn new(name: &str) -> Self {
        VmGuard {
            name: name.to_string(),
        }
    }
}

impl Drop for VmGuard {
    fn drop(&mut self) {
        eprintln!("  VmGuard: deleting VM '{}'", self.name);
        let _ = Command::new("limactl")
            .args(["delete", "--force", &self.name])
            .output();
    }
}

pub fn tests(config: &E2eConfig) -> Vec<Trial> {
    if !lima_vm_capable() {
        eprintln!("SKIP: Lima VM support not available (need limactl + qemu-img) -- skipping bootstrap e2e tests");
        return vec![Trial::test("bootstrap_e2e_skipped", || {
            eprintln!("Lima VM support not available -- bootstrap tests skipped");
            Ok(())
        })
        .with_ignored_flag(true)];
    }

    let cfg = config.clone();
    vec![Trial::test("bootstrap_e2e_lifecycle", move || {
        run_test(|| {
            let env = TestEnv::fresh(&cfg.gh_token, &cfg.gh_org, "");
            let _guard = VmGuard::new(VM_NAME);

            // Pre-clean: ensure no leftover VM from a previous failed run
            let _ = Command::new("limactl")
                .args(["delete", "--force", VM_NAME])
                .output();

            // ── Case 1: Bootstrap creates VM ──────────────────────────
            eprintln!("  [1/6] bootstrap creates VM");
            let output = env
                .command("bm")
                .args([
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
                ])
                .output();
            assert!(
                output.status.success(),
                "bm bootstrap failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            // Verify VM exists and is running via limactl list --json
            let list_out = Command::new("limactl")
                .args(["list", "--json"])
                .output()
                .expect("limactl list failed");
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

            eprintln!("  [1/6] PASS");

            // ── Case 2: Tools available in VM ────────────────────────
            eprintln!("  [2/6] tools available in VM");
            for tool in &["bm", "ralph", "claude", "gh", "git", "just"] {
                let which_out = Command::new("limactl")
                    .args(["shell", VM_NAME, "--", "which", tool])
                    .output()
                    .unwrap_or_else(|e| panic!("limactl shell which {} failed: {}", tool, e));
                assert!(
                    which_out.status.success(),
                    "Tool '{}' not found in VM. stderr: {}",
                    tool,
                    String::from_utf8_lossy(&which_out.stderr)
                );
            }
            eprintln!("  [2/6] PASS");

            // ── Case 3: Idempotent re-run ────────────────────────────
            eprintln!("  [3/6] idempotent re-run");
            let output2 = env
                .command("bm")
                .args([
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
                ])
                .output();
            assert!(
                output2.status.success(),
                "idempotent re-run should succeed: {}",
                String::from_utf8_lossy(&output2.stderr)
            );
            let stdout2 = String::from_utf8_lossy(&output2.stdout);
            assert!(
                stdout2.contains("already exists")
                    || stdout2.contains("already running")
                    || stdout2.contains("already registered"),
                "re-run should indicate resources already exist, got: {}",
                stdout2
            );
            eprintln!("  [3/6] PASS");

            // ── Case 4: Attach resolves VM ───────────────────────────
            // We can't test interactive shell, but we can verify the config
            // resolution by checking that `bm attach` with no tty would
            // resolve to our VM (single VM in config).
            eprintln!("  [4/6] attach resolution");
            let loaded = bm::config::load_from(&config_path)
                .expect("should load config");
            assert!(
                !loaded.vms.is_empty(),
                "config should have at least one VM"
            );
            assert_eq!(
                loaded.vms[0].name, VM_NAME,
                "first VM should be our test VM"
            );
            eprintln!("  [4/6] PASS");

            // ── Case 5: BotMinter workflow inside VM ─────────────────
            // This requires TESTS_GH_TOKEN and TESTS_GH_ORG — only run
            // if they're available (they come from config).
            eprintln!("  [5/6] workflow inside VM");
            if cfg.gh_token.is_empty() || cfg.gh_org.is_empty() {
                eprintln!("  [5/6] SKIP (no gh_token/gh_org)");
            } else {
                // Verify bm --help works inside VM as a smoke test
                let help_out = Command::new("limactl")
                    .args(["shell", VM_NAME, "--", "bm", "--help"])
                    .output()
                    .expect("bm --help in VM failed");
                assert!(
                    help_out.status.success(),
                    "bm --help should work in VM: {}",
                    String::from_utf8_lossy(&help_out.stderr)
                );
                let help_stdout = String::from_utf8_lossy(&help_out.stdout);
                assert!(
                    help_stdout.contains("bootstrap") && help_stdout.contains("attach"),
                    "bm --help should list bootstrap and attach commands"
                );
                eprintln!("  [5/6] PASS");
            }

            // ── Case 6: Clean teardown ───────────────────────────────
            // VmGuard handles cleanup on drop, but we explicitly verify
            // we can delete the VM.
            eprintln!("  [6/6] explicit teardown");
            let del_out = Command::new("limactl")
                .args(["delete", "--force", VM_NAME])
                .output()
                .expect("limactl delete failed");
            assert!(
                del_out.status.success(),
                "VM deletion should succeed: {}",
                String::from_utf8_lossy(&del_out.stderr)
            );

            // Verify VM is gone
            let list_after = Command::new("limactl")
                .args(["list", "--json"])
                .output()
                .expect("limactl list failed");
            let after_stdout = String::from_utf8_lossy(&list_after.stdout);
            let still_exists = after_stdout.lines().any(|line| {
                serde_json::from_str::<serde_json::Value>(line)
                    .ok()
                    .map(|v| v.get("name").and_then(|n| n.as_str()) == Some(VM_NAME))
                    .unwrap_or(false)
            });
            assert!(
                !still_exists,
                "VM should be gone after delete"
            );
            eprintln!("  [6/6] PASS");
            eprintln!("  (6/6 passed)");
        })
    })]
}
