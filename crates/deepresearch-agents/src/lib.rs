//! Agent implementations for the DeepResearch multi-agent system.
//!
//! This crate provides the core agent roles:
//! - Researcher: information retrieval via web and local search
//! - Analyst: synthesis and report generation
//! - Critic: fact-checking and validation
//!
//! Each agent implements the Agent trait and can be executed within
//! the orchestration framework.

mod agent_context;
mod analyst;
mod critic;
mod researcher;

pub use agent_context::{Agent, AgentContext, AgentMessage, AgentResult, SourceReference};
pub use analyst::AnalystAgent;
pub use critic::CriticAgent;
pub use researcher::ResearcherAgent;

// Re-export async_trait for agent implementations
pub use async_trait::async_trait;
