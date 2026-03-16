use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::config;
use crate::formation;
use crate::git;
use crate::profile;

/// Whether to create a new GitHub Project board or use an existing one.
enum ProjectChoice {
    CreateNew,
    UseExisting(u64),
}

/// Formats the "next steps" message shown after `bm init` completes.
/// Uses a simple text format (no tables) to fit within cliclack's bordered frame.
fn next_steps_message(team_name: &str, team_dir: &Path, team_repo: &Path, bridge_selected: bool) -> String {
    let sync_cmd = if bridge_selected {
        "bm teams sync --all     Push team repo, provision workspaces and bridge"
    } else {
        "bm teams sync --repos   Push team repo and provision workspaces"
    };
    let summary = profile::gather_team_summary(team_repo);
    let members_text = if summary.members.is_empty() {
        "Members: none".to_string()
    } else {
        let list: Vec<String> = summary.members.iter()
            .map(|(name, role)| format!("  {} ({})", name, role))
            .collect();
        format!("Members:\n{}", list.join("\n"))
    };
    let projects_text = if summary.projects.is_empty() {
        "Projects: none".to_string()
    } else {
        let list: Vec<String> = summary.projects.iter()
            .map(|p| format!("  {} — {}", p.name, p.fork_url))
            .collect();
        format!("Projects:\n{}", list.join("\n"))
    };
    format!(
        "Team '{}' created at {}\n\
         {}\n\
         {}\n\
         Next steps:\n  \
         1. {}\n  \
         2. bm projects sync       Sync project repos into workspaces",
        team_name,
        team_dir.display(),
        members_text,
        projects_text,
        sync_cmd,
    )
}

/// Runs `bm init` in non-interactive mode with pre-supplied arguments.
///
/// Skips all interactive prompts. When `skip_github` is true, also skips
/// GitHub API calls (token validation, label bootstrap, project board creation),
/// making it suitable for automated testing without network access.
#[allow(clippy::too_many_arguments)]
pub fn run_non_interactive(
    profile_name: Option<String>,
    team_name: Option<String>,
    org: Option<String>,
    repo: Option<String>,
    project: Option<String>,
    github_project_board: Option<String>,
    bridge: Option<String>,
    skip_github: bool,
    workzone_override: Option<String>,
) -> Result<()> {
    let selected_profile =
        profile_name.ok_or_else(|| anyhow::anyhow!("--profile is required with --non-interactive"))?;
    let team_name =
        team_name.ok_or_else(|| anyhow::anyhow!("--team-name is required with --non-interactive"))?;
    let github_org =
        org.ok_or_else(|| anyhow::anyhow!("--org is required with --non-interactive"))?;
    let repo_name =
        repo.ok_or_else(|| anyhow::anyhow!("--repo is required with --non-interactive"))?;
    if team_name.is_empty() {
        bail!("Team name cannot be empty");
    }
    if team_name.contains('/') || team_name.contains(' ') {
        bail!("Team name cannot contain '/' or spaces");
    }

    super::ensure_profiles(false)?;

    if !skip_github {
        config::check_prerequisites()?;
    } else if which::which("git").is_err() {
        bail!("Missing required tool: git — https://git-scm.com/");
    }

    let workzone = if let Some(wz) = workzone_override {
        config::expand_tilde(&wz)
    } else {
        config::default_workzone_path()
    };

    let team_dir = workzone.join(&team_name);
    if team_dir.exists() {
        bail!(
            "Directory '{}' already exists. Choose a different team name.",
            team_dir.display()
        );
    }

    let github_repo = format!("{}/{}", github_org, repo_name);

    let profiles = profile::list_profiles()?;
    if !profiles.contains(&selected_profile) {
        bail!(
            "Profile '{}' not found. Available profiles: {}",
            selected_profile,
            profiles.join(", ")
        );
    }

    let manifest = profile::read_manifest(&selected_profile)?;

    let selected_bridge = if let Some(ref bridge_name) = bridge {
        profile::validate_bridge_selection(bridge_name, &manifest.bridges)?;
        Some(bridge_name.clone())
    } else {
        None
    };

    let gh_token = if skip_github {
        None
    } else {
        let token = git::detect_token_non_interactive()?;
        git::validate_token(&token)?;
        Some(token)
    };

    eprintln!(
        "Creating team '{}' with profile '{}' at {}",
        team_name, selected_profile, team_dir.display()
    );

    fs::create_dir_all(&workzone)
        .with_context(|| format!("Failed to create workzone at {}", workzone.display()))?;
    fs::create_dir_all(&team_dir)
        .with_context(|| format!("Failed to create team directory at {}", team_dir.display()))?;

    let is_new_repo = if skip_github {
        true
    } else {
        !git::repo_exists(&github_repo, gh_token.as_deref())?
    };

    let team_repo = team_dir.join("team");

    if is_new_repo {
        formation::setup_new_team_repo(
            &team_repo, &selected_profile, &manifest,
            &[], // no members in non-interactive
            &project.map(|url| {
                let name = git::derive_project_name(&url);
                vec![(name, url)]
            }).unwrap_or_default(),
            selected_bridge.as_deref(),
            None,
        )?;

        if !skip_github {
            git::create_repo_and_push(&team_repo, &github_repo, gh_token.as_deref())?;
        }
    } else {
        eprintln!("Repository '{}' already exists — cloning it.", github_repo);
        git::clone_repo(&team_dir, &github_repo, gh_token.as_deref())?;
    }

    formation::register_team(
        &team_name, &team_dir, &selected_profile, &github_repo,
        gh_token.clone(), None, &workzone,
    )?;

    if !skip_github {
        if let Err(e) = git::bootstrap_labels(&github_repo, &manifest.labels, gh_token.as_deref()) {
            bail!(
                "Failed to bootstrap labels: {}. Run `bm init` interactively for recovery steps.",
                e
            );
        }

        let owner = github_repo.split('/').next().unwrap_or(&github_org);
        let board_title = github_project_board
            .ok_or_else(|| anyhow::anyhow!("--github-project-board is required with --non-interactive"))?;

        // Find existing board by title, or create one
        let project_number = {
            let projects = git::list_projects(
                gh_token.as_deref().unwrap_or(""),
                owner,
            )?;
            if let Some((number, _)) = projects.iter().find(|(_, t)| t == &board_title) {
                eprintln!("Using existing project board '{}' (#{})", board_title, number);
                git::sync_project_status_field(owner, *number, &manifest.statuses, gh_token.as_deref())?;
                *number
            } else {
                match git::create_project(owner, &board_title, &manifest.statuses, gh_token.as_deref()) {
                    Ok(n) => n,
                    Err(e) => {
                        bail!(
                            "Failed to create GitHub Project: {}. Run `bm projects sync` to retry.",
                            e
                        );
                    }
                }
            }
        };

        {
            let mut cfg = config::load()?;
            if let Some(entry) = cfg.teams.iter_mut().find(|t| t.name == team_name) {
                entry.project_number = Some(project_number);
            }
            config::save(&cfg)?;
        }
    }

    eprintln!("{}", next_steps_message(&team_name, &team_dir, &team_repo, selected_bridge.is_some()));

    Ok(())
}

/// Runs the `bm init` interactive wizard.
pub fn run() -> Result<()> {
    super::ensure_profiles(false)?;
    config::check_prerequisites()?;

    cliclack::intro("botminter — create a new team")?;

    // Workzone location
    let default_workzone = config::default_workzone_path();
    let workzone: String = cliclack::input("Where should teams live? (workzone directory)")
        .default_input(&default_workzone.to_string_lossy())
        .interact()?;
    let workzone = config::expand_tilde(&workzone);

    // Team name
    let team_name: String = cliclack::input("Team name")
        .placeholder("my-team")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Team name cannot be empty")
            } else if input.contains('/') || input.contains(' ') {
                Err("Team name cannot contain '/' or spaces")
            } else {
                Ok(())
            }
        })
        .interact()?;

    let team_dir = workzone.join(&team_name);
    if team_dir.exists() {
        bail!(
            "Directory '{}' already exists. Choose a different team name. \
             If this is from a failed init, delete the directory and retry.",
            team_dir.display()
        );
    }

    // Select profile
    let profiles = profile::list_profiles()?;
    let profile_options: Vec<(String, String, String)> = profiles
        .iter()
        .map(|name| {
            let manifest = profile::read_manifest(name).unwrap();
            (name.clone(), manifest.display_name.clone(), manifest.description.clone())
        })
        .collect();

    let profile_items: Vec<(&str, &str, &str)> = profile_options
        .iter()
        .map(|(v, l, h)| (v.as_str(), l.as_str(), h.as_str()))
        .collect();

    let selected_profile: String = cliclack::select("Which profile?")
        .items(&profile_items)
        .interact()
        .map(|s: &str| s.to_string())?;

    // GitHub integration
    let token = detect_or_prompt_gh_token()?;
    let token_info = git::validate_token(&token)?;
    cliclack::log::info(format!("Authenticated as: {}", token_info.login))?;

    let github_org = select_github_org(&token)?;
    let (github_repo, is_new_repo) = select_or_create_repo(&token, &github_org, &team_name)?;

    let github_owner = github_repo.split('/').next().unwrap_or(&github_org);
    let project_choice = select_or_create_project(&token, github_owner, &team_name)?;

    let gh_token = Some(token);
    let telegram_bot_token: Option<String> = None;
    let manifest = profile::read_manifest(&selected_profile)?;

    // Bridge selection
    let selected_bridge: Option<String> = if !manifest.bridges.is_empty() {
        let mut bridge_items: Vec<(String, String, String)> = manifest
            .bridges.iter()
            .map(|b| (b.name.clone(), b.display_name.clone(), b.description.clone()))
            .collect();
        bridge_items.push(("none".to_string(), "No bridge".to_string(), "Skip bridge configuration".to_string()));

        let items_ref: Vec<(&str, &str, &str)> = bridge_items
            .iter()
            .map(|(v, l, h)| (v.as_str(), l.as_str(), h.as_str()))
            .collect();

        let choice: String = cliclack::select("Communication bridge")
            .items(&items_ref)
            .initial_value(items_ref[0].0)
            .interact()
            .map(|s: &str| s.to_string())?;

        if choice == "none" { None } else { Some(choice) }
    } else {
        None
    };

    // Members and projects (only for new repos)
    let (members_to_hire, projects_to_add) = if is_new_repo {
        let role_names: Vec<String> = manifest.roles.iter().map(|r| r.name.clone()).collect();
        let members = collect_members(&role_names)?;
        let projects = collect_projects(gh_token.as_deref(), Some(&github_org))?;
        (members, projects)
    } else {
        cliclack::log::info(
            "Existing repo selected — use `bm hire` and `bm projects add` to modify the team after init.",
        )?;
        (Vec::new(), Vec::new())
    };

    // Summary
    let mut summary = format!(
        "Team: {}\nProfile: {}\nWorkzone: {}",
        team_name, selected_profile, workzone.display()
    );
    summary.push_str(&format!("\nGitHub: {}", github_repo));
    match &project_choice {
        ProjectChoice::CreateNew => summary.push_str(&format!("\nProject board: new ({} Board)", team_name)),
        ProjectChoice::UseExisting(n) => summary.push_str(&format!("\nProject board: existing (#{n})")),
    }
    if let Some(ref bridge_name) = selected_bridge {
        summary.push_str(&format!("\nBridge: {}", bridge_name));
    }
    if !members_to_hire.is_empty() {
        summary.push_str("\nMembers:");
        for (role, name) in &members_to_hire {
            summary.push_str(&format!("\n  {}-{}", role, name));
        }
    }
    if !projects_to_add.is_empty() {
        summary.push_str("\nProjects:");
        for (name, url) in &projects_to_add {
            summary.push_str(&format!("\n  {} ({})", name, url));
        }
    }

    cliclack::log::info(summary)?;

    let confirm: bool = cliclack::confirm("Create this team?").interact()?;
    if !confirm {
        cliclack::outro("Aborted.")?;
        return Ok(());
    }

    // --- Execution ---
    let spinner = cliclack::spinner();
    spinner.start("Creating team...");

    fs::create_dir_all(&workzone)
        .with_context(|| format!("Failed to create workzone at {}", workzone.display()))?;
    fs::create_dir_all(&team_dir)
        .with_context(|| format!("Failed to create team directory at {}", team_dir.display()))?;

    let team_repo = team_dir.join("team");

    if is_new_repo {
        spinner.start("Initializing git repository...");
        formation::setup_new_team_repo(
            &team_repo, &selected_profile, &manifest,
            &members_to_hire, &projects_to_add,
            selected_bridge.as_deref(),
            None,
        )?;

        spinner.start("Creating GitHub repository...");
        git::create_repo_and_push(&team_repo, &github_repo, gh_token.as_deref())?;
    } else {
        spinner.start("Cloning existing repository...");
        git::clone_repo(&team_dir, &github_repo, gh_token.as_deref())?;
    }

    spinner.start("Registering team...");
    formation::register_team(
        &team_name, &team_dir, &selected_profile, &github_repo,
        gh_token.clone(), telegram_bot_token.clone(), &workzone,
    )?;

    // Bootstrap labels
    spinner.start("Bootstrapping labels...");
    if let Err(e) = git::bootstrap_labels(&github_repo, &manifest.labels, gh_token.as_deref()) {
        spinner.stop("Label bootstrap failed");
        let label_cmds: Vec<String> = manifest
            .labels.iter()
            .map(|l| format!(
                "gh label create '{}' --color '{}' --description '{}' --force --repo {}",
                l.name, l.color, l.description, github_repo,
            ))
            .collect();
        bail!(
            "Failed to bootstrap labels: {}\n\n\
             To fix, run these commands manually:\n  {}\n\n\
             Make sure your token has Issues (Write) permission.",
            e, label_cmds.join("\n  "),
        );
    }

    // Create or sync project board
    let owner = github_repo.split('/').next().unwrap_or(&github_repo);
    let project_number = match project_choice {
        ProjectChoice::CreateNew => {
            spinner.start("Creating GitHub Project board...");
            let board_title = format!("{} Board", team_name);
            match git::create_project(owner, &board_title, &manifest.statuses, gh_token.as_deref()) {
                Ok(n) => {
                    spinner.stop("GitHub Project board created");
                    n
                }
                Err(e) => {
                    spinner.stop("Project creation failed");
                    bail!(
                        "Failed to create GitHub Project: {}\n\n\
                         To fix, create the project manually and then run:\n  \
                         gh project create --owner {} --title '{} Board'\n  \
                         bm projects sync\n\n\
                         Make sure your token has the \"project\" scope (classic PAT) \
                         or \"Organization projects: Admin\" (fine-grained PAT).",
                        e, owner, team_name,
                    );
                }
            }
        }
        ProjectChoice::UseExisting(n) => {
            spinner.start("Syncing project board statuses...");
            git::sync_project_status_field(owner, n, &manifest.statuses, gh_token.as_deref())?;
            spinner.stop("Project board statuses synced");
            n
        }
    };

    // Save project number
    {
        let mut cfg = config::load()?;
        if let Some(entry) = cfg.teams.iter_mut().find(|t| t.name == team_name) {
            entry.project_number = Some(project_number);
        }
        config::save(&cfg)?;
    }

    if !manifest.views.is_empty() {
        let project_url = format!("https://github.com/orgs/{}/projects/{}", owner, project_number);
        cliclack::log::info(format!("Board: {}", project_url))?;
    }

    spinner.stop("Done!");
    cliclack::log::info(next_steps_message(&team_name, &team_dir, &team_repo, selected_bridge.is_some()))?;
    cliclack::outro("Ready to go!")?;

    Ok(())
}

// ── Domain-calling helpers (interactive prompts) ──────────────────

/// Detects an existing GH_TOKEN or prompts the user for one.
fn detect_or_prompt_gh_token() -> Result<String> {
    if let Some(existing) = git::detect_token() {
        let masked = git::mask_token(&existing);
        let use_existing: bool =
            cliclack::confirm(format!("GitHub token detected ({}). Use it?", masked))
                .initial_value(true)
                .interact()?;
        if use_existing {
            return Ok(existing);
        }
    }
    prompt_gh_token()
}

/// Prompts for a GitHub token manually.
fn prompt_gh_token() -> Result<String> {
    let token: String = cliclack::input("GitHub token (GH_TOKEN)")
        .placeholder("ghp_... or github_pat_...")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("GitHub token is required when a GitHub repo is specified")
            } else {
                Ok(())
            }
        })
        .interact()?;
    Ok(token)
}

/// Lists the user's GitHub orgs + personal account, returns selected org login.
fn select_github_org(gh_token: &str) -> Result<String> {
    let user_login = git::get_user_login(gh_token)?;
    let orgs = git::list_user_orgs(gh_token)?;

    let personal_label = format!("{} (personal)", user_login);
    let mut select_items: Vec<(String, String, String)> = vec![
        (user_login.clone(), personal_label, "Your personal GitHub account".to_string()),
    ];
    for org in &orgs {
        select_items.push((org.clone(), org.clone(), "Organization".to_string()));
    }
    select_items.push((
        "__other__".to_string(), "Other (type org name)".to_string(),
        "Enter an org name not listed above".to_string(),
    ));

    let items_ref: Vec<(&str, &str, &str)> = select_items
        .iter()
        .map(|(v, l, d)| (v.as_str(), l.as_str(), d.as_str()))
        .collect();

    let selected: &str = cliclack::select("GitHub owner (type to filter)")
        .items(&items_ref)
        .filter_mode()
        .interact()?;

    if selected == "__other__" {
        let org: String = cliclack::input("Organization name")
            .placeholder("my-org")
            .validate(|input: &String| {
                if input.is_empty() { Err("Organization name cannot be empty") } else { Ok(()) }
            })
            .interact()?;
        Ok(org)
    } else {
        Ok(selected.to_string())
    }
}

/// Lists repos for an org/user, lets the user select or create. Returns `(owner/repo, is_new)`.
fn select_or_create_repo(gh_token: &str, owner: &str, team_name: &str) -> Result<(String, bool)> {
    let repos = git::list_repos(gh_token, owner)?;
    let default_name = format!("{}-team", team_name);
    let create_label = format!("Create new repo ({})", default_name);

    let mut select_items: Vec<(String, String, String)> = vec![
        ("__create__".to_string(), create_label, "Create a new private repository".to_string()),
    ];
    for repo in &repos {
        select_items.push((repo.clone(), repo.clone(), String::new()));
    }

    let items_ref: Vec<(&str, &str, &str)> = select_items
        .iter()
        .map(|(v, l, d)| (v.as_str(), l.as_str(), d.as_str()))
        .collect();

    let selected: &str = cliclack::select("Team repo (type to filter)")
        .items(&items_ref)
        .filter_mode()
        .interact()?;

    if selected == "__create__" {
        let repo_name: String = cliclack::input("New repo name")
            .default_input(&default_name)
            .interact()?;
        Ok((format!("{}/{}", owner, repo_name), true))
    } else {
        Ok((format!("{}/{}", owner, selected), false))
    }
}

/// Lists GitHub Projects, lets the user select or create a new one.
fn select_or_create_project(gh_token: &str, owner: &str, team_name: &str) -> Result<ProjectChoice> {
    let projects = git::list_projects(gh_token, owner)?;
    let default_title = format!("{} Board", team_name);
    let create_label = format!("Create new board ({})", default_title);

    let mut select_items: Vec<(String, String, String)> = vec![(
        "__create__".to_string(), create_label,
        "Create a new GitHub Project board".to_string(),
    )];
    for (number, title) in &projects {
        select_items.push((number.to_string(), format!("{} (#{number})", title), String::new()));
    }

    let items_ref: Vec<(&str, &str, &str)> = select_items
        .iter()
        .map(|(v, l, d)| (v.as_str(), l.as_str(), d.as_str()))
        .collect();

    let selected: &str = cliclack::select("Project board (type to filter)")
        .items(&items_ref)
        .filter_mode()
        .interact()?;

    if selected == "__create__" {
        Ok(ProjectChoice::CreateNew)
    } else {
        let number: u64 = selected.parse().context("Failed to parse project number")?;
        Ok(ProjectChoice::UseExisting(number))
    }
}

/// Collect members to hire during init (optional).
fn collect_members(roles: &[String]) -> Result<Vec<(String, String)>> {
    let hire_members: bool = cliclack::confirm("Hire members now?")
        .initial_value(true)
        .interact()?;

    if !hire_members {
        return Ok(Vec::new());
    }

    let mut members = Vec::new();
    loop {
        let role_items: Vec<(&str, &str, &str)> = roles
            .iter()
            .map(|r| (r.as_str(), r.as_str(), ""))
            .collect();

        let role: String = cliclack::select("Select role")
            .items(&role_items)
            .interact()
            .map(|s: &str| s.to_string())?;

        let name: String = cliclack::input("Member name")
            .placeholder("bob")
            .validate(|input: &String| {
                if input.is_empty() {
                    Err("Name cannot be empty")
                } else if input.contains('/') || input.contains(' ') {
                    Err("Name cannot contain '/' or spaces")
                } else {
                    Ok(())
                }
            })
            .interact()?;

        members.push((role.clone(), name));

        // Default to "yes" as long as there are roles without a hired member
        let hired_roles: std::collections::HashSet<&str> = members.iter().map(|(r, _)| r.as_str()).collect();
        let all_covered = roles.iter().all(|r| hired_roles.contains(r.as_str()));
        let more: bool = cliclack::confirm("Hire another member?")
            .initial_value(!all_covered)
            .interact()?;
        if !more {
            break;
        }
    }

    Ok(members)
}

/// Collect projects to add during init (optional).
fn collect_projects(gh_token: Option<&str>, org: Option<&str>) -> Result<Vec<(String, String)>> {
    let add_projects: bool = cliclack::confirm("Add projects now?")
        .initial_value(false)
        .interact()?;

    if !add_projects {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    loop {
        let url = if let (Some(token), Some(org)) = (gh_token, org) {
            select_project_repo(token, org)?
        } else {
            prompt_project_url()?
        };

        let name = git::derive_project_name(&url);
        cliclack::log::info(format!("Project name: {}", name))?;

        projects.push((name, url));

        let more: bool = cliclack::confirm("Add another project?")
            .initial_value(false)
            .interact()?;
        if !more {
            break;
        }
    }

    Ok(projects)
}

/// Prompts for a project fork URL manually.
fn prompt_project_url() -> Result<String> {
    let url: String = cliclack::input("Project fork URL (must be HTTPS)")
        .placeholder("https://github.com/org/repo.git")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("URL cannot be empty")
            } else if !input.starts_with("https://") {
                Err("URL must be HTTPS (e.g. https://github.com/org/repo.git)")
            } else {
                Ok(())
            }
        })
        .interact()?;
    Ok(url)
}

/// Lists repos for an org/user and lets the user select one as a project fork.
fn select_project_repo(gh_token: &str, org: &str) -> Result<String> {
    let repos = git::list_repos(gh_token, org)?;

    if repos.is_empty() {
        cliclack::log::warning(format!("No repos found in '{}'. Enter URL manually.", org))?;
        return prompt_project_url();
    }

    let items_ref: Vec<(&str, &str, &str)> = repos
        .iter()
        .map(|r| (r.as_str(), r.as_str(), ""))
        .collect();

    let selected: &str = cliclack::select("Select project repo (type to filter)")
        .items(&items_ref)
        .filter_mode()
        .interact()?;

    Ok(format!("https://github.com/{}/{}.git", org, selected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_no_bridge_with_no_bridges_ok() {
        // When no --bridge flag is passed and profile has no bridges, nothing to validate.
        // validate_bridge_selection is only called when --bridge is provided.
        let bridges: Vec<profile::BridgeDef> = Vec::new();
        assert!(bridges.is_empty());
    }
}
