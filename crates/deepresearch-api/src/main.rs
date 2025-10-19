use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use deepresearch_core::{
    IngestDocument, IngestOptions, LoadOptions, RetrieverChoice, SessionOptions, SessionOutcome,
    TraceEvent, ingest_documents, load_session_report, run_research_session_with_report,
};
use graph_flow::{InMemorySessionStorage, SessionStorage};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    signal,
    sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError},
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    storage: Arc<dyn SessionStorage>,
    retriever: RetrieverChoice,
    trace_dir: PathBuf,
    session_permits: Arc<Semaphore>,
    max_sessions: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,deepresearch_core=info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    let addr: SocketAddr = std::env::var("DEEPRESEARCH_API_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()
        .expect("invalid DEEPRESEARCH_API_ADDR");

    let storage: Arc<dyn SessionStorage> = Arc::new(InMemorySessionStorage::new());

    let retriever = std::env::var("DEEPRESEARCH_QDRANT_URL")
        .map(|url| {
            let collection = std::env::var("DEEPRESEARCH_QDRANT_COLLECTION")
                .unwrap_or_else(|_| "deepresearch".to_string());
            let concurrency = std::env::var("DEEPRESEARCH_QDRANT_CONCURRENCY")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(8);
            RetrieverChoice::qdrant(url, collection, concurrency)
        })
        .unwrap_or_default();

    let trace_dir = std::env::var("DEEPRESEARCH_TRACE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/traces"));

    let session_limit = std::env::var("DEEPRESEARCH_MAX_CONCURRENT_SESSIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|limit| *limit > 0)
        .unwrap_or(5);
    let session_permits = Arc::new(Semaphore::new(session_limit));

    let state = AppState {
        storage,
        retriever,
        trace_dir,
        session_permits,
        max_sessions: session_limit,
    };

    let app = Router::new()
        .route("/health", get(handle_health))
        .route("/query", post(handle_query))
        .route("/session/:id", get(handle_session))
        .route("/ingest", post(handle_ingest))
        .with_state(state);

    info!("DeepResearch API listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("shutdown signal received, stopping server");
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum ExplainFormat {
    #[default]
    Markdown,
    Mermaid,
    Graphviz,
}

impl ExplainFormat {
    fn render(self, outcome: &SessionOutcome) -> Option<String> {
        match self {
            ExplainFormat::Markdown => outcome.explain_markdown(),
            ExplainFormat::Mermaid => outcome.explain_mermaid(),
            ExplainFormat::Graphviz => outcome.explain_graphviz(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            ExplainFormat::Markdown => "markdown",
            ExplainFormat::Mermaid => "mermaid",
            ExplainFormat::Graphviz => "graphviz",
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        AppError::new(StatusCode::INTERNAL_SERVER_ERROR, error.into().to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse {
            error: self.message,
        });
        (self.status, body).into_response()
    }
}

type ApiResult<T> = std::result::Result<T, AppError>;

fn acquire_session_permit(state: &AppState) -> ApiResult<OwnedSemaphorePermit> {
    match state.session_permits.clone().try_acquire_owned() {
        Ok(permit) => Ok(permit),
        Err(TryAcquireError::NoPermits) => Err(AppError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "session capacity reached; retry once a slot frees up",
        )),
        Err(TryAcquireError::Closed) => Err(AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "session executor unavailable",
        )),
    }
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    query: String,
    session_id: Option<String>,
    explain: Option<bool>,
    explain_format: Option<ExplainFormat>,
    persist_trace: Option<bool>,
    trace_dir: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionPayload {
    session_id: String,
    summary: Option<String>,
    trace_path: Option<String>,
    explanation: Option<String>,
    explanation_format: Option<String>,
    trace_events: Vec<TraceEvent>,
}

#[derive(Debug, Serialize)]
struct CapacityReport {
    max_sessions: usize,
    available_sessions: usize,
    active_sessions: usize,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    capacity: CapacityReport,
    retrieval_mode: &'static str,
}

fn capacity_report(state: &AppState) -> CapacityReport {
    let available = state.session_permits.available_permits();
    let active = state.max_sessions.saturating_sub(available);
    CapacityReport {
        max_sessions: state.max_sessions,
        available_sessions: available,
        active_sessions: active,
    }
}

fn retrieval_mode(retriever: &RetrieverChoice) -> &'static str {
    match retriever {
        RetrieverChoice::Stub => "stub",
        RetrieverChoice::Qdrant { .. } => "qdrant",
    }
}

async fn handle_health(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    let report = capacity_report(&state);
    Ok(Json(HealthResponse {
        status: "ok",
        capacity: report,
        retrieval_mode: retrieval_mode(&state.retriever),
    }))
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    explain: Option<bool>,
    explain_format: Option<ExplainFormat>,
    include_summary: Option<bool>,
    persist_trace: Option<bool>,
    trace_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IngestRequest {
    session_id: String,
    documents: Vec<IngestDocumentPayload>,
}

#[derive(Debug, Deserialize)]
struct IngestDocumentPayload {
    id: Option<String>,
    text: String,
    source: Option<String>,
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    session_id: String,
    documents_indexed: usize,
}

async fn handle_query(
    State(state): State<AppState>,
    Json(request): Json<QueryRequest>,
) -> ApiResult<Json<SessionPayload>> {
    let _permit = acquire_session_permit(&state)?;
    let mut options = SessionOptions::new(&request.query)
        .with_shared_storage(state.storage.clone())
        .with_retriever(state.retriever.clone());

    if let Some(session_id) = request.session_id {
        options = options.with_session_id(session_id);
    }

    let trace_requested = request.explain.unwrap_or(false)
        || request.persist_trace.unwrap_or(false)
        || request.trace_dir.is_some();
    if trace_requested {
        let dir = request
            .trace_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| state.trace_dir.clone());
        options = options.with_trace_output_dir(dir);
    }

    let outcome = run_research_session_with_report(options)
        .await
        .map_err(AppError::from)?;

    let explain_format = request.explain_format.unwrap_or(ExplainFormat::Markdown);
    let (explanation, explanation_format) = if request.explain.unwrap_or(false) {
        match explain_format.render(&outcome) {
            Some(text) => (Some(text), Some(explain_format.label().to_string())),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    let payload = SessionPayload {
        session_id: outcome.session_id.clone(),
        summary: Some(outcome.summary),
        trace_path: outcome
            .trace_path
            .as_ref()
            .map(|path| path.display().to_string()),
        explanation,
        explanation_format,
        trace_events: outcome.trace_events,
    };

    Ok(Json(payload))
}

async fn handle_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<SessionQuery>,
) -> ApiResult<Json<SessionPayload>> {
    let mut options =
        LoadOptions::new(session_id.clone()).with_shared_storage(state.storage.clone());

    if query.persist_trace.unwrap_or(false) || query.trace_dir.is_some() {
        let dir = query
            .trace_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| state.trace_dir.clone());
        options = options.with_trace_output_dir(dir);
    }

    let outcome = match load_session_report(options).await {
        Ok(outcome) => outcome,
        Err(err) => {
            let message = err.to_string();
            if message.contains("not found") {
                return Err(AppError::new(StatusCode::NOT_FOUND, message));
            }
            return Err(AppError::from(err));
        }
    };

    let explain_format = query.explain_format.unwrap_or(ExplainFormat::Markdown);

    let (explanation, explanation_format) = if query.explain.unwrap_or(false) {
        match explain_format.render(&outcome) {
            Some(text) => (Some(text), Some(explain_format.label().to_string())),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    let payload = SessionPayload {
        session_id: outcome.session_id.clone(),
        summary: if query.include_summary.unwrap_or(false) {
            Some(outcome.summary)
        } else {
            None
        },
        trace_path: outcome
            .trace_path
            .as_ref()
            .map(|path| path.display().to_string()),
        explanation,
        explanation_format,
        trace_events: outcome.trace_events,
    };

    Ok(Json(payload))
}

async fn handle_ingest(
    State(state): State<AppState>,
    Json(request): Json<IngestRequest>,
) -> ApiResult<Json<IngestResponse>> {
    if request.documents.is_empty() {
        warn!("ingest requested with no documents");
        return Ok(Json(IngestResponse {
            session_id: request.session_id,
            documents_indexed: 0,
        }));
    }

    let document_count = request.documents.len();
    let session_id = request.session_id.clone();

    let documents = request
        .documents
        .into_iter()
        .map(|doc| IngestDocument {
            id: doc.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            text: doc.text,
            source: doc.source,
        })
        .collect::<Vec<_>>();

    ingest_documents(IngestOptions {
        session_id: session_id.clone(),
        documents,
        retriever: state.retriever.clone(),
    })
    .await
    .map_err(AppError::from)?;

    Ok(Json(IngestResponse {
        session_id,
        documents_indexed: document_count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_limit_returns_429() {
        let state = AppState {
            storage: Arc::new(InMemorySessionStorage::new()),
            retriever: RetrieverChoice::default(),
            trace_dir: PathBuf::from("data/traces"),
            session_permits: Arc::new(Semaphore::new(1)),
            max_sessions: 1,
        };

        let permit = acquire_session_permit(&state).expect("first permit should succeed");
        let err = acquire_session_permit(&state).expect_err("second permit should fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        drop(permit);
    }
}
