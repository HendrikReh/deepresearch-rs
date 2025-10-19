mod health;
mod session;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use health::health_router;
use session::session_router;
use std::path::Path;
use tokio::fs::{self, canonicalize};

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let api_state = state.clone();
    let api = session_router().route_layer(middleware::from_fn(move |req, next| {
        let state = api_state.clone();
        async move { require_auth(req, next, state).await }
    }));

    Router::new()
        .nest("/health", health_router())
        .nest("/api", api)
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

async fn require_auth(
    req: Request<Body>,
    next: Next,
    state: AppState,
) -> Result<Response, StatusCode> {
    if !state.gui_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(expected) = state.auth_token() {
        let provided = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim);

        match provided {
            Some(token) if token == expected.as_str() => {}
            _ => return Err(StatusCode::UNAUTHORIZED),
        }
    }

    Ok(next.run(req).await)
}
