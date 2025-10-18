//! Core primitives for the DeepResearch agent system.
//!
//! Milestone 0 focuses on foundational infrastructure:
//! - configuration loading with guardrails
//! - shared error taxonomy
//! - tracing / telemetry bootstrap
//! - basic security helpers (environment-backed secrets)
//!
//! Milestone 1 adds orchestration core:
//! - task planning and DAG construction
//! - event bus for explainability
//! - Graph-flow executor with retry and concurrency control

mod config;
mod error;
mod events;
mod graph_executor;
// Legacy module - use graph_executor instead
mod orchestrator;
mod planner;
mod security;
mod telemetry;

pub use config::{
    Config, ConfigLoader, FactcheckConfig, LlmConfig, LoggingConfig, PlannerConfig, QdrantConfig,
};
pub use error::{DeepResearchError, TaskError};
pub use events::{Event, EventCollector, TaskOutcome, TraceCollector};
pub use graph_executor::{ExecutionReport, GraphExecutorConfig, GraphFlowExecutor, TaskResult};
// Legacy exports - use GraphFlowExecutor instead
#[deprecated(since = "0.1.0", note = "Use GraphFlowExecutor instead")]
pub use orchestrator::{OrchestratorConfig, RigOrchestrator};
pub use planner::{AgentRole, PlannerAgent, TaskGraph, TaskId, TaskNode};
pub use security::{require_env, SecretValue};
pub use telemetry::{init_telemetry, TelemetryOptions};
