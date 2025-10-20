use std::collections::HashMap;

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
    pub timeline: Vec<TimelinePoint>,
    pub task_metrics: Vec<TaskMetric>,
    pub artifacts: TraceArtifacts,
    pub requires_manual: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fact_check: Option<FactCheckSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critic: Option<CriticSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_path: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct TraceArtifacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mermaid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphviz: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FactCheckSnapshot {
    pub confidence: f32,
    pub passed: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verified_sources: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CriticSnapshot {
    pub confident: bool,
}

#[derive(Debug, Serialize)]
pub struct TimelinePoint {
    pub step_index: usize,
    pub task_id: String,
    pub message: String,
    pub timestamp_ms: u128,
    pub offset_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct TaskMetric {
    pub task_id: String,
    pub occurrences: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_duration_ms: Option<u64>,
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
        requires_manual: false,
    });

    let metrics_snapshot = service.metrics();
    crate::metrics::session_started(
        &session_id,
        metrics_snapshot.running_sessions,
        metrics_snapshot.available_permits,
    );

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
        let timeline = build_timeline(&outcome.trace_events);
        let task_metrics = build_task_metrics(&timeline);
        let response = TraceResponse {
            session_id: outcome.session_id.clone(),
            summary: outcome.summary.clone(),
            trace_events: outcome.trace_events.clone(),
            trace_summary: outcome.trace_summary.clone(),
            timeline,
            task_metrics,
            artifacts: TraceArtifacts {
                markdown: outcome.explain_markdown(),
                mermaid: outcome.explain_mermaid(),
                graphviz: outcome.explain_graphviz(),
            },
            requires_manual: outcome.requires_manual,
            fact_check: outcome
                .factcheck_confidence
                .map(|confidence| FactCheckSnapshot {
                    confidence,
                    passed: outcome.factcheck_passed.unwrap_or(false),
                    verified_sources: outcome.factcheck_verified_sources.clone(),
                }),
            critic: outcome
                .critic_confident
                .map(|confident| CriticSnapshot { confident }),
            trace_path: outcome
                .trace_path
                .as_ref()
                .map(|path| path.display().to_string()),
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

fn build_timeline(events: &[deepresearch_core::TraceEvent]) -> Vec<TimelinePoint> {
    if events.is_empty() {
        return Vec::new();
    }

    let first_timestamp = events
        .first()
        .map(|event| event.timestamp_ms)
        .unwrap_or_default();

    events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let next_timestamp = events
                .get(index + 1)
                .map(|next| next.timestamp_ms)
                .filter(|next| *next >= event.timestamp_ms);

            let duration_ms = next_timestamp.map(|next| {
                next.saturating_sub(event.timestamp_ms)
                    .min(u64::MAX as u128) as u64
            });

            TimelinePoint {
                step_index: index + 1,
                task_id: event.task_id.clone(),
                message: event.message.clone(),
                timestamp_ms: event.timestamp_ms,
                offset_ms: event
                    .timestamp_ms
                    .saturating_sub(first_timestamp)
                    .min(u64::MAX as u128) as u64,
                duration_ms,
            }
        })
        .collect()
}

fn build_task_metrics(timeline: &[TimelinePoint]) -> Vec<TaskMetric> {
    let mut order: Vec<String> = Vec::new();
    let mut aggregates: HashMap<String, (usize, u128, usize)> = HashMap::new();

    for point in timeline {
        let entry = aggregates.entry(point.task_id.clone()).or_insert_with(|| {
            order.push(point.task_id.clone());
            (0usize, 0u128, 0usize)
        });
        entry.0 += 1;
        if let Some(duration) = point.duration_ms {
            entry.1 += duration as u128;
            entry.2 += 1;
        }
    }

    order
        .into_iter()
        .filter_map(|task_id| {
            aggregates
                .remove(&task_id)
                .map(|(occurrences, total_duration, duration_samples)| {
                    let total = if duration_samples > 0 {
                        Some(total_duration.min(u64::MAX as u128) as u64)
                    } else {
                        None
                    };
                    let average = if duration_samples > 0 {
                        Some(
                            (total_duration / duration_samples as u128).min(u64::MAX as u128)
                                as u64,
                        )
                    } else {
                        None
                    };

                    TaskMetric {
                        task_id,
                        occurrences,
                        total_duration_ms: total,
                        average_duration_ms: average,
                    }
                })
        })
        .collect()
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
