use std::sync::OnceLock;

use tracing_subscriber::{fmt, EnvFilter};

use crate::DeepResearchError;

static TELEMETRY_GUARD: OnceLock<()> = OnceLock::new();

/// Configuration options when initialising telemetry.
#[derive(Debug, Clone)]
pub struct TelemetryOptions {
    pub env_filter: Option<String>,
    pub with_ansi: bool,
}

impl Default for TelemetryOptions {
    fn default() -> Self {
        Self {
            env_filter: None,
            with_ansi: true,
        }
    }
}

/// Initialise the global tracing subscriber.
///
/// Safe to call multiple times; only the first invocation installs the subscriber.
pub fn init_telemetry(options: TelemetryOptions) -> Result<(), DeepResearchError> {
    if TELEMETRY_GUARD.get().is_some() {
        return Ok(());
    }

    let env_filter = options
        .env_filter
        .or_else(|| std::env::var("RUST_LOG").ok())
        .unwrap_or_else(|| "info".to_string());

    fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new(env_filter))
        .with_ansi(options.with_ansi)
        .try_init()
        .map_err(|err| {
            DeepResearchError::InvalidConfiguration(format!("telemetry init failed: {err}"))
        })?;

    TELEMETRY_GUARD.get_or_init(|| ());
    Ok(())
}
