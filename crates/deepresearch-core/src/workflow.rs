use crate::logging::{log_session_completion, SessionLogInput};
#[cfg(feature = "qdrant-retriever")]
use crate::memory::qdrant::{HybridRetriever, QdrantConfig};
use crate::memory::{DynRetriever, IngestDocument, StubRetriever};
use crate::tasks::{
    AnalystOutput, AnalystTask, CriticTask, FactCheckSettings, FactCheckTask, FinalizeTask,
    ManualReviewTask, ResearchTask,
};
use crate::trace::{persist_trace, TraceCollector, TraceEvent, TraceSummary};
use anyhow::{anyhow, Result};
use graph_flow::{
    ExecutionStatus, FlowRunner, GraphBuilder, InMemorySessionStorage, Session, SessionStorage,
    Task,
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

#[cfg(feature = "postgres-session")]
use graph_flow::storage_postgres::PostgresSessionStorage;

const DEFAULT_TRACE_DIR: &str = "data/traces";

/// Bundle of the default tasks used in the DeepResearch workflow.
#[derive(Clone)]
pub struct BaseGraphTasks {
    pub research: Arc<ResearchTask>,
    pub analyst: Arc<AnalystTask>,
    pub fact_check: Arc<FactCheckTask>,
    pub critic: Arc<CriticTask>,
    pub finalize: Arc<FinalizeTask>,
    pub manual_review: Arc<ManualReviewTask>,
}

impl BaseGraphTasks {
    fn new(retriever: DynRetriever, fact_settings: FactCheckSettings) -> Self {
        Self {
            research: Arc::new(ResearchTask::new(retriever)),
            analyst: Arc::new(AnalystTask),
            fact_check: Arc::new(FactCheckTask::new(fact_settings)),
            critic: Arc::new(CriticTask),
            finalize: Arc::new(FinalizeTask),
            manual_review: Arc::new(ManualReviewTask),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionOutcome {
    pub session_id: String,
    pub summary: String,
    pub trace_events: Vec<TraceEvent>,
    pub trace_summary: TraceSummary,
    pub trace_path: Option<PathBuf>,
}

impl SessionOutcome {
    pub fn explain_markdown(&self) -> Option<String> {
        if self.trace_events.is_empty() {
            None
        } else {
            Some(self.trace_summary.render_markdown())
        }
    }

    pub fn explain_mermaid(&self) -> Option<String> {
        if self.trace_events.is_empty() {
            None
        } else {
            Some(self.trace_summary.render_mermaid())
        }
    }

    pub fn explain_graphviz(&self) -> Option<String> {
        if self.trace_events.is_empty() {
            None
        } else {
            Some(self.trace_summary.render_graphviz())
        }
    }
}

fn build_outcome(
    session: &Session,
    session_id: &str,
    trace_output_dir: Option<&PathBuf>,
) -> Result<SessionOutcome> {
    let summary = extract_final_summary(session);

    let trace_enabled = session
        .context
        .get_sync::<bool>("trace.enabled")
        .unwrap_or(false);

    let collector = session
        .context
        .get_sync::<TraceCollector>("trace.collector")
        .unwrap_or_else(|| {
            let legacy: Vec<TraceEvent> =
                session.context.get_sync("trace.events").unwrap_or_default();
            TraceCollector::from_events(legacy)
        });

    let events = collector.into_events();
    let trace_summary = TraceSummary::from_events(&events);

    let mut trace_path = None;
    if trace_enabled && !events.is_empty() {
        let dir = trace_output_dir
            .cloned()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_TRACE_DIR));
        match persist_trace(&dir, session_id, &events) {
            Ok(path) => trace_path = Some(path),
            Err(err) => warn!(%session_id, error = %err, "failed to persist trace to disk"),
        }
    }

    let trace_path_str = trace_path.as_ref().map(|path| path.display().to_string());
    let query = session.context.get_sync::<String>("query");
    let verdict = session.context.get_sync::<String>("critique.verdict");
    let requires_manual = session
        .context
        .get_sync::<bool>("final.requires_manual")
        .unwrap_or(false);
    let sources = session
        .context
        .get_sync::<AnalystOutput>("analysis.output")
        .unwrap_or_default()
        .sources;

    if let Err(err) = log_session_completion(SessionLogInput {
        session_id: session_id.to_string(),
        query,
        summary: summary.clone(),
        verdict,
        requires_manual,
        sources,
        trace_path: trace_path_str.clone(),
    }) {
        warn!(%session_id, error = %err, "failed to record session log");
    }

    Ok(SessionOutcome {
        session_id: session_id.to_string(),
        summary,
        trace_events: events,
        trace_summary,
        trace_path,
    })
}

/// Hook for callers to mutate the graph before default wiring occurs.
pub type GraphCustomizer = dyn Fn(GraphBuilder, &BaseGraphTasks) -> GraphBuilder + Send + Sync;

#[derive(Clone, Default)]
pub enum RetrieverChoice {
    #[default]
    Stub,
    Qdrant {
        url: String,
        collection: String,
        concurrency_limit: usize,
    },
}

impl RetrieverChoice {
    pub fn qdrant(
        url: impl Into<String>,
        collection: impl Into<String>,
        concurrency_limit: usize,
    ) -> Self {
        Self::Qdrant {
            url: url.into(),
            collection: collection.into(),
            concurrency_limit,
        }
    }
}

#[derive(Clone, Default)]
pub enum StorageChoice {
    #[default]
    InMemory,
    #[cfg(feature = "postgres-session")]
    Postgres {
        database_url: String,
    },
    Custom {
        storage: Arc<dyn SessionStorage>,
    },
}

impl StorageChoice {
    #[cfg(feature = "postgres-session")]
    pub fn postgres(database_url: impl Into<String>) -> Self {
        StorageChoice::Postgres {
            database_url: database_url.into(),
        }
    }
}

fn build_graph(
    customizer: Option<&GraphCustomizer>,
    retriever: DynRetriever,
    fact_settings: FactCheckSettings,
) -> (Arc<graph_flow::Graph>, BaseGraphTasks) {
    let tasks = BaseGraphTasks::new(retriever, fact_settings);

    let builder = GraphBuilder::new("deepresearch_workflow")
        .add_task(tasks.research.clone())
        .add_task(tasks.analyst.clone())
        .add_task(tasks.fact_check.clone())
        .add_task(tasks.critic.clone())
        .add_task(tasks.finalize.clone())
        .add_task(tasks.manual_review.clone());

    let builder = if let Some(customize) = customizer {
        customize(builder, &tasks)
    } else {
        builder
    };

    let builder = builder
        .add_edge(tasks.research.id(), tasks.analyst.id())
        .add_edge(tasks.analyst.id(), tasks.fact_check.id())
        .add_edge(tasks.fact_check.id(), tasks.critic.id())
        .add_conditional_edge(
            tasks.critic.id(),
            |ctx| ctx.get_sync::<bool>("critique.confident").unwrap_or(false),
            tasks.finalize.id(),
            tasks.manual_review.id(),
        )
        .set_start_task(tasks.research.id());

    let graph = Arc::new(builder.build());

    (graph, tasks)
}

async fn init_storage(choice: &StorageChoice) -> Result<Arc<dyn SessionStorage>> {
    match choice {
        StorageChoice::InMemory => Ok(Arc::new(InMemorySessionStorage::new())),
        #[cfg(feature = "postgres-session")]
        StorageChoice::Postgres { database_url } => {
            let storage = PostgresSessionStorage::connect(database_url)
                .await
                .map_err(|err| anyhow!("failed to connect Postgres session storage: {err}"))?;
            Ok(Arc::new(storage))
        }
        StorageChoice::Custom { storage } => Ok(storage.clone()),
    }
}

fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}

async fn build_retriever(choice: &RetrieverChoice) -> Result<DynRetriever> {
    match choice {
        RetrieverChoice::Stub => Ok(Arc::new(StubRetriever::new())),
        RetrieverChoice::Qdrant {
            url,
            collection,
            concurrency_limit,
        } => {
            #[cfg(feature = "qdrant-retriever")]
            {
                let retriever = HybridRetriever::new(QdrantConfig {
                    url: url.clone(),
                    collection: collection.clone(),
                    concurrency_limit: *concurrency_limit,
                })
                .await?;
                Ok(Arc::new(retriever))
            }
            #[cfg(not(feature = "qdrant-retriever"))]
            {
                let _ = (url, collection, concurrency_limit);
                Err(anyhow!(
                    "qdrant retriever support not enabled; rebuild with `--features deepresearch-core/qdrant-retriever`"
                ))
            }
        }
    }
}

/// Options for running a new research session.
pub struct SessionOptions<'a> {
    pub query: &'a str,
    pub session_id: Option<String>,
    pub customize_graph: Option<Box<GraphCustomizer>>,
    pub initial_context: Vec<(String, Value)>,
    pub storage: StorageChoice,
    pub retriever: RetrieverChoice,
    pub fact_check_settings: FactCheckSettings,
    pub trace_enabled: bool,
    pub trace_output_dir: Option<PathBuf>,
}

impl<'a> SessionOptions<'a> {
    pub fn new(query: &'a str) -> Self {
        Self {
            query,
            session_id: None,
            customize_graph: None,
            initial_context: Vec::new(),
            storage: StorageChoice::InMemory,
            retriever: RetrieverChoice::default(),
            fact_check_settings: FactCheckSettings::default(),
            trace_enabled: false,
            trace_output_dir: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_customizer(mut self, customizer: Box<GraphCustomizer>) -> Self {
        self.customize_graph = Some(customizer);
        self
    }

    pub fn with_initial_context(mut self, key: impl Into<String>, value: Value) -> Self {
        self.initial_context.push((key.into(), value));
        self
    }

    pub fn with_fact_check_settings(mut self, settings: FactCheckSettings) -> Self {
        self.fact_check_settings = settings;
        self
    }

    pub fn with_storage(mut self, storage: StorageChoice) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_shared_storage(mut self, storage: Arc<dyn SessionStorage>) -> Self {
        self.storage = StorageChoice::Custom { storage };
        self
    }

    #[cfg(feature = "postgres-session")]
    pub fn with_postgres_storage(mut self, database_url: impl Into<String>) -> Self {
        self.storage = StorageChoice::postgres(database_url);
        self
    }

    pub fn with_retriever(mut self, retriever: RetrieverChoice) -> Self {
        self.retriever = retriever;
        self
    }

    pub fn with_qdrant_retriever(
        mut self,
        url: impl Into<String>,
        collection: impl Into<String>,
        concurrency_limit: usize,
    ) -> Self {
        self.retriever = RetrieverChoice::qdrant(url, collection, concurrency_limit);
        self
    }

    pub fn enable_trace(mut self) -> Self {
        self.trace_enabled = true;
        self
    }

    pub fn with_trace_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.trace_enabled = true;
        self.trace_output_dir = Some(dir.into());
        self
    }
}

fn extract_final_summary(session: &Session) -> String {
    session
        .context
        .get_sync::<String>("final.summary")
        .unwrap_or_else(|| "No final summary recorded".to_string())
}

/// Run the research workflow end-to-end with a detailed outcome (summary + trace).
pub async fn run_research_session_with_report(
    options: SessionOptions<'_>,
) -> Result<SessionOutcome> {
    let retriever = build_retriever(&options.retriever).await?;
    let (graph, tasks) = build_graph(
        options.customize_graph.as_deref(),
        retriever,
        options.fact_check_settings.clone(),
    );
    let storage = init_storage(&options.storage).await?;
    let runner = FlowRunner::new(graph, storage.clone());

    let session_id = options.session_id.clone().unwrap_or_else(new_session_id);
    let session = Session::new_from_task(session_id.clone(), tasks.research.id());

    session
        .context
        .set("query", options.query.to_string())
        .await;
    session.context.set("session_id", session_id.clone()).await;
    for (key, value) in options.initial_context.iter() {
        session.context.set(key, value.clone()).await;
    }
    if options.trace_enabled {
        session.context.set("trace.enabled", true).await;
        session
            .context
            .set("trace.collector", TraceCollector::new())
            .await;
    }

    storage
        .save(session)
        .await
        .map_err(|err| anyhow!("failed to persist session: {err}"))?;

    execute_until_complete(&runner, &session_id).await?;

    let session = load_session(&storage, &session_id).await?;
    build_outcome(&session, &session_id, options.trace_output_dir.as_ref())
}

/// Run the research workflow end-to-end for the provided query using default settings.
pub async fn run_research_session(query: &str) -> Result<String> {
    run_research_session_with_report(SessionOptions::new(query))
        .await
        .map(|outcome| outcome.summary)
}

/// Run the research workflow with custom options (session ID, storage, graph customisation, seeded context).
pub async fn run_research_session_with_options(options: SessionOptions<'_>) -> Result<String> {
    run_research_session_with_report(options)
        .await
        .map(|outcome| outcome.summary)
}

async fn execute_until_complete(runner: &FlowRunner, session_id: &str) -> Result<()> {
    loop {
        let result = runner
            .run(session_id)
            .await
            .map_err(|err| anyhow!("graph execution failure: {err}"))?;

        match result.status {
            ExecutionStatus::Completed => break,
            ExecutionStatus::WaitingForInput => continue,
            ExecutionStatus::Error(message) => return Err(anyhow!(message)),
        }
    }
    Ok(())
}

async fn load_session(storage: &Arc<dyn SessionStorage>, session_id: &str) -> Result<Session> {
    storage
        .get(session_id)
        .await
        .map_err(|err| anyhow!("failed to load session: {err}"))?
        .ok_or_else(|| anyhow!("session '{session_id}' not found"))
}

/// Options for resuming an existing session.
pub struct ResumeOptions {
    pub session_id: String,
    pub customize_graph: Option<Box<GraphCustomizer>>,
    pub storage: StorageChoice,
    pub retriever: RetrieverChoice,
    pub fact_check_settings: FactCheckSettings,
    pub trace_enabled: bool,
    pub trace_output_dir: Option<PathBuf>,
}

impl ResumeOptions {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            customize_graph: None,
            storage: StorageChoice::InMemory,
            retriever: RetrieverChoice::default(),
            fact_check_settings: FactCheckSettings::default(),
            trace_enabled: false,
            trace_output_dir: None,
        }
    }

    pub fn with_customizer(mut self, customizer: Box<GraphCustomizer>) -> Self {
        self.customize_graph = Some(customizer);
        self
    }

    pub fn with_storage(mut self, storage: StorageChoice) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_shared_storage(mut self, storage: Arc<dyn SessionStorage>) -> Self {
        self.storage = StorageChoice::Custom { storage };
        self
    }

    #[cfg(feature = "postgres-session")]
    pub fn with_postgres_storage(mut self, database_url: impl Into<String>) -> Self {
        self.storage = StorageChoice::postgres(database_url);
        self
    }

    pub fn with_retriever(mut self, retriever: RetrieverChoice) -> Self {
        self.retriever = retriever;
        self
    }

    pub fn with_fact_check_settings(mut self, settings: FactCheckSettings) -> Self {
        self.fact_check_settings = settings;
        self
    }

    pub fn with_qdrant_retriever(
        mut self,
        url: impl Into<String>,
        collection: impl Into<String>,
        concurrency_limit: usize,
    ) -> Self {
        self.retriever = RetrieverChoice::qdrant(url, collection, concurrency_limit);
        self
    }

    pub fn enable_trace(mut self) -> Self {
        self.trace_enabled = true;
        self
    }

    pub fn with_trace_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.trace_enabled = true;
        self.trace_output_dir = Some(dir.into());
        self
    }
}

pub struct LoadOptions {
    pub session_id: String,
    pub storage: StorageChoice,
    pub trace_output_dir: Option<PathBuf>,
}

impl LoadOptions {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            storage: StorageChoice::InMemory,
            trace_output_dir: None,
        }
    }

    pub fn with_storage(mut self, storage: StorageChoice) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_shared_storage(mut self, storage: Arc<dyn SessionStorage>) -> Self {
        self.storage = StorageChoice::Custom { storage };
        self
    }

    #[cfg(feature = "postgres-session")]
    pub fn with_postgres_storage(mut self, database_url: impl Into<String>) -> Self {
        self.storage = StorageChoice::postgres(database_url);
        self
    }

    pub fn with_trace_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.trace_output_dir = Some(dir.into());
        self
    }
}

pub struct DeleteOptions {
    pub session_id: String,
    pub storage: StorageChoice,
}

impl DeleteOptions {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            storage: StorageChoice::InMemory,
        }
    }

    pub fn with_storage(mut self, storage: StorageChoice) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_shared_storage(mut self, storage: Arc<dyn SessionStorage>) -> Self {
        self.storage = StorageChoice::Custom { storage };
        self
    }

    #[cfg(feature = "postgres-session")]
    pub fn with_postgres_storage(mut self, database_url: impl Into<String>) -> Self {
        self.storage = StorageChoice::postgres(database_url);
        self
    }
}

/// Resume a previously started session and return a detailed outcome.
pub async fn resume_research_session_with_report(options: ResumeOptions) -> Result<SessionOutcome> {
    let retriever = build_retriever(&options.retriever).await?;
    let (graph, _tasks) = build_graph(
        options.customize_graph.as_deref(),
        retriever,
        options.fact_check_settings.clone(),
    );
    let storage = init_storage(&options.storage).await?;
    let runner = FlowRunner::new(graph, storage.clone());

    let session = load_session(&storage, &options.session_id).await?;
    if options.trace_enabled {
        session.context.set("trace.enabled", true).await;
        if session
            .context
            .get_sync::<TraceCollector>("trace.collector")
            .is_none()
        {
            let legacy: Vec<TraceEvent> =
                session.context.get_sync("trace.events").unwrap_or_default();
            let collector = if legacy.is_empty() {
                TraceCollector::new()
            } else {
                TraceCollector::from_events(legacy)
            };
            session.context.set("trace.collector", collector).await;
        }
        storage
            .save(session)
            .await
            .map_err(|err| anyhow!("failed to persist session: {err}"))?;
    }

    execute_until_complete(&runner, &options.session_id).await?;

    let session = load_session(&storage, &options.session_id).await?;
    build_outcome(
        &session,
        &options.session_id,
        options.trace_output_dir.as_ref(),
    )
}

/// Resume a previously started session and return the latest summary.
pub async fn resume_research_session(options: ResumeOptions) -> Result<String> {
    resume_research_session_with_report(options)
        .await
        .map(|outcome| outcome.summary)
}

pub async fn load_session_report(options: LoadOptions) -> Result<SessionOutcome> {
    let storage = init_storage(&options.storage).await?;
    let session = load_session(&storage, &options.session_id).await?;
    build_outcome(
        &session,
        &options.session_id,
        options.trace_output_dir.as_ref(),
    )
}

pub async fn delete_session(options: DeleteOptions) -> Result<()> {
    let storage = init_storage(&options.storage).await?;
    let session = storage
        .get(&options.session_id)
        .await
        .map_err(|err| anyhow!("failed to load session '{}': {err}", options.session_id))?;

    if session.is_none() {
        return Err(anyhow!("session '{}' not found", options.session_id));
    }

    storage
        .delete(&options.session_id)
        .await
        .map_err(|err| anyhow!("failed to delete session '{}': {err}", options.session_id))?;
    Ok(())
}

pub struct IngestOptions {
    pub session_id: String,
    pub documents: Vec<IngestDocument>,
    pub retriever: RetrieverChoice,
}

pub async fn ingest_documents(options: IngestOptions) -> Result<()> {
    let retriever = build_retriever(&options.retriever).await?;
    retriever
        .ingest(&options.session_id, options.documents)
        .await?;
    Ok(())
}
