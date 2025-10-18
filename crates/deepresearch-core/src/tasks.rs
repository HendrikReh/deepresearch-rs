use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tracing::{debug, info, instrument, warn};

use crate::memory::{DynRetriever, RetrievedDocument};

/// Utilities shared across tasks.
fn default_sources() -> Vec<String> {
    vec![
        "https://example.com/industry-overview".to_string(),
        "https://example.com/market-trends".to_string(),
    ]
}

pub struct ResearchTask {
    retriever: DynRetriever,
}

impl ResearchTask {
    pub fn new(retriever: DynRetriever) -> Self {
        Self { retriever }
    }

    async fn run_retrieval(&self, session_id: &str, query: &str) -> Vec<RetrievedDocument> {
        match self.retriever.retrieve(session_id, query, 5).await {
            Ok(results) => {
                if results
                    .iter()
                    .all(|doc| doc.score <= 0.0 || doc.text.trim().is_empty())
                {
                    vec![RetrievedDocument {
                        text:
                            "Automated placeholder insight. Additional manual review recommended."
                                .to_string(),
                        score: 0.1,
                        source: Some("stub://memory".to_string()),
                    }]
                } else {
                    results
                }
            }
            Err(err) => {
                warn!(%session_id, %query, error = %err, "retriever failed; using placeholder");
                vec![RetrievedDocument {
                    text: format!("Unable to query memory for '{query}'"),
                    score: 0.0,
                    source: Some("stub://error".to_string()),
                }]
            }
        }
    }
}

#[async_trait]
impl Task for ResearchTask {
    fn id(&self) -> &str {
        "researcher"
    }

    #[instrument(name = "task.research", skip(self, context))]
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let query: String = context
            .get("query")
            .await
            .unwrap_or_else(|| "general market outlook".to_string());
        let session_id: String = context
            .get("session_id")
            .await
            .unwrap_or_else(|| "default-session".to_string());

        info!(%query, %session_id, "researcher collecting findings");

        // Simulate latency when external systems are slow
        sleep(Duration::from_millis(150)).await;

        let documents = self.run_retrieval(&session_id, &query).await;

        let findings: Vec<String> = documents.iter().map(|doc| doc.text.clone()).collect();
        let sources: Vec<String> = documents
            .iter()
            .filter_map(|doc| doc.source.clone())
            .collect();

        context.set("research.findings", &findings).await;
        context.set("research.sources", &sources).await;

        debug!(
            findings_count = findings.len(),
            sources_count = sources.len(),
            "research task populated context"
        );

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

    #[instrument(name = "task.analyst", skip(self, context))]
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let findings: Vec<String> = context.get("research.findings").await.unwrap_or_default();
        let sources: Vec<String> = context
            .get("research.sources")
            .await
            .unwrap_or_else(default_sources);

        debug!(
            findings_count = findings.len(),
            sources_count = sources.len(),
            "analyst synthesizing results"
        );

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

        info!(
            summary = %structured.summary,
            key_insight = %structured.highlight,
            "analyst produced structured summary"
        );

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

    #[instrument(name = "task.critic", skip(self, context))]
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let analysis: AnalystOutput = context
            .get("analysis.output")
            .await
            .unwrap_or_else(AnalystOutput::default);

        let passes_confidence =
            analysis.summary.split('.').count() >= 2 && !analysis.sources.is_empty();

        context.set_sync("critique.confident", passes_confidence);
        let verdict = if passes_confidence {
            "Analysis passes automated checks"
        } else {
            "Insufficient evidence; requires manual review"
        };
        context.set_sync("critique.verdict", verdict);

        info!(
            confident = passes_confidence,
            sources = analysis.sources.len(),
            "critic evaluated analysis"
        );

        let response = format!(
            "{}\nSummary: {}\nKey Insight: {}\nSources: {}",
            context
                .get_sync::<String>("critique.verdict")
                .unwrap_or_default(),
            analysis.summary,
            analysis.highlight,
            analysis.sources.join(", ")
        );

        Ok(TaskResult::new(
            Some(response),
            NextAction::ContinueAndExecute,
        ))
    }
}

#[derive(Default)]
pub struct FinalizeTask;

#[async_trait]
impl Task for FinalizeTask {
    fn id(&self) -> &str {
        "finalize"
    }

    #[instrument(name = "task.finalize", skip(self, context))]
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let analysis: AnalystOutput = context
            .get("analysis.output")
            .await
            .unwrap_or_else(AnalystOutput::default);

        let verdict = context
            .get::<String>("critique.verdict")
            .await
            .unwrap_or_else(|| "No verdict recorded".to_string());

        let confident = context
            .get::<bool>("critique.confident")
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
                "  (none recorded)".to_string()
            } else {
                analysis
                    .sources
                    .iter()
                    .enumerate()
                    .map(|(idx, src)| format!("  {}. {}", idx + 1, src))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
        );

        context.set("final.summary", summary.clone()).await;
        context.set("final.requires_manual", false).await;

        info!(confident, "finalize task completed");

        Ok(TaskResult::new(Some(summary), NextAction::End))
    }
}

#[derive(Default)]
pub struct ManualReviewTask;

#[async_trait]
impl Task for ManualReviewTask {
    fn id(&self) -> &str {
        "manual_review"
    }

    #[instrument(name = "task.manual_review", skip(self, context))]
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let summary = String::from(
            "Automated checks flagged low confidence. Please perform manual verification.",
        );

        context.set("final.summary", summary.clone()).await;
        context.set("final.requires_manual", true).await;

        info!("manual review required");

        Ok(TaskResult::new(Some(summary), NextAction::End))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalystOutput {
    pub summary: String,
    pub highlight: String,
    pub sources: Vec<String>,
}
