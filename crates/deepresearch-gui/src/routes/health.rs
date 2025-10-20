use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::state::{AppState, SessionMetrics};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    gui_enabled: bool,
    metrics: SessionMetrics,
}

pub fn health_router() -> Router<AppState> {
    Router::new()
        .route("/live", get(live))
        .route("/ready", get(ready))
}

async fn live(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(build_response("ok", state))
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    if !state.gui_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(build_response("disabled", state)),
        );
    }

    let metrics = state.metrics();
    if metrics.available_permits == 0 {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "degraded",
                gui_enabled: true,
                metrics,
            }),
        );
    }

    (StatusCode::OK, Json(build_response("ok", state)))
}

fn build_response(status: &'static str, state: AppState) -> HealthResponse {
    HealthResponse {
        status,
        gui_enabled: state.gui_enabled(),
        metrics: state.metrics(),
    }
}
