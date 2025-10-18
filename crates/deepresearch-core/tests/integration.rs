use async_trait::async_trait;
use deepresearch_core::{
    resume_research_session, run_research_session, run_research_session_with_options,
    BaseGraphTasks, ResumeOptions, SessionOptions,
};
use graph_flow::{Context, GraphBuilder, InMemorySessionStorage, NextAction, Task, TaskResult};
use std::sync::Arc;
use uuid::Uuid;

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

#[tokio::test]
async fn resume_session_returns_summary() {
    let session_id = Uuid::new_v4().to_string();
    let shared_storage = Arc::new(InMemorySessionStorage::new());

    let options = SessionOptions::new("Assess lithium battery market drivers 2024")
        .with_session_id(session_id.clone())
        .with_shared_storage(shared_storage.clone());

    let summary = run_research_session_with_options(options)
        .await
        .expect("initial run succeeds");

    assert!(summary.contains("Analysis passes"));

    let resume_summary =
        resume_research_session(ResumeOptions::new(session_id).with_shared_storage(shared_storage))
            .await
            .expect("resume should succeed");

    assert!(resume_summary.contains("Analysis passes"));
}
