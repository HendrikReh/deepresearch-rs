use anyhow::Result;
use async_trait::async_trait;
use deepresearch_core::{
    FactCheckSettings, ResumeOptions, SandboxExecutor, SandboxRequest, SandboxResult,
    SessionOptions, resume_research_session, run_research_session,
    run_research_session_with_options,
};
use graph_flow::{InMemorySessionStorage, SessionStorage};
use insta::assert_snapshot;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
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

#[tokio::test]
async fn manual_review_branch_triggers() {
    let options =
        SessionOptions::new("Trigger manual review").with_fact_check_settings(FactCheckSettings {
            min_confidence: 0.95,
            verification_count: 0,
            timeout_ms: 0,
        });

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

#[tokio::test]
async fn finalize_summary_snapshot() {
    let summary = run_research_session("Snapshot regression baseline")
        .await
        .expect("workflow should succeed");

    assert_snapshot!("finalize_summary_default", summary);
}

#[tokio::test]
async fn math_context_keys_are_stable() {
    let session_id = Uuid::new_v4().to_string();
    let storage = Arc::new(InMemorySessionStorage::new());
    let sandbox: Arc<dyn SandboxExecutor> = Arc::new(StubSandbox);

    let options = SessionOptions::new("use context7 verify math context keys")
        .with_session_id(session_id.clone())
        .with_shared_storage(storage.clone())
        .with_sandbox_executor(sandbox)
        .with_initial_context(
            "math.request",
            json!({
                "script_name": "stub_math.py",
                "script": "print('ok')",
                "args": [],
                "files": [],
                "expected_outputs": [],
                "timeout_ms": 1000
            }),
        );

    run_research_session_with_options(options)
        .await
        .expect("workflow should succeed");

    let session = storage
        .get(&session_id)
        .await
        .expect("storage lookup succeeds")
        .expect("session should exist after run");

    let status = session
        .context
        .get_sync::<String>("math.status")
        .expect("math.status key missing");
    assert!(!status.is_empty(), "math.status should be non-empty");

    assert!(
        session
            .context
            .get_sync::<bool>("math.alert_required")
            .is_some(),
        "math.alert_required key missing"
    );
    assert!(
        session
            .context
            .get_sync::<bool>("math.retry_recommended")
            .is_some(),
        "math.retry_recommended key missing"
    );
    assert!(
        session
            .context
            .get_sync::<String>("math.degradation_note")
            .is_some(),
        "math.degradation_note key missing"
    );
    assert!(
        session
            .context
            .get_sync::<bool>("analysis.math_alert_required")
            .is_some(),
        "analysis.math_alert_required key missing"
    );
    assert!(
        session
            .context
            .get_sync::<bool>("analysis.math_retry_recommended")
            .is_some(),
        "analysis.math_retry_recommended key missing"
    );
}

struct StubSandbox;

#[async_trait]
impl SandboxExecutor for StubSandbox {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult> {
        Ok(SandboxResult {
            exit_code: Some(0),
            stdout: format!("stubbed execution for {}", request.script_name),
            stderr: String::new(),
            outputs: Vec::new(),
            timed_out: false,
            duration: Duration::from_millis(12),
        })
    }
}
