//! DeepResearch core abstractions built directly on top of `graph_flow`.
//!
//! This crate provides reusable tasks and helper utilities to orchestrate a
//! research workflow consisting of Researcher, Analyst, and Critic agents.

mod memory;
mod tasks;
mod workflow;

pub use memory::{IngestDocument, RetrievedDocument};
pub use tasks::{
    AnalystOutput, AnalystTask, CriticTask, FinalizeTask, ManualReviewTask, ResearchTask,
};
pub use workflow::{
    ingest_documents, resume_research_session, run_research_session,
    run_research_session_with_options, BaseGraphTasks, GraphCustomizer, IngestOptions,
    ResumeOptions, RetrieverChoice, SessionOptions, StorageChoice,
};
