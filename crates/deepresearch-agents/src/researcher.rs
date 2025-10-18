//! Researcher agent for retrieving facts via web and local search.

use crate::agent_context::{Agent, AgentContext, AgentResult, SourceReference};
use deepresearch_core::AgentRole;

/// Researcher agent that performs information retrieval
pub struct ResearcherAgent {}

impl ResearcherAgent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ResearcherAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Agent for ResearcherAgent {
    async fn execute(&self, context: &AgentContext) -> Result<AgentResult, anyhow::Error> {
        tracing::info!(task_id = %context.task_id, "Researcher agent executing");

        context.send_message(
            None,
            "Starting research task".to_string(),
            serde_json::json!({"stage": "init"}),
        );

        // TODO: Implement actual search via MCP SDK
        // For now, return stub data

        let query = context
            .get_param("query")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown query");

        tracing::debug!(query = %query, "Performing search (stub)");

        // Simulate retrieval work
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let sources = vec![
            SourceReference {
                id: "src_1".to_string(),
                uri: "https://example.com/article1".to_string(),
                title: Some("Example Article 1".to_string()),
                snippet: Some("Relevant information about the query...".to_string()),
            },
            SourceReference {
                id: "src_2".to_string(),
                uri: "qdrant://local/doc_42".to_string(),
                title: Some("Local Document".to_string()),
                snippet: Some("Additional context from local corpus...".to_string()),
            },
        ];

        context.send_message(
            None,
            format!("Retrieved {} sources", sources.len()),
            serde_json::json!({"source_count": sources.len()}),
        );

        Ok(AgentResult {
            task_id: context.task_id.clone(),
            role: AgentRole::Researcher,
            output: serde_json::json!({
                "query": query,
                "findings": "Research findings would go here...",
                "source_ids": ["src_1", "src_2"]
            }),
            sources,
            confidence: 0.85,
        })
    }

    fn role(&self) -> AgentRole {
        AgentRole::Researcher
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deepresearch_core::EventCollector;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_researcher_execution() {
        let (collector, _receiver) = EventCollector::new();
        let agent = ResearcherAgent::new();

        let mut params = HashMap::new();
        params.insert("query".to_string(), serde_json::json!("test query"));

        let context = AgentContext::new(
            "test_task".to_string(),
            AgentRole::Researcher,
            params,
            collector,
        );

        let result = agent.execute(&context).await.unwrap();
        assert_eq!(result.role, AgentRole::Researcher);
        assert!(!result.sources.is_empty());
    }
}
