//! Analyst agent for synthesizing findings into structured reports.

use crate::agent_context::{Agent, AgentContext, AgentResult};
use deepresearch_core::AgentRole;

/// Analyst agent that synthesizes research findings
pub struct AnalystAgent {}

impl AnalystAgent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AnalystAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Agent for AnalystAgent {
    async fn execute(&self, context: &AgentContext) -> Result<AgentResult, anyhow::Error> {
        tracing::info!(task_id = %context.task_id, "Analyst agent executing");

        context.send_message(
            None,
            "Starting analysis and synthesis".to_string(),
            serde_json::json!({"stage": "analysis"}),
        );

        // TODO: Implement actual LLM-driven synthesis
        // For now, return stub data

        let synthesis_mode = context
            .get_param("synthesis_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("standard");

        tracing::debug!(mode = %synthesis_mode, "Synthesizing findings (stub)");

        // Simulate analysis work
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        context.send_message(
            None,
            "Synthesis complete".to_string(),
            serde_json::json!({"stage": "complete"}),
        );

        Ok(AgentResult {
            task_id: context.task_id.clone(),
            role: AgentRole::Analyst,
            output: serde_json::json!({
                "summary": "Based on the research findings, the analysis indicates...",
                "key_points": [
                    "Point 1: Major finding from sources",
                    "Point 2: Supporting evidence",
                    "Point 3: Implications and trends"
                ],
                "synthesis_mode": synthesis_mode
            }),
            sources: vec![],
            confidence: 0.82,
        })
    }

    fn role(&self) -> AgentRole {
        AgentRole::Analyst
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deepresearch_core::EventCollector;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_analyst_execution() {
        let (collector, _receiver) = EventCollector::new();
        let agent = AnalystAgent::new();

        let mut params = HashMap::new();
        params.insert(
            "synthesis_mode".to_string(),
            serde_json::json!("comprehensive"),
        );

        let context = AgentContext::new(
            "test_task".to_string(),
            AgentRole::Analyst,
            params,
            collector,
        );

        let result = agent.execute(&context).await.unwrap();
        assert_eq!(result.role, AgentRole::Analyst);
        assert!(result.confidence > 0.0);
    }
}
