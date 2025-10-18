use crate::tasks::{AnalystTask, CriticTask, FinalizeTask, ManualReviewTask, ResearchTask};
use anyhow::{anyhow, Result};
use graph_flow::{
    ExecutionStatus, FlowRunner, GraphBuilder, InMemorySessionStorage, Session, SessionStorage,
    Task,
};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "postgres-session")]
use graph_flow::storage_postgres::PostgresSessionStorage;

/// Bundle of the default tasks used in the DeepResearch workflow.
#[derive(Clone)]
pub struct BaseGraphTasks {
    pub research: Arc<ResearchTask>,
    pub analyst: Arc<AnalystTask>,
    pub critic: Arc<CriticTask>,
    pub finalize: Arc<FinalizeTask>,
    pub manual_review: Arc<ManualReviewTask>,
}

impl BaseGraphTasks {
    fn new() -> Self {
        Self {
            research: Arc::new(ResearchTask),
            analyst: Arc::new(AnalystTask),
            critic: Arc::new(CriticTask),
            finalize: Arc::new(FinalizeTask),
            manual_review: Arc::new(ManualReviewTask),
        }
    }
}

/// Hook for callers to mutate the graph before default wiring occurs.
pub type GraphCustomizer = dyn Fn(GraphBuilder, &BaseGraphTasks) -> GraphBuilder + Send + Sync;

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

fn build_graph(customizer: Option<&GraphCustomizer>) -> (Arc<graph_flow::Graph>, BaseGraphTasks) {
    let tasks = BaseGraphTasks::new();

    let builder = GraphBuilder::new("deepresearch_workflow")
        .add_task(tasks.research.clone())
        .add_task(tasks.analyst.clone())
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
        .add_edge(tasks.analyst.id(), tasks.critic.id())
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

/// Options for running a new research session.
pub struct SessionOptions<'a> {
    pub query: &'a str,
    pub session_id: Option<String>,
    pub customize_graph: Option<Box<GraphCustomizer>>,
    pub initial_context: Vec<(String, Value)>,
    pub storage: StorageChoice,
}

impl<'a> SessionOptions<'a> {
    pub fn new(query: &'a str) -> Self {
        Self {
            query,
            session_id: None,
            customize_graph: None,
            initial_context: Vec::new(),
            storage: StorageChoice::InMemory,
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

fn extract_final_summary(session: &Session) -> String {
    session
        .context
        .get_sync::<String>("final.summary")
        .unwrap_or_else(|| "No final summary recorded".to_string())
}

/// Run the research workflow end-to-end for the provided query using default settings.
pub async fn run_research_session(query: &str) -> Result<String> {
    run_research_session_with_options(SessionOptions::new(query)).await
}

/// Run the research workflow with custom options (session ID, storage, graph customisation, seeded context).
pub async fn run_research_session_with_options(options: SessionOptions<'_>) -> Result<String> {
    let (graph, tasks) = build_graph(options.customize_graph.as_deref());
    let storage = init_storage(&options.storage).await?;
    let runner = FlowRunner::new(graph, storage.clone());

    let session_id = options.session_id.clone().unwrap_or_else(new_session_id);
    let session = Session::new_from_task(session_id.clone(), tasks.research.id());

    session
        .context
        .set("query", options.query.to_string())
        .await;
    for (key, value) in options.initial_context.iter() {
        session.context.set(key, value.clone()).await;
    }

    storage
        .save(session)
        .await
        .map_err(|err| anyhow!("failed to persist session: {err}"))?;

    execute_until_complete(&runner, &session_id).await?;

    let session = load_session(&storage, &session_id).await?;
    Ok(extract_final_summary(&session))
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
}

impl ResumeOptions {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            customize_graph: None,
            storage: StorageChoice::InMemory,
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
}

/// Resume a previously started session and return the latest summary.
pub async fn resume_research_session(options: ResumeOptions) -> Result<String> {
    let (graph, _tasks) = build_graph(options.customize_graph.as_deref());
    let storage = init_storage(&options.storage).await?;
    let runner = FlowRunner::new(graph, storage.clone());

    // Ensure session exists before attempting to resume
    load_session(&storage, &options.session_id).await?;

    execute_until_complete(&runner, &options.session_id).await?;

    let session = load_session(&storage, &options.session_id).await?;
    Ok(extract_final_summary(&session))
}
