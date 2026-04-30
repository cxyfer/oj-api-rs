mod common;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn status_endpoint_returns_200_with_version() {
    let app = common::build_test_app();

    // Status is behind bearer auth, but token_auth is disabled in test config
    let response = app
        .oneshot(
            Request::builder()
                .uri("/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("version").is_some());
    assert!(json.get("platforms").is_some());
}

#[tokio::test]
async fn problems_list_returns_empty_for_unknown_source() {
    let app = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/problems/leetcode?page=1&per_page=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("data").is_some());
    assert!(json["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn problem_detail_returns_404_for_missing_problem() {
    let app = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/problems/leetcode/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_source_returns_error() {
    let app = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/problems/invalid_source/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 or 422 for invalid source
    assert!(
        response.status().is_client_error(),
        "expected 4xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn tags_list_returns_empty_for_empty_db() {
    let app = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tags/leetcode")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn daily_endpoint_responds() {
    let app = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/daily")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // May return 200 (cached) or 202 (triggering background fetch)
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::ACCEPTED
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "expected 200, 202, or 503, got {}",
        response.status()
    );
}
