mod common;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use rusqlite::params;
use tower::ServiceExt;

#[tokio::test]
async fn status_endpoint_returns_200_with_version() {
    let (app, _guard) = common::build_test_app();

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
async fn problems_list_returns_empty_for_empty_db() {
    let (app, _guard) = common::build_test_app();

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
    let (app, _guard) = common::build_test_app();

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
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/problems/invalid_source/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "expected 400, got {}",
        response.status()
    );
}

#[tokio::test]
async fn tags_list_returns_empty_for_empty_db() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/problems/tags/leetcode")
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
async fn daily_endpoint_returns_compact_response_shape() {
    let (app, guard) = common::build_test_app();
    seed_daily_problem(
        guard.db_path(),
        "1",
        "two-sum",
        Some("Two Sum"),
        Some("兩數之和"),
        Some("English content"),
        Some("中文內容"),
        &["three-sum"],
    );
    seed_daily_problem(
        guard.db_path(),
        "15",
        "three-sum",
        Some("3Sum"),
        None,
        None,
        None,
        &[],
    );
    seed_daily_row(
        guard.db_path(),
        "2026-01-01",
        "leetcode.com",
        &["leetcode:1"],
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/daily?source=leetcode.com&date=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["date"], "2026-01-01");
    assert_eq!(json["source"], "leetcode.com");
    assert!(json.get("id").is_none());
    assert!(json.get("slug").is_none());
    assert!(json.get("title").is_none());

    let problem = &json["problems"][0];
    assert_eq!(problem["id"], "1");
    assert_eq!(problem["title"], "Two Sum");
    assert_eq!(problem["content"], "English content");
    assert_eq!(problem["link"], "https://leetcode.com/problems/two-sum/");
    assert_eq!(problem["similar_questions"][0]["slug"], "three-sum");
}

#[tokio::test]
async fn daily_endpoint_projects_cn_localization_and_aliases() {
    let (app, guard) = common::build_test_app();
    seed_daily_problem(
        guard.db_path(),
        "1",
        "two-sum",
        Some("Two Sum"),
        Some("兩數之和"),
        Some("English content"),
        Some("中文內容"),
        &[],
    );
    seed_daily_row(
        guard.db_path(),
        "2026-01-01",
        "leetcode.cn",
        &["leetcode:1"],
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/daily?domain=cn&source=leetcode.cn&date=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let problem = &json["problems"][0];
    assert_eq!(json["source"], "leetcode.cn");
    assert_eq!(problem["title"], "兩數之和");
    assert_eq!(problem["content"], "中文內容");
    assert_eq!(problem["link"], "https://leetcode.cn/problems/two-sum/");
}

#[tokio::test]
async fn daily_endpoint_resolves_similar_questions_from_problem_source() {
    let (app, guard) = common::build_test_app();
    seed_daily_problem_with_source(
        guard.db_path(),
        "atcoder",
        "abc001_a",
        "abc001-a",
        Some("ABC001 A"),
        None,
        None,
        None,
        &["abc002-a"],
    );
    seed_daily_problem_with_source(
        guard.db_path(),
        "atcoder",
        "abc002_a",
        "abc002-a",
        Some("ABC002 A"),
        None,
        None,
        None,
        &[],
    );
    seed_daily_problem_with_source(
        guard.db_path(),
        "leetcode",
        "999",
        "abc002-a",
        Some("Wrong Source"),
        None,
        None,
        None,
        &[],
    );
    seed_daily_row(
        guard.db_path(),
        "2026-01-01",
        "leetcode.com",
        &["atcoder:abc001_a"],
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/daily?source=leetcode.com&date=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let similar = &json["problems"][0]["similar_questions"][0];
    assert_eq!(similar["source"], "atcoder");
    assert_eq!(similar["slug"], "abc002-a");
}

#[tokio::test]
async fn daily_endpoint_rejects_conflicting_domain_and_source() {
    let (app, _guard) = common::build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/daily?domain=com&source=leetcode.cn")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

fn seed_daily_problem(
    db_path: &std::path::Path,
    id: &str,
    slug: &str,
    title: Option<&str>,
    title_cn: Option<&str>,
    content: Option<&str>,
    content_cn: Option<&str>,
    similar_questions: &[&str],
) {
    seed_daily_problem_with_source(
        db_path,
        "leetcode",
        id,
        slug,
        title,
        title_cn,
        content,
        content_cn,
        similar_questions,
    );
}

fn seed_daily_problem_with_source(
    db_path: &std::path::Path,
    source: &str,
    id: &str,
    slug: &str,
    title: Option<&str>,
    title_cn: Option<&str>,
    content: Option<&str>,
    content_cn: Option<&str>,
    similar_questions: &[&str],
) {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO problems (
            id, source, slug, title, title_cn, difficulty, ac_rate, tags, link,
            category, paid_only, content, content_cn, similar_questions
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, 'Easy', 50.0, '[]', ?6,
            'Algorithms', 0, ?7, ?8, ?9
         )",
        params![
            id,
            source,
            slug,
            title,
            title_cn,
            format!("https://example.com/problems/{slug}/"),
            content,
            content_cn,
            serde_json::to_string(similar_questions).unwrap()
        ],
    )
    .unwrap();
}

fn seed_daily_row(db_path: &std::path::Path, date: &str, source: &str, refs: &[&str]) {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO daily_challenge (date, source, problems) VALUES (?1, ?2, ?3)",
        params![date, source, serde_json::to_string(refs).unwrap()],
    )
    .unwrap();
}
