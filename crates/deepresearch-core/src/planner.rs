//! Task planning and DAG construction for multi-agent research workflows.
//!
//! The planner decomposes user queries into sub-tasks, assigns agent roles,
//! and constructs a directed acyclic graph (DAG) representing task dependencies.

use crate::error::DeepResearchError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Unique identifier for a task node
pub type TaskId = String;

/// Agent role assignment for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Retrieves facts via web and local search
    Researcher,
    /// Synthesizes findings into structured reports
    Analyst,
    /// Validates claims and checks consistency
    Critic,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Researcher => "Researcher",
            AgentRole::Analyst => "Analyst",
            AgentRole::Critic => "Critic",
        }
    }
}

/// A single task node in the execution graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    /// Unique task identifier
    pub id: TaskId,
    /// Human-readable description
    pub description: String,
    /// Agent role assigned to execute this task
    pub role: AgentRole,
    /// Task-specific parameters (search queries, analysis instructions, etc.)
    pub parameters: HashMap<String, serde_json::Value>,
    /// IDs of tasks that must complete before this one
    pub dependencies: Vec<TaskId>,
}

impl TaskNode {
    pub fn new(id: TaskId, description: String, role: AgentRole) -> Self {
        Self {
            id,
            description,
            role,
            parameters: HashMap::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    pub fn with_dependency(mut self, dep_id: TaskId) -> Self {
        self.dependencies.push(dep_id);
        self
    }
}

/// Directed acyclic graph of tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    /// All task nodes indexed by ID
    nodes: HashMap<TaskId, TaskNode>,
    /// Adjacency list: task_id -> [dependent_task_ids]
    edges: HashMap<TaskId, Vec<TaskId>>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    /// Add a task node to the graph
    pub fn add_node(&mut self, node: TaskNode) -> Result<(), DeepResearchError> {
        if self.nodes.contains_key(&node.id) {
            return Err(DeepResearchError::PlanningError(format!(
                "Task node with ID '{}' already exists",
                node.id
            )));
        }

        // Validate dependencies exist
        for dep_id in &node.dependencies {
            if !self.nodes.contains_key(dep_id) {
                return Err(DeepResearchError::PlanningError(format!(
                    "Dependency '{}' not found for task '{}'",
                    dep_id, node.id
                )));
            }
        }

        // Build reverse edges
        for dep_id in &node.dependencies {
            self.edges
                .entry(dep_id.clone())
                .or_default()
                .push(node.id.clone());
        }

        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Validate that the graph is acyclic
    pub fn validate(&self) -> Result<(), DeepResearchError> {
        // Topological sort via Kahn's algorithm
        let mut in_degree: HashMap<&TaskId, usize> = HashMap::new();

        // Initialize in-degrees - count incoming edges for each node
        for node_id in self.nodes.keys() {
            in_degree.insert(node_id, 0);
        }

        // For each node, increment in-degree count for the node itself based on its dependencies
        for node in self.nodes.values() {
            *in_degree.get_mut(&node.id).unwrap() = node.dependencies.len();
        }

        // Queue nodes with zero in-degree (no dependencies)
        let mut queue: VecDeque<&TaskId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut visited_count = 0;

        while let Some(node_id) = queue.pop_front() {
            visited_count += 1;

            // Reduce in-degree for nodes that depend on this one
            if let Some(dependents) = self.edges.get(node_id) {
                for dep_id in dependents {
                    let degree = in_degree.get_mut(dep_id).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dep_id);
                    }
                }
            }
        }

        if visited_count != self.nodes.len() {
            return Err(DeepResearchError::PlanningError(
                "Graph contains cycles".to_string(),
            ));
        }

        Ok(())
    }

    /// Get topological ordering of tasks
    pub fn topological_order(&self) -> Result<Vec<TaskId>, DeepResearchError> {
        self.validate()?;

        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        for node_id in self.nodes.keys() {
            in_degree.insert(node_id.clone(), 0);
        }

        for node in self.nodes.values() {
            *in_degree.get_mut(&node.id).unwrap() = node.dependencies.len();
        }

        let mut queue: VecDeque<TaskId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut order = Vec::new();

        while let Some(node_id) = queue.pop_front() {
            order.push(node_id.clone());

            if let Some(dependents) = self.edges.get(&node_id) {
                for dep_id in dependents {
                    let degree = in_degree.get_mut(dep_id).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dep_id.clone());
                    }
                }
            }
        }

        Ok(order)
    }

    /// Get a task node by ID
    pub fn get_node(&self, id: &TaskId) -> Option<&TaskNode> {
        self.nodes.get(id)
    }

    /// Get all task nodes
    pub fn nodes(&self) -> impl Iterator<Item = &TaskNode> {
        self.nodes.values()
    }

    /// Get nodes that can execute immediately (no dependencies)
    pub fn ready_nodes(&self) -> Vec<&TaskNode> {
        self.nodes
            .values()
            .filter(|node| node.dependencies.is_empty())
            .collect()
    }

    /// Get number of nodes in graph
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Planner agent that decomposes queries into task graphs
pub struct PlannerAgent {
    #[allow(dead_code)] // TODO: Use in iterative planning
    max_iterations: usize,
    confidence_threshold: f64,
}

impl PlannerAgent {
    pub fn new(max_iterations: usize, confidence_threshold: f64) -> Self {
        Self {
            max_iterations,
            confidence_threshold,
        }
    }

    /// Decompose a query into a task graph
    ///
    /// This is a stub implementation. In production, this would call an LLM
    /// to dynamically generate tasks based on the query.
    pub async fn plan(&self, query: &str) -> Result<TaskGraph, DeepResearchError> {
        tracing::info!(query = %query, "Planning task decomposition");

        let mut graph = TaskGraph::new();

        // For MVP: simple three-stage pipeline
        // TODO: Replace with LLM-driven dynamic planning

        let research_task = TaskNode::new(
            "research_1".to_string(),
            format!("Research: {}", query),
            AgentRole::Researcher,
        )
        .with_param("query", serde_json::json!(query))
        .with_param("sources", serde_json::json!(["web", "local"]));

        let analysis_task = TaskNode::new(
            "analyze_1".to_string(),
            format!("Analyze findings for: {}", query),
            AgentRole::Analyst,
        )
        .with_param("synthesis_mode", serde_json::json!("comprehensive"))
        .with_dependency("research_1".to_string());

        let critique_task = TaskNode::new(
            "critique_1".to_string(),
            format!("Fact-check and validate: {}", query),
            AgentRole::Critic,
        )
        .with_param(
            "min_confidence",
            serde_json::json!(self.confidence_threshold),
        )
        .with_dependency("analyze_1".to_string());

        graph.add_node(research_task)?;
        graph.add_node(analysis_task)?;
        graph.add_node(critique_task)?;

        graph.validate()?;

        tracing::debug!(
            task_count = graph.len(),
            "Task graph constructed successfully"
        );

        Ok(graph)
    }

    /// Update task graph based on intermediate results
    ///
    /// Enables iterative refinement as new facts arrive
    pub async fn refine_plan(
        &self,
        _graph: &mut TaskGraph,
        _results: &HashMap<TaskId, serde_json::Value>,
    ) -> Result<(), DeepResearchError> {
        // TODO: Implement iterative planning based on intermediate results
        tracing::debug!("Plan refinement not yet implemented");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_graph_creation() {
        let mut graph = TaskGraph::new();

        let node = TaskNode::new(
            "task1".to_string(),
            "Test task".to_string(),
            AgentRole::Researcher,
        );

        assert!(graph.add_node(node).is_ok());
        assert_eq!(graph.len(), 1);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = TaskGraph::new();

        // Create valid linear chain
        let node1 = TaskNode::new(
            "task1".to_string(),
            "Task 1".to_string(),
            AgentRole::Researcher,
        );
        let node2 = TaskNode::new(
            "task2".to_string(),
            "Task 2".to_string(),
            AgentRole::Analyst,
        )
        .with_dependency("task1".to_string());

        graph.add_node(node1).unwrap();
        graph.add_node(node2).unwrap();

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_topological_order() {
        let mut graph = TaskGraph::new();

        let node1 = TaskNode::new("1".to_string(), "One".to_string(), AgentRole::Researcher);
        let node2 = TaskNode::new("2".to_string(), "Two".to_string(), AgentRole::Analyst)
            .with_dependency("1".to_string());
        let node3 = TaskNode::new("3".to_string(), "Three".to_string(), AgentRole::Critic)
            .with_dependency("2".to_string());

        graph.add_node(node1).unwrap();
        graph.add_node(node2).unwrap();
        graph.add_node(node3).unwrap();

        let order = graph.topological_order().unwrap();
        assert_eq!(
            order,
            vec!["1".to_string(), "2".to_string(), "3".to_string()]
        );
    }

    #[tokio::test]
    async fn test_planner_creates_valid_graph() {
        let planner = PlannerAgent::new(10, 0.8);
        let graph = planner.plan("test query").await.unwrap();

        assert!(!graph.is_empty());
        assert!(graph.validate().is_ok());
    }
}
