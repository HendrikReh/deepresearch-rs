use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{FromRequestParts, Path},
    http::{StatusCode, header, request::Parts},
    response::sse::{KeepAlive, Sse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::AppError;
use crate::state::{
    AppState, SessionMetrics, SessionRequest, SessionState, SessionStatus, SseStream,
};

#[derive(Debug, Deserialize)]
pub struct StartSessionRequest {
    pub query: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub enable_trace: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct StartSessionResponse {
    pub session_id: String,
    pub state: SessionState,
    pub capacity: CapacitySnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TraceResponse {
    pub session_id: String,
    pub summary: String,
    pub trace_events: Vec<deepresearch_core::TraceEvent>,
    pub trace_summary: deepresearch_core::TraceSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain_markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain_mermaid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain_graphviz: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CapacitySnapshot {
    pub max_concurrency: usize,
    pub available_permits: usize,
    pub running_sessions: usize,
    pub total_sessions: usize,
}

impl From<SessionMetrics> for CapacitySnapshot {
    fn from(value: SessionMetrics) -> Self {
        Self {
            max_concurrency: value.max_concurrency,
            available_permits: value.available_permits,
            running_sessions: value.running_sessions,
            total_sessions: value.total_sessions,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionStatus>,
    pub capacity: CapacitySnapshot,
}

pub fn session_router() -> Router<AppState> {
    Router::new()
        .route("/sessions", post(start_session).get(list_sessions))
        .route("/sessions/:id", get(get_session))
        .route("/sessions/:id/trace", get(get_session_trace))
        .route("/sessions/:id/stream", get(stream_session))
}

#[instrument(skip_all, fields(session_id = %payload.session_id.as_deref().unwrap_or("new")))]
async fn start_session(
    GuardedState(state): GuardedState,
    Json(payload): Json<StartSessionRequest>,
) -> Result<(StatusCode, Json<StartSessionResponse>), AppError> {
    if payload.query.trim().is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "query must not be empty",
        ));
    }

    let request = SessionRequest::new(payload.query)
        .with_session_id(payload.session_id)
        .with_trace(payload.enable_trace);

    let service = state.session_service();
    let session_id = service
        .start_session(request)
        .await
        .map_err(AppError::from)?;

    let state_snapshot = service.status(&session_id).unwrap_or(SessionStatus {
        session_id: session_id.clone(),
        state: SessionState::Running,
        summary: None,
        error: None,
        trace_available: false,
    });

    let response = StartSessionResponse {
        session_id,
        state: state_snapshot.state,
        capacity: service.metrics().into(),
        message: Some("session started".into()),
    };

    Ok((StatusCode::ACCEPTED, Json(response)))
}

async fn get_session(
    GuardedState(state): GuardedState,
    Path(session_id): Path<String>,
) -> Result<Json<SessionStatus>, AppError> {
    match state.session_service().status(&session_id) {
        Some(status) => Ok(Json(status)),
        None => Err(AppError::new(StatusCode::NOT_FOUND, "session not found")),
    }
}

async fn get_session_trace(
    GuardedState(state): GuardedState,
    Path(session_id): Path<String>,
) -> Result<Json<TraceResponse>, AppError> {
    if let Some(outcome) = state.session_service().outcome(&session_id) {
        let response = TraceResponse {
            session_id: outcome.session_id.clone(),
            summary: outcome.summary.clone(),
            trace_events: outcome.trace_events.clone(),
            trace_summary: outcome.trace_summary.clone(),
            trace_path: outcome
                .trace_path
                .as_ref()
                .map(|path| path.display().to_string()),
            explain_markdown: outcome.explain_markdown(),
            explain_mermaid: outcome.explain_mermaid(),
            explain_graphviz: outcome.explain_graphviz(),
        };
        return Ok(Json(response));
    }

    match state.session_service().status(&session_id) {
        Some(status) if matches!(status.state, SessionState::Running) => Err(AppError::new(
            StatusCode::CONFLICT,
            "session is still running",
        )),
        _ => Err(AppError::new(StatusCode::NOT_FOUND, "session not found")),
    }
}

async fn stream_session(
    GuardedState(state): GuardedState,
    Path(session_id): Path<String>,
) -> Result<Sse<SseStream>, AppError> {
    match state.session_service().event_stream(&session_id) {
        Some(stream) => Ok(Sse::new(stream).keep_alive(KeepAlive::new())),
        None => Err(AppError::new(StatusCode::NOT_FOUND, "session not found")),
    }
}

async fn list_sessions(
    GuardedState(state): GuardedState,
) -> Result<Json<ListSessionsResponse>, AppError> {
    let service = state.session_service();
    let sessions = service.list_sessions();
    let capacity = service.metrics().into();
    Ok(Json(ListSessionsResponse { sessions, capacity }))
}

pub struct GuardedState(pub AppState);

#[async_trait]
impl FromRequestParts<AppState> for GuardedState {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let app_state = state.clone();

        if !app_state.gui_enabled() {
            return Err(AppError::new(StatusCode::FORBIDDEN, "GUI disabled"));
        }

        if let Some(expected) = app_state.auth_token() {
            let provided = parts
                .headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::trim);

            match provided {
                Some(token) if token == expected.as_str() => {}
                _ => {
                    return Err(AppError::new(
                        StatusCode::UNAUTHORIZED,
                        "invalid auth token",
                    ));
                }
            }
        }

        Ok(GuardedState(app_state))
    }
}
