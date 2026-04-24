use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderName, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use domain::{Recipe, RecipeIngredient};
use infrastructure::recipes::InMemoryRecipeRepository;
use serde::Deserialize;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::info_span;

pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<InMemoryRecipeRepository>,
}

pub fn build_app(state: AppState) -> Router {
    // Layer order in axum: last `.layer(…)` call becomes the outermost
    // (runs first on requests, last on responses). We want:
    //   request : SetRequestId → TraceLayer → PropagateRequestId → handler
    //   response: handler → PropagateRequestId → TraceLayer → SetRequestId
    // so add them bottom-up.
    let traced = Router::new()
        .route("/", get(index))
        .route("/recipes", get(list_recipes).post(create_recipe))
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID))
        .layer(TraceLayer::new_for_http().make_span_with(make_request_span))
        .layer(SetRequestIdLayer::new(X_REQUEST_ID, MakeRequestUuid))
        .with_state(state);
    let health = Router::new().route("/healthz", get(healthz));
    traced.merge(health)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(Deserialize)]
struct NewRecipe {
    name: String,
    ingredients: Vec<RecipeIngredient>,
}

async fn create_recipe(
    State(state): State<AppState>,
    Json(payload): Json<NewRecipe>,
) -> Result<(StatusCode, Json<Recipe>), AppError> {
    let recipe =
        application::recipes::create_recipe(&*state.repo, payload.name, payload.ingredients)
            .await?;
    Ok((StatusCode::CREATED, Json(recipe)))
}

async fn list_recipes(State(state): State<AppState>) -> Result<Json<Vec<Recipe>>, AppError> {
    let recipes = application::recipes::list_recipes(&*state.repo).await?;
    Ok(Json(recipes))
}

struct AppError(application::recipes::RepoError);

impl From<application::recipes::RepoError> for AppError {
    fn from(e: application::recipes::RepoError) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
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
