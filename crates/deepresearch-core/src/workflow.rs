use crate::tasks::{AnalystOutput, AnalystTask, CriticTask, ResearchTask};
use anyhow::{anyhow, Result};
use graph_flow::{
    ExecutionStatus, FlowRunner, GraphBuilder, InMemorySessionStorage, Session, SessionStorage,
    Task,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Build the default DeepResearch workflow graph using graph_flow tasks.
fn build_graph() -> (
    Arc<graph_flow::Graph>,
    Arc<ResearchTask>,
    Arc<AnalystTask>,
    Arc<CriticTask>,
) {
    let research = Arc::new(ResearchTask::default());
    let analyst = Arc::new(AnalystTask::default());
    let critic = Arc::new(CriticTask::default());

    let graph = Arc::new(
        GraphBuilder::new("deepresearch_workflow")
            .add_task(research.clone())
            .add_task(analyst.clone())
            .add_task(critic.clone())
            .add_edge(research.id(), analyst.id())
            .add_edge(analyst.id(), critic.id())
            .set_start_task(research.id())
            .build(),
    );

    (graph, research, analyst, critic)
}

fn new_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("session-{}", nanos)
}

/// Run the research workflow end-to-end for the provided query.
pub async fn run_research_session(query: &str) -> Result<String> {
    let (graph, research_task, _analyst_task, _critic_task) = build_graph();

    let storage = Arc::new(InMemorySessionStorage::new());
    let runner = FlowRunner::new(graph.clone(), storage.clone());

    let session_id = new_session_id();
    let session = Session::new_from_task(session_id.clone(), research_task.id());
    session.context.set("query", query.to_string()).await;
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

    let analysis: AnalystOutput = session
        .context
        .get("analysis.output")
        .await
        .unwrap_or_default();

    let verdict: String = session
        .context
        .get("critique.verdict")
        .await
        .unwrap_or_else(|| "No verdict recorded".to_string());

    let confident: bool = session
        .context
        .get("critique.confident")
        .await
        .unwrap_or(false);

    let summary = format!(
        "{verdict}\n\nSummary:\n{}\n\nKey Insight: {}\nConfidence: {}\nSources:\n{}",
        analysis.summary,
        analysis.highlight,
        if confident {
            "High"
        } else {
            "Review suggested"
        },
        if analysis.sources.is_empty() {
            "No sources recorded".to_string()
        } else {
            analysis
                .sources
                .into_iter()
                .enumerate()
                .map(|(idx, src)| format!("  {}. {}", idx + 1, src))
                .collect::<Vec<_>>()
                .join("\n")
        }
    );

    Ok(summary)
}
