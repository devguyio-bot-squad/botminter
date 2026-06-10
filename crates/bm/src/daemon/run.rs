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
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::api;
use super::config::{load_poll_state, save_poll_state, DaemonConfig, DaemonPaths};
use super::event::{
    is_relevant_event, load_webhook_secret, poll_github_events, resolve_github_repo,
    validate_webhook_signature,
};
use super::log::daemon_log;
use super::process::handle_member_launch;
use super::sessions_api::{
    run_credential_refresh_loop, sessions_router, BridgeContext, CredentialRefreshable,
    SessionsApiState,
};
use crate::bridge;
use crate::config as app_config;
use crate::workspace::HydrationWorkspaceConfig;
use crate::web::state::WebState;
use crate::web::web_router;

/// Shared state for axum handlers.
#[derive(Clone)]
pub(super) struct DaemonState {
    pub(super) team_name: String,
    pub(super) paths: Arc<DaemonPaths>,
    pub(super) webhook_secret: Option<String>,
    pub(super) shutdown: Arc<AtomicBool>,
    pub(super) mode: String,
    pub(super) started_at: Option<std::time::Instant>,
    /// Cached config loaded once at daemon startup. API handlers use this
    /// instead of re-reading from disk on every request, which avoids failures
    /// when the HOME directory changes (e.g., in E2E tests).
    pub(super) config: Arc<app_config::BotminterConfig>,
    pub(super) team_entry: Arc<app_config::TeamEntry>,
    /// Sessions API state — shared with the poll loop and webhook handler so
    /// event-driven member launches go through the sessions API (not the legacy
    /// formation path), creating ephemeral session workspaces on disk.
    pub(super) sessions_state: SessionsApiState,
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
    // Resolve the isolated keyring D-Bus address BEFORE creating the tokio
    // runtime. `with_keyring_dbus` in credential.rs swaps DBUS_SESSION_BUS_ADDRESS
    // via `std::env::set_var`, which is unsound in multi-threaded processes.
    // By setting DBUS_SESSION_BUS_ADDRESS here and removing BM_KEYRING_DBUS,
    // `with_keyring_dbus` becomes a no-op and the keyring uses the right
    // D-Bus session without any env var mutation during runtime.
    if let Ok(dbus) = std::env::var("BM_KEYRING_DBUS") {
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &dbus);
        std::env::remove_var("BM_KEYRING_DBUS");
    }

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
    // NOTE: Do NOT set SIGCHLD=SIG_IGN here. While it prevents zombie children,
    // it also breaks Command::output() (used by gh api in poll mode) because
    // the auto-reaped child causes waitpid to return ECHILD. Instead, each
    // spawn site calls reap_child() which spawns a thread that waits on the
    // Child handle, calling waitpid() to reap the process on exit.

    let paths = Arc::new(DaemonPaths::new(team_name)?);
    let shutdown = Arc::new(AtomicBool::new(false));

    daemon_log(&paths, "INFO", &format!("Daemon starting in {} mode", mode));

    // Load config once at startup and cache it. API handlers use these
    // cached values instead of re-reading config from disk on every request.
    let cfg = app_config::load()
        .context("Daemon failed to load config at startup")?;
    let team_entry = app_config::resolve_team(&cfg, Some(team_name))
        .context("Daemon failed to resolve team at startup")?
        .clone();

    // Build sessions_state first so it can be embedded in DaemonState and
    // shared with the poll loop and webhook handler.
    let sessions_state = {
        let team_repo_path = team_entry.path.join("team");
        let repo_urls = read_project_repos(&team_repo_path);
        let team_repo_url = format!("https://github.com/{}.git", team_entry.github_repo);
        let credential_resolver = crate::formation::create_local_formation(team_name)
            .ok()
            .and_then(|f| {
                f.credential_store(crate::formation::CredentialDomain::GitHubApp {
                    team_name: team_name.to_string(),
                    member_name: String::new(),
                })
                .ok()
            })
            .map(|store| {
                let store: std::sync::Arc<dyn crate::formation::KeyValueCredentialStore> =
                    std::sync::Arc::from(store);
                let provider = crate::workspace::KeyringAppTokenProvider::new(store);
                std::sync::Arc::new(provider) as std::sync::Arc<dyn crate::workspace::AppTokenProvider>
            });

        let hydration_config = HydrationWorkspaceConfig {
            clones_dir: paths.sessions_base().join("clones"),
            sessions_base: paths.sessions_base(),
            team_repo_path: team_repo_path.clone(),
            credential_base: paths.sessions_base().join("credentials"),
            freshness_threshold: std::time::Duration::from_secs(300),
            repo_urls,
            team_repo_url,
            team_repo_branch: "main".to_string(),
            workspace_base: team_entry.path.clone(),
            project_number: team_entry.project_number,
            skill_dirs: vec![],
            credential_resolver,
            project_names: vec![],
        };

        // Resolve bridge credentials for injecting env vars when launching ralph.
        let bridge_context = resolve_bridge_context(&team_repo_path, &team_entry, &cfg);

        SessionsApiState::new_with_workspace_ops(
            paths.sessions_registry(),
            hydration_config,
            bridge_context,
        )
    };

    let state = DaemonState {
        team_name: team_name.to_string(),
        paths: Arc::clone(&paths),
        webhook_secret: load_webhook_secret(team_name),
        shutdown: Arc::clone(&shutdown),
        mode: mode.to_string(),
        started_at: Some(std::time::Instant::now()),
        config: Arc::new(cfg),
        team_entry: Arc::new(team_entry),
        sessions_state: sessions_state.clone(),
    };

    // Resolve config path for the web API (console routes)
    let config_path = app_config::config_path()
        .unwrap_or_else(|_| std::path::PathBuf::from("~/.botminter/config.yml"));
    let web_state = WebState {
        config_path: Arc::new(config_path),
    };

    // CORS: allow requests from localhost dev servers (Vite on :5173, etc.)
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin
                .to_str()
                .map(|o| {
                    o.starts_with("http://localhost:") || o.starts_with("http://127.0.0.1:")
                })
                .unwrap_or(false)
        }))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    // Startup recovery: mark Active/Finalizing sessions with dead PIDs as Failed
    let recovered = sessions_state.recover_stale_sessions();
    if !recovered.is_empty() {
        daemon_log(
            &paths,
            "INFO",
            &format!("Recovery: {} stale sessions marked Failed", recovered.len()),
        );
    }

    // Retention GC: periodically clean up expired/over-budget retained sessions
    {
        let gc_state = sessions_state.clone();
        let gc_paths = Arc::clone(&paths);
        let gc_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let scan_interval = crate::session::retention::RetentionPolicy::default().scan_interval;
            loop {
                tokio::time::sleep(scan_interval).await;
                if gc_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                let cleaned = gc_state.run_retention_cycle();
                if !cleaned.is_empty() {
                    daemon_log(
                        &gc_paths,
                        "INFO",
                        &format!("GC: cleaned {} expired sessions", cleaned.len()),
                    );
                }
            }
        });
    }

    // Credential refresh: periodically renew GitHub App tokens for active-session members
    {
        let cred_state = sessions_state.clone();
        let cred_refresher: Arc<dyn CredentialRefreshable> =
            Arc::new(sessions_state.clone());
        let cred_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            run_credential_refresh_loop(
                cred_state,
                cred_refresher,
                std::time::Duration::from_secs(300),
                cred_shutdown,
            )
            .await;
        });
    }

    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .route("/health", get(health_handler))
        // Member lifecycle API
        .route("/api/members/stop", post(api::stop_members_handler))
        .route("/api/members", get(api::list_members_handler))
        .route("/api/health", get(api::health_check_handler))
        .with_state(state.clone())
        // Session management API
        .merge(sessions_router(sessions_state.clone()))
        .merge(web_router(web_state))
        .layer(cors);

    // In poll mode, spawn the background poll loop
    if mode == "poll" {
        let poll_team = team_name.to_string();
        let poll_paths = Arc::clone(&paths);
        let poll_shutdown = Arc::clone(&shutdown);
        let poll_sessions = sessions_state.clone();
        tokio::spawn(async move {
            run_poll_loop(&poll_team, &poll_paths, interval, &poll_shutdown, poll_sessions).await;
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
    daemon_log(
        &paths,
        "INFO",
        &format!("Console available at http://{}:{}", bind, port),
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    // After binding, write the daemon config with the actual port. This is
    // critical when port=0 (OS-assigned): the parent process and clients
    // read this file to discover the daemon's address.
    let actual_addr = listener.local_addr()
        .context("Failed to get listener local address")?;
    let daemon_cfg = DaemonConfig {
        team: team_name.to_string(),
        mode: mode.to_string(),
        port: actual_addr.port(),
        interval_secs: interval,
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    let cfg_contents = serde_json::to_string_pretty(&daemon_cfg)
        .context("Failed to serialize daemon config")?;
    std::fs::write(paths.config(), &cfg_contents)
        .with_context(|| format!("Failed to write daemon config to {}", paths.config().display()))?;

    daemon_log(
        &paths,
        "INFO",
        &format!("Daemon config written (port={})", actual_addr.port()),
    );

    let shutdown_flag = Arc::clone(&shutdown);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_flag))
        .await
        .context("Server error")?;

    // Clean up: stop all active sessions before exiting.
    // Send SIGTERM, wait up to 5 seconds, then SIGKILL any survivors.
    daemon_log(&paths, "INFO", "Stopping sessions before exit...");
    state.sessions_state.stop_autonomous_sessions_gracefully();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if !state.sessions_state.has_alive_autonomous_sessions() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    state.sessions_state.force_stop_autonomous_sessions();

    daemon_log(&paths, "INFO", "Daemon stopped");
    Ok(())
}

/// Reads the project repo list from `botminter.yml` in the team repo.
/// Returns `Vec<(url, project_name)>` for each project entry.
/// Returns an empty list if the manifest is absent or unparseable.
fn read_project_repos(team_repo_path: &std::path::Path) -> Vec<(String, String)> {
    let manifest_path = team_repo_path.join("botminter.yml");
    let contents = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let manifest: serde_yml::Value = match serde_yml::from_str(&contents) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    manifest["projects"]
        .as_sequence()
        .map(|ps| {
            ps.iter()
                .filter_map(|p| {
                    let name = p["name"].as_str()?.to_string();
                    let url = p["fork_url"].as_str()?.to_string();
                    Some((url, name))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve bridge credentials for per-member env injection when launching ralph.
/// Returns None if no bridge is configured or the bridge state cannot be read.
fn resolve_bridge_context(
    team_repo_path: &std::path::Path,
    team_entry: &app_config::TeamEntry,
    cfg: &app_config::BotminterConfig,
) -> Option<BridgeContext> {
    let bridge_dir = bridge::discover(team_repo_path, &team_entry.name).ok().flatten()?;
    let bridge_manifest = bridge::load_manifest(&bridge_dir).ok()?;
    // metadata.name is the plugin identifier (e.g. "tuwunel", "rocketchat", "telegram").
    // launch_ralph() uses this name to select the correct env var (RALPH_MATRIX_ACCESS_TOKEN, etc.).
    let bridge_name = bridge_manifest.metadata.name.clone();
    let bstate_path = bridge::state_path(&cfg.workzone, &team_entry.name);
    let credential_store = bridge::LocalCredentialStore::new(
        &team_entry.name,
        &bridge_name,
        bstate_path.clone(),
    )
    .with_collection(cfg.keyring_collection.clone());

    Some(BridgeContext {
        bridge_type_name: bridge_name,
        bstate_path,
        credential_store,
    })
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
            let sessions = state.sessions_state.clone();
            tokio::task::spawn_blocking(move || {
                handle_member_launch(&team, &paths, &shutdown, &sessions);
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
    sessions_state: SessionsApiState,
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
        let poll_sessions = sessions_state.clone();

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
                handle_member_launch(&poll_team, &poll_paths, &poll_shutdown, &poll_sessions);
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
                    &format!("Poll cycle failed: {:#}", e),
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
