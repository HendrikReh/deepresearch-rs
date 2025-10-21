use crate::tasks::MathToolResult;
use crate::workflow::SessionOutcome;
use chrono::{DateTime, Utc};
use graph_flow::Session;
use serde::Serialize;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Serialize)]
struct MathArtifactRecord {
    path: String,
    kind: String,
    bytes_len: usize,
}

#[derive(Serialize)]
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
}

fn pipeline_dir() -> PathBuf {
    std::env::var("DEEPRESEARCH_PIPELINE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/pipeline/raw"))
}

fn todays_file(dir: &Path) -> PathBuf {
    let filename = format!("{}.jsonl", Utc::now().format("%Y-%m-%d"));
    dir.join(filename)
}

fn collect_math_outputs(result: &MathToolResult) -> Vec<MathArtifactRecord> {
    result
        .outputs
        .iter()
        .map(|output| MathArtifactRecord {
            path: output.path.clone(),
            kind: format_kind(&output.kind),
            bytes_len: output.bytes.len(),
        })
        .collect()
}

fn format_kind(kind: &crate::sandbox::SandboxOutputKind) -> String {
    match kind {
        crate::sandbox::SandboxOutputKind::Binary => "binary".to_string(),
        crate::sandbox::SandboxOutputKind::Text => "text".to_string(),
    }
}

pub fn persist_session_record(session: &Session, outcome: &SessionOutcome) {
    let dir = pipeline_dir();
    if let Err(err) = create_dir_all(&dir) {
        warn!(error = %err, path = %dir.display(), "unable to create pipeline directory");
        return;
    }

    let math_result = session.context.get_sync::<MathToolResult>("math.result");
    let math_status = session
        .context
        .get_sync::<String>("math.status")
        .unwrap_or_else(|| "skipped".to_string());
    let math_alert_required = session
        .context
        .get_sync::<bool>("math.alert_required")
        .unwrap_or(false);
    let math_stdout = session
        .context
        .get_sync::<String>("math.stdout")
        .unwrap_or_default();
    let math_stderr = session
        .context
        .get_sync::<String>("math.stderr")
        .unwrap_or_default();

    let query = session
        .context
        .get_sync::<String>("query")
        .unwrap_or_default();
    let verdict = session
        .context
        .get_sync::<String>("critique.verdict")
        .unwrap_or_default();

    let math_outputs = math_result
        .as_ref()
        .map(collect_math_outputs)
        .unwrap_or_default();

    let record = SessionRecord {
        session_id: outcome.session_id.clone(),
        timestamp: Utc::now(),
        query,
        verdict,
        requires_manual_review: outcome.requires_manual,
        math_status,
        math_alert_required,
        math_outputs,
        math_stdout,
        math_stderr,
        trace_path: outcome.trace_path.as_ref().map(|p| p.display().to_string()),
    };

    let file_path = todays_file(&dir);
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
    {
        Ok(file) => file,
        Err(err) => {
            warn!(error = %err, path = %file_path.display(), "unable to open pipeline log");
            return;
        }
    };

    if let Err(err) = serde_json::to_writer(&mut file, &record) {
        warn!(error = %err, "failed to serialise session record");
        return;
    }
    if let Err(err) = writeln!(file) {
        warn!(error = %err, "failed to append newline to pipeline log");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::TraceSummary;
    use graph_flow::Session;
    use tempfile::tempdir;

    fn dummy_session() -> Session {
        let session = Session::new_from_task("test".to_string(), "researcher");
        session.context.set_sync("query", "use context7 dummy");
        session.context.set_sync("critique.verdict", "Looks good");
        session
    }

    #[test]
    fn writes_record_when_enabled() {
        let dir = tempdir().unwrap();
        unsafe {
            std::env::set_var("DEEPRESEARCH_PIPELINE_DIR", dir.path());
        }

        let session = dummy_session();
        let outcome = SessionOutcome {
            session_id: "test".into(),
            summary: "Summary".into(),
            trace_events: vec![],
            trace_summary: TraceSummary::default(),
            trace_path: None,
            requires_manual: false,
            factcheck_confidence: None,
            factcheck_passed: None,
            factcheck_verified_sources: vec![],
            critic_confident: None,
        };

        persist_session_record(&session, &outcome);

        let files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1);

        let contents = std::fs::read_to_string(&files[0]).unwrap();
        assert!(contents.contains("\"session_id\":\"test\""));

        unsafe {
            std::env::remove_var("DEEPRESEARCH_PIPELINE_DIR");
        }
    }
}
