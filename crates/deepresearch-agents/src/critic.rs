//! Critic agent for fact-checking and consistency validation.

use crate::agent_context::{Agent, AgentContext, AgentResult};
use deepresearch_core::AgentRole;

/// Critic agent that validates claims and checks consistency
pub struct CriticAgent {}

impl CriticAgent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CriticAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Agent for CriticAgent {
    async fn execute(&self, context: &AgentContext) -> Result<AgentResult, anyhow::Error> {
        tracing::info!(task_id = %context.task_id, "Critic agent executing");

        context.send_message(
            None,
            "Starting fact-checking and validation".to_string(),
            serde_json::json!({"stage": "validation"}),
        );

        // TODO: Implement actual fact-checking logic
        // For now, return stub data

        let min_confidence = context
            .get_param("min_confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75);

        tracing::debug!(
            min_confidence = %min_confidence,
            "Performing fact-check (stub)"
        );

        // Simulate validation work
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

        let validation_results = serde_json::json!({
            "claims_checked": 5,
            "claims_verified": 4,
            "claims_flagged": 1,
            "verification_rate": 0.80,
            "confidence_scores": {
                "claim_1": 0.92,
                "claim_2": 0.85,
                "claim_3": 0.78,
                "claim_4": 0.88,
                "claim_5": 0.65
            }
        });

        context.send_message(
            None,
            "Validation complete".to_string(),
            validation_results.clone(),
        );

        Ok(AgentResult {
            task_id: context.task_id.clone(),
            role: AgentRole::Critic,
            output: serde_json::json!({
                "validation_summary": "4 of 5 claims verified above threshold",
                "details": validation_results,
                "recommendations": [
                    "Claim 5 requires additional verification (0.65 < 0.75)",
                    "Overall report meets confidence threshold"
                ]
            }),
            sources: vec![],
            confidence: 0.80,
        })
    }

    fn role(&self) -> AgentRole {
        AgentRole::Critic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deepresearch_core::EventCollector;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_critic_execution() {
        let (collector, _receiver) = EventCollector::new();
        let agent = CriticAgent::new();

        let mut params = HashMap::new();
        params.insert("min_confidence".to_string(), serde_json::json!(0.75));

        let context = AgentContext::new(
            "test_task".to_string(),
            AgentRole::Critic,
            params,
            collector,
        );

        let result = agent.execute(&context).await.unwrap();
        assert_eq!(result.role, AgentRole::Critic);
        assert!(result.confidence > 0.0);
    }
}
