use crate::{config::AppConfig, metrics};
use anyhow::Result;
use tracing::warn;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

pub fn init_tracing(config: &AppConfig) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer();
    let subscriber = Registry::default().with(env_filter).with(fmt_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    if let Some(endpoint) = config.otel_endpoint.as_deref() {
        metrics::init_telemetry(endpoint)?;
        warn!(
            target = "telemetry.gui",
            endpoint,
            "GUI_OTEL_ENDPOINT set; attach an OTLP subscriber (e.g. OpenTelemetry collector) to forward tracing spans"
        );
    }

    Ok(())
}
