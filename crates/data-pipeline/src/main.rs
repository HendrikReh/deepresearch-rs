use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::de::Deserializer;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use walkdir::WalkDir;

mod postgres;

#[derive(Debug, Deserialize, Serialize)]
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

fn read_records(path: &Path) -> Result<Vec<SessionRecord>> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);
    let stream = Deserializer::from_reader(reader).into_iter::<SessionRecord>();
    let mut records = Vec::new();
    for record in stream {
        let record = record.with_context(|| format!("parse JSONL in {}", path.display()))?;
        if record.consent_provided.unwrap_or(true) {
            records.push(record);
        }
    }
    Ok(records)
}

fn write_snapshot(dir: &Path, records: &[SessionRecord]) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S");
    let path = dir.join(format!("sessions_{}.json", timestamp));
    let file = File::create(&path)?;
    serde_json::to_writer_pretty(file, records)?;
    Ok(path)
}

fn run(args: Args) -> Result<()> {
    let files = collect_jsonl_files(&args.raw_dir)?;
    if files.is_empty() {
        println!(
            "No raw records found in {}; skipping",
            args.raw_dir.display()
        );
        return Ok(());
    }

    let mut all_records = Vec::new();
    for file in files {
        let mut batch = read_records(&file)?;
        all_records.append(&mut batch);
    }

    if all_records.is_empty() {
        println!("No consented records found; skipping output");
        return Ok(());
    }

    let snapshot_path = write_snapshot(&args.output_dir, &all_records)?;
    println!(
        "Wrote {} records to {}",
        all_records.len(),
        snapshot_path.display()
    );

    if let Some(url) = args.postgres_url.as_deref() {
        let rt = Runtime::new()?;
        rt.block_on(async {
            let pool = postgres::init_pool(url).await?;
            postgres::insert_records(&pool, &all_records).await
        })?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    run(args)
}
