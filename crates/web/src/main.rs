use axum::{Json, Router, extract::Request, http::HeaderName, routing::get};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::info_span;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = infrastructure::telemetry::init("meal-planner-web")?;

    // Layer order in axum: last `.layer(…)` call becomes the outermost
    // (runs first on requests, last on responses). We want:
    //   request : SetRequestId → TraceLayer → PropagateRequestId → handler
    //   response: handler → PropagateRequestId → TraceLayer → SetRequestId
    // so add them bottom-up.
    let traced = Router::new()
        .route("/", get(index))
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID))
        .layer(TraceLayer::new_for_http().make_span_with(make_request_span))
        .layer(SetRequestIdLayer::new(X_REQUEST_ID, MakeRequestUuid));
    let health = Router::new().route("/healthz", get(healthz));
    let app = traced.merge(health);

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

async fn index() -> &'static str {
    "meal planner — v0.0.1"
}

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

fn make_request_span(req: &Request) -> tracing::Span {
    let request_id = req
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    info_span!(
        "request",
        method = %req.method(),
        uri = %req.uri(),
        version = ?req.version(),
        request_id = %request_id,
    )
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
