use anyhow::Result;
use deepresearch_core::{init_telemetry, ConfigLoader, TelemetryOptions};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    init_telemetry(TelemetryOptions::default())?;
    let config = ConfigLoader::load(None)?;

    info!(
        "deepresearch-api initialised (provider={}, qdrant={})",
        config.llm.provider, config.qdrant.url
    );
    info!("API server stub - routes will be implemented in milestone 1+");

    Ok(())
}
