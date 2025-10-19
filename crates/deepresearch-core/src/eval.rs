use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct FactCheckLog {
    pub confidence: f32,
    pub passed: bool,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct LogEntry {
    pub session_id: Option<String>,
    #[serde(default)]
    pub factcheck: Option<FactCheckLog>,
}

#[derive(Debug, Default, Clone)]
pub struct EvaluationMetrics {
    pub total_sessions: usize,
    pub evaluated_sessions: usize,
    pub average_confidence: f32,
    pub failures: Vec<String>,
}

impl EvaluationMetrics {
    pub fn record(&mut self, session_id: Option<String>, log: &FactCheckLog) {
        self.evaluated_sessions += 1;
        self.average_confidence =
            ((self.average_confidence * (self.evaluated_sessions - 1) as f32) + log.confidence)
                / self.evaluated_sessions as f32;
        if !log.passed {
            if let Some(id) = session_id {
                self.failures.push(id);
            }
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "evaluated {}/{} sessions • avg confidence {:.2} • {} failure(s)",
            self.evaluated_sessions,
            self.total_sessions,
            self.average_confidence,
            self.failures.len()
        )
    }
}

pub struct EvaluationHarness;

impl EvaluationHarness {
    pub fn analyze_log(path: impl AsRef<Path>) -> Result<EvaluationMetrics> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("failed to open log file {}", path.as_ref().display()))?;
        let mut metrics = EvaluationMetrics::default();

        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<LogEntry>(&line) {
                Ok(entry) => {
                    metrics.total_sessions += 1;
                    if let Some(fact) = entry.factcheck {
                        metrics.record(entry.session_id, &fact);
                    }
                }
                Err(err) => {
                    tracing::debug!(%err, "skipping malformed evaluation log entry");
                }
            }
        }

        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use uuid::Uuid;

    #[test]
    fn evaluation_harness_aggregates_confidence() {
        let mut path = std::env::temp_dir();
        path.push(format!("deepresearch-eval-{}.log", Uuid::new_v4()));
        let mut writer = BufWriter::new(File::create(&path).expect("temp file"));
        writeln!(
            writer,
            r#"{{"session_id":"a","factcheck":{{"confidence":0.8,"passed":true}}}}"#
        )
        .unwrap();
        writeln!(
            writer,
            r#"{{"session_id":"b","factcheck":{{"confidence":0.4,"passed":false}}}}"#
        )
        .unwrap();
        writer.flush().unwrap();

        let metrics = EvaluationHarness::analyze_log(&path).expect("metrics");
        std::fs::remove_file(path).ok();

        assert_eq!(metrics.total_sessions, 2);
        assert_eq!(metrics.evaluated_sessions, 2);
        assert!((metrics.average_confidence - 0.6).abs() < f32::EPSILON);
        assert_eq!(metrics.failures, vec!["b".to_string()]);
    }
}
