use anyhow::{Context, Result};
use chrono::{Datelike, Utc};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::warn;

const LOG_DIR_ENV: &str = "DEEPRESEARCH_LOG_DIR";
const RETENTION_ENV: &str = "DEEPRESEARCH_LOG_RETENTION_DAYS";
const DEFAULT_LOG_DIR: &str = "data/logs";
const DEFAULT_RETENTION_DAYS: u64 = 90;

static REDACTION_PATTERNS: Lazy<Vec<(String, Regex)>> = Lazy::new(|| {
    vec![
        (
            "api_key".to_string(),
            Regex::new(r"(?i)(api[_-]?key\s*[:=]\s*)([A-Za-z0-9\-_.+/]+)")
                .expect("invalid api_key regex"),
        ),
        (
            "secret".to_string(),
            Regex::new(r"(?i)(secret\s*[:=]\s*)([A-Za-z0-9\-_.+/]+)")
                .expect("invalid secret regex"),
        ),
        (
            "bearer".to_string(),
            Regex::new(r"(?i)(bearer\s+)([A-Za-z0-9\-_.+=/]+)").expect("invalid bearer regex"),
        ),
        (
            "sk_token".to_string(),
            Regex::new(r"(sk-[A-Za-z0-9]{16,})").expect("invalid sk_token regex"),
        ),
    ]
});

#[derive(Debug, Clone)]
pub struct SessionLogInput {
    pub session_id: String,
    pub query: Option<String>,
    pub summary: String,
    pub verdict: Option<String>,
    pub requires_manual: bool,
    pub sources: Vec<String>,
    pub trace_path: Option<String>,
}

#[derive(Serialize)]
struct SessionLogRecord {
    timestamp: String,
    session_id: String,
    query: Option<String>,
    summary: String,
    verdict: Option<String>,
    requires_manual: bool,
    sources: Vec<String>,
    trace_path: Option<String>,
    redactions: Vec<String>,
}

#[derive(Serialize)]
struct AuditLogRecord {
    timestamp: String,
    session_id: String,
    redactions: Vec<String>,
}

fn log_base_dir() -> PathBuf {
    std::env::var(LOG_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_LOG_DIR))
}

fn retention_days() -> u64 {
    std::env::var(RETENTION_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS)
}

fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let line = serde_json::to_string(value)?;
    writeln!(writer, "{}", line)
        .with_context(|| format!("failed to append log entry to {}", path.display()))?;
    writer.flush()?;
    Ok(())
}

fn sanitize_text(input: &str, redactions: &mut HashSet<String>) -> String {
    let mut output = input.to_string();
    for (name, regex) in REDACTION_PATTERNS.iter() {
        let mut matched = false;
        output = regex
            .replace_all(&output, |caps: &Captures| {
                matched = true;
                if caps.len() > 1 {
                    format!("{}[REDACTED]", &caps[1])
                } else {
                    "[REDACTED]".to_string()
                }
            })
            .to_string();
        if matched {
            redactions.insert(name.clone());
        }
    }
    output
}

pub fn log_session_completion(input: SessionLogInput) -> Result<()> {
    let timestamp = Utc::now();
    let mut redactions = HashSet::new();

    let query = input
        .query
        .as_deref()
        .map(|value| sanitize_text(value, &mut redactions));
    let summary = sanitize_text(&input.summary, &mut redactions);
    let verdict = input
        .verdict
        .as_deref()
        .map(|value| sanitize_text(value, &mut redactions));
    let sources: Vec<String> = input
        .sources
        .into_iter()
        .map(|source| sanitize_text(&source, &mut redactions))
        .collect();

    let record = SessionLogRecord {
        timestamp: timestamp.to_rfc3339(),
        session_id: input.session_id.clone(),
        query,
        summary,
        verdict,
        requires_manual: input.requires_manual,
        sources,
        trace_path: input.trace_path,
        redactions: redactions.iter().cloned().collect(),
    };

    let base_dir = log_base_dir();
    let month_dir = base_dir
        .join(format!("{:04}", timestamp.year()))
        .join(format!("{:02}", timestamp.month()));
    let session_log_path = month_dir.join("session.jsonl");
    append_json_line(&session_log_path, &record)?;

    if !record.redactions.is_empty() {
        let audit = AuditLogRecord {
            timestamp: record.timestamp.clone(),
            session_id: input.session_id.clone(),
            redactions: record.redactions.clone(),
        };
        let audit_path = month_dir.join("audit.jsonl");
        append_json_line(&audit_path, &audit)?;
        warn!(
            session_id = %input.session_id,
            fields = ?record.redactions,
            "redacted potential secrets from session log"
        );
    }

    enforce_retention(&base_dir)?;

    Ok(())
}

fn enforce_retention(base_dir: &Path) -> Result<()> {
    let retention = retention_days();
    if retention == 0 || !base_dir.exists() {
        return Ok(());
    }
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retention.saturating_mul(86_400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    prune_directory(base_dir, cutoff)?;
    Ok(())
}

fn prune_directory(dir: &Path, cutoff: SystemTime) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            prune_directory(&path, cutoff)?;
            if path.read_dir()?.next().is_none() {
                fs::remove_dir(&path).ok();
            }
        } else if metadata.is_file()
            && metadata
                .modified()
                .map(|time| time < cutoff)
                .unwrap_or(false)
        {
            fs::remove_file(&path).ok();
        }
    }

    Ok(())
}

pub fn remove_session_logs(session_id: &str) -> Result<()> {
    let base_dir = log_base_dir();
    if !base_dir.exists() {
        return Ok(());
    }

    for year_entry in fs::read_dir(&base_dir)? {
        let year_entry = year_entry?;
        if !year_entry.file_type()?.is_dir() {
            continue;
        }
        for month_entry in fs::read_dir(year_entry.path())? {
            let month_entry = month_entry?;
            if !month_entry.file_type()?.is_dir() {
                continue;
            }
            let session_path = month_entry.path().join("session.jsonl");
            rewrite_jsonl_without(&session_path, session_id)?;
            cleanup_empty_file(&session_path)?;

            let audit_path = month_entry.path().join("audit.jsonl");
            rewrite_jsonl_without(&audit_path, session_id)?;
            cleanup_empty_file(&audit_path)?;

            if month_entry.path().read_dir()?.next().is_none() {
                fs::remove_dir(month_entry.path()).ok();
            }
        }
        if year_entry.path().read_dir()?.next().is_none() {
            fs::remove_dir(year_entry.path()).ok();
        }
    }

    Ok(())
}

fn rewrite_jsonl_without(path: &Path, session_id: &str) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let file =
        File::open(path).with_context(|| format!("failed to open log file {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut retained = Vec::new();
    let mut removed = false;
    for line in reader.lines() {
        let line = line?;
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
            if value.get("session_id").and_then(|v| v.as_str()) == Some(session_id) {
                removed = true;
                continue;
            }
        }
        retained.push(line);
    }

    if removed {
        let file = File::create(path)
            .with_context(|| format!("failed to rewrite log file {}", path.display()))?;
        let mut writer = BufWriter::new(file);
        for line in retained {
            writeln!(writer, "{}", line)?;
        }
        writer.flush()?;
    }

    Ok(())
}

fn cleanup_empty_file(path: &Path) -> Result<()> {
    if path.exists() {
        let metadata = path.metadata()?;
        if metadata.len() == 0 {
            fs::remove_file(path).ok();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::TempDir;

    #[test]
    fn session_logging_sanitizes_and_persists() -> Result<()> {
        let temp = TempDir::new().expect("temp dir");
        std::env::set_var(LOG_DIR_ENV, temp.path());
        std::env::set_var(RETENTION_ENV, "0");

        let input = SessionLogInput {
            session_id: "test-session".to_string(),
            query: Some("Find api_key=abcd1234".to_string()),
            summary: "Summary with secret=topsecret".to_string(),
            verdict: Some("bearer XYZ".to_string()),
            requires_manual: false,
            sources: vec!["sk-abcdef1234567890".to_string()],
            trace_path: Some("data/traces/test.json".to_string()),
        };

        log_session_completion(input)?;

        let year_dir = temp.path().read_dir()?.next().unwrap()?.path();
        let month_dir = year_dir.read_dir()?.next().unwrap()?.path();
        let session_log = month_dir.join("session.jsonl");
        assert!(session_log.exists());
        let line = std::fs::read_to_string(&session_log)?;
        let record: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(record["session_id"], "test-session");
        assert!(record["summary"].as_str().unwrap().contains("[REDACTED]"));

        let audit_log = month_dir.join("audit.jsonl");
        assert!(audit_log.exists());

        remove_session_logs("test-session")?;
        if session_log.exists() {
            assert_eq!(std::fs::metadata(&session_log)?.len(), 0);
        }

        Ok(())
    }
}
