pub mod files;
pub mod members;
pub mod overview;
pub mod process;
pub mod state;
pub mod teams;

use axum::routing::get;
use axum::Router;

use self::files::{list_tree, read_file, write_file};
use self::members::{get_member, list_members};
use self::overview::team_overview;
use self::process::team_process;
use self::state::WebState;
use self::teams::list_teams;

/// Builds the console web API router with all `/api/*` routes.
pub fn web_router(state: WebState) -> Router {
    Router::new()
        .route("/api/teams", get(list_teams))
        .route("/api/teams/{team}/overview", get(team_overview))
        .route("/api/teams/{team}/process", get(team_process))
        .route("/api/teams/{team}/members", get(list_members))
        .route("/api/teams/{team}/members/{name}", get(get_member))
        .route("/api/teams/{team}/tree", get(list_tree))
        .route(
            "/api/teams/{team}/files/{*path}",
            get(read_file).put(write_file),
        )
        .with_state(state)
}
