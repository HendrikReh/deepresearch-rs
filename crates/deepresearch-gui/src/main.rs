use anyhow::Result;
use axum::Router;
use deepresearch_gui::{config, routes, state::AppState, telemetry::init_tracing};
use tokio::net::TcpListener;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::from_env()?;
    init_tracing(&config)?;

    let state = AppState::try_new(&config).await?;

    let app: Router = routes::build_router(state);

    let listener = TcpListener::bind(&config.listen_addr).await?;
    info!(address = %config.listen_addr, "deepresearch-gui listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|err| {
            error!(error = %err, "server shutdown with error");
            err
        })?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut stream) = signal(SignalKind::terminate()) {
            stream.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    info!("shutdown signal received");
}
