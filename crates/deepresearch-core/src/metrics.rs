use anyhow::Result;
use once_cell::sync::OnceCell;
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::{KeyValue, global};
use tracing::info;

struct SandboxMetrics {
    runs: Counter<u64>,
    duration_ms: Histogram<f64>,
    alerts: Counter<u64>,
}

static METRICS: OnceCell<SandboxMetrics> = OnceCell::new();

fn handles() -> &'static SandboxMetrics {
    METRICS.get_or_init(|| {
        let meter: Meter = global::meter("deepresearch.sandbox");
        SandboxMetrics {
            runs: meter
                .u64_counter("sandbox_runs_total")
                .with_description("Total sandbox executions by status")
                .init(),
            duration_ms: meter
                .f64_histogram("sandbox_duration_ms")
                .with_description("Sandbox runtime in milliseconds")
                .init(),
            alerts: meter
                .u64_counter("sandbox_alerts_total")
                .with_description("Number of sandbox executions triggering alert thresholds")
                .init(),
        }
    })
}

/// Hint to operators that OTEL metrics export can be configured externally.
pub fn init_metrics_from_env(service_name: &str) -> Result<()> {
    if std::env::var("DEEPRESEARCH_OTEL_METRICS_ENDPOINT").is_ok() {
        info!(
            target = "telemetry",
            "DEEPRESEARCH_OTEL_METRICS_ENDPOINT detected for {service_name}. Configure an OTLP meter provider in your deployment to export sandbox metrics."
        );
    }
    Ok(())
}

/// No-op placeholder for symmetry with tracer shutdown.
pub fn shutdown_metrics() {}

/// Record OTEL metrics for a sandbox execution (no-op if no provider installed).
pub fn record_sandbox_metrics(status: &str, duration_ms: u64, outputs: usize, failure_streak: u64) {
    let metrics = handles();
    let attrs = [
        KeyValue::new("status", status.to_string()),
        KeyValue::new("outputs", outputs as i64),
    ];

    metrics.runs.add(1, &attrs);
    metrics.duration_ms.record(duration_ms as f64, &attrs);

    if failure_streak >= 3 {
        metrics.alerts.add(1, &attrs);
    }
}
