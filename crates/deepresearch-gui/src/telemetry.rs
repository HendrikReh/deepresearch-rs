use crate::config::AppConfig;
use anyhow::Result;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

pub fn init_tracing(config: &AppConfig) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer();
    let subscriber = Registry::default().with(env_filter).with(fmt_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    if let Some(endpoint) = config.otel_endpoint.as_deref() {
        tracing::warn!(target: "telemetry", endpoint, "GUI_OTEL_ENDPOINT configured, but OpenTelemetry export requires enabling the 'otel' feature in deepresearch-gui");
    }

    Ok(())
}
