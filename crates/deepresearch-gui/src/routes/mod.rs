mod health;
mod session;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    response::{IntoResponse, Response},
};
use health::health_router;
use session::session_router;
use std::path::Path;
use tokio::fs::{self, canonicalize};

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .nest("/health", health_router())
        .nest("/api", session_router())
        .fallback(spa_fallback)
        .with_state(state)
}

async fn spa_fallback(State(state): State<AppState>, req: Request<Body>) -> Response {
    if !state.gui_enabled() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let assets_dir = state.assets_dir();
    let request_path = req.uri().path().trim_start_matches('/');

    let candidate = if request_path.is_empty() {
        assets_dir.join("index.html")
    } else {
        let joined = assets_dir.join(request_path);
        if is_safe_file(assets_dir.as_ref(), &joined).await {
            joined
        } else {
            assets_dir.join("index.html")
        }
    };

    match fs::read(&candidate).await {
        Ok(bytes) => {
            let content_type = mime_guess::from_path(&candidate).first_or_octet_stream();
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type.as_ref())
                .body(Body::from(bytes));

            match response {
                Ok(resp) => resp,
                Err(error) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to build static response: {error}"),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Static file error: {error}"),
        )
            .into_response(),
    }
}

async fn is_safe_file(base: &Path, candidate: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(candidate).await
        && metadata.is_file()
        && let Ok(resolved) = canonicalize(candidate).await
    {
        return resolved.starts_with(base);
    }
    false
}
