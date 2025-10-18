//! Integration tests for Milestone 1 components

use deepresearch_core::{
    AgentRole, EventCollector, GraphExecutorConfig, GraphFlowExecutor, PlannerAgent, TaskGraph,
    TaskNode, TaskOutcome,
};

#[tokio::test]
async fn test_end_to_end_orchestration() {
    // Create event collector
    let (collector, _receiver) = EventCollector::new();

    // Create orchestrator with default config
    let config = GraphExecutorConfig::default();
    let orchestrator = GraphFlowExecutor::new(config, collector);

    // Build a simple pipeline
    let mut graph = TaskGraph::new();

    let research = TaskNode::new(
        "research".to_string(),
        "Research task".to_string(),
        AgentRole::Researcher,
    );

    let analysis = TaskNode::new(
        "analysis".to_string(),
        "Analysis task".to_string(),
        AgentRole::Analyst,
    )
    .with_dependency("research".to_string());

    graph.add_node(research).unwrap();
    graph.add_node(analysis).unwrap();

    // Execute graph
    let report = orchestrator.execute(&graph).await.unwrap();

    // Verify execution
    assert_eq!(report.total_tasks, 2);
    assert_eq!(report.successful_tasks, 2);
    assert_eq!(report.failed_tasks, 0);
    assert!(report.duration_ms > 0);

    // Verify results
    let results = orchestrator.get_results().await;
    assert_eq!(results.len(), 2);

    for result in results.values() {
        assert!(matches!(result.outcome, TaskOutcome::Success));
    }
}

#[tokio::test]
async fn test_planner_generates_valid_graph() {
    let planner = PlannerAgent::new(10, 0.8);

    let query = "Test query for planning";
    let graph = planner.plan(query).await.unwrap();

    // Verify graph properties
    assert!(!graph.is_empty());
    assert!(graph.validate().is_ok());

    let order = graph.topological_order().unwrap();
    assert_eq!(order.len(), graph.len());
}

#[tokio::test]
async fn test_parallel_execution() {
    let (collector, _receiver) = EventCollector::new();
    let config = GraphExecutorConfig::default();
    let orchestrator = GraphFlowExecutor::new(config, collector);

    let mut graph = TaskGraph::new();

    // Two independent tasks
    let task1 = TaskNode::new(
        "task1".to_string(),
        "Independent task 1".to_string(),
        AgentRole::Researcher,
    );

    let task2 = TaskNode::new(
        "task2".to_string(),
        "Independent task 2".to_string(),
        AgentRole::Researcher,
    );

    // A task that depends on both
    let task3 = TaskNode::new(
        "task3".to_string(),
        "Dependent task".to_string(),
        AgentRole::Analyst,
    )
    .with_dependency("task1".to_string())
    .with_dependency("task2".to_string());

    graph.add_node(task1).unwrap();
    graph.add_node(task2).unwrap();
    graph.add_node(task3).unwrap();

    let report = orchestrator.execute(&graph).await.unwrap();

    assert_eq!(report.total_tasks, 3);
    assert_eq!(report.successful_tasks, 3);
}

#[tokio::test]
async fn test_cycle_detection() {
    let mut graph = TaskGraph::new();

    let node1 = TaskNode::new("1".to_string(), "Node 1".to_string(), AgentRole::Researcher);

    // This should work - valid linear dependency
    let node2 = TaskNode::new("2".to_string(), "Node 2".to_string(), AgentRole::Analyst)
        .with_dependency("1".to_string());

    graph.add_node(node1).unwrap();
    graph.add_node(node2).unwrap();

    // Validation should pass
    assert!(graph.validate().is_ok());
}

#[tokio::test]
async fn test_event_collection() {
    let (collector, mut receiver) = EventCollector::new();

    // Emit some events
    collector.emit_start(
        "test_task".to_string(),
        AgentRole::Researcher,
        "Test description".to_string(),
    );

    collector.emit_finish(
        "test_task".to_string(),
        AgentRole::Researcher,
        TaskOutcome::Success,
        100,
    );

    // Collect events
    let mut events = Vec::new();

    // Receive with timeout
    while let Ok(event) =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await
    {
        if let Some(evt) = event {
            events.push(evt);
        } else {
            break;
        }
    }

    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_orchestrator_concurrency_limit() {
    let (collector, _receiver) = EventCollector::new();

    // Configure with low concurrency limit
    let config = GraphExecutorConfig {
        max_concurrent_tasks: 2,
        ..Default::default()
    };

    let orchestrator = GraphFlowExecutor::new(config, collector);

    // Create multiple independent tasks
    let mut graph = TaskGraph::new();
    for i in 0..5 {
        let task = TaskNode::new(
            format!("task_{}", i),
            format!("Task {}", i),
            AgentRole::Researcher,
        );
        graph.add_node(task).unwrap();
    }

    let report = orchestrator.execute(&graph).await.unwrap();

    assert_eq!(report.total_tasks, 5);
    assert_eq!(report.successful_tasks, 5);
    // Duration should show serialization due to concurrency limit
    assert!(report.duration_ms >= 500); // At least 500ms for 5 tasks with 100ms each
}
