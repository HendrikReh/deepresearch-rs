//! Agent role definitions (placeholder for milestone 1+).

use deepresearch_core::TaskError;
use tracing::info;

/// Placeholder researcher role to be expanded in later milestones.
pub struct ResearcherAgent;

impl ResearcherAgent {
    pub async fn execute(&self, _query: &str) -> Result<(), TaskError> {
        info!("researcher agent skeleton executed");
        Ok(())
    }
}
