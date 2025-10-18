use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

/// Utilities shared across tasks.
fn default_sources() -> Vec<String> {
    vec![
        "https://example.com/industry-overview".to_string(),
        "https://example.com/market-trends".to_string(),
    ]
}

#[derive(Default)]
pub struct ResearchTask;

#[async_trait]
impl Task for ResearchTask {
    fn id(&self) -> &str {
        "researcher"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let query: String = context
            .get("query")
            .await
            .unwrap_or_else(|| "general market outlook".to_string());

        // Simulate retrieval latency
        sleep(Duration::from_millis(150)).await;

        let findings = vec![
            format!("Identified three primary drivers impacting {}", query),
            "Global demand continues to outpace supply in Q4 forecasts".to_string(),
            "Capital expenditure is shifting toward sustainable extraction methods".to_string(),
        ];

        context.set("research.findings", &findings).await;
        context.set("research.sources", default_sources()).await;

        Ok(TaskResult::new(
            Some(format!("Research completed for \"{}\"", query)),
            NextAction::ContinueAndExecute,
        ))
    }
}

#[derive(Default)]
pub struct AnalystTask;

#[async_trait]
impl Task for AnalystTask {
    fn id(&self) -> &str {
        "analyst"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let findings: Vec<String> = context.get("research.findings").await.unwrap_or_default();
        let sources: Vec<String> = context
            .get("research.sources")
            .await
            .unwrap_or_else(default_sources);

        let summary = if findings.is_empty() {
            "No findings available; analyst requires additional research input".to_string()
        } else {
            format!(
                "Top insights: {}. Confidence supported by {} sources.",
                findings.join("; "),
                sources.len()
            )
        };

        let structured = AnalystOutput {
            summary: summary.clone(),
            highlight: findings.first().cloned().unwrap_or_default(),
            sources,
        };

        context.set("analysis.output", &structured).await;

        Ok(TaskResult::new(
            Some("Analyst prepared synthesis".to_string()),
            NextAction::ContinueAndExecute,
        ))
    }
}

#[derive(Default)]
pub struct CriticTask;

#[async_trait]
impl Task for CriticTask {
    fn id(&self) -> &str {
        "critic"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let analysis: AnalystOutput = context
            .get("analysis.output")
            .await
            .unwrap_or_else(AnalystOutput::default);

        let passes_confidence =
            analysis.summary.split('.').count() >= 2 && !analysis.sources.is_empty();

        context.set("critique.confident", passes_confidence).await;
        context
            .set(
                "critique.verdict",
                if passes_confidence {
                    "Analysis passes automated checks"
                } else {
                    "Insufficient evidence; requires manual review"
                },
            )
            .await;

        let response = format!(
            "{}\nSummary: {}\nKey Insight: {}\nSources: {}",
            context
                .get::<String>("critique.verdict")
                .await
                .unwrap_or_default(),
            analysis.summary,
            analysis.highlight,
            analysis.sources.join(", ")
        );

        Ok(TaskResult::new(Some(response), NextAction::End))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalystOutput {
    pub summary: String,
    pub highlight: String,
    pub sources: Vec<String>,
}
