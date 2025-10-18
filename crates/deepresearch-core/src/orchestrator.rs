//! Rig orchestrator for DAG execution with retry logic and concurrency control.
//!
//! Executes task graphs in topological order with configurable retry policies,
//! backpressure limits, and graceful error handling.

use crate::error::{DeepResearchError, TaskError};
use crate::events::{EventCollector, TaskOutcome};
use crate::planner::{TaskGraph, TaskId, TaskNode};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};

/// Configuration for orchestrator behavior
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Maximum retry attempts for retryable failures
    pub max_retries: usize,
    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 5,
            max_retries: 2,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
        }
    }
}

/// Result of task execution
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub outcome: TaskOutcome,
    pub output: Option<serde_json::Value>,
    pub duration_ms: u64,
}

/// Orchestrator that executes task graphs
pub struct RigOrchestrator {
    config: OrchestratorConfig,
    event_collector: EventCollector,
    semaphore: Arc<Semaphore>,
    results: Arc<RwLock<HashMap<TaskId, TaskResult>>>,
}

impl RigOrchestrator {
    pub fn new(config: OrchestratorConfig, event_collector: EventCollector) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_tasks));

        Self {
            config,
            event_collector,
            semaphore,
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute a task graph
    pub async fn execute(&self, graph: &TaskGraph) -> Result<ExecutionReport, DeepResearchError> {
        tracing::info!(task_count = graph.len(), "Starting graph execution");

        let start_time = Instant::now();
        let order = graph.topological_order()?;

        for task_id in order {
            let node = graph.get_node(&task_id).ok_or_else(|| {
                DeepResearchError::OrchestrationError(format!("Task {} not found", task_id))
            })?;

            self.execute_task(node).await?;
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let results = self.results.read().await;

        let success_count = results
            .values()
            .filter(|r| matches!(r.outcome, TaskOutcome::Success))
            .count();

        tracing::info!(
            duration_ms = duration_ms,
            success_count = success_count,
            total_count = results.len(),
            "Graph execution complete"
        );

        Ok(ExecutionReport {
            total_tasks: graph.len(),
            successful_tasks: success_count,
            failed_tasks: results.len() - success_count,
            duration_ms,
        })
    }

    /// Execute a single task with retry logic
    async fn execute_task(&self, node: &TaskNode) -> Result<TaskResult, DeepResearchError> {
        let mut attempt = 0;
        let mut backoff_ms = self.config.initial_backoff_ms;

        loop {
            // Acquire semaphore permit for concurrency control
            let _permit = self.semaphore.acquire().await.unwrap();

            tracing::debug!(
                task_id = %node.id,
                role = %node.role.as_str(),
                attempt = attempt,
                "Executing task"
            );

            let start_time = Instant::now();
            self.event_collector
                .emit_start(node.id.clone(), node.role, node.description.clone());

            let result = self.run_task(node).await;
            let duration_ms = start_time.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    let task_result = TaskResult {
                        task_id: node.id.clone(),
                        outcome: TaskOutcome::Success,
                        output: Some(output),
                        duration_ms,
                    };

                    self.event_collector.emit_finish(
                        node.id.clone(),
                        node.role,
                        TaskOutcome::Success,
                        duration_ms,
                    );

                    self.results
                        .write()
                        .await
                        .insert(node.id.clone(), task_result.clone());
                    return Ok(task_result);
                }
                Err(e) if attempt < self.config.max_retries && e.is_retryable() => {
                    attempt += 1;
                    tracing::warn!(
                        task_id = %node.id,
                        error = %e,
                        attempt = attempt,
                        backoff_ms = backoff_ms,
                        "Task failed, retrying"
                    );

                    self.event_collector.emit_finish(
                        node.id.clone(),
                        node.role,
                        TaskOutcome::Failure {
                            reason: e.to_string(),
                            retryable: true,
                        },
                        duration_ms,
                    );

                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(self.config.max_backoff_ms);
                    continue;
                }
                Err(e) => {
                    tracing::error!(
                        task_id = %node.id,
                        error = %e,
                        "Task failed permanently"
                    );

                    let task_result = TaskResult {
                        task_id: node.id.clone(),
                        outcome: TaskOutcome::Failure {
                            reason: e.to_string(),
                            retryable: false,
                        },
                        output: None,
                        duration_ms,
                    };

                    self.event_collector.emit_finish(
                        node.id.clone(),
                        node.role,
                        task_result.outcome.clone(),
                        duration_ms,
                    );

                    self.results
                        .write()
                        .await
                        .insert(node.id.clone(), task_result.clone());
                    return Ok(task_result); // Continue graph execution despite failure
                }
            }
        }
    }

    /// Execute task logic (stub for MVP)
    async fn run_task(&self, node: &TaskNode) -> Result<serde_json::Value, TaskError> {
        // TODO: Implement actual agent execution logic
        // For now, simulate task execution
        tracing::debug!(
            task_id = %node.id,
            role = %node.role.as_str(),
            "Running task (stub implementation)"
        );

        self.event_collector.emit_message(
            node.id.clone(),
            None,
            node.role,
            format!("Executing: {}", node.description),
            serde_json::json!({"parameters": node.parameters}),
        );

        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(serde_json::json!({
            "task_id": node.id,
            "role": node.role.as_str(),
            "status": "completed",
            "output": "Task executed successfully (stub)"
        }))
    }

    /// Get results for all executed tasks
    pub async fn get_results(&self) -> HashMap<TaskId, TaskResult> {
        self.results.read().await.clone()
    }
}

/// Summary report of graph execution
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionReport {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventCollector;
    use crate::planner::{AgentRole, TaskGraph, TaskNode};

    #[tokio::test]
    async fn test_orchestrator_execution() {
        let (collector, _receiver) = EventCollector::new();
        let config = OrchestratorConfig::default();
        let orchestrator = RigOrchestrator::new(config, collector);

        let mut graph = TaskGraph::new();
        let node = TaskNode::new(
            "test1".to_string(),
            "Test task".to_string(),
            AgentRole::Researcher,
        );
        graph.add_node(node).unwrap();

        let report = orchestrator.execute(&graph).await.unwrap();
        assert_eq!(report.total_tasks, 1);
        assert_eq!(report.successful_tasks, 1);
    }

    #[tokio::test]
    async fn test_orchestrator_with_dependencies() {
        let (collector, _receiver) = EventCollector::new();
        let config = OrchestratorConfig::default();
        let orchestrator = RigOrchestrator::new(config, collector);

        let mut graph = TaskGraph::new();

        let node1 = TaskNode::new("1".to_string(), "One".to_string(), AgentRole::Researcher);
        let node2 = TaskNode::new("2".to_string(), "Two".to_string(), AgentRole::Analyst)
            .with_dependency("1".to_string());

        graph.add_node(node1).unwrap();
        graph.add_node(node2).unwrap();

        let report = orchestrator.execute(&graph).await.unwrap();
        assert_eq!(report.total_tasks, 2);
        assert_eq!(report.successful_tasks, 2);
    }
}
