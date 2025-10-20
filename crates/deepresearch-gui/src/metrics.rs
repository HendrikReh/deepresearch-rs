use anyhow::Result;
use tracing::info;

pub fn init_telemetry(endpoint: &str) -> Result<()> {
    info!(
        target = "telemetry.gui",
        endpoint,
        "OpenTelemetry endpoint configured; forward tracing spans via collector-side subscriber"
    );
    Ok(())
}

pub fn session_started(session_id: &str, running: usize, available_permits: usize) {
    info!(
        target = "telemetry.gui",
        session_id,
        running_sessions = running,
        available_permits,
        event = "session_started"
    );
}

pub fn session_completed(
    session_id: &str,
    requires_manual: bool,
    trace_events: usize,
    running: usize,
    available_permits: usize,
) {
    info!(
        target = "telemetry.gui",
        session_id,
        requires_manual,
        trace_events,
        running_sessions = running,
        available_permits,
        event = "session_completed"
    );
}

pub fn session_failed(session_id: &str, running: usize, available_permits: usize, error: &str) {
    info!(
        target = "telemetry.gui",
        session_id,
        running_sessions = running,
        available_permits,
        error,
        event = "session_failed"
    );
}

pub fn stream_opened(session_id: &str, active_streams: usize) {
    info!(
        target = "telemetry.gui",
        session_id,
        active_streams,
        event = "stream_opened"
    );
}

pub fn stream_closed(session_id: &str, active_streams: usize) {
    info!(
        target = "telemetry.gui",
        session_id,
        active_streams,
        event = "stream_closed"
    );
}
