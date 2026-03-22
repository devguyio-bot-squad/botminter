use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;

use super::config::{load_poll_state, save_poll_state, DaemonPaths};
use super::event::{
    is_relevant_event, load_webhook_secret, poll_github_events, resolve_github_repo,
    validate_webhook_signature,
};
use super::log::daemon_log;
use super::process::handle_member_launch;

/// Shared state for axum handlers.
#[derive(Clone)]
struct DaemonState {
    team_name: String,
    paths: Arc<DaemonPaths>,
    webhook_secret: Option<String>,
    shutdown: Arc<AtomicBool>,
}

/// Runs the daemon event loop. Called by the hidden `bm daemon-run` command.
/// This function does not return until the daemon is signaled to stop.
pub fn run_daemon(
    team_name: &str,
    mode: &str,
    port: u16,
    interval: u64,
    bind: &str,
) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    rt.block_on(run_daemon_async(team_name, mode, port, interval, bind))
}

async fn run_daemon_async(
    team_name: &str,
    mode: &str,
    port: u16,
    interval: u64,
    bind: &str,
) -> Result<()> {
    let paths = Arc::new(DaemonPaths::new(team_name)?);
    let shutdown = Arc::new(AtomicBool::new(false));

    daemon_log(&paths, "INFO", &format!("Daemon starting in {} mode", mode));

    let state = DaemonState {
        team_name: team_name.to_string(),
        paths: Arc::clone(&paths),
        webhook_secret: load_webhook_secret(team_name),
        shutdown: Arc::clone(&shutdown),
    };

    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .route("/health", get(health_handler))
        .with_state(state.clone());

    // In poll mode, spawn the background poll loop
    if mode == "poll" {
        let poll_team = team_name.to_string();
        let poll_paths = Arc::clone(&paths);
        let poll_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            run_poll_loop(&poll_team, &poll_paths, interval, &poll_shutdown).await;
        });
    }

    let addr: SocketAddr = format!("{}:{}", bind, port)
        .parse()
        .with_context(|| format!("Invalid bind address: {}:{}", bind, port))?;

    daemon_log(
        &paths,
        "INFO",
        &format!(
            "{} server listening on {}",
            match mode {
                "webhook" => "Webhook",
                "poll" => "Poll",
                _ => mode,
            },
            addr
        ),
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    let shutdown_flag = Arc::clone(&shutdown);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_flag))
        .await
        .context("Server error")?;

    daemon_log(&paths, "INFO", "Daemon stopped");
    Ok(())
}

/// Waits for SIGTERM or SIGINT, then sets the shutdown flag.
async fn shutdown_signal(shutdown: Arc<AtomicBool>) {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    shutdown.store(true, Ordering::SeqCst);
}

/// Axum handler for POST /webhook.
async fn webhook_handler(
    State(state): State<DaemonState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let body_str = match std::str::from_utf8(&body) {
        Ok(s) => s.to_string(),
        Err(_) => {
            daemon_log(&state.paths, "ERROR", "Failed to read request body as UTF-8");
            return StatusCode::BAD_REQUEST;
        }
    };

    // Validate signature if webhook secret is configured
    if let Some(ref secret) = state.webhook_secret {
        let sig_header = headers
            .get("x-hub-signature-256")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if !validate_webhook_signature(secret, &body_str, sig_header.as_deref()) {
            daemon_log(&state.paths, "WARN", "Webhook signature validation failed");
            return StatusCode::FORBIDDEN;
        }
    }

    // Parse event type from header
    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Some(event_type) = event_type {
        if is_relevant_event(&event_type) {
            daemon_log(
                &state.paths,
                "INFO",
                &format!("Received relevant event: {}", event_type),
            );
            let team = state.team_name.clone();
            let paths = Arc::clone(&state.paths);
            let shutdown = Arc::clone(&state.shutdown);
            tokio::task::spawn_blocking(move || {
                handle_member_launch(&team, &paths, &shutdown);
            });
        } else {
            daemon_log(
                &state.paths,
                "DEBUG",
                &format!("Ignoring irrelevant event: {}", event_type),
            );
        }
    }

    StatusCode::OK
}

/// Axum handler for GET /health.
async fn health_handler() -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    let body = serde_json::json!({ "ok": true, "version": version });
    (StatusCode::OK, axum::Json(body))
}

/// Runs the poll loop as a background async task.
async fn run_poll_loop(
    team_name: &str,
    paths: &DaemonPaths,
    interval: u64,
    shutdown: &Arc<AtomicBool>,
) {
    daemon_log(
        paths,
        "INFO",
        &format!("Poll mode started, interval: {}s", interval),
    );

    let poll_state_file = paths.poll_state();
    let mut poll_state = load_poll_state(&poll_state_file);

    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval));
    // First tick fires immediately — skip it to match original behavior of polling after
    // a sleep. Actually, the original code polled first then slept, so let the first tick
    // proceed.

    loop {
        ticker.tick().await;

        if shutdown.load(Ordering::SeqCst) {
            daemon_log(
                paths,
                "INFO",
                "Received shutdown signal, stopping poll loop",
            );
            break;
        }

        // All poll operations (resolve_github_repo, poll_github_events,
        // handle_member_launch) are blocking sync calls that spawn subprocesses
        // or do file I/O. Run them on the blocking thread pool to avoid starving
        // the async runtime's worker threads.
        let poll_team = team_name.to_string();
        let poll_state_clone = poll_state.clone();
        let poll_paths = paths.clone();
        let poll_shutdown = Arc::clone(shutdown);

        let result = tokio::task::spawn_blocking(move || {
            let github_repo = resolve_github_repo(&poll_team)?;
            let events = poll_github_events(&github_repo, &poll_state_clone)?;
            let relevant_count = events
                .iter()
                .filter(|e| is_relevant_event(&e.event_type))
                .count();

            if relevant_count > 0 {
                daemon_log(
                    &poll_paths,
                    "INFO",
                    &format!("Found {} relevant event(s)", relevant_count),
                );
                handle_member_launch(&poll_team, &poll_paths, &poll_shutdown);
            }

            Ok::<_, anyhow::Error>(events)
        })
        .await;

        match result {
            Ok(Ok(events)) => {
                if let Some(latest) = events.first() {
                    poll_state.last_event_id = Some(latest.id.clone());
                }
                poll_state.last_poll_at = Some(chrono::Utc::now().to_rfc3339());
                save_poll_state(&poll_state_file, &poll_state);
            }
            Ok(Err(e)) => {
                daemon_log(
                    paths,
                    "ERROR",
                    &format!("Poll cycle failed: {}", e),
                );
            }
            Err(e) => {
                daemon_log(
                    paths,
                    "ERROR",
                    &format!("Poll task panicked: {}", e),
                );
            }
        }
    }

    daemon_log(paths, "INFO", "Poll loop stopped");
}
