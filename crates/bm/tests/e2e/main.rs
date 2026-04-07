//! Custom E2E test harness for the `bm` CLI.
//!
//! Uses libtest-mimic to accept custom CLI arguments.
//! The --gh-token argument is MANDATORY — tests exercise real GitHub APIs.
//!
//! Arguments:
//!   --gh-token <TOKEN>    GitHub token for API access (required)
//!   --gh-org <ORG>        GitHub org for test repos/projects (required)
//!   --progressive [SUITE] Step through one case at a time, persisting state
//!   --progressive-reset [SUITE]  Clean up persisted progressive state

mod helpers;
mod test_env;

mod bootstrap;
mod github;
mod github_mock;
mod rocketchat;
mod telegram;
mod tuwunel;

mod isolated;
mod scenarios;

use helpers::{E2eConfig, ProgressState, ProgressiveMode};
use libtest_mimic::Arguments;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Extract custom args before passing remaining args to libtest-mimic
    let parsed = extract_custom_args(&args);

    // Handle --progressive-reset before anything else
    if let Some(ProgressiveMode::Reset(ref suite_filter)) = parsed.progressive {
        handle_reset(suite_filter);
        return;
    }

    // Pre-flight: verify GitHub auth before running any tests
    helpers::preflight_gh_auth();

    let config = E2eConfig {
        gh_token: parsed.gh_token,
        gh_org: parsed.gh_org,
        progressive: parsed.progressive,
        app_id: parsed.app_id,
        app_client_id: parsed.app_client_id,
        app_installation_id: parsed.app_installation_id,
        app_private_key_file: parsed.app_private_key_file,
    };

    // Parse libtest-mimic arguments from remaining args
    let test_args = Arguments::from_iter(parsed.remaining);

    // Collect tests based on mode
    let mut tests = Vec::new();
    if config.progressive.is_none() {
        // Normal mode: isolated + all scenarios (bootstrap cases are in operator journey)
        tests.extend(isolated::tests(&config));
    }
    tests.extend(scenarios::tests(&config));

    libtest_mimic::run(&test_args, tests).exit();
}

/// Parsed custom arguments from the CLI.
struct ParsedArgs {
    gh_token: String,
    gh_org: String,
    app_id: String,
    app_client_id: String,
    app_installation_id: String,
    app_private_key_file: String,
    progressive: Option<ProgressiveMode>,
    remaining: Vec<String>,
}

/// Extracts custom args from the CLI.
fn extract_custom_args(args: &[String]) -> ParsedArgs {
    let mut token: Option<String> = None;
    let mut org: Option<String> = None;
    let mut app_id: Option<String> = None;
    let mut app_client_id: Option<String> = None;
    let mut app_installation_id: Option<String> = None;
    let mut app_private_key_file: Option<String> = None;
    let mut progressive: Option<ProgressiveMode> = None;
    let mut remaining = Vec::new();
    let mut iter = args.iter().peekable();

    // Always keep the binary name
    if let Some(bin) = iter.next() {
        remaining.push(bin.clone());
    }

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--gh-token" => {
                token = iter.next().cloned();
                if token.is_none() {
                    eprintln!("Error: --gh-token requires a value");
                    std::process::exit(1);
                }
            }
            "--gh-org" => {
                org = iter.next().cloned();
                if org.is_none() {
                    eprintln!("Error: --gh-org requires a value");
                    std::process::exit(1);
                }
            }
            "--app-id" => {
                app_id = iter.next().cloned();
                if app_id.is_none() {
                    eprintln!("Error: --app-id requires a value");
                    std::process::exit(1);
                }
            }
            "--app-client-id" => {
                app_client_id = iter.next().cloned();
                if app_client_id.is_none() {
                    eprintln!("Error: --app-client-id requires a value");
                    std::process::exit(1);
                }
            }
            "--app-installation-id" => {
                app_installation_id = iter.next().cloned();
                if app_installation_id.is_none() {
                    eprintln!("Error: --app-installation-id requires a value");
                    std::process::exit(1);
                }
            }
            "--app-private-key-file" => {
                app_private_key_file = iter.next().cloned();
                if app_private_key_file.is_none() {
                    eprintln!("Error: --app-private-key-file requires a value");
                    std::process::exit(1);
                }
            }
            "--progressive" => {
                // Optional suite name follows if next arg doesn't start with --
                let suite = iter
                    .peek()
                    .filter(|a| !a.starts_with("--"))
                    .cloned()
                    .cloned();
                if suite.is_some() {
                    iter.next(); // consume the suite name
                }
                progressive = Some(ProgressiveMode::Step(suite));
            }
            "--progressive-reset" => {
                let suite = iter
                    .peek()
                    .filter(|a| !a.starts_with("--"))
                    .cloned()
                    .cloned();
                if suite.is_some() {
                    iter.next();
                }
                progressive = Some(ProgressiveMode::Reset(suite));
            }
            _ => remaining.push(arg.clone()),
        }
    }

    let mut missing = Vec::new();
    // Token, org, and App creds not required for --progressive-reset
    if !matches!(progressive, Some(ProgressiveMode::Reset(_))) {
        if token.is_none() {
            missing.push("--gh-token <TOKEN>");
        }
        if org.is_none() {
            missing.push("--gh-org <ORG>");
        }
        if app_id.is_none() {
            missing.push("--app-id <APP_ID>");
        }
        if app_client_id.is_none() {
            missing.push("--app-client-id <CLIENT_ID>");
        }
        if app_installation_id.is_none() {
            missing.push("--app-installation-id <INSTALLATION_ID>");
        }
        if app_private_key_file.is_none() {
            missing.push("--app-private-key-file <PATH>");
        }
    }
    if !missing.is_empty() {
        eprintln!(
            "Error: missing required arguments: {}",
            missing.join(", ")
        );
        std::process::exit(1);
    }

    ParsedArgs {
        gh_token: token.unwrap_or_default(),
        gh_org: org.unwrap_or_default(),
        app_id: app_id.unwrap_or_default(),
        app_client_id: app_client_id.unwrap_or_default(),
        app_installation_id: app_installation_id.unwrap_or_default(),
        app_private_key_file: app_private_key_file.unwrap_or_default(),
        progressive,
        remaining,
    }
}

/// Handle --progressive-reset: clean up GitHub repos, tg-mock containers, and state files.
fn handle_reset(suite_filter: &Option<String>) {
    let suites = if let Some(name) = suite_filter {
        vec![name.clone()]
    } else {
        ProgressState::list_all()
    };

    if suites.is_empty() {
        eprintln!("No progressive state to reset.");
        return;
    }

    for suite_name in &suites {
        if let Some(state) = ProgressState::load(suite_name) {
            // Delete GitHub repo
            eprintln!("Deleting repo {}...", state.repo_full_name);
            let _ = std::process::Command::new("gh")
                .args(["repo", "delete", &state.repo_full_name, "--yes"])
                .output();

            // Stop tg-mock container
            if let Some(cid) = &state.tg_mock_container_id {
                eprintln!("Stopping tg-mock container {}...", &cid[..12.min(cid.len())]);
                let _ = std::process::Command::new("podman")
                    .args(["stop", "-t", "2", cid])
                    .output();
                let _ = std::process::Command::new("podman")
                    .args(["rm", "-f", cid])
                    .output();
            }
        }

        // Delete state files + home dir
        ProgressState::delete(suite_name);
        eprintln!("Progress reset for {}", suite_name);
    }

    if suite_filter.is_none() {
        // Nuclear option: wipe entire progress dir
        let _ = std::fs::remove_dir_all(ProgressState::progress_base());
        eprintln!("All e2e progress reset.");
    }
}
