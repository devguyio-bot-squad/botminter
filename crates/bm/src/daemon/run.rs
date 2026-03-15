use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};

use super::config::{load_poll_state, save_poll_state, DaemonPaths};
use super::event::{
    is_relevant_event, load_webhook_secret, poll_github_events, resolve_github_repo,
    validate_webhook_signature,
};
use super::log::daemon_log;
use super::process::{handle_member_launch, setup_signal_handlers, sleep_interruptible};

/// Runs the daemon event loop. Called by the hidden `bm daemon-run` command.
/// This function does not return until the daemon is signaled to stop.
pub fn run_daemon(team_name: &str, mode: &str, port: u16, interval: u64) -> Result<()> {
    let paths = DaemonPaths::new(team_name)?;
    let shutdown = setup_signal_handlers();

    daemon_log(&paths, "INFO", &format!("Daemon starting in {} mode", mode));

    match mode {
        "webhook" => run_webhook_mode(team_name, &paths, port, &shutdown),
        "poll" => run_poll_mode(team_name, &paths, interval, &shutdown),
        _ => bail!("Invalid daemon mode: {}", mode),
    }
}

/// Runs the daemon in webhook mode using tiny_http.
fn run_webhook_mode(
    team_name: &str,
    paths: &DaemonPaths,
    port: u16,
    shutdown: &Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    daemon_log(
        paths,
        "INFO",
        &format!("Webhook server listening on port {}", port),
    );

    let webhook_secret = load_webhook_secret(team_name);

    loop {
        if shutdown.load(Ordering::SeqCst) {
            daemon_log(
                paths,
                "INFO",
                "Received shutdown signal, stopping webhook server",
            );
            break;
        }

        match server.recv_timeout(Duration::from_secs(1)) {
            Ok(Some(mut request)) => {
                let path = request.url().to_string();
                let method = request.method().to_string();

                if method != "POST" || path != "/webhook" {
                    let response =
                        tiny_http::Response::from_string("Not Found").with_status_code(404);
                    let _ = request.respond(response);
                    continue;
                }

                // Read body
                let mut body = String::new();
                if let Err(e) = request.as_reader().read_to_string(&mut body) {
                    daemon_log(
                        paths,
                        "ERROR",
                        &format!("Failed to read request body: {}", e),
                    );
                    let response =
                        tiny_http::Response::from_string("Bad Request").with_status_code(400);
                    let _ = request.respond(response);
                    continue;
                }

                // Validate signature if webhook secret is configured
                if let Some(ref secret) = webhook_secret {
                    let sig_header = request
                        .headers()
                        .iter()
                        .find(|h| {
                            h.field.as_str() == "X-Hub-Signature-256"
                                || h.field.as_str() == "x-hub-signature-256"
                        })
                        .map(|h| h.value.as_str().to_string());

                    if !validate_webhook_signature(secret, &body, sig_header.as_deref()) {
                        daemon_log(paths, "WARN", "Webhook signature validation failed");
                        let response = tiny_http::Response::from_string("Forbidden")
                            .with_status_code(403);
                        let _ = request.respond(response);
                        continue;
                    }
                }

                // Parse event type from header
                let event_type = request
                    .headers()
                    .iter()
                    .find(|h| {
                        h.field.as_str() == "X-GitHub-Event"
                            || h.field.as_str() == "x-github-event"
                    })
                    .map(|h| h.value.as_str().to_string());

                let response =
                    tiny_http::Response::from_string("OK").with_status_code(200);
                let _ = request.respond(response);

                if let Some(event_type) = event_type {
                    if is_relevant_event(&event_type) {
                        daemon_log(
                            paths,
                            "INFO",
                            &format!("Received relevant event: {}", event_type),
                        );
                        handle_member_launch(team_name, paths, shutdown);
                    } else {
                        daemon_log(
                            paths,
                            "DEBUG",
                            &format!("Ignoring irrelevant event: {}", event_type),
                        );
                    }
                }
            }
            Ok(None) => {
                // Timeout — no request, check shutdown flag on next iteration
            }
            Err(e) => {
                daemon_log(paths, "ERROR", &format!("Server error: {}", e));
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }

    daemon_log(paths, "INFO", "Daemon stopped");
    Ok(())
}

/// Runs the daemon in poll mode using gh API.
fn run_poll_mode(
    team_name: &str,
    paths: &DaemonPaths,
    interval: u64,
    shutdown: &Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    daemon_log(
        paths,
        "INFO",
        &format!("Poll mode started, interval: {}s", interval),
    );

    let poll_state_file = paths.poll_state();
    let mut poll_state = load_poll_state(&poll_state_file);

    loop {
        if shutdown.load(Ordering::SeqCst) {
            daemon_log(
                paths,
                "INFO",
                "Received shutdown signal, stopping poll loop",
            );
            break;
        }

        let github_repo = match resolve_github_repo(team_name) {
            Ok(repo) => repo,
            Err(e) => {
                daemon_log(
                    paths,
                    "ERROR",
                    &format!("Failed to resolve GitHub repo: {}", e),
                );
                sleep_interruptible(interval, shutdown);
                continue;
            }
        };

        match poll_github_events(&github_repo, &poll_state) {
            Ok(events) => {
                let relevant_count = events
                    .iter()
                    .filter(|e| is_relevant_event(&e.event_type))
                    .count();

                if relevant_count > 0 {
                    daemon_log(
                        paths,
                        "INFO",
                        &format!("Found {} relevant event(s)", relevant_count),
                    );
                    handle_member_launch(team_name, paths, shutdown);
                }

                if let Some(latest) = events.first() {
                    poll_state.last_event_id = Some(latest.id.clone());
                }
                poll_state.last_poll_at = Some(chrono::Utc::now().to_rfc3339());
                save_poll_state(&poll_state_file, &poll_state);
            }
            Err(e) => {
                daemon_log(
                    paths,
                    "ERROR",
                    &format!("Failed to poll GitHub events: {}", e),
                );
            }
        }

        sleep_interruptible(interval, shutdown);
    }

    daemon_log(paths, "INFO", "Daemon stopped");
    Ok(())
}
