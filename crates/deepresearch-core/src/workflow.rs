use crate::tasks::{AnalystTask, CriticTask, FinalizeTask, ManualReviewTask, ResearchTask};
use anyhow::{anyhow, Result};
use graph_flow::{
    ExecutionStatus, FlowRunner, GraphBuilder, InMemorySessionStorage, Session, SessionStorage,
    Task,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Exposes the core tasks used in the default workflow so callers can extend the graph.
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

/// Customisation hook for callers to add tasks/edges before the default wiring is applied.
pub type GraphCustomizer = dyn Fn(GraphBuilder, &BaseGraphTasks) -> GraphBuilder + Send + Sync;

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

fn new_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("session-{}", nanos)
}

/// Options for running a research session.
pub struct SessionOptions<'a> {
    pub query: &'a str,
    pub session_id: Option<String>,
    pub customize_graph: Option<Box<GraphCustomizer>>, // Additional wiring
    pub initial_context: Vec<(String, Value)>,         // Pre-seeded context values
}

impl<'a> SessionOptions<'a> {
    pub fn new(query: &'a str) -> Self {
        Self {
            query,
            session_id: None,
            customize_graph: None,
            initial_context: Vec::new(),
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
}

/// Run the research workflow end-to-end for the provided query using default settings.
pub async fn run_research_session(query: &str) -> Result<String> {
    run_research_session_with_options(SessionOptions::new(query)).await
}

/// Run the research workflow with custom options (session ID, graph customisation, seeded context).
pub async fn run_research_session_with_options(options: SessionOptions<'_>) -> Result<String> {
    let (graph, tasks) = build_graph(options.customize_graph.as_deref());

    let storage = Arc::new(InMemorySessionStorage::new());
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

    loop {
        let result = runner
            .run(&session_id)
            .await
            .map_err(|err| anyhow!("graph execution failure: {err}"))?;

        match result.status {
            ExecutionStatus::Completed => break,
            ExecutionStatus::WaitingForInput => continue,
            ExecutionStatus::Error(message) => return Err(anyhow!(message)),
        }
    }

    let session = storage
        .get(&session_id)
        .await
        .map_err(|err| anyhow!("failed to reload session: {err}"))?
        .ok_or_else(|| anyhow!("session missing after execution"))?;

    let final_summary: String = session
        .context
        .get("final.summary")
        .await
        .unwrap_or_else(|| "No final summary recorded".to_string());

    Ok(final_summary)
}
