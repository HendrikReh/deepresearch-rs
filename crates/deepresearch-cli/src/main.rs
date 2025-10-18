use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use deepresearch_core::{
    resume_research_session, run_research_session_with_options, ResumeOptions, SessionOptions,
};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[cfg(feature = "qdrant-retriever")]
use anyhow::Context;
#[cfg(feature = "qdrant-retriever")]
use deepresearch_core::{
    ingest_documents as ingest_docs, IngestDocument, IngestOptions, RetrieverChoice,
};
#[cfg(feature = "qdrant-retriever")]
use std::fs;
#[cfg(feature = "qdrant-retriever")]
use std::path::Path;
#[cfg(feature = "qdrant-retriever")]
use uuid::Uuid;
#[cfg(feature = "qdrant-retriever")]
use walkdir::WalkDir;

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
    /// Ingest local documents into the retrieval store.
    Ingest(IngestArgs),
}

#[derive(Args, Debug)]
struct RunArgs {
    /// Query to research.
    #[arg(long, default_value = "Assess lithium battery market drivers 2024")]
    query: String,

    /// Optional session ID (UUID recommended when using Postgres storage).
    #[arg(long)]
    session: Option<String>,

    /// Optional Qdrant endpoint to enable hybrid retrieval.
    #[arg(long)]
    qdrant_url: Option<String>,

    /// Qdrant collection name (defaults to `deepresearch`).
    #[arg(long, default_value = "deepresearch")]
    qdrant_collection: String,

    /// Maximum concurrent Qdrant operations.
    #[arg(long, default_value_t = 8)]
    qdrant_concurrency: usize,

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

    /// Optional Qdrant endpoint to enable hybrid retrieval.
    #[arg(long)]
    qdrant_url: Option<String>,

    /// Qdrant collection name.
    #[arg(long, default_value = "deepresearch")]
    qdrant_collection: String,

    /// Maximum concurrent Qdrant operations.
    #[arg(long, default_value_t = 8)]
    qdrant_concurrency: usize,

    /// Use Postgres-backed session storage (falls back to in-memory if omitted).
    #[cfg(feature = "postgres-session")]
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
}

#[derive(Args, Debug)]
struct IngestArgs {
    /// Session namespace the documents belong to.
    #[arg(long)]
    session: String,

    /// File or directory to ingest (text files expected).
    #[arg(long)]
    path: PathBuf,

    /// Recurse into subdirectories when ingesting.
    #[arg(long, default_value_t = true)]
    recursive: bool,

    /// Qdrant endpoint hosting the vector collection.
    #[arg(long)]
    qdrant_url: Option<String>,

    /// Name of the Qdrant collection to upsert into.
    #[arg(long, default_value = "deepresearch")]
    qdrant_collection: String,

    /// Maximum concurrent Qdrant operations.
    #[arg(long, default_value_t = 8)]
    qdrant_concurrency: usize,
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
            Command::Ingest(args) => ingest_command(args).await?,
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

    #[cfg(not(feature = "qdrant-retriever"))]
    if args.qdrant_url.is_some() {
        warn!("qdrant retriever feature not enabled; falling back to stub retrieval");
    }

    if let Some(qdrant_url) = args.qdrant_url {
        options = options.with_qdrant_retriever(
            qdrant_url,
            args.qdrant_collection,
            args.qdrant_concurrency,
        );
    }

    let summary = run_research_session_with_options(options).await?;
    println!("{}", summary);
    Ok(())
}

async fn resume_command(args: ResumeArgs) -> Result<()> {
    info!(session = %args.session, "resuming DeepResearch session");

    let mut options = ResumeOptions::new(args.session);

    #[cfg(feature = "postgres-session")]
    if let Some(url) = args.database_url {
        options = options.with_postgres_storage(url);
    }

    #[cfg(not(feature = "qdrant-retriever"))]
    if args.qdrant_url.is_some() {
        warn!("qdrant retriever feature not enabled; falling back to stub retrieval");
    }

    if let Some(url) = args.qdrant_url {
        options =
            options.with_qdrant_retriever(url, args.qdrant_collection, args.qdrant_concurrency);
    }

    let summary = resume_research_session(options).await?;
    println!("{}", summary);
    Ok(())
}

#[cfg(feature = "qdrant-retriever")]
async fn ingest_command(args: IngestArgs) -> Result<()> {
    let qdrant_url = match args.qdrant_url {
        Some(url) => url,
        None => {
            warn_stub_ingest();
            return Ok(());
        }
    };

    let documents = collect_documents(&args.path, args.recursive)?;
    if documents.is_empty() {
        info!(path = %args.path.display(), "no documents matched ingestion criteria");
        return Ok(());
    }

    let options = IngestOptions {
        session_id: args.session,
        documents,
        retriever: RetrieverChoice::qdrant(
            qdrant_url,
            args.qdrant_collection,
            args.qdrant_concurrency,
        ),
    };

    ingest_docs(options).await?;
    info!("ingestion complete");
    Ok(())
}

#[cfg(not(feature = "qdrant-retriever"))]
async fn ingest_command(_args: IngestArgs) -> Result<()> {
    warn!(
        "qdrant retriever feature not enabled; ingestion requires building with `--features deepresearch-cli/qdrant-retriever`"
    );
    Ok(())
}

#[cfg(feature = "qdrant-retriever")]
fn warn_stub_ingest() {
    warn!("no Qdrant URL provided; ingestion skipped (only stub retriever active)");
}

#[cfg(feature = "qdrant-retriever")]
fn collect_documents(path: &Path, recursive: bool) -> Result<Vec<IngestDocument>> {
    let mut docs = Vec::new();
    let entries: Box<dyn Iterator<Item = PathBuf>> = if path.is_file() {
        Box::new(std::iter::once(path.to_path_buf()))
    } else {
        let walker =
            WalkDir::new(path)
                .min_depth(0)
                .max_depth(if recursive { usize::MAX } else { 1 });
        Box::new(
            walker
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_file())
                .map(|entry| entry.into_path()),
        )
    };

    for file in entries {
        let text = fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        if text.trim().is_empty() {
            continue;
        }
        docs.push(IngestDocument {
            id: Uuid::new_v4().to_string(),
            text,
            source: Some(file.display().to_string()),
        });
    }

    Ok(docs)
}
