pub mod overview;
pub mod state;
pub mod teams;

use axum::routing::get;
use axum::Router;

use self::overview::team_overview;
use self::state::WebState;
use self::teams::list_teams;

/// Builds the console web API router with all `/api/*` routes.
pub fn web_router(state: WebState) -> Router {
    Router::new()
        .route("/api/teams", get(list_teams))
        .route("/api/teams/{team}/overview", get(team_overview))
        .with_state(state)
}
