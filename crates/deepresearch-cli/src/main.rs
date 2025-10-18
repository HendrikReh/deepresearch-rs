use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use deepresearch_core::{init_telemetry, ConfigLoader, TelemetryOptions};
use tracing::{debug, info};

#[derive(Parser, Debug)]
#[command(name = "deepresearch-cli")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "DeepResearch command-line interface")]
struct Cli {
    /// Override configuration file path.
    #[arg(long = "config", value_name = "FILE")]
    config_path: Option<PathBuf>,

    /// Disable ANSI colours in logs.
    #[arg(long)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute a research query (skeleton implementation).
    Query {
        #[arg(value_name = "QUERY")]
        query: String,
    },
    /// Ingest local documents into the knowledge base (skeleton).
    Ingest {
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
    /// Evaluate a previous run using a log file (skeleton).
    Eval {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Print explanation for last session (skeleton).
    Explain {
        #[arg(long, value_name = "SESSION", help = "Session identifier")]
        session: Option<String>,
    },
    /// Resume a previous session (skeleton).
    Resume {
        #[arg(value_name = "SESSION")]
        session: String,
    },
    /// Purge stored data for a session (security baseline).
    Purge {
        #[arg(value_name = "SESSION")]
        session: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    init_telemetry(TelemetryOptions {
        env_filter: None,
        with_ansi: !cli.no_color,
    })?;

    let config = ConfigLoader::load(cli.config_path)?;
    info!("configuration loaded for provider: {}", config.llm.provider);

    match cli.command {
        Commands::Query { query } => {
            info!("query request received");
            debug!(%query, "query skeleton");
        }
        Commands::Ingest { path } => {
            info!("ingest request received");
            debug!(path = %path.display(), "ingest skeleton");
        }
        Commands::Eval { file } => {
            info!("evaluation request received");
            debug!(file = %file.display(), "eval skeleton");
        }
        Commands::Explain { session } => {
            info!("explain request received");
            debug!(?session, "explain skeleton");
        }
        Commands::Resume { session } => {
            info!("resume request received");
            debug!(%session, "resume skeleton");
        }
        Commands::Purge { session } => {
            info!("purge request received");
            debug!(%session, "purge skeleton - no data removed (stub)");
        }
    }

    Ok(())
}
