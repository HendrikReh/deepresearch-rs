//! Core primitives for the DeepResearch agent system.
//!
//! Milestone 0 focuses on foundational infrastructure:
//! - configuration loading with guardrails
//! - shared error taxonomy
//! - tracing / telemetry bootstrap
//! - basic security helpers (environment-backed secrets)

mod config;
mod error;
mod security;
mod telemetry;

pub use config::{
    Config, ConfigLoader, FactcheckConfig, LlmConfig, LoggingConfig, PlannerConfig, QdrantConfig,
};
pub use error::{DeepResearchError, TaskError};
pub use security::{require_env, SecretValue};
pub use telemetry::{init_telemetry, TelemetryOptions};
