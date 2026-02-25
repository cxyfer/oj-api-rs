use std::sync::Arc;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;

use crate::AppState;

pub mod handlers;
pub mod pages;

pub fn admin_router() -> Router<Arc<AppState>> {
    let public = Router::new()
        .route(
            "/admin/login",
            get(pages::login_page).post(handlers::login_submit),
        )
        .route("/admin/logout", post(handlers::logout));

    let protected = Router::new()
        .route("/admin/", get(pages::index))
        .route("/admin/problems", get(pages::problems_page))
        .route("/admin/tokens", get(pages::tokens_page))
        .route("/admin/crawlers", get(pages::crawlers_page))
        .route("/admin/embeddings", get(pages::embeddings_page))
        .route("/admin/api/problems", post(handlers::create_problem))
        .route(
            "/admin/api/problems/{source}",
            get(handlers::get_problems_list),
        )
        .route("/admin/api/tags/{source}", get(handlers::get_tags_list))
        .route(
            "/admin/api/problems/{source}/{id}",
            get(handlers::get_problem_detail)
                .put(handlers::update_problem)
                .delete(handlers::delete_problem),
        )
        .route(
            "/admin/api/tokens",
            get(handlers::list_tokens).post(handlers::create_token),
        )
        .route("/admin/api/tokens/{token}", delete(handlers::revoke_token))
        .route(
            "/admin/api/settings/token-auth",
            get(handlers::get_token_auth_setting).put(handlers::set_token_auth_setting),
        )
        .route(
            "/admin/api/crawlers/trigger",
            post(handlers::trigger_crawler),
        )
        .route("/admin/api/crawlers/status", get(handlers::crawler_status))
        .route(
            "/admin/api/crawlers/{job_id}/output",
            get(handlers::crawler_output),
        )
        .route(
            "/admin/api/embeddings/stats",
            get(handlers::embedding_stats),
        )
        .route(
            "/admin/api/embeddings/trigger",
            post(handlers::trigger_embedding),
        )
        .route(
            "/admin/api/embeddings/status",
            get(handlers::embedding_status),
        )
        .route(
            "/admin/api/embeddings/{job_id}/output",
            get(handlers::embedding_output),
        )
        .route(
            "/admin/api/embeddings/{job_id}/progress",
            get(handlers::embedding_progress),
        )
        .route_layer(middleware::from_fn(crate::auth::admin_auth));

    public.merge(protected)
}
