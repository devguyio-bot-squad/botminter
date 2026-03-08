//! Custom E2E test harness for the `bm` CLI.
//!
//! Uses libtest-mimic to accept custom CLI arguments.
//! The --gh-token argument is MANDATORY — tests exercise real GitHub APIs.
//!
//! Arguments:
//!   --gh-token <TOKEN>    GitHub token for API access (required)
//!   --gh-org <ORG>        GitHub org for test repos/projects (required)

mod helpers;

mod daemon_lifecycle;
mod github;
mod init_to_sync;
mod start_to_stop;
mod telegram;

use helpers::E2eConfig;
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Extract custom args before passing remaining args to libtest-mimic
    let (gh_token, gh_org, remaining_args) = extract_custom_args(&args);

    // Pre-flight: verify GitHub auth before running any tests
    helpers::preflight_gh_auth();

    let config = E2eConfig {
        gh_token,
        gh_org,
    };

    // Parse libtest-mimic arguments from remaining args
    let test_args = Arguments::from_iter(remaining_args);

    // Collect all tests from all modules
    let mut tests = Vec::new();
    tests.extend(smoke_tests(&config));
    tests.extend(init_to_sync::tests(&config));
    tests.extend(daemon_lifecycle::tests(&config));
    tests.extend(start_to_stop::tests(&config));
    tests.extend(telegram::tests(&config));

    libtest_mimic::run(&test_args, tests).exit();
}

/// Extracts --gh-token and --gh-org from args. Returns (token, org, remaining_args).
/// Exits with usage message if either is missing.
fn extract_custom_args(args: &[String]) -> (String, String, Vec<String>) {
    let mut token: Option<String> = None;
    let mut org: Option<String> = None;
    let mut remaining = Vec::new();
    let mut iter = args.iter();

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
            _ => remaining.push(arg.clone()),
        }
    }

    let mut missing = Vec::new();
    if token.is_none() {
        missing.push("--gh-token <TOKEN>");
    }
    if org.is_none() {
        missing.push("--gh-org <ORG>");
    }
    if !missing.is_empty() {
        eprintln!(
            "Error: missing required arguments: {}",
            missing.join(", ")
        );
        std::process::exit(1);
    }

    (token.unwrap(), org.unwrap(), remaining)
}

/// Smoke tests — verify GitHub and Telegram infrastructure before running the full suite.
fn smoke_tests(config: &E2eConfig) -> Vec<Trial> {
    let cfg = config.clone();
    vec![Trial::test("e2e_harness_smoke", move || {
        helpers::run_test(|| {
            // GitHub smoke
            let repo = github::TempRepo::new_in_org("bm-e2e-smoke", &cfg.gh_org)
                .expect("GitHub repo creation failed");
            let labels = github::list_labels(&repo.full_name);
            eprintln!(
                "GitHub smoke: repo {} has {} default labels",
                repo.full_name,
                labels.len()
            );

            let issues = github::list_issues(&repo.full_name);
            assert!(
                issues.is_empty(),
                "fresh repo should have no issues, found: {:?}",
                issues
            );

            // Telegram smoke
            if telegram::podman_available() {
                let mock = telegram::TgMock::start();
                let token = "test-token-smoke";
                let chat_id = 12345i64;
                mock.inject_message(token, "hello from e2e smoke test", chat_id);
                let requests = mock.get_requests(token, "sendMessage");
                eprintln!(
                    "Telegram smoke: tg-mock has {} sendMessage requests",
                    requests.len()
                );
            } else {
                eprintln!("SKIP: podman not available -- skipping tg-mock smoke");
            }
        })
    })]
}
