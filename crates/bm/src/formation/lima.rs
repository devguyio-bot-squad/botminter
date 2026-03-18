use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::{self, VmEntry};

/// Structured result from a bootstrap operation.
pub struct BootstrapResult {
    pub vm_name: String,
    pub template_path: PathBuf,
    pub created: bool,
    pub started: bool,
}

/// Status of a Lima VM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmStatus {
    Running,
    Stopped(String),
    NotFound,
}

/// Lima VM manager — wraps all `limactl` interactions.
pub struct Lima;

impl Lima {
    /// Checks that `limactl` (and `qemu-img` on Linux) are available.
    pub fn check_prerequisites() -> Result<Self> {
        if which::which("limactl").is_err() {
            bail!(
                "limactl is not installed.\n\n\
                 Install Lima to provision VMs:\n\
                 \n\
                 macOS:   brew install lima\n\
                 Linux:   brew install lima (or download from https://github.com/lima-vm/lima/releases)\n\
                 Windows: See https://lima-vm.io/docs/installation/ (requires WSL2)\n\
                 \n\
                 Then run `bm bootstrap` again."
            );
        }

        // The QEMU driver calls qemu-img directly (not through Lima's native fallback)
        // to inspect the downloaded image before booting. On Linux, QEMU is the only
        // available backend, so qemu-img is required.
        if cfg!(target_os = "linux") && which::which("qemu-img").is_err() {
            bail!(
                "qemu-img is not installed.\n\n\
                 Lima's QEMU backend requires qemu-img to inspect disk images.\n\
                 \n\
                 Fedora:  sudo dnf install qemu-img\n\
                 Ubuntu:  sudo apt install qemu-utils\n\
                 Arch:    sudo pacman -S qemu-img\n\
                 \n\
                 Then run `bm bootstrap` again."
            );
        }

        Ok(Lima)
    }

    /// Provisions a VM end-to-end: generate template, create, start, register.
    /// Idempotent — skips steps that are already done.
    pub fn bootstrap(
        &self,
        vm_name: &str,
        cpus: u32,
        memory: &str,
        disk: &str,
    ) -> Result<BootstrapResult> {
        let template = generate_template(vm_name, cpus, memory, disk);
        let template_path = persist_template(vm_name, &template)?;

        let created = match self.status(vm_name)? {
            VmStatus::NotFound => {
                self.create(vm_name, &template_path)?;
                true
            }
            _ => false,
        };

        let started = match self.status(vm_name)? {
            VmStatus::Running => false,
            _ => {
                self.start(vm_name)?;
                true
            }
        };

        self.register(vm_name)?;

        Ok(BootstrapResult {
            vm_name: vm_name.to_string(),
            template_path,
            created,
            started,
        })
    }

    /// Queries the status of a VM by name.
    pub fn status(&self, vm_name: &str) -> Result<VmStatus> {
        let output = Command::new("limactl")
            .args(["list", "--json"])
            .output()
            .context("Failed to run limactl list")?;

        if !output.status.success() {
            return Ok(VmStatus::NotFound);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // limactl list --json outputs one JSON object per line (JSONL)
        for line in stdout.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if v.get("name").and_then(|n| n.as_str()) == Some(vm_name) {
                    let status = v
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    return if status == "Running" {
                        Ok(VmStatus::Running)
                    } else {
                        Ok(VmStatus::Stopped(status))
                    };
                }
            }
        }

        Ok(VmStatus::NotFound)
    }

    /// Starts a VM via `limactl start`.
    pub fn start(&self, vm_name: &str) -> Result<()> {
        let output = Command::new("limactl")
            .args(["start", vm_name])
            .output()
            .context("Failed to run limactl start")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("limactl start failed:\n{}", stderr);
        }

        Ok(())
    }

    /// Execs into a VM shell via `limactl shell`. Replaces the current process.
    pub fn exec_shell(&self, vm_name: &str) -> Result<()> {
        use std::os::unix::process::CommandExt as _;

        let err = Command::new("limactl")
            .args(["shell", vm_name])
            .exec();

        // exec() only returns on error
        bail!("Failed to exec into VM '{}': {}", vm_name, err);
    }

    /// Creates a VM via `limactl create`.
    fn create(&self, vm_name: &str, template_path: &Path) -> Result<()> {
        let output = Command::new("limactl")
            .args(["create", "--name", vm_name, "--tty=false"])
            .arg(template_path)
            .output()
            .context("Failed to run limactl create")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "limactl create failed:\n{}\n\nTemplate saved at: {}",
                stderr,
                template_path.display(),
            );
        }

        Ok(())
    }

    /// Registers the VM in ~/.botminter/config.yml. Idempotent.
    fn register(&self, vm_name: &str) -> Result<()> {
        let mut cfg = config::load_or_default();

        if cfg.vms.iter().any(|v| v.name == vm_name) {
            return Ok(());
        }

        cfg.vms.push(VmEntry {
            name: vm_name.to_string(),
        });

        config::save(&cfg)?;
        Ok(())
    }
}

/// Persists the Lima template to `~/.config/botminter/vms/<name>.yaml`.
fn persist_template(vm_name: &str, template: &str) -> Result<PathBuf> {
    let config_base = dirs::config_dir().context("Could not determine config directory")?;
    let vms_dir = config_base.join("botminter").join("vms");
    std::fs::create_dir_all(&vms_dir)
        .with_context(|| format!("Failed to create vms directory at {}", vms_dir.display()))?;

    let template_path = vms_dir.join(format!("{}.yaml", vm_name));
    std::fs::write(&template_path, template)
        .with_context(|| format!("Failed to write Lima template to {}", template_path.display()))?;

    Ok(template_path)
}

/// Generates the Lima YAML template for a BotMinter VM.
pub fn generate_template(vm_name: &str, cpus: u32, memory: &str, disk: &str) -> String {
    let bm_install_url = "https://github.com/botminter/botminter/releases/download/v0.2.0-pre-alpha/bm-installer.sh";
    let ralph_install_url = "https://github.com/botminter/ralph-orchestrator/releases/download/v2.8.1-bm.137b1b3.1/ralph-cli-installer.sh";

    format!(
        r#"# Lima template generated by `bm bootstrap` for VM "{vm_name}"
minimumLimaVersion: "2.0.0"

images:
- location: "https://download.fedoraproject.org/pub/fedora/linux/releases/43/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-43-1.6.x86_64.qcow2"
  arch: "x86_64"
  digest: "sha256:846574c8a97cd2d8dc1f231062d73107cc85cbbbda56335e264a46e3a6c8ab2f"
- location: "https://download.fedoraproject.org/pub/fedora/linux/releases/43/Cloud/aarch64/images/Fedora-Cloud-Base-Generic-43-1.6.aarch64.qcow2"
  arch: "aarch64"
  digest: "sha256:66031aea9ec61e6d0d5bba12b9454e80ca94e8a79c913d37ded4c60311705b8b"

ssh:
  # ssh.overVsock does not work with Fedora 43 due to a SELinux policy issue
  # https://github.com/lima-vm/lima/issues/4334#issuecomment-3561294333
  overVsock: false

cpus: {cpus}
memory: "{memory}"
disk: "{disk}"

mounts:
- location: "~"
  writable: true

containerd:
  system: false
  user: false

provision:
- mode: system
  script: |
    #!/bin/bash
    set -eux -o pipefail

    # System packages
    dnf install -y git jq curl gnome-keyring podman nodejs npm

    # gh CLI (GitHub CLI)
    dnf install -y 'dnf-command(config-manager)'
    dnf config-manager addrepo --from-repofile=https://cli.github.com/packages/rpm/gh-cli.repo
    dnf install -y gh

    # just (command runner)
    dnf install -y just

    # claude (Claude Code) — npm global install
    if ! command -v claude >/dev/null 2>&1; then
      npm install -g @anthropic-ai/claude-code
    fi

- mode: user
  script: |
    #!/bin/bash
    set -eux -o pipefail

    # bm (BotMinter CLI) — cargo-dist installer
    if ! command -v bm >/dev/null 2>&1; then
      curl --proto '=https' --tlsv1.2 -LsSf "{bm_install_url}" | sh
    fi

    # ralph (Ralph Orchestrator CLI) — cargo-dist installer from botminter fork
    if ! command -v ralph >/dev/null 2>&1; then
      curl --proto '=https' --tlsv1.2 -LsSf "{ralph_install_url}" | sh
    fi

probes:
- mode: readiness
  description: All BotMinter tools installed
  script: |
    #!/bin/bash
    set -eux -o pipefail
    if ! timeout 120s bash -c "until command -v bm && command -v ralph && command -v claude && command -v gh && command -v git && command -v just; do sleep 5; done"; then
      echo >&2 "BotMinter tools are not fully installed yet"
      exit 1
    fi
  hint: |
    Tool installation is still in progress. Check /var/log/cloud-init-output.log in the guest.

message: |
  BotMinter VM "{{{{.Name}}}}" is ready!
  Run `bm attach` to connect, then `bm init` inside the VM to set up your team.
"#,
        vm_name = vm_name,
        cpus = cpus,
        memory = memory,
        disk = disk,
        bm_install_url = bm_install_url,
        ralph_install_url = ralph_install_url,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_template_contains_required_tools() {
        let template = generate_template("test-vm", 4, "8GiB", "100GiB");

        assert!(template.contains("Fedora-Cloud-Base-Generic-43"));
        assert!(template.contains("x86_64"));
        assert!(template.contains("aarch64"));

        assert!(template.contains("git"));
        assert!(template.contains("jq"));
        assert!(template.contains("curl"));
        assert!(template.contains("gnome-keyring"));
        assert!(template.contains("podman"));
        assert!(template.contains("gh"));
        assert!(template.contains("just"));

        assert!(template.contains("nodejs"));
        assert!(template.contains("npm"));

        assert!(template.contains("botminter/botminter"));
        assert!(template.contains("botminter/ralph-orchestrator"));
        assert!(template.contains("@anthropic-ai/claude-code"));

        assert!(template.contains("cpus: 4"));
        assert!(template.contains("memory: \"8GiB\""));
        assert!(template.contains("disk: \"100GiB\""));
    }

    #[test]
    fn generate_template_custom_resources() {
        let template = generate_template("custom", 8, "16GiB", "200GiB");
        assert!(template.contains("cpus: 8"));
        assert!(template.contains("memory: \"16GiB\""));
        assert!(template.contains("disk: \"200GiB\""));
    }

    #[test]
    fn generate_template_has_readiness_probe() {
        let template = generate_template("probe-vm", 4, "8GiB", "100GiB");
        assert!(template.contains("probes:"));
        assert!(template.contains("mode: readiness"));
        assert!(template.contains("command -v bm"));
        assert!(template.contains("command -v ralph"));
        assert!(template.contains("command -v claude"));
    }

    #[test]
    fn generate_template_is_idempotent() {
        let template = generate_template("idemp-vm", 4, "8GiB", "100GiB");
        assert!(template.contains("command -v bm >/dev/null 2>&1"));
        assert!(template.contains("command -v ralph >/dev/null 2>&1"));
        assert!(template.contains("command -v claude >/dev/null 2>&1"));
    }

    #[test]
    fn generate_template_home_mount_writable() {
        let template = generate_template("mount-vm", 4, "8GiB", "100GiB");
        assert!(template.contains("writable: true"));
    }

    #[test]
    fn generate_template_embeds_vm_name() {
        let template = generate_template("my-team", 4, "8GiB", "100GiB");
        assert!(template.contains(r#"for VM "my-team""#));
    }

    #[test]
    fn generate_template_is_valid_yaml() {
        let template = generate_template("yaml-check", 4, "8GiB", "100GiB");
        let parsed: serde_yml::Value = serde_yml::from_str(&template).unwrap();
        assert_eq!(
            parsed.get("cpus").and_then(|v| v.as_u64()),
            Some(4),
        );
        assert_eq!(
            parsed.get("memory").and_then(|v| v.as_str()),
            Some("8GiB"),
        );
    }

    #[test]
    fn generate_template_user_mode_provision() {
        let template = generate_template("mode-vm", 4, "8GiB", "100GiB");
        // bm and ralph install as user, not system
        assert!(template.contains("mode: user"));
        // Verify the user block contains bm/ralph but not claude
        let user_pos = template.find("mode: user").unwrap();
        let after_user = &template[user_pos..];
        assert!(after_user.contains("command -v bm"));
        assert!(after_user.contains("command -v ralph"));
    }

    #[test]
    fn generate_template_selinux_comment() {
        let template = generate_template("sel-vm", 4, "8GiB", "100GiB");
        assert!(template.contains("SELinux"));
        assert!(template.contains("lima-vm/lima/issues/4334"));
    }

    #[test]
    fn check_prerequisites_returns_result() {
        // In CI, limactl and qemu-img may or may not be available.
        let _ = Lima::check_prerequisites();
    }

    #[test]
    fn register_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join(".botminter").join("config.yml");

        let mut cfg = config::BotminterConfig {
            workzone: tmp.path().to_path_buf(),
            default_team: None,
            teams: Vec::new(),
            vms: vec![VmEntry {
                name: "existing-vm".to_string(),
            }],
            keyring_collection: None,
        };

        config::save_to(&config_path, &cfg).unwrap();

        assert!(cfg.vms.iter().any(|v| v.name == "existing-vm"));

        if !cfg.vms.iter().any(|v| v.name == "existing-vm") {
            cfg.vms.push(VmEntry {
                name: "existing-vm".to_string(),
            });
        }
        assert_eq!(cfg.vms.len(), 1);
    }
}
