use anyhow::Result;
use deepresearch_core::run_research_session;
use tokio::runtime::Runtime;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let rt = Runtime::new()?;
    rt.block_on(async {
        info!("starting DeepResearch demo session");
        let summary = run_research_session("Assess lithium battery market drivers 2024").await?;
        println!("{}", summary);
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
