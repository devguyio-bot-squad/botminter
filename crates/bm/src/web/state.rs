use std::path::PathBuf;
use std::sync::Arc;

/// Shared state for the console web API handlers.
#[derive(Clone)]
pub struct WebState {
    /// Path to the botminter config file (e.g., ~/.botminter/config.yml).
    pub config_path: Arc<PathBuf>,
}
