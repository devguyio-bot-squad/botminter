//! E2E tests for the init -> hire -> projects add -> sync lifecycle.
//!
//! These tests create real GitHub repos under the configured org
//! and verify that the full `bm` CLI pipeline produces correct workspaces,
//! labels, and member listings.
//!
//! The `team_lifecycle` suite combines 5 tests that share a single TempRepo
//! to reduce API rate limit consumption.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use bm::config::{BotminterConfig, Credentials, TeamEntry};
use bm::profile::{self, CodingAgentDef};
use libtest_mimic::Trial;

use super::helpers::{assert_cmd_success, bm_cmd, run_test, E2eConfig, GithubSuite};

/// Returns the default Claude Code coding agent definition for E2E tests.
fn claude_code_agent() -> CodingAgentDef {
    CodingAgentDef {
        name: "claude-code".into(),
        display_name: "Claude Code".into(),
        context_file: "CLAUDE.md".into(),
        agent_dir: ".claude".into(),
        binary: "claude".into(),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Runs a git command in a directory.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {} failed to run: {}", args.join(" "), e));
    assert!(
        output.status.success(),
        "git {} in {} failed: {}",
        args.join(" "),
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Pushes to GitHub. Credential helper is configured via $HOME/.gitconfig
/// (set up by `setup_git_auth`).
fn git_push(dir: &Path) {
    git(dir, &["push", "-u", "origin", "main"]);
}

/// Finds a profile with at least `min_roles` roles using embedded data (no disk access).
fn find_profile_with_roles(min_roles: usize) -> (String, Vec<String>) {
    for name in bm::profile::list_embedded_profiles() {
        let roles = bm::profile::list_embedded_roles(&name);
        if roles.len() >= min_roles {
            return (name, roles);
        }
    }
    panic!("No embedded profile has at least {} roles", min_roles);
}

/// Sets up a team repo programmatically and pushes it to a real GitHub repo.
fn setup_team_with_github(
    tmp: &Path,
    team_name: &str,
    profile_name: &str,
    github_full_name: &str,
    profiles_base: &Path,
    gh_token: Option<&str>,
) -> PathBuf {
    let workzone = tmp.join("workspaces");
    let team_dir = workzone.join(team_name);
    let team_repo = team_dir.join("team");

    fs::create_dir_all(&team_repo).unwrap();

    // Set up git auth in temp HOME (credential helper + user identity)
    super::helpers::setup_git_auth(tmp);

    // Git init
    git(&team_repo, &["init", "-b", "main"]);

    // Extract profile content into team repo (from temp profiles, not real HOME)
    profile::extract_profile_from(profiles_base, profile_name, &team_repo, &claude_code_agent())
        .unwrap();

    // Create members/ and projects/ dirs (as bm init does)
    fs::create_dir_all(team_repo.join("members")).unwrap();
    fs::create_dir_all(team_repo.join("projects")).unwrap();
    fs::write(team_repo.join("members/.gitkeep"), "").unwrap();
    fs::write(team_repo.join("projects/.gitkeep"), "").unwrap();

    // Initial commit
    git(&team_repo, &["add", "-A"]);
    git(&team_repo, &["commit", "-m", "feat: init team repo"]);

    // Push to GitHub (use gh credential helper since libsecret may not work)
    let remote_url = format!("https://github.com/{}.git", github_full_name);
    git(&team_repo, &["remote", "add", "origin", &remote_url]);
    git_push(&team_repo);

    // Write config
    let config = BotminterConfig {
        workzone,
        default_team: Some(team_name.to_string()),
        teams: vec![TeamEntry {
            name: team_name.to_string(),
            path: team_dir,
            profile: profile_name.to_string(),
            github_repo: github_full_name.to_string(),
            credentials: Credentials {
                gh_token: gh_token.map(|t| t.to_string()),
                ..Credentials::default()
            },
            coding_agent: None,
            project_number: None,
        }],
    };
    let config_path = tmp.join(".botminter").join("config.yml");
    bm::config::save_to(&config_path, &config).unwrap();

    team_repo
}

/// Bootstraps labels on GitHub from the profile manifest.
fn bootstrap_labels(repo: &str, profile_name: &str, profiles_base: &Path) {
    let manifest = profile::read_manifest_from(profile_name, profiles_base).unwrap();
    for label in &manifest.labels {
        let output = Command::new("gh")
            .args([
                "label",
                "create",
                &label.name,
                "--color",
                &label.color,
                "--description",
                &label.description,
                "--force",
                "--repo",
                repo,
            ])
            .output()
            .unwrap_or_else(|e| panic!("failed to create label '{}': {}", label.name, e));
        if !output.status.success() {
            eprintln!(
                "Warning: failed to create label '{}': {}",
                label.name,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

/// Creates a local git repo for use as a project fork URL.
fn create_fake_fork(tmp: &Path, name: &str) -> PathBuf {
    let fork = tmp.join(name);
    fs::create_dir_all(&fork).unwrap();
    git(&fork, &["init", "-b", "main"]);
    git(&fork, &["config", "user.email", "e2e@botminter.test"]);
    git(&fork, &["config", "user.name", "BM E2E"]);
    fs::write(fork.join("README.md"), format!("# {}", name)).unwrap();
    git(&fork, &["add", "-A"]);
    git(&fork, &["commit", "-m", "init"]);
    fork
}

// ── Test registration ────────────────────────────────────────────────

pub fn tests(config: &E2eConfig) -> Vec<Trial> {
    let cfg = config.clone();

    let mut trials = Vec::new();

    // Suite 1: team_lifecycle — 5 tests sharing 1 TempRepo
    trials.push(team_lifecycle_suite(&cfg));

    // Remaining isolated tests that need their own repos/projects
    let isolated: Vec<Trial> = vec![
        Trial::test("e2e_projects_sync_status_and_views", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_projects_sync_status_and_views_impl(&cfg))
        }),
        Trial::test("e2e_clone_existing_repo", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_clone_existing_repo_impl(&cfg))
        }),
        Trial::test("e2e_list_gh_projects", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_list_gh_projects_impl(&cfg))
        }),
        Trial::test("e2e_sync_status_on_existing_project", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_sync_status_on_existing_project_impl(&cfg))
        }),
        Trial::test("e2e_projects_add_creates_label_on_github", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_projects_add_creates_label_on_github_impl(&cfg))
        }),
        Trial::test("e2e_init_non_interactive_full_github", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_init_non_interactive_full_github_impl(&cfg))
        }),
        Trial::test("e2e_sync_push_creates_workspace_repo", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_sync_push_creates_workspace_repo_impl(&cfg))
        }),
        Trial::test("e2e_bridge_lifecycle", {
            let cfg = cfg.clone();
            move || run_test(|| e2e_bridge_lifecycle_impl(&cfg))
        }),
    ];
    trials.extend(isolated);

    trials
}

// ── Suite: team_lifecycle ─────────────────────────────────────────────

fn team_lifecycle_suite(config: &E2eConfig) -> Trial {
    GithubSuite::new("team_lifecycle", "bm-e2e-lifecycle")
        .setup(|ctx| {
            let (profile_name, roles) = find_profile_with_roles(2);
            let role_1 = &roles[0];
            let role_2 = &roles[1];
            let team_name = "e2e-lifecycle";

            setup_team_with_github(
                ctx.home.path(),
                team_name,
                &profile_name,
                &ctx.repo.full_name,
                &ctx.profiles_base,
                Some(&ctx.gh_token),
            );
            bootstrap_labels(&ctx.repo.full_name, &profile_name, &ctx.profiles_base);

            let fork = create_fake_fork(ctx.home.path(), "test-project");

            // Hire alice (role_1)
            let mut cmd = bm_cmd();
            cmd.args(["hire", role_1, "--name", "alice", "-t", team_name])
                .env("HOME", ctx.home.path());
            let out = assert_cmd_success(&mut cmd);
            eprintln!("hire alice: {}", out.trim());

            // Hire bob (role_2)
            let mut cmd = bm_cmd();
            cmd.args(["hire", role_2, "--name", "bob", "-t", team_name])
                .env("HOME", ctx.home.path());
            let out = assert_cmd_success(&mut cmd);
            eprintln!("hire bob: {}", out.trim());

            // Add project
            let mut cmd = bm_cmd();
            cmd.args([
                "projects",
                "add",
                &fork.to_string_lossy(),
                "-t",
                team_name,
            ])
            .env("HOME", ctx.home.path());
            let out = assert_cmd_success(&mut cmd);
            eprintln!("projects add: {}", out.trim());

            // Sync workspaces
            let mut cmd = bm_cmd();
            cmd.args(["teams", "sync", "-t", team_name])
                .env("HOME", ctx.home.path());
            let out = assert_cmd_success(&mut cmd);
            eprintln!("teams sync: {}", out.trim());
        })
        .case("workspace_structure", |ctx| {
            let (_, roles) = find_profile_with_roles(2);
            let role_1 = &roles[0];
            let role_2 = &roles[1];
            let team_name = "e2e-lifecycle";

            let team_dir = ctx.home.path().join("workspaces").join(team_name);
            let alice_dir = format!("{}-alice", role_1);
            let bob_dir = format!("{}-bob", role_2);

            for member_name in [&alice_dir, &bob_dir] {
                let ws = team_dir.join(member_name);

                assert!(
                    ws.join(".botminter.workspace").exists(),
                    "{} should have .botminter.workspace marker",
                    member_name
                );
                assert!(
                    ws.join("team").is_dir(),
                    "{} should have team/ submodule",
                    member_name
                );
                for file in ["PROMPT.md", "CLAUDE.md", "ralph.yml"] {
                    assert!(
                        ws.join(file).exists(),
                        "{} should have {}",
                        member_name,
                        file
                    );
                }
                assert!(
                    ws.join(".claude/agents").is_dir(),
                    "{} should have .claude/agents/",
                    member_name
                );
                assert!(
                    ws.join("projects/test-project").is_dir(),
                    "{} should have projects/test-project/ submodule",
                    member_name
                );
            }
        })
        .case("labels_on_github", |ctx| {
            let (profile_name, _) = find_profile_with_roles(1);

            let manifest =
                profile::read_manifest_from(&profile_name, &ctx.profiles_base).unwrap();
            let gh_labels = super::github::list_labels_json(&ctx.repo.full_name);

            for expected in &manifest.labels {
                let found = gh_labels.iter().find(|(name, _)| name == &expected.name);
                assert!(
                    found.is_some(),
                    "Label '{}' from profile manifest not found on GitHub. GitHub has: {:?}",
                    expected.name,
                    gh_labels
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .collect::<Vec<_>>()
                );
                let (_, gh_color) = found.unwrap();
                let norm_expected = expected.color.trim_start_matches('#').to_lowercase();
                let norm_actual = gh_color.trim_start_matches('#').to_lowercase();
                assert_eq!(
                    norm_expected, norm_actual,
                    "Label '{}' color mismatch: expected '{}', got '{}'",
                    expected.name, expected.color, gh_color
                );
            }

            let github_defaults: &[&str] = &[
                "bug",
                "documentation",
                "duplicate",
                "enhancement",
                "good first issue",
                "help wanted",
                "invalid",
                "question",
                "wontfix",
            ];
            for (name, _) in &gh_labels {
                let is_expected = manifest.labels.iter().any(|l| l.name == *name);
                let is_default = github_defaults.contains(&name.as_str());
                let is_project = name.starts_with("project/");
                assert!(
                    is_expected || is_default || is_project,
                    "Unexpected label '{}' on GitHub",
                    name
                );
            }
        })
        .case("sync_idempotent", |ctx| {
            let team_name = "e2e-lifecycle";

            // Second sync (first happened in setup)
            let mut cmd = bm_cmd();
            cmd.args(["teams", "sync", "-t", team_name])
                .env("HOME", ctx.home.path());
            let out = assert_cmd_success(&mut cmd);
            eprintln!("sync 2: {}", out.trim());

            let (_, roles) = find_profile_with_roles(1);
            let role = &roles[0];
            let member = format!("{}-alice", role);
            let team_dir = ctx.home.path().join("workspaces").join(team_name);
            let ws = team_dir.join(&member);

            assert!(ws.join(".botminter.workspace").exists());
            assert!(ws.join("team").is_dir());
            assert!(ws.join("PROMPT.md").exists());
            assert!(ws.join("CLAUDE.md").exists());
            assert!(ws.join("ralph.yml").exists());
            assert!(ws.join(".claude").is_dir());
        })
        .case("members_list", |ctx| {
            let (_, roles) = find_profile_with_roles(2);
            let role_1 = &roles[0];
            let role_2 = &roles[1];
            let team_name = "e2e-lifecycle";

            let mut cmd = bm_cmd();
            cmd.args(["members", "list", "-t", team_name])
                .env("HOME", ctx.home.path());
            let stdout = assert_cmd_success(&mut cmd);

            let alice = format!("{}-alice", role_1);
            let bob = format!("{}-bob", role_2);
            assert!(stdout.contains(&alice), "should show '{}'", alice);
            assert!(stdout.contains(&bob), "should show '{}'", bob);
            assert!(stdout.contains(role_1.as_str()));
            assert!(stdout.contains(role_2.as_str()));
        })
        .case("teams_list", |ctx| {
            let mut cmd = bm_cmd();
            cmd.args(["teams", "list"]).env("HOME", ctx.home.path());
            let stdout = assert_cmd_success(&mut cmd);

            assert!(
                stdout.contains(&ctx.repo.full_name),
                "teams list should show GitHub repo '{}', output:\n{}",
                ctx.repo.full_name,
                stdout
            );
        })
        .build(config)
}

// ── Isolated test implementations ─────────────────────────────────────

fn e2e_projects_sync_status_and_views_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let team_name = "e2e-project-sync";

    // Use a TempRepo for the github_repo field
    let repo = super::github::TempRepo::new_in_org("bm-e2e-psync", &config.gh_org)
        .expect("Failed to create temp GitHub repo");

    let workzone = tmp.path().join("workspaces");
    let team_dir = workzone.join(team_name);
    let team_repo = team_dir.join("team");
    fs::create_dir_all(&team_repo).unwrap();

    git(&team_repo, &["init", "-b", "main"]);
    git(
        &team_repo,
        &["config", "user.email", "e2e@botminter.test"],
    );
    git(&team_repo, &["config", "user.name", "BM E2E"]);
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp.path());
    profile::extract_profile_from(
        &profiles_base,
        "scrum-compact",
        &team_repo,
        &claude_code_agent(),
    )
    .unwrap();
    fs::create_dir_all(team_repo.join("members")).unwrap();
    fs::create_dir_all(team_repo.join("projects")).unwrap();
    fs::write(team_repo.join("members/.gitkeep"), "").unwrap();
    fs::write(team_repo.join("projects/.gitkeep"), "").unwrap();
    git(&team_repo, &["add", "-A"]);
    git(&team_repo, &["commit", "-m", "feat: init team repo"]);

    let bm_config = BotminterConfig {
        workzone,
        default_team: Some(team_name.to_string()),
        teams: vec![TeamEntry {
            name: team_name.to_string(),
            path: team_dir,
            profile: "scrum-compact".to_string(),
            github_repo: repo.full_name.clone(),
            credentials: Credentials {
                gh_token: Some(config.gh_token.clone()),
                ..Credentials::default()
            },
            coding_agent: None,
            project_number: None,
        }],
    };
    let config_path = tmp.path().join(".botminter").join("config.yml");
    bm::config::save_to(&config_path, &bm_config).unwrap();

    let project = super::github::TempProject::new(&config.gh_org, &format!("{} Board", team_name))
        .expect("Failed to create temp GitHub Project");

    let mut cmd = bm_cmd();
    cmd.args(["projects", "sync", "-t", team_name])
        .env("HOME", tmp.path());
    let stdout = assert_cmd_success(&mut cmd);

    let options = super::github::list_project_status_options(&config.gh_org, project.number);
    assert!(
        options.len() >= 20,
        "Status field should have at least 20 options, got {}: {:?}",
        options.len(),
        options
    );
    assert!(options.contains(&"po:triage".to_string()));
    assert!(options.contains(&"done".to_string()));
    assert!(options.contains(&"error".to_string()));

    assert!(stdout.contains("Status field synced"));
    assert!(stdout.contains("View"));
    assert!(stdout.contains("Filter"));
    assert!(stdout.contains("status:po:"));
    assert!(stdout.contains("status:arch:"));

    // Idempotency
    let mut cmd = bm_cmd();
    cmd.args(["projects", "sync", "-t", team_name])
        .env("HOME", tmp.path());
    let stdout2 = assert_cmd_success(&mut cmd);
    assert!(stdout2.contains("Status field synced"));
}

fn e2e_clone_existing_repo_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();

    let repo = super::github::TempRepo::new_in_org("bm-e2e-clone", &config.gh_org)
        .expect("Failed to create temp GitHub repo");

    let (profile_name, _) = find_profile_with_roles(1);
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp.path());

    let staging = tmp.path().join("staging");
    let staging_repo = staging.join("team");
    fs::create_dir_all(&staging_repo).unwrap();

    git(&staging_repo, &["init", "-b", "main"]);
    git(
        &staging_repo,
        &["config", "user.email", "e2e@botminter.test"],
    );
    git(&staging_repo, &["config", "user.name", "BM E2E"]);

    profile::extract_profile_from(
        &profiles_base,
        &profile_name,
        &staging_repo,
        &claude_code_agent(),
    )
    .unwrap();
    fs::create_dir_all(staging_repo.join("members")).unwrap();
    fs::write(staging_repo.join("members/.gitkeep"), "").unwrap();

    git(&staging_repo, &["add", "-A"]);
    git(&staging_repo, &["commit", "-m", "feat: init"]);

    super::helpers::setup_git_auth(tmp.path());
    let remote_url = format!("https://github.com/{}.git", repo.full_name);
    git(&staging_repo, &["remote", "add", "origin", &remote_url]);
    git_push(&staging_repo);

    let clone_dir = tmp.path().join("cloned");
    fs::create_dir_all(&clone_dir).unwrap();

    bm::commands::init::clone_existing_repo(&clone_dir, &repo.full_name, None)
        .expect("clone_existing_repo should succeed");

    let cloned_repo = clone_dir.join("team");
    assert!(cloned_repo.join(".git").is_dir());
    assert!(cloned_repo.join("botminter.yml").exists());
    assert!(cloned_repo.join("members/.gitkeep").exists());
}

fn e2e_list_gh_projects_impl(config: &E2eConfig) {
    let project =
        super::github::TempProject::new(&config.gh_org, "bm-e2e-list-projects")
            .expect("Failed to create temp GitHub Project");

    let projects = bm::commands::init::list_gh_projects(&config.gh_token, &config.gh_org)
        .expect("list_gh_projects should succeed");

    let found = projects.iter().find(|(n, _)| *n == project.number);
    assert!(
        found.is_some(),
        "list_gh_projects should include project #{}, got: {:?}",
        project.number,
        projects
    );

    let (_, title) = found.unwrap();
    assert_eq!(title, "bm-e2e-list-projects");

    // Idempotency
    let projects2 = bm::commands::init::list_gh_projects(&config.gh_token, &config.gh_org)
        .expect("second list_gh_projects should succeed");
    let found2 = projects2.iter().find(|(n, _)| *n == project.number);
    assert!(found2.is_some());
}

fn e2e_sync_status_on_existing_project_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp.path());

    let project = super::github::TempProject::new(&config.gh_org, "bm-e2e-sync-existing")
        .expect("Failed to create temp GitHub Project");

    let manifest = profile::read_manifest_from("scrum-compact", &profiles_base).unwrap();

    bm::commands::init::sync_project_status_field(
        &config.gh_org,
        project.number,
        &manifest.statuses,
        None,
    )
    .expect("first sync should succeed");

    let options1 = super::github::list_project_status_options(&config.gh_org, project.number);
    assert!(options1.len() >= 20);
    assert!(options1.contains(&"po:triage".to_string()));

    bm::commands::init::sync_project_status_field(
        &config.gh_org,
        project.number,
        &manifest.statuses,
        None,
    )
    .expect("second sync should succeed");

    let options2 = super::github::list_project_status_options(&config.gh_org, project.number);
    assert_eq!(options1.len(), options2.len());
    assert_eq!(options1, options2);
}

fn e2e_projects_add_creates_label_on_github_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();

    let repo = super::github::TempRepo::new_in_org("bm-e2e-plabels", &config.gh_org)
        .expect("Failed to create temp GitHub repo");

    let (profile_name, _) = find_profile_with_roles(1);
    let team_name = "e2e-project-labels";
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp.path());

    setup_team_with_github(
        tmp.path(),
        team_name,
        &profile_name,
        &repo.full_name,
        &profiles_base,
        Some(&config.gh_token),
    );

    let fork = create_fake_fork(tmp.path(), "test-project");
    let fork_url = fork.to_string_lossy().to_string();

    let labels_before = super::github::list_labels(&repo.full_name);
    assert!(!labels_before.contains(&"project/test-project".to_string()));

    let mut cmd = bm_cmd();
    cmd.args(["projects", "add", &fork_url, "-t", team_name])
        .env("HOME", tmp.path());
    let output = assert_cmd_success(&mut cmd);
    eprintln!("projects add output: {}", output.trim());

    let labels_after = super::github::list_labels(&repo.full_name);
    assert!(
        labels_after.contains(&"project/test-project".to_string()),
        "Label 'project/test-project' should exist. Found: {:?}",
        labels_after
    );

    let fork2 = create_fake_fork(tmp.path(), "another-project");
    let fork2_url = fork2.to_string_lossy().to_string();

    let mut cmd = bm_cmd();
    cmd.args(["projects", "add", &fork2_url, "-t", team_name])
        .env("HOME", tmp.path());
    assert_cmd_success(&mut cmd);

    let labels_final = super::github::list_labels(&repo.full_name);
    assert!(labels_final.contains(&"project/test-project".to_string()));
    assert!(labels_final.contains(&"project/another-project".to_string()));
}

fn e2e_init_non_interactive_full_github_impl(config: &E2eConfig) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let repo_name = format!("bm-e2e-init-{}", timestamp);
    let full_name = format!("{}/{}", config.gh_org, repo_name);

    // RAII guard: deletes the repo on drop (even if bm init created it)
    let _cleanup = super::github::TempRepo {
        full_name: full_name.clone(),
    };

    let tmp = tempfile::tempdir().unwrap();
    let workzone = tmp.path().join("workspaces");

    let mut cmd = bm_cmd();
    cmd.args([
        "init",
        "--non-interactive",
        "--profile",
        "scrum-compact",
        "--team-name",
        "e2e-init-test",
        "--org",
        &config.gh_org,
        "--repo",
        &repo_name,
        "--workzone",
        &workzone.to_string_lossy(),
    ])
    .env("HOME", tmp.path())
    .env("GH_TOKEN", &config.gh_token)
    .env("GIT_AUTHOR_NAME", "BM E2E")
    .env("GIT_AUTHOR_EMAIL", "e2e@botminter.test")
    .env("GIT_COMMITTER_NAME", "BM E2E")
    .env("GIT_COMMITTER_EMAIL", "e2e@botminter.test");
    let stdout = assert_cmd_success(&mut cmd);
    eprintln!("bm init output: {}", stdout.trim());

    let labels = super::github::list_labels(&full_name);
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp.path());
    let manifest = profile::read_manifest_from("scrum-compact", &profiles_base).unwrap();
    for expected_label in &manifest.labels {
        assert!(
            labels.contains(&expected_label.name),
            "Label '{}' should exist after init, found: {:?}",
            expected_label.name,
            labels
        );
    }

    let config_path = tmp.path().join(".botminter").join("config.yml");
    assert!(config_path.exists());
    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("e2e-init-test"));
    assert!(config_content.contains(&full_name));

    let team_repo = workzone.join("e2e-init-test").join("team");
    assert!(team_repo.join(".git").is_dir());
    assert!(team_repo.join("botminter.yml").exists());
    assert!(team_repo.join("PROCESS.md").exists());
    assert!(team_repo.join("members/.gitkeep").exists());
    assert!(team_repo.join("projects/.gitkeep").exists());

    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&team_repo)
        .output()
        .unwrap();
    let remote = String::from_utf8_lossy(&output.stdout);
    assert!(remote.contains(&full_name));

    // Clean up project boards
    let output = Command::new("gh")
        .args([
            "project",
            "list",
            "--owner",
            &config.gh_org,
            "--format",
            "json",
            "--limit",
            "100",
        ])
        .env("GH_TOKEN", &config.gh_token)
        .output()
        .expect("failed to list projects");

    if output.status.success() {
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
        if let Some(projects) = json["projects"].as_array() {
            for project in projects {
                let title = project["title"].as_str().unwrap_or("");
                if title.contains("e2e-init-test") {
                    if let Some(number) = project["number"].as_u64() {
                        eprintln!("Cleaning up project board #{}: {}", number, title);
                        let _ = Command::new("gh")
                            .args([
                                "project",
                                "delete",
                                "--owner",
                                &config.gh_org,
                                &number.to_string(),
                                "--format",
                                "json",
                            ])
                            .env("GH_TOKEN", &config.gh_token)
                            .output();
                    }
                }
            }
        }
    }
}

fn e2e_sync_push_creates_workspace_repo_impl(config: &E2eConfig) {
    let tmp = tempfile::tempdir().unwrap();

    let repo = super::github::TempRepo::new_in_org("bm-e2e-ws", &config.gh_org)
        .expect("Failed to create temp GitHub repo");

    let (profile_name, roles) = find_profile_with_roles(1);
    let role = &roles[0];
    let team_name = "e2e-ws";
    let profiles_base = super::helpers::bootstrap_profiles_to_tmp(tmp.path());

    setup_team_with_github(
        tmp.path(),
        team_name,
        &profile_name,
        &repo.full_name,
        &profiles_base,
        Some(&config.gh_token),
    );

    let mut cmd = bm_cmd();
    cmd.args(["hire", role, "--name", "alice", "-t", team_name])
        .env("HOME", tmp.path());
    assert_cmd_success(&mut cmd);

    let team_repo = tmp
        .path()
        .join("workspaces")
        .join(team_name)
        .join("team");
    // bm hire already commits; just push
    git_push(&team_repo);

    let member_dir = format!("{}-alice", role);
    let ws_repo_name = format!("{}/{}-{}", config.gh_org, team_name, member_dir);

    // Pre-cleanup
    let _ = Command::new("gh")
        .args(["repo", "delete", &ws_repo_name, "--yes"])
        .env("GH_TOKEN", &config.gh_token)
        .output();

    let mut cmd = bm_cmd();
    cmd.args(["teams", "sync", "--repos", "-t", team_name])
        .env("HOME", tmp.path())
        .env("GH_TOKEN", &config.gh_token);
    let out = assert_cmd_success(&mut cmd);
    eprintln!("sync --push: {}", out.trim());

    let output = Command::new("gh")
        .args(["repo", "view", &ws_repo_name, "--json", "name"])
        .env("GH_TOKEN", &config.gh_token)
        .output()
        .expect("failed to run gh repo view");
    assert!(
        output.status.success(),
        "workspace repo '{}' should exist on GitHub: {}",
        ws_repo_name,
        String::from_utf8_lossy(&output.stderr)
    );

    let ws_path = tmp
        .path()
        .join("workspaces")
        .join(team_name)
        .join(&member_dir);
    assert!(ws_path.join(".botminter.workspace").exists());
    assert!(ws_path.join("team").is_dir());
    assert!(ws_path.join(".gitmodules").exists());
    let gitmodules = fs::read_to_string(ws_path.join(".gitmodules")).unwrap();
    assert!(gitmodules.contains("[submodule \"team\"]"));
    assert!(ws_path.join("CLAUDE.md").exists());
    assert!(ws_path.join("PROMPT.md").exists());
    assert!(ws_path.join("ralph.yml").exists());

    // Idempotency
    let mut cmd = bm_cmd();
    cmd.args(["teams", "sync", "--repos", "-t", team_name])
        .env("HOME", tmp.path())
        .env("GH_TOKEN", &config.gh_token);
    let out2 = assert_cmd_success(&mut cmd);
    eprintln!("sync --push (2): {}", out2.trim());

    // Cleanup workspace repo
    let _ = Command::new("gh")
        .args(["repo", "delete", &ws_repo_name, "--yes"])
        .env("GH_TOKEN", &config.gh_token)
        .output();
}

/// E2E: scrum-compact + Telegram bridge — full operator journey.
///
/// Happy path covering the bridge profile variation:
/// init --bridge telegram → hire → teams show (bridge visible) →
/// bridge identity add → identity list → sync --bridge → verify RObot.enabled
fn e2e_bridge_lifecycle_impl(config: &E2eConfig) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let repo_name = format!("bm-e2e-bridge-{}", timestamp);
    let full_name = format!("{}/{}", config.gh_org, repo_name);

    // RAII guard: deletes the repo on drop
    let _cleanup = super::github::TempRepo {
        full_name: full_name.clone(),
    };

    let tmp = tempfile::tempdir().unwrap();
    let workzone = tmp.path().join("workspaces");

    // 1. bm init --non-interactive --profile scrum-compact --bridge telegram
    let mut cmd = bm_cmd();
    cmd.args([
        "init",
        "--non-interactive",
        "--profile",
        "scrum-compact",
        "--team-name",
        "e2e-bridge",
        "--org",
        &config.gh_org,
        "--repo",
        &repo_name,
        "--bridge",
        "telegram",
        "--workzone",
        &workzone.to_string_lossy(),
    ])
    .env("HOME", tmp.path())
    .env("GH_TOKEN", &config.gh_token)
    .env("GIT_AUTHOR_NAME", "BM E2E")
    .env("GIT_AUTHOR_EMAIL", "e2e@botminter.test")
    .env("GIT_COMMITTER_NAME", "BM E2E")
    .env("GIT_COMMITTER_EMAIL", "e2e@botminter.test");
    let stdout = assert_cmd_success(&mut cmd);
    eprintln!("bm init: {}", stdout.trim());

    // Set up git auth in temp HOME (credential helper + user identity)
    super::helpers::setup_git_auth(tmp.path());

    // Verify bridge recorded in botminter.yml
    let team_repo = workzone.join("e2e-bridge").join("team");
    let manifest_content = fs::read_to_string(team_repo.join("botminter.yml")).unwrap();
    assert!(
        manifest_content.contains("bridge:"),
        "botminter.yml should contain bridge key after init --bridge telegram"
    );

    // 2. bm hire superman --name bob
    let mut cmd = bm_cmd();
    cmd.args(["hire", "superman", "--name", "bob", "-t", "e2e-bridge"])
        .env("HOME", tmp.path())
        .env("GIT_AUTHOR_NAME", "BM E2E")
        .env("GIT_AUTHOR_EMAIL", "e2e@botminter.test")
        .env("GIT_COMMITTER_NAME", "BM E2E")
        .env("GIT_COMMITTER_EMAIL", "e2e@botminter.test");
    let stdout = assert_cmd_success(&mut cmd);
    eprintln!("bm hire: {}", stdout.trim());

    // 3. bm teams show → assert Bridge: visible
    let mut cmd = bm_cmd();
    cmd.args(["teams", "show", "-t", "e2e-bridge"])
        .env("HOME", tmp.path());
    let stdout = assert_cmd_success(&mut cmd);
    eprintln!("bm teams show: {}", stdout.trim());
    assert!(
        stdout.contains("Bridge:"),
        "teams show should display bridge info, got:\n{}",
        stdout
    );

    // 4. bm bridge identity add superman-bob (with env var token)
    let mut cmd = bm_cmd();
    cmd.args(["bridge", "identity", "add", "superman-bob", "-t", "e2e-bridge"])
        .env("HOME", tmp.path())
        .env("BM_BRIDGE_TOKEN_SUPERMAN_BOB", "123456:ABC-DEF-e2e-test-token");
    let stdout = assert_cmd_success(&mut cmd);
    eprintln!("bm bridge identity add: {}", stdout.trim());
    assert!(
        stdout.contains("superman-bob"),
        "identity add should confirm member name, got:\n{}",
        stdout
    );

    // 5. bm bridge identity list → contains superman-bob
    let mut cmd = bm_cmd();
    cmd.args(["bridge", "identity", "list", "-t", "e2e-bridge"])
        .env("HOME", tmp.path());
    let stdout = assert_cmd_success(&mut cmd);
    eprintln!("bm bridge identity list: {}", stdout.trim());
    assert!(
        stdout.contains("superman-bob"),
        "identity list should show superman-bob, got:\n{}",
        stdout
    );

    // 6. bm teams sync --bridge → provisions identities
    let mut cmd = bm_cmd();
    cmd.args(["teams", "sync", "--bridge", "-t", "e2e-bridge"])
        .env("HOME", tmp.path());
    let stdout = assert_cmd_success(&mut cmd);
    eprintln!("bm teams sync --bridge: {}", stdout.trim());
    assert!(
        !stdout.contains("No bridge configured"),
        "sync --bridge should NOT say 'No bridge configured', got:\n{}",
        stdout
    );

    // 7. Verify workspace ralph.yml has RObot.enabled after sync
    let ws_path = workzone.join("e2e-bridge").join("superman-bob");
    if ws_path.join("ralph.yml").exists() {
        let ralph_content = fs::read_to_string(ws_path.join("ralph.yml")).unwrap();
        // RObot.enabled should be set based on credential availability
        eprintln!("ralph.yml content:\n{}", ralph_content);
        // With credentials available, RObot.enabled should be true
        assert!(
            ralph_content.contains("enabled: true") || ralph_content.contains("enabled:true"),
            "ralph.yml should have RObot.enabled: true when credentials exist, got:\n{}",
            ralph_content
        );
    }

    // Cleanup project boards (same pattern as e2e_init_non_interactive)
    let output = Command::new("gh")
        .args([
            "project", "list", "--owner", &config.gh_org,
            "--format", "json", "--limit", "100",
        ])
        .env("GH_TOKEN", &config.gh_token)
        .output()
        .expect("failed to list projects");

    if output.status.success() {
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
        if let Some(projects) = json["projects"].as_array() {
            for project in projects {
                let title = project["title"].as_str().unwrap_or("");
                if title.contains("e2e-bridge") {
                    if let Some(number) = project["number"].as_u64() {
                        eprintln!("Cleaning up project board #{}: {}", number, title);
                        let _ = Command::new("gh")
                            .args([
                                "project", "delete", "--owner", &config.gh_org,
                                &number.to_string(), "--format", "json",
                            ])
                            .env("GH_TOKEN", &config.gh_token)
                            .output();
                    }
                }
            }
        }
    }
}
