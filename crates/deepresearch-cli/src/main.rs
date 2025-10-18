use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use deepresearch_core::{
    resume_research_session, run_research_session_with_options, ResumeOptions, SessionOptions,
};
use tokio::runtime::Runtime;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "deepresearch-cli",
    version,
    about = "DeepResearch GraphFlow demo"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a research session from scratch.
    Run(RunArgs),
    /// Resume a previously created session.
    Resume(ResumeArgs),
}

#[derive(Args, Debug)]
struct RunArgs {
    /// Query to research.
    #[arg(long, default_value = "Assess lithium battery market drivers 2024")]
    query: String,

    /// Optional session ID (UUID recommended when using Postgres storage).
    #[arg(long)]
    session: Option<String>,

    /// Use Postgres-backed session storage (falls back to in-memory if omitted).
    #[cfg(feature = "postgres-session")]
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
}

#[derive(Args, Debug)]
struct ResumeArgs {
    /// Session ID to resume (must exist in storage).
    #[arg(long)]
    session: String,

    /// Use Postgres-backed session storage (falls back to in-memory if omitted).
    #[cfg(feature = "postgres-session")]
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
}

fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,deepresearch_core=info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    let cli = Cli::parse();

    let rt = Runtime::new()?;
    rt.block_on(async move {
        match cli.command {
            Command::Run(args) => run_command(args).await?,
            Command::Resume(args) => resume_command(args).await?,
        }
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

async fn run_command(args: RunArgs) -> Result<()> {
    info!(query = %args.query, "starting DeepResearch session");

    let mut options = SessionOptions::new(&args.query);

    if let Some(session_id) = args.session {
        options = options.with_session_id(session_id);
    }

    #[cfg(feature = "postgres-session")]
    if let Some(url) = args.database_url {
        options = options.with_postgres_storage(url);
    }

    let summary = run_research_session_with_options(options).await?;
    println!("{}", summary);
    Ok(())
}

async fn resume_command(args: ResumeArgs) -> Result<()> {
    info!(session = %args.session, "resuming DeepResearch session");

    #[cfg(feature = "postgres-session")]
    let options = {
        let mut opts = ResumeOptions::new(args.session);
        if let Some(url) = args.database_url {
            opts = opts.with_postgres_storage(url);
        }
        opts
    };

    #[cfg(not(feature = "postgres-session"))]
    let options = ResumeOptions::new(args.session);

    let summary = resume_research_session(options).await?;
    println!("{}", summary);
    Ok(())
}
