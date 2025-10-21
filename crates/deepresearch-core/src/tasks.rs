use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, instrument, warn};

use crate::memory::{DynRetriever, RetrievedDocument};
use crate::sandbox::{
    SandboxExecutor, SandboxFile, SandboxOutputKind, SandboxOutputSpec, SandboxRequest,
    SandboxResult,
};
use crate::trace::TraceCollector;

#[derive(Debug, Clone)]
pub struct FactCheckSettings {
    pub min_confidence: f32,
    pub verification_count: usize,
    pub timeout_ms: u64,
}

impl Default for FactCheckSettings {
    fn default() -> Self {
        Self {
            min_confidence: 0.6,
            verification_count: 3,
            timeout_ms: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MathToolRequest {
    #[serde(default)]
    pub script_name: Option<String>,
    #[serde(default)]
    pub script: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub files: Vec<SandboxFile>,
    #[serde(default)]
    pub expected_outputs: Vec<SandboxOutputSpec>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathToolOutput {
    pub path: String,
    pub kind: SandboxOutputKind,
    #[serde(default)]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MathToolStatus {
    #[default]
    Skipped,
    Success,
    Timeout,
    Failure,
}

impl std::fmt::Display for MathToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            MathToolStatus::Skipped => "skipped",
            MathToolStatus::Success => "success",
            MathToolStatus::Timeout => "timeout",
            MathToolStatus::Failure => "failure",
        };
        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathToolResult {
    pub status: MathToolStatus,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub outputs: Vec<MathToolOutput>,
}

impl Default for MathToolResult {
    fn default() -> Self {
        Self {
            status: MathToolStatus::Skipped,
            exit_code: None,
            timed_out: false,
            duration_ms: 0,
            stdout: String::new(),
            stderr: String::new(),
            outputs: Vec::new(),
        }
    }
}

impl MathToolResult {
    fn from_sandbox(result: SandboxResult) -> Self {
        let duration_ms = result.duration.as_millis().min(u128::from(u64::MAX)) as u64;
        let outputs = result
            .outputs
            .into_iter()
            .map(|output| MathToolOutput {
                path: output.spec.path,
                kind: output.spec.kind,
                bytes: output.bytes,
            })
            .collect::<Vec<_>>();

        let status = if result.timed_out {
            MathToolStatus::Timeout
        } else if result.exit_code.unwrap_or(-1) == 0 {
            MathToolStatus::Success
        } else {
            MathToolStatus::Failure
        };

        Self {
            status,
            exit_code: result.exit_code,
            timed_out: result.timed_out,
            duration_ms,
            stdout: result.stdout,
            stderr: result.stderr,
            outputs,
        }
    }
}

async fn persist_math_result(
    context: &Context,
    result: &MathToolResult,
    script_name: Option<&str>,
) {
    context.set("math.result", result).await;
    context.set("math.status", result.status.to_string()).await;
    context.set("math.stdout", &result.stdout).await;
    context.set("math.stderr", &result.stderr).await;
    context.set("math.exit_code", result.exit_code).await;
    context.set("math.timed_out", result.timed_out).await;
    context.set("math.duration_ms", result.duration_ms).await;
    context.set("math.outputs", &result.outputs).await;
    if let Some(name) = script_name {
        context.set("math.script_name", name.to_string()).await;
    }
    let retry_recommended = matches!(
        result.status,
        MathToolStatus::Failure | MathToolStatus::Timeout
    );
    context
        .set("math.retry_recommended", retry_recommended)
        .await;
    let degradation_note = if retry_recommended {
        Some(format!(
            "Math tool {}. Falling back to non-numeric reasoning.",
            result.status
        ))
    } else {
        None
    };
    context
        .set(
            "math.degradation_note",
            degradation_note.unwrap_or_default(),
        )
        .await;
}

async fn record_trace(context: &Context, task_id: &str, message: impl Into<String>) {
    if !context.get::<bool>("trace.enabled").await.unwrap_or(false) {
        return;
    }

    let mut collector: TraceCollector = context.get("trace.collector").await.unwrap_or_default();
    collector.record(task_id, message);
    context.set("trace.collector", &collector).await;
}

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

        record_trace(
            &context,
            self.id(),
            format!(
                "captured {} findings ({} sources)",
                findings.len(),
                sources.len()
            ),
        )
        .await;

        Ok(TaskResult::new(
            Some(format!("Research completed for \"{}\"", query)),
            NextAction::ContinueAndExecute,
        ))
    }
}

pub struct FactCheckTask {
    settings: FactCheckSettings,
}

impl FactCheckTask {
    pub fn new(settings: FactCheckSettings) -> Self {
        Self { settings }
    }
}

#[async_trait]
impl Task for FactCheckTask {
    fn id(&self) -> &str {
        "fact_check"
    }

    #[instrument(name = "task.fact_check", skip(self, context))]
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let analysis: AnalystOutput = context
            .get("analysis.output")
            .await
            .unwrap_or_else(AnalystOutput::default);
        let sources = analysis.sources.clone();

        if self.settings.timeout_ms > 0 {
            sleep(Duration::from_millis(self.settings.timeout_ms.min(500))).await;
        }

        let verified_sources: Vec<String> = sources
            .iter()
            .take(self.settings.verification_count)
            .cloned()
            .collect();

        let coverage = if sources.is_empty() {
            0.0
        } else {
            verified_sources.len() as f32 / sources.len() as f32
        };
        let confidence = (0.5 + coverage * 0.5).min(1.0);
        let passed = confidence >= self.settings.min_confidence;

        context.set("factcheck.confidence", confidence).await;
        context
            .set("factcheck.verified_sources", &verified_sources)
            .await;
        context.set("factcheck.passed", passed).await;
        context
            .set(
                "factcheck.notes",
                format!(
                    "verified {} sources (coverage {:.0}%)",
                    verified_sources.len(),
                    coverage * 100.0
                ),
            )
            .await;

        info!(
            confidence,
            passed,
            verified = verified_sources.len(),
            "fact-check task completed"
        );

        record_trace(
            &context,
            self.id(),
            format!(
                "confidence {:.2} ({} verified)",
                confidence,
                verified_sources.len()
            ),
        )
        .await;

        Ok(TaskResult::new(
            Some("Fact-check completed".to_string()),
            NextAction::ContinueAndExecute,
        ))
    }
}

#[derive(Default)]
pub struct AnalystTask;

pub struct MathToolTask {
    runner: Arc<dyn SandboxExecutor>,
}

impl MathToolTask {
    pub fn new(runner: Arc<dyn SandboxExecutor>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl Task for MathToolTask {
    fn id(&self) -> &str {
        "math_tool"
    }

    #[instrument(name = "task.math_tool", skip(self, context))]
    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let mut result = MathToolResult::default();
        let request = context.get::<MathToolRequest>("math.request").await;

        let Some(request) = request else {
            persist_math_result(&context, &result, None).await;
            record_trace(&context, self.id(), "skipped (no request)").await;
            return Ok(TaskResult::new(
                Some("Math tool skipped (no request)".to_string()),
                NextAction::ContinueAndExecute,
            ));
        };

        if request.script.trim().is_empty() {
            persist_math_result(&context, &result, request.script_name.as_deref()).await;
            record_trace(&context, self.id(), "skipped (empty script)").await;
            return Ok(TaskResult::new(
                Some("Math tool skipped (empty script)".to_string()),
                NextAction::ContinueAndExecute,
            ));
        }

        let script_name = request
            .script_name
            .clone()
            .unwrap_or_else(|| "math_tool.py".to_string());

        let mut sandbox_request = SandboxRequest::new(script_name.clone(), request.script.clone());
        sandbox_request.args = request.args.clone();
        sandbox_request.files = request.files.clone();
        sandbox_request.expected_outputs = request.expected_outputs.clone();
        if let Some(timeout_ms) = request.timeout_ms {
            sandbox_request.timeout = Duration::from_millis(timeout_ms);
        }

        result = match self.runner.execute(sandbox_request).await {
            Ok(sandbox_result) => MathToolResult::from_sandbox(sandbox_result),
            Err(err) => {
                warn!(error = %err, "math sandbox execution failed");
                MathToolResult {
                    status: MathToolStatus::Failure,
                    stderr: err.to_string(),
                    ..MathToolResult::default()
                }
            }
        };

        persist_math_result(&context, &result, Some(&script_name)).await;

        let trace_message = format!(
            "{} (outputs {}, exit {:?})",
            result.status,
            result.outputs.len(),
            result.exit_code
        );
        record_trace(&context, self.id(), trace_message).await;

        let message = match result.status {
            MathToolStatus::Success => "Math tool completed successfully",
            MathToolStatus::Timeout => "Math tool timed out",
            MathToolStatus::Failure => "Math tool failed",
            MathToolStatus::Skipped => "Math tool skipped",
        };

        Ok(TaskResult::new(
            Some(message.to_string()),
            NextAction::ContinueAndExecute,
        ))
    }
}

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

        record_trace(
            &context,
            self.id(),
            format!("highlight: {}", structured.highlight),
        )
        .await;

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
        let fact_confidence: f32 = context.get("factcheck.confidence").await.unwrap_or(0.0);
        let fact_passed: bool = context.get("factcheck.passed").await.unwrap_or(true);
        let verified_sources: Vec<String> = context
            .get("factcheck.verified_sources")
            .await
            .unwrap_or_default();

        let passes_confidence =
            fact_passed && analysis.summary.split('.').count() >= 2 && !analysis.sources.is_empty();

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
            fact_confidence = fact_confidence,
            "critic evaluated analysis"
        );

        record_trace(
            &context,
            self.id(),
            format!(
                "verdict: {} (fact {:.2})",
                if passes_confidence {
                    "auto-approved"
                } else {
                    "manual review"
                },
                fact_confidence
            ),
        )
        .await;

        let sources_line = if analysis.sources.is_empty() {
            String::from("(none)")
        } else {
            analysis.sources.join(", ")
        };
        let verified_line = if verified_sources.is_empty() {
            String::from("(none)")
        } else {
            verified_sources.join(", ")
        };

        let response = format!(
            "{}\nSummary: {}\nKey Insight: {}\nSources: {}\nFact-Check Confidence: {:.2}\nVerified Sources: {}",
            context
                .get_sync::<String>("critique.verdict")
                .unwrap_or_default(),
            analysis.summary,
            analysis.highlight,
            sources_line,
            fact_confidence,
            verified_line
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
        let fact_confidence = context
            .get::<f32>("factcheck.confidence")
            .await
            .unwrap_or(0.0);
        let verified_sources: Vec<String> = context
            .get("factcheck.verified_sources")
            .await
            .unwrap_or_default();

        let sources_block = if analysis.sources.is_empty() {
            "  (none recorded)".to_string()
        } else {
            analysis
                .sources
                .iter()
                .enumerate()
                .map(|(idx, src)| format!("  {}. {}", idx + 1, src))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let verified_block = if verified_sources.is_empty() {
            "  (none verified)".to_string()
        } else {
            verified_sources
                .iter()
                .enumerate()
                .map(|(idx, src)| format!("  {}. {}", idx + 1, src))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let summary = format!(
            "{verdict}\n\nSummary:\n{}\n\nKey Insight: {}\nConfidence: {}\nSources:\n{}\n\nFact-Check Confidence: {:.2}\nVerified Sources:\n{}",
            analysis.summary,
            analysis.highlight,
            if confident {
                "High"
            } else {
                "Review suggested"
            },
            sources_block,
            fact_confidence,
            verified_block,
        );

        context.set("final.summary", summary.clone()).await;
        context.set("final.requires_manual", false).await;

        info!(confident, "finalize task completed");

        record_trace(&context, self.id(), "final summary emitted").await;

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

        record_trace(&context, self.id(), "manual review requested").await;

        Ok(TaskResult::new(Some(summary), NextAction::End))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalystOutput {
    pub summary: String,
    pub highlight: String,
    pub sources: Vec<String>,
}
