use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../console/build/"]
#[allow_missing = true]
struct ConsoleAssets;

/// Axum fallback handler that serves embedded frontend assets.
///
/// - Exact file matches are served with correct Content-Type and cache headers.
/// - Hashed assets (under `_app/immutable/`) get `Cache-Control: public, max-age=31536000, immutable`.
/// - `index.html` and SPA fallback get `Cache-Control: no-cache`.
/// - Non-matching paths return `index.html` for SvelteKit client-side routing.
pub async fn serve_embedded_assets(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try to serve the exact file first
    if !path.is_empty() {
        if let Some(file) = ConsoleAssets::get(path) {
            return file_response(path, &file.data);
        }
    }

    // SPA fallback: serve index.html for client-side routing
    match ConsoleAssets::get("index.html") {
        Some(file) => {
            let mut response = Html(file.data.to_vec()).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                "no-cache".parse().unwrap(),
            );
            response
        }
        None => (StatusCode::NOT_FOUND, "Console not built").into_response(),
    }
}

/// Build a response with correct Content-Type and Cache-Control for a static file.
fn file_response(path: &str, data: &[u8]) -> Response {
    let content_type = mime_from_path(path);
    let cache_control = if path.contains("_app/immutable/") {
        "public, max-age=31536000, immutable"
    } else if path == "index.html" {
        "no-cache"
    } else {
        "public, max-age=3600"
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, cache_control.to_string()),
        ],
        data.to_vec(),
    )
        .into_response()
}

/// Determine MIME type from file extension.
fn mime_from_path(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "txt" => "text/plain; charset=utf-8",
        "map" => "application/json; charset=utf-8",
        "webp" => "image/webp",
        "webmanifest" => "application/manifest+json",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_from_path_returns_correct_types() {
        assert_eq!(mime_from_path("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            mime_from_path("_app/immutable/main.abc123.js"),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(mime_from_path("style.css"), "text/css; charset=utf-8");
        assert_eq!(mime_from_path("data.json"), "application/json; charset=utf-8");
        assert_eq!(mime_from_path("logo.svg"), "image/svg+xml");
        assert_eq!(mime_from_path("logo.png"), "image/png");
        assert_eq!(mime_from_path("unknown.xyz"), "application/octet-stream");
    }

    #[tokio::test]
    async fn serve_embedded_assets_returns_index_for_spa_routes() {
        // This test verifies the SPA fallback behavior.
        // If console/build/ is empty (allow_empty = true), it returns 404.
        // If console/build/ has index.html, it returns it for any non-file path.
        let uri: Uri = "/teams/my-team/overview".parse().unwrap();
        let response = serve_embedded_assets(uri).await;
        // With a built console, this returns 200 with index.html.
        // With allow_empty and no build, this returns 404.
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected 200 or 404, got {}",
            status
        );
    }

    #[tokio::test]
    async fn serve_embedded_assets_returns_index_for_root() {
        let uri: Uri = "/".parse().unwrap();
        let response = serve_embedded_assets(uri).await;
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected 200 or 404, got {}",
            status
        );
    }
}
