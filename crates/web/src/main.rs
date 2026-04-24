use std::sync::Arc;

use infrastructure::recipes::InMemoryRecipeRepository;
use web::{AppState, build_app};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = infrastructure::telemetry::init("meal-planner-web")?;

    let state = AppState {
        repo: Arc::new(InMemoryRecipeRepository::new()),
    };
    let app = build_app(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Resolves on `Ctrl+C` or (on Unix) `SIGTERM` so axum stops accepting new
/// connections and drains in-flight requests before the telemetry `Guard`
/// drops and flushes buffered spans/metrics.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl+c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install sigterm handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
