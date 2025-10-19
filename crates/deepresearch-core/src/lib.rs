//! DeepResearch core abstractions built directly on top of `graph_flow`.
//!
//! This crate provides reusable tasks and helper utilities to orchestrate a
//! research workflow consisting of Researcher, Analyst, and Critic agents.

mod eval;
mod memory;
mod tasks;
mod trace;
mod workflow;

pub use eval::{EvaluationHarness, EvaluationMetrics};
pub use memory::{IngestDocument, RetrievedDocument};
pub use tasks::{
    AnalystOutput, AnalystTask, CriticTask, FactCheckSettings, FactCheckTask, FinalizeTask,
    ManualReviewTask, ResearchTask,
};
pub use trace::{persist_trace, TraceCollector, TraceEvent, TraceStep, TraceSummary};
pub use workflow::{
    delete_session, ingest_documents, load_session_report, resume_research_session,
    resume_research_session_with_report, run_research_session, run_research_session_with_options,
    run_research_session_with_report, BaseGraphTasks, DeleteOptions, GraphCustomizer,
    IngestOptions, LoadOptions, ResumeOptions, RetrieverChoice, SessionOptions, SessionOutcome,
    StorageChoice,
};
