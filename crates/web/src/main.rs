use axum::{Router, routing::get};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = infrastructure::telemetry::init("meal-planner-web")?;

    let app = Router::new().route("/", get(index)).layer(
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().level(Level::INFO)),
    );

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> &'static str {
    "meal planner — v0.0.1"
}
