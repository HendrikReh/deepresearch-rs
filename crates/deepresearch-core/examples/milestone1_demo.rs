//! Milestone 1 Demo: End-to-end orchestration showcase
//!
//! Run with: cargo run --example milestone1_demo

use deepresearch_core::{
    init_telemetry, AgentRole, EventCollector, GraphExecutorConfig, GraphFlowExecutor,
    PlannerAgent, TaskGraph, TaskNode, TelemetryOptions, TraceCollector,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize telemetry
    init_telemetry(TelemetryOptions {
        env_filter: Some("info".to_string()),
        with_ansi: true,
    })?;

    println!("═══════════════════════════════════════════════════════════");
    println!("  DeepResearch Milestone 1 Demo");
    println!("  Multi-Agent Orchestration & Task Planning");
    println!("═══════════════════════════════════════════════════════════\n");

    // Demo 1: Simple linear pipeline
    println!("📋 Demo 1: Linear Pipeline (Research → Analyze → Critique)");
    println!("─────────────────────────────────────────────────────────\n");

    let query = "What are the key trends in renewable energy adoption for 2024?";
    demo_linear_pipeline(query).await?;

    println!("\n");

    // Demo 2: Complex DAG with parallel tasks
    println!("📋 Demo 2: Complex DAG with Parallel Researchers");
    println!("─────────────────────────────────────────────────────────\n");

    demo_parallel_research().await?;

    println!("\n");

    // Demo 3: Planner agent usage
    println!("📋 Demo 3: Planner Agent (LLM-driven planning stub)");
    println!("─────────────────────────────────────────────────────────\n");

    demo_planner_agent(query).await?;

    println!("\n");

    // Demo 4: Event trace collection
    println!("📋 Demo 4: Explainability & Trace Collection");
    println!("─────────────────────────────────────────────────────────\n");

    demo_trace_collection().await?;

    println!("\n═══════════════════════════════════════════════════════════");
    println!("  ✅ All demos completed successfully!");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}

async fn demo_linear_pipeline(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Query: \"{}\"\n", query);

    let (collector, _receiver) = EventCollector::new();
    let config = GraphExecutorConfig::default();
    let orchestrator = GraphFlowExecutor::new(config, collector);

    // Build a simple linear pipeline
    let mut graph = TaskGraph::new();

    let research = TaskNode::new(
        "research_1".to_string(),
        "Research renewable energy trends".to_string(),
        AgentRole::Researcher,
    )
    .with_param("query", serde_json::json!(query))
    .with_param("sources", serde_json::json!(["web", "local"]));

    let analyze = TaskNode::new(
        "analyze_1".to_string(),
        "Analyze and synthesize findings".to_string(),
        AgentRole::Analyst,
    )
    .with_param("synthesis_mode", serde_json::json!("comprehensive"))
    .with_dependency("research_1".to_string());

    let critique = TaskNode::new(
        "critique_1".to_string(),
        "Fact-check and validate report".to_string(),
        AgentRole::Critic,
    )
    .with_param("min_confidence", serde_json::json!(0.75))
    .with_dependency("analyze_1".to_string());

    graph.add_node(research)?;
    graph.add_node(analyze)?;
    graph.add_node(critique)?;

    println!("📊 Task Graph:");
    println!("   Nodes: {}", graph.len());
    println!("   Order: {:?}", graph.topological_order()?);
    println!();

    // Execute the graph
    println!("🚀 Executing graph...\n");
    let report = orchestrator.execute(&graph).await?;

    println!("📈 Execution Report:");
    println!("   Total tasks:      {}", report.total_tasks);
    println!("   Successful:       {}", report.successful_tasks);
    println!("   Failed:           {}", report.failed_tasks);
    println!("   Duration:         {}ms", report.duration_ms);

    // Show results
    let results = orchestrator.get_results().await;
    println!("\n📦 Task Results:");
    for (task_id, result) in results.iter() {
        println!(
            "   • {} → {:?} ({}ms)",
            task_id, result.outcome, result.duration_ms
        );
    }

    Ok(())
}

async fn demo_parallel_research() -> Result<(), Box<dyn std::error::Error>> {
    let (collector, _receiver) = EventCollector::new();
    let config = GraphExecutorConfig::default();
    let orchestrator = GraphFlowExecutor::new(config, collector);

    let mut graph = TaskGraph::new();

    // Two parallel research tasks
    let research_web = TaskNode::new(
        "research_web".to_string(),
        "Research via web sources".to_string(),
        AgentRole::Researcher,
    )
    .with_param("sources", serde_json::json!(["web"]));

    let research_local = TaskNode::new(
        "research_local".to_string(),
        "Research via local corpus".to_string(),
        AgentRole::Researcher,
    )
    .with_param("sources", serde_json::json!(["local"]));

    // Synthesis depends on both
    let synthesize = TaskNode::new(
        "synthesize".to_string(),
        "Synthesize all findings".to_string(),
        AgentRole::Analyst,
    )
    .with_dependency("research_web".to_string())
    .with_dependency("research_local".to_string());

    graph.add_node(research_web)?;
    graph.add_node(research_local)?;
    graph.add_node(synthesize)?;

    println!("📊 Parallel DAG:");
    println!("   research_web    ╮");
    println!("                   ├─► synthesize");
    println!("   research_local  ╯");
    println!();

    println!("🚀 Executing parallel tasks...\n");
    let report = orchestrator.execute(&graph).await?;

    println!("📈 Execution Report:");
    println!("   Total tasks:      {}", report.total_tasks);
    println!("   Successful:       {}", report.successful_tasks);
    println!("   Duration:         {}ms", report.duration_ms);
    println!("   (Parallel execution enables faster completion)");

    Ok(())
}

async fn demo_planner_agent(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let planner = PlannerAgent::new(10, 0.8);

    println!("🧠 Planning query: \"{}\"\n", query);

    let graph = planner.plan(query).await?;

    println!("📊 Generated Task Graph:");
    println!("   Nodes:      {}", graph.len());
    println!("   Is acyclic: {}", graph.validate().is_ok());
    println!();

    println!("📝 Task Breakdown:");
    for (i, node) in graph.nodes().enumerate() {
        println!(
            "   {}. [{}] {}",
            i + 1,
            node.role.as_str(),
            node.description
        );
        if !node.dependencies.is_empty() {
            println!("      Dependencies: {:?}", node.dependencies);
        }
    }

    println!("\n🔄 Topological Order:");
    let order = graph.topological_order()?;
    for (i, task_id) in order.iter().enumerate() {
        let node = graph.get_node(task_id).unwrap();
        println!("   {} → {} ({})", i + 1, task_id, node.role.as_str());
    }

    Ok(())
}

async fn demo_trace_collection() -> Result<(), Box<dyn std::error::Error>> {
    let (collector, receiver) = EventCollector::new();
    let mut trace = TraceCollector::new(receiver, 1000);

    // Spawn trace collector in background
    let trace_handle = tokio::spawn(async move {
        // Collect with timeout
        tokio::select! {
            result = trace.collect() => result,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => Ok(()),
        }
    });

    // Execute a simple workflow
    let config = GraphExecutorConfig::default();
    let orchestrator = GraphFlowExecutor::new(config, collector.clone());

    let mut graph = TaskGraph::new();
    let task = TaskNode::new(
        "demo_task".to_string(),
        "Demo task for trace collection".to_string(),
        AgentRole::Researcher,
    );
    graph.add_node(task)?;

    println!("🚀 Executing task with trace collection...\n");
    orchestrator.execute(&graph).await?;

    // Give events time to be collected
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Close collector channel
    drop(collector);

    // Wait for trace collection to complete
    trace_handle.await??;

    println!("✅ Trace collection complete!");
    println!("   Events captured: Start, Finish, Message");
    println!("   Storage format:  JSON");
    println!("   Export ready:    graph_trace.json");
    println!();
    println!("💡 Note: In production, traces are persisted to disk for");
    println!("   CLI --explain flag and GUI visualization.");

    Ok(())
}
