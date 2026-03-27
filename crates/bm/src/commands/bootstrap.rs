use std::io::IsTerminal;

use anyhow::{bail, Result};

use crate::config;
use crate::formation::lima::{self, Lima};

/// Prints the rendered Lima template to stdout and exits.
///
/// This is a dry-run — it does not require a team to exist.
pub fn render(name: Option<String>, cpus: u32, memory: &str, disk: &str, _team: Option<&str>) {
    let vm_name = name.unwrap_or_else(|| "bm-default".to_string());
    let bm_config = config::config_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.botminter".to_string());
    let xdg_config = dirs::config_dir()
        .map(|p| p.join("botminter").display().to_string())
        .unwrap_or_else(|| "~/.config/botminter".to_string());
    print!(
        "{}",
        lima::generate_template(&vm_name, cpus, memory, disk, &[&bm_config, &xdg_config], None, &[])
    );
}

/// Runs `bm runtime create` — provisions a Lima VM for a team.
pub fn run(
    non_interactive: bool,
    name: Option<String>,
    cpus: u32,
    memory: &str,
    disk: &str,
    env_vars: &[String],
    team: Option<&str>,
) -> Result<()> {
    // Team context: require a team exists
    let mut cfg = config::load()?;
    let team_entry = config::resolve_team(&cfg, team)?;
    let team_name = team_entry.name.clone();
    let gh_token = config::require_gh_token(team_entry)?;

    let bm_config = config::config_dir()?.display().to_string();
    let xdg_config = dirs::config_dir()
        .map(|p| p.join("botminter").display().to_string())
        .unwrap_or_else(|| "~/.config/botminter".to_string());
    let mounts: &[&str] = &[&bm_config, &xdg_config];

    let lima = Lima::check_prerequisites()?;

    if non_interactive {
        let vm_name =
            name.ok_or_else(|| anyhow::anyhow!("--non-interactive requires --name <vm-name>"))?;
        if vm_name.is_empty() {
            bail!("VM name cannot be empty.");
        }
        if vm_name.contains('/') || vm_name.contains(' ') {
            bail!("VM name cannot contain '/' or spaces.");
        }

        let parsed_env = parse_env_vars(env_vars)?;
        let result = lima.bootstrap(&vm_name, cpus, memory, disk, mounts, Some(&gh_token), &parsed_env)?;

        if result.created {
            eprintln!("VM '{}' created.", result.vm_name);
        }
        if result.started {
            eprintln!("VM '{}' started.", result.vm_name);
        }
        eprintln!(
            "VM '{}' is ready. Run `bm attach` to connect.",
            result.vm_name
        );

        // Associate VM with team
        associate_vm_with_team(&mut cfg, &team_name, &result.vm_name)?;

        return Ok(());
    }

    cliclack::intro("botminter — bootstrap a VM")?;

    let vm_name = resolve_vm_name(name)?;

    let cpus: u32 = cliclack::input("CPUs")
        .default_input(&cpus.to_string())
        .validate(|input: &String| {
            input
                .parse::<u32>()
                .map(|_| ())
                .map_err(|_| "Must be a positive integer")
        })
        .interact()
        .map(|s: String| s.parse().unwrap())?;

    let memory: String = cliclack::input("Memory").default_input(memory).interact()?;

    let disk: String = cliclack::input("Disk").default_input(disk).interact()?;

    // Collect environment variables interactively
    let mut collected_env: Vec<(String, String)> = parse_env_vars(env_vars)?;
    cliclack::log::info("Environment variables (enter KEY=VALUE pairs, empty to finish)")?;
    loop {
        let entry: String = cliclack::input("ENV (KEY=VALUE)")
            .placeholder("ANTHROPIC_API_KEY=sk-ant-...")
            .default_input("")
            .required(false)
            .interact()?;
        let entry = entry.trim().to_string();
        if entry.is_empty() {
            break;
        }
        match entry.split_once('=') {
            Some((key, value)) if !key.is_empty() => {
                collected_env.push((key.to_string(), value.to_string()));
            }
            _ => {
                cliclack::log::warning("Invalid format. Use KEY=VALUE (e.g. ANTHROPIC_API_KEY=sk-ant-...)")?;
            }
        }
    }
    if collected_env.is_empty() {
        cliclack::log::info("No environment variables configured.")?;
    } else {
        let env_keys: Vec<&str> = collected_env.iter().map(|(k, _)| k.as_str()).collect();
        cliclack::log::info(format!("Environment variables: {}", env_keys.join(", ")))?;
    }

    let summary = format!(
        "VM: {}\nCPUs: {}\nMemory: {}\nDisk: {}\nEnv vars: {}",
        vm_name, cpus, memory, disk, collected_env.len(),
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

    let result = lima.bootstrap(&vm_name, cpus, &memory, &disk, mounts, Some(&gh_token), &collected_env)?;

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
        "VM '{}' is ready.\nTemplate: {}\nRun `bm attach` to connect.",
        result.vm_name,
        result.template_path.display(),
    ))?;
    cliclack::outro("Ready to go!")?;

    // Associate VM with team
    associate_vm_with_team(&mut cfg, &team_name, &result.vm_name)?;

    Ok(())
}

/// Parses `KEY=VALUE` strings into `(key, value)` tuples.
fn parse_env_vars(env_vars: &[String]) -> Result<Vec<(String, String)>> {
    env_vars
        .iter()
        .map(|s| {
            s.split_once('=')
                .filter(|(k, _)| !k.is_empty())
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| anyhow::anyhow!("Invalid --env value '{}'. Expected KEY=VALUE format.", s))
        })
        .collect()
}

/// Runs `bm runtime delete` — deletes a Lima VM and removes it from config.
pub fn delete(name: &str, force: bool) -> Result<()> {
    let lima = Lima::check_prerequisites()?;

    if !force && std::io::stdin().is_terminal() {
        let confirm: bool = cliclack::confirm(format!("Delete VM '{}'? This cannot be undone.", name))
            .initial_value(false)
            .interact()?;
        if !confirm {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    let result = lima.delete(name)?;

    if result.existed {
        eprintln!("VM '{}' deleted.", result.vm_name);
    } else {
        eprintln!("VM '{}' was not found (already deleted).", result.vm_name);
    }
    eprintln!("Config updated.");

    Ok(())
}

/// Associates a VM with a team in config, then saves.
fn associate_vm_with_team(
    cfg: &mut config::BotminterConfig,
    team_name: &str,
    vm_name: &str,
) -> Result<()> {
    if let Some(entry) = cfg.teams.iter_mut().find(|t| t.name == team_name) {
        entry.vm = Some(vm_name.to_string());
    }
    config::save(cfg)?;
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
    fn resolve_vm_name_with_explicit_name() {
        let result = resolve_vm_name(Some("my-vm".to_string()));
        assert_eq!(result.unwrap(), "my-vm");
    }
}
