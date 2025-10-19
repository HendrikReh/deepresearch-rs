//! DeepResearch core abstractions built directly on top of `graph_flow`.
//!
//! This crate provides reusable tasks and helper utilities to orchestrate a
//! research workflow consisting of Researcher, Analyst, and Critic agents.

mod eval;
mod logging;
mod memory;
mod tasks;
mod trace;
mod workflow;

pub use eval::{EvaluationHarness, EvaluationMetrics};
pub use logging::remove_session_logs;
pub use memory::{IngestDocument, RetrievedDocument};
pub use tasks::{
    AnalystOutput, AnalystTask, CriticTask, FactCheckSettings, FactCheckTask, FinalizeTask,
    ManualReviewTask, ResearchTask,
};
pub use trace::{TraceCollector, TraceEvent, TraceStep, TraceSummary, persist_trace};
pub use workflow::{
    BaseGraphTasks, DeleteOptions, GraphCustomizer, IngestOptions, LoadOptions, ResumeOptions,
    RetrieverChoice, SessionOptions, SessionOutcome, StorageChoice, delete_session,
    ingest_documents, load_session_report, resume_research_session,
    resume_research_session_with_report, run_research_session, run_research_session_with_options,
    run_research_session_with_report,
};
