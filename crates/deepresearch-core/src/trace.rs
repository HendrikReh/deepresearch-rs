use std::fmt::Write as _;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub task_id: String,
    pub message: String,
    pub timestamp_ms: u128,
}

impl TraceEvent {
    pub fn new(task_id: impl Into<String>, message: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self {
            task_id: task_id.into(),
            message: message.into(),
            timestamp_ms,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceCollector {
    events: Vec<TraceEvent>,
}

impl TraceCollector {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn from_events(events: Vec<TraceEvent>) -> Self {
        Self { events }
    }

    pub fn record(&mut self, task_id: impl Into<String>, message: impl Into<String>) {
        self.events.push(TraceEvent::new(task_id, message));
    }

    pub fn extend<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TraceEvent>,
    {
        self.events.extend(events);
    }

    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<TraceEvent> {
        self.events
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn summary(&self) -> TraceSummary {
        TraceSummary::from_events(&self.events)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub index: usize,
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceSummary {
    pub steps: Vec<TraceStep>,
}

impl TraceSummary {
    pub fn from_events(events: &[TraceEvent]) -> Self {
        let steps = events
            .iter()
            .enumerate()
            .map(|(idx, event)| TraceStep {
                index: idx + 1,
                task_id: event.task_id.clone(),
                message: event.message.clone(),
            })
            .collect();
        Self { steps }
    }

    pub fn render_markdown(&self) -> String {
        if self.steps.is_empty() {
            return "No trace events recorded.".to_string();
        }
        let mut output = String::from("### Trace Summary\n");
        for step in &self.steps {
            let _ = writeln!(
                output,
                "{}. {} → {}",
                step.index, step.task_id, step.message
            );
        }
        output
    }

    pub fn render_mermaid(&self) -> String {
        if self.steps.is_empty() {
            return "flowchart TD\n  %% no trace events captured".to_string();
        }

        let mut output = String::from("flowchart TD\n  %% auto-generated trace\n");
        for step in &self.steps {
            let node_id = format!("step{}", step.index);
            let label = sanitize_mermaid(&format!("{}: {}", step.task_id, step.message));
            let _ = writeln!(output, "  {node_id}[\"{label}\"]");
        }

        for idx in 0..self.steps.len().saturating_sub(1) {
            let from = format!("step{}", idx + 1);
            let to = format!("step{}", idx + 2);
            let _ = writeln!(output, "  {from} --> {to}");
        }

        if !output.ends_with('\n') {
            output.push('\n');
        }

        output
    }

    pub fn render_graphviz(&self) -> String {
        if self.steps.is_empty() {
            return "digraph Trace {\n  // no trace events captured\n}".to_string();
        }

        let mut output = String::from("digraph Trace {\n  rankdir=LR;\n  node [shape=box];\n");
        for step in &self.steps {
            let node_id = format!("step{}", step.index);
            let label = format!("{}: {}", step.task_id, escape_graphviz(&step.message));
            let _ = writeln!(output, "  {node_id} [label=\"{label}\"];\n");
        }

        for idx in 1..self.steps.len() {
            let _ = writeln!(output, "  step{idx} -> step{};", idx + 1);
        }

        output.push_str("}\n");
        output
    }
}

fn sanitize_mermaid(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('[', "(")
        .replace(']', ")")
        .replace('\n', "<br/>")
}

fn escape_graphviz(text: &str) -> String {
    text.replace('"', "\\\"").replace('\n', " ")
}

pub fn persist_trace<P: AsRef<Path>>(
    dir: P,
    session_id: &str,
    events: &[TraceEvent],
) -> Result<PathBuf> {
    let dir = dir.as_ref();
    create_dir_all(dir)
        .with_context(|| format!("failed to create trace directory {}", dir.display()))?;
    let path = dir.join(format!("{session_id}.json"));
    let payload = serde_json::to_vec_pretty(events)?;
    let mut file = File::create(&path)
        .with_context(|| format!("failed to create trace file {}", path.display()))?;
    file.write_all(&payload)
        .with_context(|| format!("failed to write trace file {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_renders_steps() {
        let mut collector = TraceCollector::new();
        collector.record("researcher", "captured findings");
        collector.record("analyst", "highlight: growth insight");

        let summary = collector.summary();
        let markdown = summary.render_markdown();

        assert!(markdown.contains("1."));
        assert!(markdown.contains("researcher"));
        assert!(markdown.contains("analyst"));
    }

    #[test]
    fn mermaid_and_graphviz_render_sequences() {
        let events = vec![
            TraceEvent::new("fact_check", "confidence 0.8"),
            TraceEvent::new("critic", "verdict: auto"),
        ];
        let summary = TraceSummary::from_events(&events);

        let mermaid = summary.render_mermaid();
        assert!(mermaid.contains("flowchart TD"));
        assert!(mermaid.contains("step1"));

        let graphviz = summary.render_graphviz();
        assert!(graphviz.contains("digraph Trace"));
        assert!(graphviz.contains("step1"));
    }
}
