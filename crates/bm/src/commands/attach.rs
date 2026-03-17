use std::io::IsTerminal;
use std::os::unix::process::CommandExt as _;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::{self, BotminterConfig};

/// Handles `bm attach [-t <team>]`.
pub fn run(team: Option<&str>) -> Result<()> {
    // 1. Check limactl prerequisite
    check_limactl()?;

    // 2. Load config and resolve VM
    let cfg = config::load_or_default();
    let vm_name = resolve_vm(&cfg, team)?;

    // 3. Check VM status and start if needed
    ensure_vm_running(&vm_name)?;

    // 4. Exec into limactl shell (replaces current process)
    exec_shell(&vm_name)
}

fn check_limactl() -> Result<()> {
    if which::which("limactl").is_ok() {
        return Ok(());
    }
    bail!(
        "limactl is not installed.\n\n\
         Install Lima to provision VMs:\n\
         \n\
         macOS:   brew install lima\n\
         Linux:   brew install lima (or download from https://github.com/lima-vm/lima/releases)\n\
         Windows: See https://lima-vm.io/docs/installation/ (requires WSL2)\n\
         \n\
         Then run `bm bootstrap` to create a VM."
    );
}

/// Resolves which VM to attach to using 3-step resolution:
/// 1. If `-t <team>` given and that team has `vm` set → use it
/// 2. If exactly one entry in `vms` → use it
/// 3. If multiple → prompt (or error if non-interactive)
fn resolve_vm(cfg: &BotminterConfig, team: Option<&str>) -> Result<String> {
    // Step 1: team flag → team's VM
    if let Some(team_flag) = team {
        if let Ok(team_entry) = config::resolve_team(cfg, Some(team_flag)) {
            if let Some(ref vm) = team_entry.vm {
                return Ok(vm.clone());
            }
        }
    } else if let Ok(team_entry) = config::resolve_team(cfg, None) {
        if let Some(ref vm) = team_entry.vm {
            return Ok(vm.clone());
        }
    }

    // Step 2: single VM → use it
    if cfg.vms.is_empty() {
        bail!("No VM found. Run `bm bootstrap` first.");
    }

    if cfg.vms.len() == 1 {
        return Ok(cfg.vms[0].name.clone());
    }

    // Step 3: multiple VMs → prompt or error
    if !std::io::stdin().is_terminal() {
        bail!(
            "Multiple VMs configured. Use `-t <team>` to select one, \
             or set `vm` on a team entry in ~/.botminter/config.yml."
        );
    }

    let names: Vec<&str> = cfg.vms.iter().map(|v| v.name.as_str()).collect();
    let selection: String = cliclack::select("Select a VM to attach to")
        .items(
            &names
                .iter()
                .map(|n| (n.to_string(), *n, ""))
                .collect::<Vec<_>>(),
        )
        .interact()?;

    Ok(selection)
}

/// Checks if the VM is running. If stopped, offers to start it.
fn ensure_vm_running(vm_name: &str) -> Result<()> {
    let status = vm_status(vm_name)?;

    match status.as_deref() {
        Some("Running") => Ok(()),
        Some(_) => {
            // VM exists but not running — offer to start
            println!("VM '{}' is not running.", vm_name);

            if std::io::stdin().is_terminal() {
                let start: bool = cliclack::confirm("Start the VM?")
                    .initial_value(true)
                    .interact()?;
                if !start {
                    bail!("VM '{}' is not running. Start it with `limactl start {}`.", vm_name, vm_name);
                }
            } else {
                println!("Starting VM '{}'...", vm_name);
            }

            let output = Command::new("limactl")
                .args(["start", vm_name])
                .output()
                .context("Failed to run limactl start")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("limactl start failed:\n{}", stderr);
            }

            println!("VM '{}' started.", vm_name);
            Ok(())
        }
        None => {
            bail!(
                "VM '{}' does not exist. Run `bm bootstrap --name {}` to create it.",
                vm_name,
                vm_name
            );
        }
    }
}

/// Queries limactl for the VM's status. Returns None if VM not found.
fn vm_status(vm_name: &str) -> Result<Option<String>> {
    let output = Command::new("limactl")
        .args(["list", "--json"])
        .output()
        .context("Failed to run limactl list")?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("name").and_then(|n| n.as_str()) == Some(vm_name) {
                return Ok(v.get("status").and_then(|s| s.as_str()).map(String::from));
            }
        }
    }

    Ok(None)
}

/// Replaces the current process with `limactl shell <vm-name>`.
fn exec_shell(vm_name: &str) -> Result<()> {
    let err = Command::new("limactl")
        .args(["shell", vm_name])
        .exec();

    // exec() only returns on error
    bail!("Failed to exec into VM '{}': {}", vm_name, err);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BotminterConfig, Credentials, TeamEntry, VmEntry};
    use std::path::PathBuf;

    fn make_config(
        vms: Vec<VmEntry>,
        teams: Vec<TeamEntry>,
        default_team: Option<String>,
    ) -> BotminterConfig {
        BotminterConfig {
            workzone: PathBuf::from("/tmp/test"),
            default_team,
            teams,
            vms,
            keyring_collection: None,
        }
    }

    fn make_team(name: &str, vm: Option<&str>) -> TeamEntry {
        TeamEntry {
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{}", name)),
            profile: "scrum-compact".to_string(),
            github_repo: format!("org/{}", name),
            credentials: Credentials::default(),
            coding_agent: None,
            project_number: None,
            bridge_lifecycle: Default::default(),
            vm: vm.map(String::from),
        }
    }

    #[test]
    fn resolve_vm_no_vms_configured() {
        let cfg = make_config(vec![], vec![], None);
        let result = resolve_vm(&cfg, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No VM found"));
        assert!(err.contains("bm bootstrap"));
    }

    #[test]
    fn resolve_vm_single_vm_auto_selects() {
        let cfg = make_config(
            vec![VmEntry {
                name: "bm-solo".to_string(),
            }],
            vec![],
            None,
        );
        let result = resolve_vm(&cfg, None).unwrap();
        assert_eq!(result, "bm-solo");
    }

    #[test]
    fn resolve_vm_team_flag_with_vm() {
        let cfg = make_config(
            vec![
                VmEntry {
                    name: "vm-a".to_string(),
                },
                VmEntry {
                    name: "vm-b".to_string(),
                },
            ],
            vec![make_team("alpha", Some("vm-a"))],
            None,
        );
        let result = resolve_vm(&cfg, Some("alpha")).unwrap();
        assert_eq!(result, "vm-a");
    }

    #[test]
    fn resolve_vm_team_flag_without_vm_falls_through() {
        // Team exists but has no VM set; single global VM available
        let cfg = make_config(
            vec![VmEntry {
                name: "only-vm".to_string(),
            }],
            vec![make_team("beta", None)],
            None,
        );
        let result = resolve_vm(&cfg, Some("beta")).unwrap();
        assert_eq!(result, "only-vm");
    }

    #[test]
    fn resolve_vm_default_team_with_vm() {
        let cfg = make_config(
            vec![
                VmEntry {
                    name: "vm-x".to_string(),
                },
                VmEntry {
                    name: "vm-y".to_string(),
                },
            ],
            vec![make_team("default-team", Some("vm-y"))],
            Some("default-team".to_string()),
        );
        // No explicit team flag — should use default team's VM
        let result = resolve_vm(&cfg, None).unwrap();
        assert_eq!(result, "vm-y");
    }

    #[test]
    fn resolve_vm_multiple_vms_no_tty_errors() {
        // Multiple VMs, no team VM, no TTY → should error
        let cfg = make_config(
            vec![
                VmEntry {
                    name: "vm-1".to_string(),
                },
                VmEntry {
                    name: "vm-2".to_string(),
                },
            ],
            vec![],
            None,
        );
        // In CI/test, stdin is not a terminal, so this should error
        let result = resolve_vm(&cfg, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Multiple VMs"));
    }

    #[test]
    fn resolve_vm_team_not_found_falls_through() {
        // Team flag given but team doesn't exist; single VM available
        let cfg = make_config(
            vec![VmEntry {
                name: "fallback".to_string(),
            }],
            vec![],
            None,
        );
        let result = resolve_vm(&cfg, Some("nonexistent")).unwrap();
        assert_eq!(result, "fallback");
    }

    #[test]
    fn check_limactl_returns_result() {
        // Just verifies the function runs without panic.
        let _ = check_limactl();
    }
}
