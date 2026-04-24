use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use infrastructure::recipes::SqlxRecipeRepository;
use serde_json::{Value, json};
use tower::ServiceExt;
use web::{AppState, build_app};

async fn fresh_app() -> Router {
    let repo = SqlxRecipeRepository::connect("sqlite::memory:")
        .await
        .expect("connect sqlite::memory:");
    build_app(AppState {
        repo: Arc::new(repo),
    })
}

async fn body_json(body: Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn post_json(payload: &Value) -> Request<Body> {
    Request::post("/recipes")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap()
}

#[tokio::test]
async fn list_returns_empty_on_fresh_app() {
    let response = fresh_app()
        .await
        .oneshot(Request::get("/recipes").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response.into_body()).await, json!([]));
}

#[tokio::test]
async fn create_then_list_round_trip() {
    let app = fresh_app().await;

    let payload = json!({
        "name": "Toast",
        "ingredients": [
            { "name": "bread", "amount": 50.0, "unit": "g" }
        ]
    });
    let create = app.clone().oneshot(post_json(&payload)).await.unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let created = body_json(create.into_body()).await;
    assert_eq!(created["name"], "Toast");
    assert!(
        created["id"].as_str().is_some_and(|s| !s.is_empty()),
        "expected server-generated id, got {created:?}"
    );
    assert_eq!(created["ingredients"][0]["ingredient"]["name"], "bread");
    assert_eq!(created["ingredients"][0]["quantity"]["unit"], "kilogram");
    assert!(
        (created["ingredients"][0]["quantity"]["amount"]
            .as_f64()
            .unwrap()
            - 0.05)
            .abs()
            < 1e-9
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
    assert_eq!(arr[0]["ingredients"][0]["ingredient"]["name"], "bread");
}

#[tokio::test]
async fn shared_ingredient_name_returns_same_id() {
    let app = fresh_app().await;

    let toast = app
        .clone()
        .oneshot(post_json(&json!({
            "name": "Toast",
            "ingredients": [{ "name": "bread", "amount": 50.0, "unit": "g" }],
        })))
        .await
        .unwrap();
    let toast_body = body_json(toast.into_body()).await;
    let bread_id_in_toast = &toast_body["ingredients"][0]["ingredient"]["id"];

    let sandwich = app
        .clone()
        .oneshot(post_json(&json!({
            "name": "Sandwich",
            "ingredients": [
                { "name": "bread", "amount": 100.0, "unit": "g" },
                { "name": "cheese", "amount": 30.0, "unit": "g" },
            ],
        })))
        .await
        .unwrap();
    let sandwich_body = body_json(sandwich.into_body()).await;
    let bread_id_in_sandwich = &sandwich_body["ingredients"][0]["ingredient"]["id"];

    assert_eq!(bread_id_in_toast, bread_id_in_sandwich);
}

#[tokio::test]
async fn cooking_units_normalize_to_canonical() {
    let app = fresh_app().await;

    let res = app
        .oneshot(post_json(&json!({
            "name": "Vinaigrette",
            "ingredients": [
                { "name": "oil",     "amount": 2.0, "unit": "tbsp" },
                { "name": "vinegar", "amount": 1.0, "unit": "tsp"  },
            ],
        })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = body_json(res.into_body()).await;
    let oil = &body["ingredients"][0]["quantity"];
    let vinegar = &body["ingredients"][1]["quantity"];
    assert_eq!(oil["unit"], "liter");
    assert_eq!(vinegar["unit"], "liter");
    assert!((oil["amount"].as_f64().unwrap() - 0.030).abs() < 1e-9);
    assert!((vinegar["amount"].as_f64().unwrap() - 0.005).abs() < 1e-9);
}

#[tokio::test]
async fn create_with_malformed_json_returns_400() {
    let response = fresh_app()
        .await
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

#[tokio::test]
async fn create_with_unknown_unit_is_rejected() {
    let response = fresh_app()
        .await
        .oneshot(post_json(&json!({
            "name": "Mystery",
            "ingredients": [{ "name": "x", "amount": 1.0, "unit": "ounce" }],
        })))
        .await
        .unwrap();

    // axum returns 422 for JSON-syntax-valid bodies that fail enum
    // deserialization (vs 400 for outright malformed JSON above).
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
