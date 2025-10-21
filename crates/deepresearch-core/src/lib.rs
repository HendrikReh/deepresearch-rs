//! DeepResearch core abstractions built directly on top of `graph_flow`.
//!
//! This crate provides reusable tasks and helper utilities to orchestrate a
//! research workflow consisting of Researcher, Analyst, and Critic agents.

mod eval;
mod logging;
mod memory;
mod metrics;
mod pipeline;
mod sandbox;
mod tasks;
mod trace;
mod workflow;

pub use eval::{EvaluationHarness, EvaluationMetrics};
pub use logging::remove_session_logs;
pub use memory::{IngestDocument, RetrievedDocument};
pub use metrics::{init_metrics_from_env, record_sandbox_metrics, shutdown_metrics};
pub use pipeline::persist_session_record;
pub use sandbox::{
    DockerRuntimeUser, DockerSandboxConfig, DockerSandboxRunner, SandboxExecutor, SandboxOutput,
    SandboxOutputKind, SandboxOutputSpec, SandboxRequest, SandboxResult,
};
pub use tasks::{
    AnalystOutput, AnalystTask, CriticTask, FactCheckSettings, FactCheckTask, FinalizeTask,
    ManualReviewTask, MathToolOutput, MathToolRequest, MathToolResult, MathToolStatus,
    MathToolTask, ResearchTask,
};
pub use trace::{TraceCollector, TraceEvent, TraceStep, TraceSummary, persist_trace};
pub use workflow::{
    BaseGraphTasks, DeleteOptions, GraphCustomizer, IngestOptions, LoadOptions, ResumeOptions,
    RetrieverChoice, SessionOptions, SessionOutcome, StorageChoice, delete_session,
    ingest_documents, load_session_report, resume_research_session,
    resume_research_session_with_report, run_research_session, run_research_session_with_options,
    run_research_session_with_report,
};
