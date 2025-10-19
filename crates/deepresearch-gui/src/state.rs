use crate::config::AppConfig;
use anyhow::Result;
use axum::response::sse::Event;
use dashmap::DashMap;
use deepresearch_core::{SessionOptions, SessionOutcome, run_research_session_with_report};
use graph_flow::{InMemorySessionStorage, SessionStorage};
use serde::Serialize;
use std::convert::Infallible;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Semaphore, broadcast};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{self as stream, Stream, StreamExt};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    session_service: Arc<SessionService>,
    assets_dir: Arc<PathBuf>,
    gui_enabled: bool,
    auth_token: Option<Arc<String>>,
}

impl AppState {
    pub async fn try_new(config: &AppConfig) -> Result<Self> {
        let storage: Arc<dyn SessionStorage> = Arc::new(InMemorySessionStorage::new());
        let service =
            SessionService::new(storage, config.max_concurrency, config.default_enable_trace);

        Ok(Self {
            session_service: Arc::new(service),
            assets_dir: Arc::new(config.assets_dir.clone()),
            gui_enabled: config.gui_enabled,
            auth_token: config
                .auth_token
                .as_ref()
                .map(|token| Arc::new(token.to_string())),
        })
    }

    pub fn session_service(&self) -> Arc<SessionService> {
        self.session_service.clone()
    }

    pub fn assets_dir(&self) -> Arc<PathBuf> {
        self.assets_dir.clone()
    }

    pub fn gui_enabled(&self) -> bool {
        self.gui_enabled
    }

    pub fn auth_token(&self) -> Option<Arc<String>> {
        self.auth_token.clone()
    }
}

#[derive(Clone)]
pub struct SessionService {
    semaphore: Arc<Semaphore>,
    storage: Arc<dyn SessionStorage>,
    default_enable_trace: bool,
    sessions: Arc<DashMap<String, SessionRecord>>,
    streams: Arc<DashMap<String, broadcast::Sender<SessionEvent>>>,
}

impl SessionService {
    pub fn new(
        storage: Arc<dyn SessionStorage>,
        max_concurrency: usize,
        default_enable_trace: bool,
    ) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrency.max(1))),
            storage,
            default_enable_trace,
            sessions: Arc::new(DashMap::new()),
            streams: Arc::new(DashMap::new()),
        }
    }

    pub async fn start_session(&self, mut request: SessionRequest) -> Result<String> {
        let session_id = request
            .session_id
            .take()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let prompt = ensure_context7_prefix(&request.query);
        let enable_trace = request.enable_trace.unwrap_or(self.default_enable_trace);

        let sender = self
            .streams
            .entry(session_id.clone())
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(32);
                tx
            })
            .clone();
        let _ = sender.send(SessionEvent::started());
        self.sessions
            .insert(session_id.clone(), SessionRecord::Running);

        let semaphore = self.semaphore.clone();
        let sessions = self.sessions.clone();
        let streams = self.streams.clone();
        let storage = self.storage.clone();
        let session_id_for_task = session_id.clone();
        let sender_for_task = sender.clone();

        tokio::spawn(async move {
            let permit = match semaphore.acquire_owned().await {
                Ok(permit) => permit,
                Err(err) => {
                    let event = SessionEvent::error(&err);
                    let _ = sender_for_task.send(event.clone());
                    sessions.insert(
                        session_id_for_task.clone(),
                        SessionRecord::Failed {
                            error: err.to_string(),
                            event,
                        },
                    );
                    streams.remove(&session_id_for_task);
                    return;
                }
            };

            let mut options = SessionOptions::new(&prompt)
                .with_session_id(session_id_for_task.clone())
                .with_shared_storage(storage);

            if enable_trace {
                options = options.enable_trace();
            }

            let result = run_research_session_with_report(options).await;
            drop(permit);

            match result {
                Ok(outcome) => {
                    info!(session_id = %session_id_for_task, "session completed");
                    let event = SessionEvent::completed(&outcome);
                    let outcome = Arc::new(outcome);
                    sessions.insert(
                        session_id_for_task.clone(),
                        SessionRecord::Completed {
                            outcome: outcome.clone(),
                            event: event.clone(),
                        },
                    );
                    let _ = sender_for_task.send(event);
                }
                Err(err) => {
                    error!(session_id = %session_id_for_task, error = %err, "session failed");
                    let event = SessionEvent::error(&err);
                    sessions.insert(
                        session_id_for_task.clone(),
                        SessionRecord::Failed {
                            error: err.to_string(),
                            event: event.clone(),
                        },
                    );
                    let _ = sender_for_task.send(event);
                }
            }

            streams.remove(&session_id_for_task);
        });

        Ok(session_id)
    }

    pub fn status(&self, session_id: &str) -> Option<SessionStatus> {
        self.sessions
            .get(session_id)
            .map(|record| match record.value() {
                SessionRecord::Running => SessionStatus {
                    session_id: session_id.to_string(),
                    state: SessionState::Running,
                    summary: None,
                    error: None,
                    trace_available: false,
                },
                SessionRecord::Completed { outcome, .. } => SessionStatus {
                    session_id: session_id.to_string(),
                    state: SessionState::Completed,
                    summary: Some(outcome.summary.clone()),
                    error: None,
                    trace_available: !outcome.trace_events.is_empty(),
                },
                SessionRecord::Failed { error, .. } => SessionStatus {
                    session_id: session_id.to_string(),
                    state: SessionState::Failed,
                    summary: None,
                    error: Some(error.clone()),
                    trace_available: false,
                },
            })
    }

    pub fn outcome(&self, session_id: &str) -> Option<Arc<SessionOutcome>> {
        self.sessions
            .get(session_id)
            .and_then(|record| match record.value() {
                SessionRecord::Completed { outcome, .. } => Some(outcome.clone()),
                _ => None,
            })
    }

    pub fn event_stream(&self, session_id: &str) -> Option<SseStream> {
        if let Some(record) = self.sessions.get(session_id) {
            match record.value() {
                SessionRecord::Completed { event, .. } => {
                    let event = event.clone().into_sse_event();
                    let stream = stream::iter(vec![Result::<Event, Infallible>::Ok(event)]);
                    return Some(Box::pin(stream));
                }
                SessionRecord::Failed { event, .. } => {
                    let event = event.clone().into_sse_event();
                    let stream = stream::iter(vec![Result::<Event, Infallible>::Ok(event)]);
                    return Some(Box::pin(stream));
                }
                SessionRecord::Running => {}
            }
        }

        self.streams.get(session_id).map(|sender| {
            let rx = sender.subscribe();
            let stream = BroadcastStream::new(rx).filter_map(|event| match event {
                Ok(event) => Some(Result::<Event, Infallible>::Ok(event.into_sse_event())),
                Err(err) => {
                    warn!(error = %err, "session event stream closed");
                    None
                }
            });
            Box::pin(stream) as SseStream
        })
    }
}

pub type SseStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

#[derive(Debug)]
pub enum SessionRecord {
    Running,
    Completed {
        outcome: Arc<SessionOutcome>,
        event: SessionEvent,
    },
    Failed {
        error: String,
        event: SessionEvent,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionStatus {
    pub session_id: String,
    pub state: SessionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub trace_available: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionEvent {
    pub kind: SessionEventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_available: Option<bool>,
}

impl SessionEvent {
    pub fn started() -> Self {
        Self {
            kind: SessionEventKind::Started,
            message: Some("session started".into()),
            summary: None,
            trace_available: None,
        }
    }

    pub fn completed(outcome: &SessionOutcome) -> Self {
        Self {
            kind: SessionEventKind::Completed,
            message: Some("session completed".into()),
            summary: Some(outcome.summary.clone()),
            trace_available: Some(!outcome.trace_events.is_empty()),
        }
    }

    pub fn error(error: &impl std::fmt::Display) -> Self {
        Self {
            kind: SessionEventKind::Error,
            message: Some(format!("session failed: {error}")),
            summary: None,
            trace_available: Some(false),
        }
    }

    pub fn into_sse_event(self) -> Event {
        let data = serde_json::to_string(&self).unwrap_or_else(|_| {
            serde_json::json!({
                "kind": SessionEventKind::Error,
                "message": "failed to serialize session event",
            })
            .to_string()
        });

        Event::default().event(self.kind.as_str()).data(data)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventKind {
    Started,
    Completed,
    Error,
}

impl SessionEventKind {
    fn as_str(&self) -> &'static str {
        match self {
            SessionEventKind::Started => "started",
            SessionEventKind::Completed => "completed",
            SessionEventKind::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionRequest {
    pub query: String,
    pub session_id: Option<String>,
    pub enable_trace: Option<bool>,
}

impl SessionRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            session_id: None,
            enable_trace: None,
        }
    }

    pub fn with_session_id(mut self, session_id: Option<String>) -> Self {
        self.session_id = session_id;
        self
    }

    pub fn with_trace(mut self, enable: Option<bool>) -> Self {
        self.enable_trace = enable;
        self
    }
}

fn ensure_context7_prefix(query: &str) -> String {
    const PREFIX: &str = "use context7";
    let trimmed = query.trim_start();

    if trimmed.to_ascii_lowercase().starts_with(PREFIX) {
        query.to_string()
    } else if trimmed.is_empty() {
        PREFIX.to_string()
    } else {
        format!("{PREFIX} {query}")
    }
}
