use anyhow::{bail, Result};

use crate::formation::lima::{self, Lima};

/// Prints the rendered Lima template to stdout and exits.
pub fn render(name: Option<String>, cpus: u32, memory: &str, disk: &str) {
    let vm_name = name.unwrap_or_else(|| "bm-default".to_string());
    print!("{}", lima::generate_template(&vm_name, cpus, memory, disk));
}

/// Handles `bm bootstrap` in non-interactive mode.
pub fn run_non_interactive(
    name: Option<String>,
    cpus: u32,
    memory: &str,
    disk: &str,
) -> Result<()> {
    let vm_name = name.ok_or_else(|| anyhow::anyhow!("--non-interactive requires --name <vm-name>"))?;
    if vm_name.is_empty() {
        bail!("VM name cannot be empty.");
    }

    let lima = Lima::check_prerequisites()?;
    let result = lima.bootstrap(&vm_name, cpus, memory, disk)?;

    if result.created {
        eprintln!("VM '{}' created.", result.vm_name);
    }
    if result.started {
        eprintln!("VM '{}' started.", result.vm_name);
    }
    eprintln!("VM '{}' is ready. Run `bm attach` to connect.", result.vm_name);

    Ok(())
}

/// Runs `bm bootstrap` as an interactive wizard.
pub fn run(
    non_interactive: bool,
    name: Option<String>,
    cpus: u32,
    memory: &str,
    disk: &str,
) -> Result<()> {
    if non_interactive {
        return run_non_interactive(name, cpus, memory, disk);
    }

    let lima = Lima::check_prerequisites()?;

    cliclack::intro("botminter — bootstrap a VM")?;

    let vm_name = resolve_vm_name(name)?;

    let cpus: u32 = cliclack::input("CPUs")
        .default_input(&cpus.to_string())
        .validate(|input: &String| {
            input.parse::<u32>()
                .map(|_| ())
                .map_err(|_| "Must be a positive integer")
        })
        .interact()
        .map(|s: String| s.parse().unwrap())?;

    let memory: String = cliclack::input("Memory")
        .default_input(memory)
        .interact()?;

    let disk: String = cliclack::input("Disk")
        .default_input(disk)
        .interact()?;

    let summary = format!(
        "VM: {}\nCPUs: {}\nMemory: {}\nDisk: {}",
        vm_name, cpus, memory, disk,
    );
    cliclack::log::info(summary)?;

    let confirm: bool = cliclack::confirm("Create this VM?")
        .initial_value(true)
        .interact()?;
    if !confirm {
        cliclack::outro("Aborted.")?;
        return Ok(());
    }

    let spinner = cliclack::spinner();
    spinner.start("Provisioning VM...");

    let result = lima.bootstrap(&vm_name, cpus, &memory, &disk)?;

    if result.created && result.started {
        spinner.stop("VM created and started");
    } else if result.created {
        spinner.stop("VM created (already running)");
    } else if result.started {
        spinner.stop("VM started (already existed)");
    } else {
        spinner.stop("VM already running");
    }

    cliclack::log::info(format!(
        "VM '{}' is ready.\nTemplate: {}\nRun `bm attach` to connect, then `bm init` inside the VM.",
        result.vm_name,
        result.template_path.display(),
    ))?;
    cliclack::outro("Ready to go!")?;

    Ok(())
}

fn resolve_vm_name(name: Option<String>) -> Result<String> {
    if let Some(n) = name {
        return Ok(n);
    }

    let name: String = cliclack::input("VM name")
        .default_input("bm-default")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("VM name cannot be empty")
            } else if input.contains('/') || input.contains(' ') {
                Err("VM name cannot contain '/' or spaces")
            } else {
                Ok(())
            }
        })
        .interact()?;

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_non_interactive_requires_name() {
        let result = run_non_interactive(None, 4, "8GiB", "100GiB");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("--name"));
    }

    #[test]
    fn resolve_vm_name_with_explicit_name() {
        let result = resolve_vm_name(Some("my-vm".to_string()));
        assert_eq!(result.unwrap(), "my-vm");
    }
}
