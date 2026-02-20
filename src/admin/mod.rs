use std::sync::Arc;

use axum::middleware;
use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::AppState;

pub mod handlers;
pub mod pages;

pub fn admin_router() -> Router<Arc<AppState>> {
    Router::new()
        // HTML pages
        .route("/admin/", get(pages::index))
        .route("/admin/problems", get(pages::problems_page))
        .route("/admin/tokens", get(pages::tokens_page))
        // API endpoints
        .route("/admin/api/problems", post(handlers::create_problem))
        .route(
            "/admin/api/problems/{source}/{id}",
            put(handlers::update_problem).delete(handlers::delete_problem),
        )
        .route("/admin/api/tokens", get(handlers::list_tokens).post(handlers::create_token))
        .route(
            "/admin/api/tokens/{token}",
            delete(handlers::revoke_token),
        )
        .route(
            "/admin/api/crawlers/trigger",
            post(handlers::trigger_crawler),
        )
        .route(
            "/admin/api/crawlers/status",
            get(handlers::crawler_status),
        )
        .route_layer(middleware::from_fn(crate::auth::admin_auth))
}
