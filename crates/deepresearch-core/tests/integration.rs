use async_trait::async_trait;
use deepresearch_core::{
    run_research_session, run_research_session_with_options, BaseGraphTasks, SessionOptions,
};
use graph_flow::{Context, GraphBuilder, NextAction, Task, TaskResult};
use std::sync::Arc;

#[tokio::test]
async fn critic_verdict_is_non_empty() {
    let summary = run_research_session("Assess lithium battery market drivers 2024")
        .await
        .expect("workflow should succeed");

    assert!(
        summary.contains("Sources:"),
        "summary should list sources: {summary}"
    );
    assert!(
        summary.contains("Analysis passes"),
        "expected critic verdict wording: {summary}"
    );
}

struct WipeSourcesTask;

#[async_trait]
impl Task for WipeSourcesTask {
    fn id(&self) -> &str {
        "wipe_sources"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        context.set("research.sources", Vec::<String>::new()).await;
        Ok(TaskResult::new(None, NextAction::ContinueAndExecute))
    }
}

#[tokio::test]
async fn manual_review_branch_triggers() {
    let wipe_task = Arc::new(WipeSourcesTask);

    let customizer = Box::new(move |builder: GraphBuilder, base: &BaseGraphTasks| {
        let task = wipe_task.clone();
        let wipe_id = task.id().to_string();

        builder
            .add_task(task)
            .add_edge(base.analyst.id(), wipe_id.as_str())
            .add_edge(wipe_id.as_str(), base.critic.id())
    });

    let options = SessionOptions::new("Trigger manual review").with_customizer(customizer);

    let summary = run_research_session_with_options(options)
        .await
        .expect("workflow should succeed");

    assert!(
        summary.to_lowercase().contains("manual"),
        "expected manual review message, got: {summary}"
    );
}
