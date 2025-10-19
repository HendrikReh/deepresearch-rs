use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use deepresearch_core::{
    delete_session, load_session_report, resume_research_session_with_report,
    run_research_session_with_report, DeleteOptions, EvaluationHarness, LoadOptions, ResumeOptions,
    SessionOptions, SessionOutcome,
};
#[cfg(feature = "qdrant-retriever")]
use deepresearch_core::{IngestDocument, IngestOptions, RetrieverChoice};
use serde::Serialize;
#[cfg(feature = "qdrant-retriever")]
use std::path::Path;
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[cfg(feature = "qdrant-retriever")]
use anyhow::Context;
#[cfg(feature = "qdrant-retriever")]
use deepresearch_core::ingest_documents as ingest_docs;
#[cfg(feature = "qdrant-retriever")]
use std::fs;
#[cfg(feature = "qdrant-retriever")]
use uuid::Uuid;
#[cfg(feature = "qdrant-retriever")]
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "deepresearch-cli",
    version,
    about = "DeepResearch GraphFlow interface"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a fresh research session.
    Query(QueryArgs),
    /// Resume an existing workflow.
    Resume(ResumeArgs),
    /// Render the stored trace for a session.
    Explain(ExplainArgs),
    /// Ingest local documents into the retrieval layer.
    Ingest(IngestArgs),
    /// Aggregate evaluation metrics from a JSONL log.
    Eval(EvalArgs),
    /// Delete a session from the configured storage backend.
    Purge(PurgeArgs),
}

#[derive(Copy, Clone, Debug, ValueEnum, Default)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ExplainFormat {
    Markdown,
    Mermaid,
    Graphviz,
}

impl ExplainFormat {
    fn render(self, outcome: &SessionOutcome) -> Option<String> {
        match self {
            ExplainFormat::Markdown => outcome.explain_markdown(),
            ExplainFormat::Mermaid => outcome.explain_mermaid(),
            ExplainFormat::Graphviz => outcome.explain_graphviz(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            ExplainFormat::Markdown => "markdown",
            ExplainFormat::Mermaid => "mermaid",
            ExplainFormat::Graphviz => "graphviz",
        }
    }
}

trait RenderText {
    fn render_text(&self) -> String;
}

#[derive(Serialize)]
struct SessionResponse {
    action: &'static str,
    session_id: String,
    summary: Option<String>,
    trace_path: Option<String>,
    explanation: Option<String>,
    explanation_format: Option<String>,
}

impl RenderText for SessionResponse {
    fn render_text(&self) -> String {
        let mut sections = vec![
            format!("action: {}", self.action),
            format!("session: {}", self.session_id),
        ];

        if let Some(summary) = &self.summary {
            sections.push(format!("summary:\n{}", summary));
        }

        if let Some(explanation) = &self.explanation {
            let label = self.explanation_format.as_deref().unwrap_or("markdown");
            let mut block = String::new();
            if let Some(fence) = self
                .explanation_format
                .as_deref()
                .and_then(|fmt| match fmt {
                    "mermaid" => Some("```mermaid\n"),
                    "graphviz" => Some("```dot\n"),
                    _ => None,
                })
            {
                block.push_str(fence);
                block.push_str(explanation);
                if !explanation.ends_with('\n') {
                    block.push('\n');
                }
                block.push_str("```");
            } else {
                block.push_str(explanation);
            }
            sections.push(format!("explanation ({label}):\n{block}"));
        }

        if let Some(path) = &self.trace_path {
            sections.push(format!("trace: {}", path));
        }

        sections.join("\n\n")
    }
}

#[derive(Serialize)]
struct EvalResponse {
    total_sessions: usize,
    evaluated_sessions: usize,
    average_confidence: f32,
    failures: Vec<String>,
    summary: String,
}

impl RenderText for EvalResponse {
    fn render_text(&self) -> String {
        let mut lines = vec![self.summary.clone()];
        if !self.failures.is_empty() {
            lines.push(format!("failing sessions: {}", self.failures.join(", ")));
        }
        lines.join("\n")
    }
}

#[cfg(feature = "qdrant-retriever")]
#[derive(Serialize)]
struct IngestResponse {
    session_id: String,
    documents_indexed: usize,
}

#[cfg(feature = "qdrant-retriever")]
impl RenderText for IngestResponse {
    fn render_text(&self) -> String {
        format!(
            "ingested {count} document(s) into session {id}",
            count = self.documents_indexed,
            id = self.session_id
        )
    }
}

#[derive(Serialize)]
struct PurgeResponse {
    session_id: String,
    deleted: bool,
}

impl RenderText for PurgeResponse {
    fn render_text(&self) -> String {
        if self.deleted {
            format!("session {} purged", self.session_id)
        } else {
            format!("session {} not found", self.session_id)
        }
    }
}

fn emit_output<T>(format: OutputFormat, payload: &T) -> Result<()>
where
    T: RenderText + Serialize,
{
    match format {
        OutputFormat::Text => {
            println!("{}", payload.render_text());
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(payload)?);
        }
    }
    Ok(())
}

#[derive(Args, Debug)]
struct QueryArgs {
    /// Natural-language prompt to research.
    #[arg(value_name = "PROMPT")]
    prompt: String,

    /// Optional session ID.
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

    /// Persist trace events to disk even when not printing explanations.
    #[arg(long)]
    persist_trace: bool,

    /// Directory to persist `trace.json` (defaults to `data/traces`).
    #[arg(long)]
    trace_dir: Option<PathBuf>,

    /// Include a reasoning trace in the response.
    #[arg(long)]
    explain: bool,

    /// Rendering format for the reasoning trace.
    #[arg(long, value_enum, default_value_t = ExplainFormat::Markdown)]
    explain_format: ExplainFormat,

    /// Output format (text or JSON).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Use Postgres-backed session storage.
    #[cfg(feature = "postgres-session")]
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
}

#[derive(Args, Debug)]
struct ResumeArgs {
    /// Session ID to resume.
    #[arg(value_name = "SESSION_ID")]
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

    /// Persist trace events to disk even when not printing explanations.
    #[arg(long)]
    persist_trace: bool,

    /// Directory to persist `trace.json`.
    #[arg(long)]
    trace_dir: Option<PathBuf>,

    /// Include a reasoning trace in the response.
    #[arg(long)]
    explain: bool,

    /// Rendering format for the reasoning trace.
    #[arg(long, value_enum, default_value_t = ExplainFormat::Markdown)]
    explain_format: ExplainFormat,

    /// Output format (text or JSON).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Use Postgres-backed session storage.
    #[cfg(feature = "postgres-session")]
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
}

#[derive(Args, Debug)]
struct ExplainArgs {
    /// Session ID to explain.
    #[arg(value_name = "SESSION_ID")]
    session: String,

    /// Directory to persist `trace.json` when available.
    #[arg(long)]
    trace_dir: Option<PathBuf>,

    /// Persist trace even if the session already has one.
    #[arg(long)]
    persist_trace: bool,

    /// Include the final summary in the output.
    #[arg(long)]
    include_summary: bool,

    /// Rendering format for the reasoning trace.
    #[arg(long, value_enum, default_value_t = ExplainFormat::Markdown)]
    explain_format: ExplainFormat,

    /// Output format (text or JSON).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Use Postgres-backed session storage.
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

    /// Output format (text or JSON).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Args, Debug)]
struct EvalArgs {
    /// Path to the JSONL evaluation log.
    #[arg(value_name = "LOG_PATH")]
    path: PathBuf,

    /// Output format (text or JSON).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Args, Debug)]
struct PurgeArgs {
    /// Session ID to delete.
    #[arg(value_name = "SESSION_ID")]
    session: String,

    /// Output format (text or JSON).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Use Postgres-backed session storage.
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
            Command::Query(args) => query_command(args).await?,
            Command::Resume(args) => resume_command(args).await?,
            Command::Explain(args) => explain_command(args).await?,
            Command::Ingest(args) => ingest_command(args).await?,
            Command::Eval(args) => eval_command(args).await?,
            Command::Purge(args) => purge_command(args).await?,
        }
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

async fn query_command(args: QueryArgs) -> Result<()> {
    info!(prompt = %args.prompt, "starting DeepResearch session");

    let mut options = SessionOptions::new(&args.prompt);

    if let Some(session_id) = args.session.as_deref() {
        options = options.with_session_id(session_id);
    }

    #[cfg(feature = "postgres-session")]
    if let Some(ref url) = args.database_url {
        options = options.with_postgres_storage(url.clone());
    }

    #[cfg(not(feature = "qdrant-retriever"))]
    if args.qdrant_url.is_some() {
        warn!("qdrant retriever feature not enabled; falling back to stub retrieval");
    }

    if let Some(ref qdrant_url) = args.qdrant_url {
        options = options.with_qdrant_retriever(
            qdrant_url.clone(),
            args.qdrant_collection.clone(),
            args.qdrant_concurrency,
        );
    }

    if args.explain || args.persist_trace || args.trace_dir.is_some() {
        if let Some(dir) = args.trace_dir.as_ref() {
            options = options.with_trace_output_dir(dir.clone());
        } else {
            options = options.enable_trace();
        }
    }

    let outcome = run_research_session_with_report(options).await?;
    let (explanation, explanation_format) = if args.explain {
        match args.explain_format.render(&outcome) {
            Some(text) => (Some(text), Some(args.explain_format.label().to_string())),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    let trace_path = outcome
        .trace_path
        .as_ref()
        .map(|path| path.display().to_string());

    let response = SessionResponse {
        action: "query",
        session_id: outcome.session_id,
        summary: Some(outcome.summary),
        trace_path,
        explanation,
        explanation_format,
    };

    emit_output(args.format, &response)
}

async fn resume_command(args: ResumeArgs) -> Result<()> {
    info!(session = %args.session, "resuming DeepResearch session");

    let mut options = ResumeOptions::new(args.session.clone());

    #[cfg(feature = "postgres-session")]
    if let Some(ref url) = args.database_url {
        options = options.with_postgres_storage(url.clone());
    }

    #[cfg(not(feature = "qdrant-retriever"))]
    if args.qdrant_url.is_some() {
        warn!("qdrant retriever feature not enabled; falling back to stub retrieval");
    }

    if let Some(ref url) = args.qdrant_url {
        options = options.with_qdrant_retriever(
            url.clone(),
            args.qdrant_collection.clone(),
            args.qdrant_concurrency,
        );
    }

    if args.explain || args.persist_trace || args.trace_dir.is_some() {
        if let Some(dir) = args.trace_dir.as_ref() {
            options = options.with_trace_output_dir(dir.clone());
        } else {
            options = options.enable_trace();
        }
    }

    let outcome = resume_research_session_with_report(options).await?;

    let (explanation, explanation_format) = if args.explain {
        match args.explain_format.render(&outcome) {
            Some(text) => (Some(text), Some(args.explain_format.label().to_string())),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    let trace_path = outcome
        .trace_path
        .as_ref()
        .map(|path| path.display().to_string());

    let response = SessionResponse {
        action: "resume",
        session_id: outcome.session_id,
        summary: Some(outcome.summary),
        trace_path,
        explanation,
        explanation_format,
    };

    emit_output(args.format, &response)
}

async fn explain_command(args: ExplainArgs) -> Result<()> {
    info!(session = %args.session, "rendering DeepResearch trace");

    let mut options = LoadOptions::new(args.session.clone());

    #[cfg(feature = "postgres-session")]
    if let Some(ref url) = args.database_url {
        options = options.with_postgres_storage(url.clone());
    }

    if args.persist_trace || args.trace_dir.is_some() {
        if let Some(dir) = args.trace_dir.as_ref() {
            options = options.with_trace_output_dir(dir.clone());
        } else {
            options = options.with_trace_output_dir(PathBuf::from("data/traces"));
        }
    }

    let outcome = load_session_report(options).await?;

    let explanation = args.explain_format.render(&outcome);
    let explanation_format = explanation
        .as_ref()
        .map(|_| args.explain_format.label().to_string());

    let trace_path = outcome
        .trace_path
        .as_ref()
        .map(|path| path.display().to_string());

    let summary = if args.include_summary {
        Some(outcome.summary)
    } else {
        None
    };

    let response = SessionResponse {
        action: "explain",
        session_id: outcome.session_id,
        summary,
        trace_path,
        explanation,
        explanation_format,
    };

    emit_output(args.format, &response)
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
        info!(
            path = %args.path.display(),
            "no documents matched ingestion criteria"
        );
        let response = IngestResponse {
            session_id: args.session,
            documents_indexed: 0,
        };
        emit_output(args.format, &response)?;
        return Ok(());
    }
    let count = documents.len();

    let options = IngestOptions {
        session_id: args.session.clone(),
        documents,
        retriever: RetrieverChoice::qdrant(
            qdrant_url,
            args.qdrant_collection,
            args.qdrant_concurrency,
        ),
    };

    ingest_docs(options).await?;

    let response = IngestResponse {
        session_id: args.session,
        documents_indexed: count,
    };
    emit_output(args.format, &response)
}

#[cfg(not(feature = "qdrant-retriever"))]
async fn ingest_command(args: IngestArgs) -> Result<()> {
    let _ = args;
    warn!(
        "qdrant retriever feature not enabled; ingestion requires building with `--features deepresearch-cli/qdrant-retriever`"
    );
    Ok(())
}

async fn eval_command(args: EvalArgs) -> Result<()> {
    let metrics = EvaluationHarness::analyze_log(&args.path)?;
    let response = EvalResponse {
        total_sessions: metrics.total_sessions,
        evaluated_sessions: metrics.evaluated_sessions,
        average_confidence: metrics.average_confidence,
        failures: metrics.failures.clone(),
        summary: metrics.summary(),
    };
    emit_output(args.format, &response)
}

async fn purge_command(args: PurgeArgs) -> Result<()> {
    let session_id = args.session.clone();

    #[cfg(feature = "postgres-session")]
    let options = {
        let base = DeleteOptions::new(session_id.clone());
        if let Some(ref url) = args.database_url {
            base.with_postgres_storage(url.clone())
        } else {
            base
        }
    };

    #[cfg(not(feature = "postgres-session"))]
    let options = DeleteOptions::new(session_id.clone());

    let deleted = delete_session(options).await.is_ok();
    let response = PurgeResponse {
        session_id,
        deleted,
    };
    emit_output(args.format, &response)
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
