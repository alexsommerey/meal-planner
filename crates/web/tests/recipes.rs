use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use infrastructure::recipes::InMemoryRecipeRepository;
use serde_json::{Value, json};
use tower::ServiceExt;
use web::{AppState, build_app};

fn fresh_app() -> Router {
    build_app(AppState {
        repo: Arc::new(InMemoryRecipeRepository::new()),
    })
}

async fn body_json(body: Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn list_returns_empty_on_fresh_app() {
    let response = fresh_app()
        .oneshot(Request::get("/recipes").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response.into_body()).await, json!([]));
}

#[tokio::test]
async fn create_then_list_round_trip() {
    let app = fresh_app();

    let payload = json!({
        "name": "Toast",
        "ingredients": [
            { "ingredient": "bread", "quantity": { "grams": 50.0 } }
        ]
    });
    let create = app
        .clone()
        .oneshot(
            Request::post("/recipes")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let created = body_json(create.into_body()).await;
    assert_eq!(created["name"], "Toast");
    assert!(
        created["id"].as_str().is_some_and(|s| !s.is_empty()),
        "expected server-generated id, got {created:?}"
    );

    let list = app
        .oneshot(Request::get("/recipes").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);

    let listed = body_json(list.into_body()).await;
    let arr = listed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], created["id"]);
    assert_eq!(arr[0]["name"], "Toast");
}

#[tokio::test]
async fn create_with_malformed_json_returns_400() {
    let response = fresh_app()
        .oneshot(
            Request::post("/recipes")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{ not json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
