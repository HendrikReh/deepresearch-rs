use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parquet::column::writer::ColumnWriter;
use parquet::data_type::{ByteArray, Int96};
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::parser::parse_message_type;
use serde::Deserialize;
use serde_json::de::Deserializer;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct SessionRecord {
    session_id: String,
    timestamp: DateTime<Utc>,
    query: String,
    verdict: String,
    requires_manual_review: bool,
    math_status: String,
    math_alert_required: bool,
    math_outputs: Vec<MathArtifactRecord>,
    math_stdout: String,
    math_stderr: String,
    trace_path: Option<String>,
    #[serde(default)]
    consent_provided: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MathArtifactRecord {
    path: String,
    kind: String,
    bytes_len: usize,
}

fn default_raw_dir() -> PathBuf {
    PathBuf::from("data/pipeline/raw")
}

fn default_curated_dir() -> PathBuf {
    PathBuf::from("data/pipeline/curated")
}

fn collect_jsonl_files(raw_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !raw_dir.exists() {
        return Ok(files);
    }
    for entry in WalkDir::new(raw_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "jsonl") {
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

fn parquet_writer(output: &Path) -> Result<SerializedFileWriter<File>> {
    let schema = parse_message_type(
        "message session_records {
            REQUIRED BYTE_ARRAY session_id (UTF8);
            REQUIRED INT96 timestamp;
            OPTIONAL BYTE_ARRAY query (UTF8);
            OPTIONAL BYTE_ARRAY verdict (UTF8);
            REQUIRED BOOLEAN requires_manual_review;
            OPTIONAL BYTE_ARRAY math_status (UTF8);
            REQUIRED BOOLEAN math_alert_required;
            OPTIONAL BYTE_ARRAY math_stdout (UTF8);
            OPTIONAL BYTE_ARRAY math_stderr (UTF8);
            OPTIONAL BYTE_ARRAY trace_path (UTF8);
        }",
    )?
    .root_schema_ptr()
    .clone();

    let file = File::create(output).with_context(|| format!("create {}", output.display()))?;
    let props = WriterProperties::builder().build();
    SerializedFileWriter::new(file, schema, props).context("create parquet writer")
}

fn write_parquet(output: &Path, records: &[SessionRecord]) -> Result<()> {
    let mut writer = parquet_writer(output)?;
    let mut row_group = writer
        .next_row_group()?
        .context("open row group")?;

    // session_id
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::ByteArrayColumnWriter(ref mut writer) = col {
            let values: Vec<ByteArray> = records
                .iter()
                .map(|r| ByteArray::from(r.session_id.as_str()))
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // timestamp (convert to Int96)
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::Int96ColumnWriter(ref mut writer) = col {
            let values: Vec<Int96> = records
                .iter()
                .map(|r| {
                    let nanos = r.timestamp.timestamp_nanos();
                    let mut int96 = Int96::from(0);
                    int96.data_mut()[0] = (nanos & 0xFFFF_FFFF) as u32;
                    int96.data_mut()[1] = ((nanos >> 32) & 0xFFFF_FFFF) as u32;
                    int96.data_mut()[2] = ((nanos >> 64) & 0xFFFF_FFFF) as u32;
                    int96
                })
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // query
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::ByteArrayColumnWriter(ref mut writer) = col {
            let values: Vec<ByteArray> = records
                .iter()
                .map(|r| ByteArray::from(r.query.as_str()))
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // verdict
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::ByteArrayColumnWriter(ref mut writer) = col {
            let values: Vec<ByteArray> = records
                .iter()
                .map(|r| ByteArray::from(r.verdict.as_str()))
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // requires_manual_review
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::BoolColumnWriter(ref mut writer) = col {
            let values: Vec<bool> = records.iter().map(|r| r.requires_manual_review).collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // math_status
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::ByteArrayColumnWriter(ref mut writer) = col {
            let values: Vec<ByteArray> = records
                .iter()
                .map(|r| ByteArray::from(r.math_status.as_str()))
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // math_alert_required
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::BoolColumnWriter(ref mut writer) = col {
            let values: Vec<bool> = records.iter().map(|r| r.math_alert_required).collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // math_stdout
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::ByteArrayColumnWriter(ref mut writer) = col {
            let values: Vec<ByteArray> = records
                .iter()
                .map(|r| ByteArray::from(r.math_stdout.as_str()))
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // math_stderr
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::ByteArrayColumnWriter(ref mut writer) = col {
            let values: Vec<ByteArray> = records
                .iter()
                .map(|r| ByteArray::from(r.math_stderr.as_str()))
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    // trace_path
    if let Some(mut col) = row_group.next_column()? {
        if let ColumnWriter::ByteArrayColumnWriter(ref mut writer) = col {
            let values: Vec<ByteArray> = records
                .iter()
                .map(|r| ByteArray::from(r.trace_path.clone().unwrap_or_default()))
                .collect();
            writer.write_batch(&values, None, None)?;
        }
        row_group.close_column(col)?;
    }

    writer.close_row_group(row_group)?;
    writer.close()?;
    Ok(())
}

fn run(raw_dir: &Path, curated_dir: &Path) -> Result<()> {
    let files = collect_jsonl_files(raw_dir)?;
    if files.is_empty() {
        println!("No raw records found in {}; skipping", raw_dir.display());
        return Ok(());
    }

    let mut records = Vec::new();
    for file in files {
        let mut batch = read_records(&file)?;
        records.append(&mut batch);
    }

    if records.is_empty() {
        println!("No consented records found; skipping output");
        return Ok(());
    }

    fs::create_dir_all(curated_dir).with_context(|| format!("create {}", curated_dir.display()))?;
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S");
    let output = curated_dir.join(format!("sessions_{}.parquet", timestamp));
    write_parquet(&output, &records)?;
    println!("Wrote {} records to {}", records.len(), output.display());
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let raw_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(default_raw_dir);
    let curated_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(default_curated_dir);

    println!(
        "Consolidating records from {} -> {}",
        raw_dir.display(),
        curated_dir.display()
    );
    run(&raw_dir, &curated_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_parquet_from_sample_jsonl() -> Result<()> {
        let raw_dir = tempdir()?;
        let curated_dir = tempdir()?;

        let raw_file = raw_dir.path().join("2024-01-01.jsonl");
        std::fs::write(
            &raw_file,
            r#"{"session_id":"demo","timestamp":"2024-01-01T00:00:00Z","query":"use context7 foo","verdict":"ok","requires_manual_review":false,"math_status":"success","math_alert_required":false,"math_outputs":[],"math_stdout":"","math_stderr":"","trace_path":null}
"#,
        )?;

        run(raw_dir.path(), curated_dir.path())?;

        let outputs: Vec<_> = std::fs::read_dir(curated_dir.path())?
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(outputs.len(), 1);

        Ok(())
    }
}
