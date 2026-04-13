//! Manifest Flow E2E Test
//!
//! Exercises `bm hire` WITHOUT `--reuse-app` against a mock GitHub server.
//! The mock handles form POST, code exchange, and installation query.
//! reqwest drives the browser flow programmatically — no real browser needed.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

use libtest_mimic::Trial;

use super::super::github_mock::GitHubMock;
use super::super::helpers::{E2eConfig, GithubSuite};
use super::super::test_env::TestEnv;

const SUITE_NAME: &str = "scenario_manifest_journey";
const TEAM_NAME: &str = "e2e-manifest";
const PROFILE: &str = "agentic-sdlc-minimal";
const ROLE: &str = "engineer";
const MEMBER_NAME: &str = "mflow";

pub fn scenario(config: &E2eConfig) -> Trial {
    let config = config.clone();
    GithubSuite::new_self_managed(SUITE_NAME, "mock/manifest-test")
        .setup(setup_fn(config.clone()))
        .case("hire_with_manifest_flow", hire_fn(config.clone()))
        .case("cleanup", cleanup_fn())
        .build(&config)
}

pub fn scenario_progressive(config: &E2eConfig) -> Trial {
    let config = config.clone();
    GithubSuite::new_self_managed(SUITE_NAME, "mock/manifest-test")
        .setup(setup_fn(config.clone()))
        .case("hire_with_manifest_flow", hire_fn(config.clone()))
        .case("cleanup", cleanup_fn())
        .build_progressive(&config)
}

fn setup_fn(
    _config: E2eConfig,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        // Start mock GitHub server
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mock_base = rt.block_on(async {
            let mock = GitHubMock::start().await;
            let base = mock.base_url.clone();
            std::mem::forget(mock);
            base
        });
        std::mem::forget(rt);

        env.export("mock_github_base", &mock_base);
        env.save();
        eprintln!("  mock GitHub server on {mock_base}");

        // Create a team locally (--skip-github avoids real API calls)
        let workzone = env.home.join("workspaces");
        env.command("bm")
            .args([
                "init", "--non-interactive",
                "--profile", PROFILE,
                "--team-name", TEAM_NAME,
                "--org", "test-org",
                "--repo", "test-repo",
                "--skip-github",
                "--workzone", &workzone.to_string_lossy(),
            ])
            .run();
        eprintln!("  team created locally");
    }
}

fn hire_fn(
    _config: E2eConfig,
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |env| {
        let mock_base = env.get_export("mock_github_base").unwrap().to_string();
        let bm_bin = env!("CARGO_BIN_EXE_bm");

        // Spawn `bm hire` as a background process — it blocks on the manifest flow.
        // No --reuse-app: this exercises the interactive manifest flow.
        let mut child = Command::new(bm_bin)
            .args([
                "hire", ROLE, "--name", MEMBER_NAME, "-t", TEAM_NAME,
            ])
            .envs(env.resolved_env("bm"))
            .env("BM_GITHUB_API_BASE", &mock_base)
            .env("BM_GITHUB_WEB_BASE", &mock_base)
            .env("BM_NO_BROWSER", "1")
            .env("BM_MANIFEST_TIMEOUT_SECS", "15")
            .env("BM_MANIFEST_POLL_GRACE_SECS", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start bm hire");

        // Drain stderr in background, extract the start URL
        let stderr = child.stderr.take().unwrap();
        let (url_tx, url_rx) = std::sync::mpsc::channel::<String>();
        let stderr_drain = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let mut url_sent = false;
            let mut lines = Vec::new();
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("  [hire] {line}");
                if !url_sent && line.contains("http://127.0.0.1:") && line.contains("/start") {
                    if let Some(start) = line.find("http://") {
                        let url = line[start..].trim().to_string();
                        let _ = url_tx.send(url);
                        url_sent = true;
                    }
                }
                lines.push(line);
            }
            lines
        });

        // Wait for the start URL
        let start_url = url_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("hire never printed the start URL");
        eprintln!("  start_url: {start_url}");

        // Wait for the server to accept connections
        let http = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let body = retry_get(&http, &start_url, 20);
        assert!(body.contains("manifest-form"), "/start should contain the form");

        // Parse form action and manifest from HTML
        let form_action = extract_attr(&body, "action");
        let manifest = extract_attr(&body, "value");
        eprintln!("  form_action: {form_action}");

        // POST to mock GitHub (simulates the browser form submission)
        let resp = http
            .post(&form_action)
            .form(&[("manifest", &manifest)])
            .send()
            .unwrap();
        assert!(resp.status().is_redirection(), "mock should redirect, got {}", resp.status());

        let callback_url = resp.headers().get("location").unwrap().to_str().unwrap().to_string();
        eprintln!("  callback: {callback_url}");

        // Follow redirect to /callback (triggers code exchange with mock)
        let resp = http.get(&callback_url).send().unwrap();
        eprintln!("  /callback -> {}", resp.status());

        // Wait for hire to complete (poller detects installation via mock)
        let exit = child.wait().expect("wait failed");
        let stderr_lines = stderr_drain.join().unwrap_or_default();
        let full_stderr = stderr_lines.join("\n");

        assert!(exit.success(), "bm hire should succeed. stderr:\n{full_stderr}");
        assert!(
            full_stderr.contains("GitHub App created and installed successfully")
                || full_stderr.contains("credentials stored"),
            "Should confirm success. stderr:\n{full_stderr}"
        );
        eprintln!("  manifest flow hire passed!");
    }
}

fn cleanup_fn(
) -> impl Fn(&mut TestEnv) + Send + std::panic::UnwindSafe + std::panic::RefUnwindSafe + 'static {
    move |_env| {
        eprintln!("  cleanup: no external resources");
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Retry GET until the server responds, return the body.
fn retry_get(http: &reqwest::blocking::Client, url: &str, max_attempts: u32) -> String {
    for attempt in 0..max_attempts {
        match http.get(url).send() {
            Ok(r) if r.status().is_success() => return r.text().unwrap(),
            Ok(r) => panic!("/start returned {}", r.status()),
            Err(_) => {
                if attempt == max_attempts - 1 {
                    panic!("Server never responded to {url}");
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    unreachable!()
}

/// Extract an HTML attribute value (handles HTML-escaped content).
fn extract_attr(html: &str, attr: &str) -> String {
    let needle = format!("{attr}=\"");
    let start = html.find(&needle).unwrap() + needle.len();
    let end = html[start..].find('"').unwrap() + start;
    html[start..end]
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
}
