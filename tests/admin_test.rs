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

// -- Persistent per-source crawler schedules --
#[tokio::test]
async fn schedules_are_authenticated_and_default_to_disabled_utc() {
    let (app, _guard) = common::build_test_app();
    for (method, path) in [
        ("GET", "/admin/api/crawler-schedules"),
        ("PUT", "/admin/api/crawler-schedules"),
        ("POST", "/admin/api/crawler-schedules/preview"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let response = app
        .oneshot(admin_request("GET", "/admin/api/crawler-schedules"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(json["default_timezone"], "UTC");
    let rows = json["schedules"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().all(|r| r["schedule"]["enabled"] == false
        && r["effective_timezone"] == "UTC"
        && r["next_run_at"].is_null()));
}

#[tokio::test]
async fn schedule_update_persists_only_requested_source_and_rejects_bad_updates() {
    let (app, _guard) = common::build_test_app();
    let good = r#"{"source":"leetcode","schedule":{"enabled":true,"cron":"30 3 * * *","timezone":"Asia/Taipei","args":["--sync-problemset"]}}"#;
    let response = app
        .clone()
        .oneshot(admin_json_request(
            "PUT",
            "/admin/api/crawler-schedules",
            good,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let saved: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(saved["next_runs"].as_array().unwrap().len(), 5);
    assert_eq!(saved["effective_timezone"], "Asia/Taipei");
    for (field, value) in [
        ("cron", serde_json::json!("60 * * * *")),
        ("timezone", serde_json::json!("unknown")),
        ("args", serde_json::json!(["--unknown"])),
    ] {
        let mut bad: serde_json::Value = serde_json::from_str(good).unwrap();
        bad["schedule"][field] = value;
        let response = app
            .clone()
            .oneshot(admin_json_request(
                "PUT",
                "/admin/api/crawler-schedules",
                &bad.to_string(),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{field}");
    }
    let response = app
        .clone()
        .oneshot(admin_json_request(
            "PUT",
            "/admin/api/crawler-schedules",
            r#"{"source":"atcoder","schedule":{"enabled":false,"cron":"","args":[]}}"#,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .oneshot(admin_request("GET", "/admin/api/crawler-schedules"))
        .await
        .unwrap();
    let loaded: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let leetcode = &loaded["schedules"][0];
    assert_eq!(leetcode["schedule"], saved["schedule"]);
    assert_eq!(loaded["schedules"][1]["schedule"]["enabled"], false);
}

#[tokio::test]
async fn preview_validates_without_persisting_or_starting_jobs() {
    let (app, _guard, state) = common::build_test_app_with_state();
    let payload =
        r#"{"source":"leetcode","schedule":{"enabled":false,"cron":"0 * * * *","args":[]}}"#;
    let response = app
        .clone()
        .oneshot(admin_json_request(
            "POST",
            "/admin/api/crawler-schedules/preview",
            payload,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(json["next_runs"].as_array().unwrap().len(), 5);
    assert!(oj_api_rs::db::settings::get_setting(
        &state.rw_pool,
        oj_api_rs::crawler_schedule::SETTING_KEY
    )
    .is_none());
    assert!(state.crawler_jobs.lock().await.is_empty());
}
