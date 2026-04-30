mod common;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn admin_request(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-admin-secret", "test-secret")
        .body(Body::empty())
        .unwrap()
}

fn admin_json_request(method: &str, uri: &str, json: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-admin-secret", "test-secret")
        .header("content-type", "application/json")
        .body(Body::from(json.to_string()))
        .unwrap()
}

// -- Token CRUD --

#[tokio::test]
async fn list_tokens_returns_empty_array_initially() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(admin_request("GET", "/admin/api/tokens"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let tokens: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(tokens.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn create_token_returns_token_string() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(admin_json_request(
            "POST",
            "/admin/api/tokens",
            r#"{"label":"test-token"}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let token: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(token.get("token").is_some());
    assert!(!token["token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn create_then_list_tokens_shows_token() {
    let (app, _guard) = common::build_test_app();

    // Create a token
    let response = app
        .clone()
        .oneshot(admin_json_request(
            "POST",
            "/admin/api/tokens",
            r#"{"label":"my-token"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // List tokens
    let response = app
        .oneshot(admin_request("GET", "/admin/api/tokens"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let tokens: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0]["label"], "my-token");
}

#[tokio::test]
async fn revoke_nonexistent_token_returns_404() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(admin_request("DELETE", "/admin/api/tokens/nonexistent"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// -- Admin auth --

#[tokio::test]
async fn admin_api_without_secret_returns_unauthorized() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin/api/tokens")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_api_with_wrong_secret_returns_unauthorized() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin/api/tokens")
                .header("x-admin-secret", "wrong-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// -- Settings --

#[tokio::test]
async fn get_token_auth_setting_returns_json() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(admin_request("GET", "/admin/api/settings/token-auth"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("enabled").is_some());
}

#[tokio::test]
async fn set_token_auth_setting_toggles_value() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(admin_json_request(
            "PUT",
            "/admin/api/settings/token-auth",
            r#"{"enabled":true}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["enabled"], true);
}

// -- Crawler status --

#[tokio::test]
async fn crawler_status_returns_initial_state() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(admin_request("GET", "/admin/api/crawlers/status"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["running"], false);
    assert!(json["running_jobs"].as_array().unwrap().is_empty());
}

// -- Embedding status --

#[tokio::test]
async fn embedding_status_returns_initial_state() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(admin_request("GET", "/admin/api/embeddings/status"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["running"], false);
}
