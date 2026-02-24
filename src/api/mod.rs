use std::sync::Arc;

use axum::middleware;
use axum::routing::get;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use crate::AppState;

pub mod daily;
pub mod error;
pub mod problems;
pub mod resolve;
pub mod similar;
pub mod status;

pub fn public_router() -> Router<Arc<AppState>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route(
            "/api/v1/problems/{source}/{id}",
            get(problems::get_problem),
        )
        .route(
            "/api/v1/problems/{source}",
            get(problems::list_problems),
        )
        .route(
            "/api/v1/tags/{source}",
            get(problems::list_tags),
        )
        .route("/api/v1/resolve/{*query}", get(resolve::resolve))
        .route("/api/v1/daily", get(daily::get_daily))
        .route(
            "/api/v1/similar/{source}/{id}",
            get(similar::similar_by_problem),
        )
        .route("/api/v1/similar", get(similar::similar_by_text))
        .route("/status", get(status::get_status))
        .route_layer(middleware::from_fn(crate::auth::bearer_auth))
        .layer(cors)
}
