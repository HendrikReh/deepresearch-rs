use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use serde::ser::{SerializeSeq, Serializer as _};
use serde::{Deserialize, Serialize};
use serde_json::{
    de::Deserializer,
    ser::{PrettyFormatter, Serializer as JsonSerializer},
};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use walkdir::WalkDir;

mod postgres;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SessionRecord {
    session_id: String,
    timestamp: String,
    query: String,
    verdict: String,
    requires_manual_review: bool,
    math_status: String,
    math_alert_required: bool,
    math_outputs: serde_json::Value,
    math_stdout: String,
    math_stderr: String,
    trace_path: Option<String>,
    #[serde(default)]
    sandbox_failure_streak: Option<u64>,
    #[serde(default)]
    domain_label: Option<String>,
    #[serde(default)]
    confidence_bucket: Option<String>,
    #[serde(default)]
    consent_provided: Option<bool>,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "data/pipeline/raw")]
    raw_dir: PathBuf,
    #[arg(long, default_value = "data/pipeline/curated")]
    output_dir: PathBuf,
    #[arg(long)]
    postgres_url: Option<String>,
    /// Retention window for raw JSONL files (days). Set to 0 to skip pruning.
    #[arg(long, default_value_t = 30)]
    retain_days: i64,
    /// Maintain a convenience copy (e.g. sessions_latest.json).
    #[arg(long, default_value = "sessions_latest.json")]
    snapshot_alias: String,
    /// Number of records to accumulate before flushing inserts to Postgres.
    #[arg(long, default_value_t = 1000)]
    batch_size: usize,
}

struct PostgresSink {
    runtime: Runtime,
    pool: postgres::SessionPool,
    batch: Vec<SessionRecord>,
    batch_size: usize,
    total_inserted: usize,
    batches_flushed: usize,
}

impl PostgresSink {
    fn new(url: &str, batch_size: usize) -> Result<Self> {
        let runtime = Runtime::new()?;
        let pool = runtime.block_on(postgres::init_pool(url))?;
        Ok(Self {
            runtime,
            pool,
            batch: Vec::with_capacity(batch_size.max(1)),
            batch_size: batch_size.max(1),
            total_inserted: 0,
            batches_flushed: 0,
        })
    }

    fn push(&mut self, record: SessionRecord) -> Result<()> {
        self.batch.push(record);
        if self.batch.len() >= self.batch_size {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if self.batch.is_empty() {
            return Ok(());
        }
        let batch = std::mem::take(&mut self.batch);
        let count = batch.len();
        self.runtime
            .block_on(postgres::insert_records(&self.pool, &batch))?;
        self.total_inserted += count;
        self.batches_flushed += 1;
        Ok(())
    }

    fn finish(mut self) -> Result<(usize, usize)> {
        self.flush()?;
        Ok((self.total_inserted, self.batches_flushed))
    }
}

fn collect_jsonl_files(raw_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !raw_dir.exists() {
        return Ok(files);
    }
    for entry in WalkDir::new(raw_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn copy_alias(snapshot_path: &Path, alias_dir: &Path, alias_name: &str) -> Result<()> {
    let alias_path = alias_dir.join(alias_name);
    if alias_path.exists() {
        fs::remove_file(&alias_path).ok();
    }
    fs::copy(snapshot_path, &alias_path)
        .with_context(|| format!("copy snapshot to {}", alias_path.display()))?;
    Ok(())
}

fn assign_taxonomy(record: &mut SessionRecord) {
    let query_lower = record.query.to_lowercase();
    if record.domain_label.is_none() {
        let domain = if query_lower.contains("finance") || query_lower.contains("budget") {
            "finance"
        } else if query_lower.contains("security") || query_lower.contains("risk") {
            "security"
        } else if query_lower.contains("marketing") {
            "marketing"
        } else {
            "general"
        };
        record.domain_label = Some(domain.to_string());
    }

    if record.confidence_bucket.is_none() {
        let bucket = if record.math_alert_required {
            "low"
        } else if record.math_status.eq_ignore_ascii_case("success") {
            "high"
        } else {
            "medium"
        };
        record.confidence_bucket = Some(bucket.to_string());
    }
}

fn prune_raw(raw_dir: &Path, retain_days: i64) -> Result<()> {
    if retain_days <= 0 || !raw_dir.exists() {
        return Ok(());
    }
    let cutoff = Utc::now() - chrono::Duration::days(retain_days);
    for entry in WalkDir::new(raw_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let date_str = &stem[..std::cmp::min(10, stem.len())];
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                if date
                    .and_hms_opt(0, 0, 0)
                    .map(|naive| {
                        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                            naive,
                            chrono::Utc,
                        )
                    })
                    .map(|dt| dt < cutoff)
                    .unwrap_or(false)
                {
                    fs::remove_file(path).ok();
                }
            }
        }
    }
    Ok(())
}

fn run(args: Args) -> Result<()> {
    prune_raw(&args.raw_dir, args.retain_days)?;

    let files = collect_jsonl_files(&args.raw_dir)?;
    if files.is_empty() {
        println!(
            "No raw records found in {}; skipping",
            args.raw_dir.display()
        );
        return Ok(());
    }

    fs::create_dir_all(&args.output_dir)?;
    let snapshot_path = args.output_dir.join(format!(
        "sessions_{}.json",
        Utc::now().format("%Y%m%dT%H%M%S")
    ));
    let file = File::create(&snapshot_path)?;
    let formatter = PrettyFormatter::with_indent(b"  ");
    let mut serializer = JsonSerializer::with_formatter(file, formatter);

    let mut consented_count = 0usize;
    let mut sink = match args.postgres_url.as_deref() {
        Some(url) => Some(PostgresSink::new(url, args.batch_size)?),
        None => None,
    };

    {
        let mut seq = serializer.serialize_seq(None)?;
        for path in files {
            let file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
            let reader = BufReader::new(file);
            let stream = Deserializer::from_reader(reader).into_iter::<SessionRecord>();

            for record in stream {
                let mut record =
                    record.with_context(|| format!("parse JSONL in {}", path.display()))?;
                if !record.consent_provided.unwrap_or(true) {
                    continue;
                }
                assign_taxonomy(&mut record);
                seq.serialize_element(&record)?;
                consented_count += 1;

                if let Some(writer) = sink.as_mut() {
                    writer.push(record.clone())?;
                }
            }
        }
        seq.end()?;
    }

    if consented_count == 0 {
        fs::remove_file(&snapshot_path).ok();
        println!("No consented records found; skipping output");
        if let Some(writer) = sink {
            writer.finish()?;
        }
        return Ok(());
    }

    copy_alias(&snapshot_path, &args.output_dir, &args.snapshot_alias)?;
    println!(
        "Wrote {} records to {}",
        consented_count,
        snapshot_path.display()
    );

    if let Some(writer) = sink {
        let (inserted, batches) = writer.finish()?;
        println!(
            "Inserted {} records into Postgres across {} batch(es)",
            inserted, batches
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    run(args)
}
