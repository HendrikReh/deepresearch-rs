//! Agent execution context and message contracts.
//!
//! Defines the runtime context for each agent role (Researcher, Analyst, Critic)
//! including LLM integration stubs and inter-agent messaging.

use deepresearch_core::{AgentRole, EventCollector, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context for agent execution
#[derive(Clone)]
pub struct AgentContext {
    pub task_id: TaskId,
    pub role: AgentRole,
    pub parameters: HashMap<String, serde_json::Value>,
    pub event_collector: EventCollector,
}

impl AgentContext {
    pub fn new(
        task_id: TaskId,
        role: AgentRole,
        parameters: HashMap<String, serde_json::Value>,
        event_collector: EventCollector,
    ) -> Self {
        Self {
            task_id,
            role,
            parameters,
            event_collector,
        }
    }

    /// Send a message to another agent or log internal reasoning
    pub fn send_message(
        &self,
        to_task: Option<TaskId>,
        content: String,
        metadata: serde_json::Value,
    ) {
        self.event_collector.emit_message(
            self.task_id.clone(),
            to_task,
            self.role,
            content,
            metadata,
        );
    }

    /// Get a parameter value
    pub fn get_param(&self, key: &str) -> Option<&serde_json::Value> {
        self.parameters.get(key)
    }
}

/// Message passed between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from_task: TaskId,
    pub to_task: TaskId,
    pub role: AgentRole,
    pub content: String,
    pub metadata: serde_json::Value,
}

/// Result of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub task_id: TaskId,
    pub role: AgentRole,
    pub output: serde_json::Value,
    pub sources: Vec<SourceReference>,
    pub confidence: f64,
}

/// Reference to a source used by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    pub id: String,
    pub uri: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
}

/// Trait for agent implementations
#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    /// Execute the agent's task
    async fn execute(&self, context: &AgentContext) -> Result<AgentResult, anyhow::Error>;

    /// Get the agent's role
    fn role(&self) -> AgentRole;
}
