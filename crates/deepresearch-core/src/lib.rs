//! DeepResearch core abstractions built directly on top of `graph_flow`.
//!
//! This crate provides reusable tasks and helper utilities to orchestrate a
//! research workflow consisting of Researcher, Analyst, and Critic agents.

mod tasks;
mod workflow;

pub use tasks::{
    AnalystOutput, AnalystTask, CriticTask, FinalizeTask, ManualReviewTask, ResearchTask,
};
pub use workflow::{
    run_research_session, run_research_session_with_options, BaseGraphTasks, GraphCustomizer,
    SessionOptions,
};
