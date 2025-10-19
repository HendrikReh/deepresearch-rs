use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer());

    let _ = tracing::subscriber::set_global_default(subscriber);
}
