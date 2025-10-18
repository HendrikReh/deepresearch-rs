use deepresearch_core::run_research_session;

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
