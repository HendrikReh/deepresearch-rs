//! Event bus for explainability and trace collection.
//!
//! All orchestrator and agent events flow through this system to enable
//! transparent reasoning graphs and audit trails.

use crate::planner::{AgentRole, TaskId};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Unique identifier for an event
pub type EventId = String;

/// Orchestrator and agent lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Task execution started
    Start {
        event_id: EventId,
        timestamp: u64,
        task_id: TaskId,
        role: AgentRole,
        description: String,
    },
    /// Task execution finished
    Finish {
        event_id: EventId,
        timestamp: u64,
        task_id: TaskId,
        role: AgentRole,
        outcome: TaskOutcome,
        duration_ms: u64,
    },
    /// Message between agents or internal reasoning step
    Message {
        event_id: EventId,
        timestamp: u64,
        from_task: TaskId,
        to_task: Option<TaskId>,
        role: AgentRole,
        content: String,
        metadata: serde_json::Value,
    },
}

impl Event {
    pub fn event_id(&self) -> &str {
        match self {
            Event::Start { event_id, .. } => event_id,
            Event::Finish { event_id, .. } => event_id,
            Event::Message { event_id, .. } => event_id,
        }
    }

    pub fn timestamp(&self) -> u64 {
        match self {
            Event::Start { timestamp, .. } => *timestamp,
            Event::Finish { timestamp, .. } => *timestamp,
            Event::Message { timestamp, .. } => *timestamp,
        }
    }

    pub fn task_id(&self) -> &TaskId {
        match self {
            Event::Start { task_id, .. } => task_id,
            Event::Finish { task_id, .. } => task_id,
            Event::Message { from_task, .. } => from_task,
        }
    }
}

/// Outcome of a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOutcome {
    Success,
    Failure { reason: String, retryable: bool },
    Timeout,
}

/// Event collector that aggregates events for trace generation
#[derive(Clone)]
pub struct EventCollector {
    sender: mpsc::UnboundedSender<Event>,
}

impl EventCollector {
    /// Create a new event collector
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Event>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }

    /// Emit a Start event
    pub fn emit_start(&self, task_id: TaskId, role: AgentRole, description: String) {
        let event = Event::Start {
            event_id: generate_event_id(),
            timestamp: current_timestamp(),
            task_id,
            role,
            description,
        };

        if let Err(e) = self.sender.send(event) {
            tracing::warn!(error = %e, "Failed to emit Start event");
        }
    }

    /// Emit a Finish event
    pub fn emit_finish(
        &self,
        task_id: TaskId,
        role: AgentRole,
        outcome: TaskOutcome,
        duration_ms: u64,
    ) {
        let event = Event::Finish {
            event_id: generate_event_id(),
            timestamp: current_timestamp(),
            task_id,
            role,
            outcome,
            duration_ms,
        };

        if let Err(e) = self.sender.send(event) {
            tracing::warn!(error = %e, "Failed to emit Finish event");
        }
    }

    /// Emit a Message event
    pub fn emit_message(
        &self,
        from_task: TaskId,
        to_task: Option<TaskId>,
        role: AgentRole,
        content: String,
        metadata: serde_json::Value,
    ) {
        let event = Event::Message {
            event_id: generate_event_id(),
            timestamp: current_timestamp(),
            from_task,
            to_task,
            role,
            content,
            metadata,
        };

        if let Err(e) = self.sender.send(event) {
            tracing::warn!(error = %e, "Failed to emit Message event");
        }
    }
}

impl Default for EventCollector {
    fn default() -> Self {
        Self::new().0
    }
}

/// Generate a unique event ID
fn generate_event_id() -> EventId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("evt_{}", id)
}

/// Get current Unix timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Trace collector service that persists events to storage
pub struct TraceCollector {
    receiver: mpsc::UnboundedReceiver<Event>,
    events: Vec<Event>,
    max_buffer_size: usize,
}

impl TraceCollector {
    pub fn new(receiver: mpsc::UnboundedReceiver<Event>, max_buffer_size: usize) -> Self {
        Self {
            receiver,
            events: Vec::new(),
            max_buffer_size,
        }
    }

    /// Collect events from the channel
    pub async fn collect(&mut self) -> Result<(), crate::error::DeepResearchError> {
        while let Some(event) = self.receiver.recv().await {
            tracing::trace!(event_id = %event.event_id(), "Collected event");
            self.events.push(event);

            if self.events.len() >= self.max_buffer_size {
                tracing::warn!(
                    buffer_size = self.events.len(),
                    "Event buffer full, flushing to disk"
                );
                self.flush()?;
            }
        }
        Ok(())
    }

    /// Flush events to storage
    pub fn flush(&mut self) -> Result<(), crate::error::DeepResearchError> {
        // TODO: Implement persistent storage (graph_trace.json)
        tracing::debug!(
            event_count = self.events.len(),
            "Flushing events to storage"
        );
        self.events.clear();
        Ok(())
    }

    /// Get all collected events
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Export events as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_collector() {
        let (collector, mut receiver) = EventCollector::new();

        collector.emit_start(
            "task1".to_string(),
            AgentRole::Researcher,
            "Test task".to_string(),
        );

        let event = receiver.recv().await.unwrap();
        match event {
            Event::Start { task_id, .. } => assert_eq!(task_id, "task1"),
            _ => panic!("Expected Start event"),
        }
    }

    #[tokio::test]
    async fn test_trace_collector() {
        let (collector, receiver) = EventCollector::new();
        let mut trace = TraceCollector::new(receiver, 100);

        collector.emit_start(
            "task1".to_string(),
            AgentRole::Researcher,
            "Test".to_string(),
        );

        // Give time for event to be sent
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        drop(collector); // Close channel

        trace.collect().await.unwrap();
        assert_eq!(trace.events().len(), 1);
    }
}
