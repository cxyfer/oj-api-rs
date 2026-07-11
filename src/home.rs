use std::sync::atomic::Ordering;
use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::AppState;

pub(crate) const DOCS_PATH: &str = "/docs";
pub(crate) const API_DOCS_PATH: &str = "/docs/api";
pub(crate) const MCP_DOCS_PATH: &str = "/docs/mcp";

#[derive(Clone, Copy)]
pub(crate) struct DocsRegistry {
    pub(crate) supported_sources: &'static [&'static str],
    pub(crate) homepage_cards: &'static [HomepageCard],
    #[allow(dead_code)]
    pub(crate) http_route_cards: &'static [HttpRouteCard],
    pub(crate) mcp_transport_cards: &'static [McpTransportCard],
    pub(crate) mcp_tool_cards: &'static [McpToolCard],
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) struct HomepageCard {
    pub(crate) title: &'static str,
    pub(crate) method: &'static str,
    pub(crate) path: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) docs_href: &'static str,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) struct HttpRouteCard {
    pub(crate) group: &'static str,
    pub(crate) fragment_id: &'static str,
    pub(crate) method: &'static str,
    pub(crate) path: &'static str,
    pub(crate) title: &'static str,
    pub(crate) title_i18n: &'static str,
    pub(crate) purpose: &'static str,
    pub(crate) purpose_i18n: &'static str,
    pub(crate) auth_rule: &'static str,
    pub(crate) auth_rule_i18n: &'static str,
    pub(crate) inputs: &'static str,
    pub(crate) inputs_i18n: &'static str,
    pub(crate) success_shape: &'static str,
    pub(crate) success_shape_i18n: &'static str,
    pub(crate) example: &'static str,
    pub(crate) has_source_param: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct McpTransportCard {
    pub(crate) fragment_id: &'static str,
    pub(crate) method: &'static str,
    pub(crate) path: &'static str,
    pub(crate) title: &'static str,
    pub(crate) title_i18n: &'static str,
    pub(crate) responsibility: &'static str,
    pub(crate) responsibility_i18n: &'static str,
    pub(crate) auth_rule: &'static str,
    pub(crate) auth_rule_i18n: &'static str,
    pub(crate) example: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct McpToolCard {
    pub(crate) fragment_id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) title: &'static str,
    pub(crate) title_i18n: &'static str,
    pub(crate) required_inputs: &'static str,
    pub(crate) required_inputs_i18n: &'static str,
    pub(crate) rest_capability: &'static str,
    pub(crate) rest_capability_i18n: &'static str,
    pub(crate) output_style: &'static str,
    pub(crate) output_style_i18n: &'static str,
    pub(crate) usage_note: &'static str,
    pub(crate) usage_note_i18n: &'static str,
}

const SUPPORTED_PUBLIC_SOURCES: &[&str] = crate::api::problems::VALID_SOURCES;

const HOMEPAGE_CARDS: [HomepageCard; 3] = [
    HomepageCard {
        title: "Problem detail",
        method: "GET",
        path: "/api/v1/problems/{source}/{id}",
        summary: "Fetch a normalized problem record by platform source and problem ID.",
        docs_href: "/docs#tag/problems",
    },
    HomepageCard {
        title: "Daily challenge",
        method: "GET",
        path: "/api/v1/daily",
        summary: "Fetch the current LeetCode daily challenge with the existing fallback behavior.",
        docs_href: "/docs#tag/daily",
    },
    HomepageCard {
        title: "Similar search",
        method: "POST",
        path: "/api/v1/similar",
        summary: "Search related problems by free-text query against the public similarity index.",
        docs_href: "/docs#tag/similar",
    },
];

const HTTP_ROUTE_CARDS: [HttpRouteCard; 11] = [
    HttpRouteCard {
        group: "Problems",
        fragment_id: "problem-detail",
        method: "GET",
        path: "/api/v1/problems/{source}/{id}",
        title: "Problem detail",
        title_i18n: "docs_api.cards.problem_detail.title",
        purpose: "Return a single normalized problem document.",
        purpose_i18n: "docs_api.cards.problem_detail.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.problem_detail.auth_rule",
        inputs: "Path params: {source}, {id}.",
        inputs_i18n: "docs_api.cards.problem_detail.inputs",
        success_shape: "JSON object with the requested problem fields.",
        success_shape_i18n: "docs_api.cards.problem_detail.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' http://127.0.0.1:7856/api/v1/problems/leetcode/1",
        has_source_param: true,
    },
    HttpRouteCard {
        group: "Problems",
        fragment_id: "problem-list",
        method: "GET",
        path: "/api/v1/problems/{source}",
        title: "Problem list",
        title_i18n: "docs_api.cards.problem_list.title",
        purpose: "List problems for a source with pagination and filtering.",
        purpose_i18n: "docs_api.cards.problem_list.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.problem_list.auth_rule",
        inputs: "Path param: {source}. Query params include pagination and optional filters.",
        inputs_i18n: "docs_api.cards.problem_list.inputs",
        success_shape: "JSON object with data[] and meta pagination fields.",
        success_shape_i18n: "docs_api.cards.problem_list.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' 'http://127.0.0.1:7856/api/v1/problems/leetcode?page=1&per_page=20'",
        has_source_param: true,
    },
    HttpRouteCard {
        group: "Problems",
        fragment_id: "problem-tags",
        method: "GET",
        path: "/api/v1/problems/tags/{source}",
        title: "Tags",
        title_i18n: "docs_api.cards.problem_tags.title",
        purpose: "List tag metadata for a supported source.",
        purpose_i18n: "docs_api.cards.problem_tags.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.problem_tags.auth_rule",
        inputs: "Path param: {source}.",
        inputs_i18n: "docs_api.cards.problem_tags.inputs",
        success_shape: "JSON array of tag strings.",
        success_shape_i18n: "docs_api.cards.problem_tags.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' http://127.0.0.1:7856/api/v1/problems/tags/leetcode",
        has_source_param: true,
    },
    HttpRouteCard {
        group: "Problems",
        fragment_id: "problem-difficulties",
        method: "GET",
        path: "/api/v1/problems/difficulties/{source}",
        title: "Difficulties",
        title_i18n: "docs_api.cards.problem_difficulties.title",
        purpose: "List difficulty values for a supported source.",
        purpose_i18n: "docs_api.cards.problem_difficulties.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.problem_difficulties.auth_rule",
        inputs: "Path param: {source}.",
        inputs_i18n: "docs_api.cards.problem_difficulties.inputs",
        success_shape: "JSON array of difficulty strings.",
        success_shape_i18n: "docs_api.cards.problem_difficulties.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' http://127.0.0.1:7856/api/v1/problems/difficulties/leetcode",
        has_source_param: true,
    },
    HttpRouteCard {
        group: "Problems",
        fragment_id: "problem-batch",
        method: "POST",
        path: "/api/v1/problems/batch",
        title: "Batch fetch",
        title_i18n: "docs_api.cards.problem_batch.title",
        purpose: "Fetch multiple problems in a single request by source and ID pairs.",
        purpose_i18n: "docs_api.cards.problem_batch.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.problem_batch.auth_rule",
        inputs: "JSON body: array of {source, id} objects. Optional query param: ?detail=true for full content.",
        inputs_i18n: "docs_api.cards.problem_batch.inputs",
        success_shape: "JSON object with results[] and not_found[] arrays.",
        success_shape_i18n: "docs_api.cards.problem_batch.success_shape",
        example: r#"curl -X POST -H 'Authorization: Bearer <token>' -H 'Content-Type: application/json' -d '[{"source":"leetcode","id":"1"}]' http://127.0.0.1:7856/api/v1/problems/batch"#,
        has_source_param: false,
    },
    HttpRouteCard {
        group: "Discovery",
        fragment_id: "resolve-problem",
        method: "GET",
        path: "/api/v1/resolve/{*query}",
        title: "Resolve",
        title_i18n: "docs_api.cards.resolve_problem.title",
        purpose: "Resolve a URL, slug, or prefixed identifier into a concrete problem.",
        purpose_i18n: "docs_api.cards.resolve_problem.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.resolve_problem.auth_rule",
        inputs: "Wildcard path param: {query}.",
        inputs_i18n: "docs_api.cards.resolve_problem.inputs",
        success_shape: "JSON object with source, id, and problem fields.",
        success_shape_i18n: "docs_api.cards.resolve_problem.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' 'http://127.0.0.1:7856/api/v1/resolve/leetcode:1'",
        has_source_param: false,
    },
    HttpRouteCard {
        group: "Discovery",
        fragment_id: "daily-challenge",
        method: "GET",
        path: "/api/v1/daily",
        title: "Daily challenge",
        title_i18n: "docs_api.cards.daily_challenge.title",
        purpose: "Fetch the current daily challenge, optionally by date or domain.",
        purpose_i18n: "docs_api.cards.daily_challenge.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.daily_challenge.auth_rule",
        inputs: "Query params include domain, date, and async flags.",
        inputs_i18n: "docs_api.cards.daily_challenge.inputs",
        success_shape: "JSON challenge payload or accepted/fetching response.",
        success_shape_i18n: "docs_api.cards.daily_challenge.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' 'http://127.0.0.1:7856/api/v1/daily?domain=com'",
        has_source_param: false,
    },
    HttpRouteCard {
        group: "Discovery",
        fragment_id: "similar-by-id",
        method: "GET",
        path: "/api/v1/similar/{source}/{id}",
        title: "Similar by ID",
        title_i18n: "docs_api.cards.similar_by_id.title",
        purpose: "Find similar problems using an existing problem as the seed.",
        purpose_i18n: "docs_api.cards.similar_by_id.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.similar_by_id.auth_rule",
        inputs: "Path params: {source}, {id}. Optional query params refine the search.",
        inputs_i18n: "docs_api.cards.similar_by_id.inputs",
        success_shape: "JSON object with ranked similar-problem results.",
        success_shape_i18n: "docs_api.cards.similar_by_id.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' http://127.0.0.1:7856/api/v1/similar/leetcode/1",
        has_source_param: true,
    },
    HttpRouteCard {
        group: "Discovery",
        fragment_id: "similar-search",
        method: "POST",
        path: "/api/v1/similar",
        title: "Similar by query",
        title_i18n: "docs_api.cards.similar_by_query.title",
        purpose: "Find similar problems from a free-text query.",
        purpose_i18n: "docs_api.cards.similar_by_query.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.similar_by_query.auth_rule",
        inputs: "JSON body with fields: query, limit, threshold, source.",
        inputs_i18n: "docs_api.cards.similar_by_query.inputs",
        success_shape: "JSON object with ranked similar-problem results.",
        success_shape_i18n: "docs_api.cards.similar_by_query.success_shape",
        example: "curl -X POST -H 'Authorization: Bearer <token>' -H 'Content-Type: application/json' -d '{\"query\":\"graph shortest path\",\"limit\":5}' http://127.0.0.1:7856/api/v1/similar",
        has_source_param: false,
    },
    HttpRouteCard {
        group: "Service",
        fragment_id: "service-status",
        method: "GET",
        path: "/status",
        title: "Status",
        title_i18n: "docs_api.cards.service_status.title",
        purpose: "Return service version and per-platform indexing stats.",
        purpose_i18n: "docs_api.cards.service_status.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.service_status.auth_rule",
        inputs: "No input parameters.",
        inputs_i18n: "docs_api.cards.service_status.inputs",
        success_shape: "JSON object with version and platforms[].",
        success_shape_i18n: "docs_api.cards.service_status.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' http://127.0.0.1:7856/status",
        has_source_param: false,
    },
    HttpRouteCard {
        group: "Service",
        fragment_id: "service-health",
        method: "GET",
        path: "/health",
        title: "Health",
        title_i18n: "docs_api.cards.service_health.title",
        purpose: "Run the public health check against the database and sqlite-vec.",
        purpose_i18n: "docs_api.cards.service_health.purpose",
        auth_rule: "Public without bearer auth.",
        auth_rule_i18n: "docs_api.cards.service_health.auth_rule",
        inputs: "No input parameters.",
        inputs_i18n: "docs_api.cards.service_health.inputs",
        success_shape: "JSON object with status, db, sqlite_vec, vec_dimension, and version fields.",
        success_shape_i18n: "docs_api.cards.service_health.success_shape",
        example: "curl http://127.0.0.1:7856/health",
        has_source_param: false,
    },
];

const MCP_TRANSPORT_CARDS: [McpTransportCard; 2] = [
    McpTransportCard {
        fragment_id: "mcp-post",
        method: "POST",
        path: "/mcp",
        title: "Streamable HTTP entrypoint",
        title_i18n: "docs_mcp.transport_cards.mcp_post.title",
        responsibility: "Primary Streamable HTTP endpoint for initialize and tool-call requests.",
        responsibility_i18n: "docs_mcp.transport_cards.mcp_post.responsibility",
        auth_rule: "Bearer auth follows the existing MCP middleware setting.",
        auth_rule_i18n: "docs_mcp.transport_cards.mcp_post.auth_rule",
        example: "curl -H 'Authorization: Bearer <token>' -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}' http://127.0.0.1:7856/mcp",
    },
    McpTransportCard {
        fragment_id: "mcp-get",
        method: "GET",
        path: "/mcp",
        title: "SSE session stream",
        title_i18n: "docs_mcp.transport_cards.mcp_get.title",
        responsibility: "SSE/session-resume transport surface exposed by the mounted MCP service.",
        responsibility_i18n: "docs_mcp.transport_cards.mcp_get.responsibility",
        auth_rule: "Bearer auth follows the existing MCP middleware setting.",
        auth_rule_i18n: "docs_mcp.transport_cards.mcp_get.auth_rule",
        example: "curl -N -H 'Authorization: Bearer <token>' http://127.0.0.1:7856/mcp",
    },
];

const MCP_TOOL_CARDS: [McpToolCard; 5] = [
    McpToolCard {
        fragment_id: "tool-resolve-problem",
        name: "resolve_problem",
        title: "Resolve a problem from flexible input",
        title_i18n: "docs_mcp.tool_cards.resolve_problem.title",
        required_inputs: "query",
        required_inputs_i18n: "docs_mcp.tool_cards.resolve_problem.required_inputs",
        rest_capability: "Maps to GET /api/v1/resolve/{*query}.",
        rest_capability_i18n: "docs_mcp.tool_cards.resolve_problem.rest_capability",
        output_style: "Text summary of the resolved problem detail.",
        output_style_i18n: "docs_mcp.tool_cards.resolve_problem.output_style",
        usage_note: "Use when the source/ID format is uncertain or when the input is a URL.",
        usage_note_i18n: "docs_mcp.tool_cards.resolve_problem.usage_note",
    },
    McpToolCard {
        fragment_id: "tool-get-problem",
        name: "get_problem",
        title: "Fetch one normalized problem",
        title_i18n: "docs_mcp.tool_cards.get_problem.title",
        required_inputs: "source, id",
        required_inputs_i18n: "docs_mcp.tool_cards.get_problem.required_inputs",
        rest_capability: "Maps to GET /api/v1/problems/{source}/{id}.",
        rest_capability_i18n: "docs_mcp.tool_cards.get_problem.rest_capability",
        output_style: "Text summary of the requested problem detail.",
        output_style_i18n: "docs_mcp.tool_cards.get_problem.output_style",
        usage_note: "Use when the caller already knows the normalized source and problem ID.",
        usage_note_i18n: "docs_mcp.tool_cards.get_problem.usage_note",
    },
    McpToolCard {
        fragment_id: "tool-get-daily-challenge",
        name: "get_daily_challenge",
        title: "Read the current daily challenge",
        title_i18n: "docs_mcp.tool_cards.get_daily_challenge.title",
        required_inputs: "domain?, date?",
        required_inputs_i18n: "docs_mcp.tool_cards.get_daily_challenge.required_inputs",
        rest_capability: "Maps to GET /api/v1/daily.",
        rest_capability_i18n: "docs_mcp.tool_cards.get_daily_challenge.rest_capability",
        output_style: "Text summary of the daily challenge or the active fetch state.",
        output_style_i18n: "docs_mcp.tool_cards.get_daily_challenge.output_style",
        usage_note: "Use when the caller wants the current daily challenge without composing REST query strings.",
        usage_note_i18n: "docs_mcp.tool_cards.get_daily_challenge.usage_note",
    },
    McpToolCard {
        fragment_id: "tool-find-similar-problems",
        name: "find_similar_problems",
        title: "Search similar problems",
        title_i18n: "docs_mcp.tool_cards.find_similar_problems.title",
        required_inputs: "query or source+id, plus limit/threshold/source_filter?",
        required_inputs_i18n: "docs_mcp.tool_cards.find_similar_problems.required_inputs",
        rest_capability: "Maps to POST /api/v1/similar and GET /api/v1/similar/{source}/{id}.",
        rest_capability_i18n: "docs_mcp.tool_cards.find_similar_problems.rest_capability",
        output_style: "Text ranking of similar problems with similarity scores.",
        output_style_i18n: "docs_mcp.tool_cards.find_similar_problems.output_style",
        usage_note: "Use text query mode for semantic search or source+id mode for seed-problem lookup.",
        usage_note_i18n: "docs_mcp.tool_cards.find_similar_problems.usage_note",
    },
    McpToolCard {
        fragment_id: "tool-get-platform-status",
        name: "get_platform_status",
        title: "Check indexed platform status",
        title_i18n: "docs_mcp.tool_cards.get_platform_status.title",
        required_inputs: "none",
        required_inputs_i18n: "docs_mcp.tool_cards.get_platform_status.required_inputs",
        rest_capability: "Maps to GET /status.",
        rest_capability_i18n: "docs_mcp.tool_cards.get_platform_status.rest_capability",
        output_style: "Text summary of service version and platform stats.",
        output_style_i18n: "docs_mcp.tool_cards.get_platform_status.output_style",
        usage_note: "Use for a compact operational overview of available indexed platforms.",
        usage_note_i18n: "docs_mcp.tool_cards.get_platform_status.usage_note",
    },
];

pub(crate) fn docs_registry() -> DocsRegistry {
    DocsRegistry {
        supported_sources: SUPPORTED_PUBLIC_SOURCES,
        homepage_cards: &HOMEPAGE_CARDS,
        http_route_cards: &HTTP_ROUTE_CARDS,
        mcp_transport_cards: &MCP_TRANSPORT_CARDS,
        mcp_tool_cards: &MCP_TOOL_CARDS,
    }
}

pub fn public_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index))
        .route(API_DOCS_PATH, get(api_docs))
        .route(MCP_DOCS_PATH, get(mcp_docs))
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    total_problems: u64,
    token_auth_enabled: bool,
    version: &'static str,
    docs: DocsRegistry,
    docs_path: &'static str,
    mcp_docs_path: &'static str,
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "docs_api.html")]
struct ApiDocsTemplate {
    docs: DocsRegistry,
    version: &'static str,
    home_path: &'static str,
    mcp_docs_path: &'static str,
    token_auth_enabled: bool,
}

#[derive(Template)]
#[template(path = "docs_mcp.html")]
struct McpDocsTemplate {
    docs: DocsRegistry,
    version: &'static str,
    home_path: &'static str,
    docs_path: &'static str,
    token_auth_enabled: bool,
}

pub async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.ro_pool.clone();
    let total_problems = tokio::task::spawn_blocking(move || {
        let conn = pool.get().ok()?;
        conn.query_row("SELECT COUNT(*) FROM problems", [], |row| {
            row.get::<_, u64>(0)
        })
        .ok()
    })
    .await
    .unwrap_or(None)
    .unwrap_or(0);

    let token_auth_enabled = state.token_auth_enabled.load(Ordering::Acquire);

    HomeTemplate {
        total_problems,
        token_auth_enabled,
        version: env!("CARGO_PKG_VERSION"),
        docs: docs_registry(),
        docs_path: DOCS_PATH,
        mcp_docs_path: MCP_DOCS_PATH,
    }
    .render()
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    .into_response()
}

pub async fn api_docs() -> impl IntoResponse {
    axum::response::Redirect::permanent("/docs")
}

pub async fn mcp_docs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let token_auth_enabled = state.token_auth_enabled.load(Ordering::Acquire);

    McpDocsTemplate {
        docs: docs_registry(),
        version: env!("CARGO_PKG_VERSION"),
        home_path: "/",
        docs_path: DOCS_PATH,
        token_auth_enabled,
    }
    .render()
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::collections::{HashMap, VecDeque};
    use std::sync::atomic::AtomicBool;

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::{Extension, Router};
    use tokio::sync::{RwLock, Semaphore};
    use tower::ServiceExt;

    use crate::auth::{AuthRwPool, TokenAuthEnabled};
    use crate::config::Config;
    use crate::db;

    fn test_state(token_auth_enabled: bool) -> Arc<AppState> {
        let config = Config::default();
        let db_path = std::env::temp_dir().join(format!(
            "oj-api-home-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos()
        ));
        let db_path_str = db_path.to_string_lossy().into_owned();
        db::register_sqlite_vec();
        let ro_pool = db::create_ro_pool(&db_path_str, 1, config.database.busy_timeout_ms);
        let rw_pool = db::create_rw_pool(&db_path_str, 1, config.database.busy_timeout_ms);
        db::ensure_data_tables(&rw_pool);
        db::ensure_api_tokens_table(&rw_pool);
        db::ensure_app_settings_table(&rw_pool);

        Arc::new(AppState {
            ro_pool,
            rw_pool,
            config,
            crawler_jobs: tokio::sync::Mutex::new(HashMap::new()),
            manual_crawler_guard: tokio::sync::Mutex::new(None),
            crawler_history: tokio::sync::Mutex::new(VecDeque::new()),
            embedding_lock: tokio::sync::Mutex::new(None),
            embedding_launch_guard: tokio::sync::Mutex::new(None),
            embedding_history: tokio::sync::Mutex::new(VecDeque::new()),
            active_crawler_pids: tokio::sync::Mutex::new(HashMap::new()),
            active_embedding_pid: tokio::sync::Mutex::new(None),
            daily_fallback: tokio::sync::Mutex::new(HashMap::new()),
            retained_refresh: tokio::sync::Mutex::new(crate::utils::RetainedRefreshState::default()),
            embed_semaphore: Semaphore::new(1),
            token_auth_enabled: Arc::new(AtomicBool::new(token_auth_enabled)),
            admin_sessions: Arc::new(RwLock::new(HashMap::new())),
            config_path: None,
        })
    }

    #[test]
    fn renders_algorithmic_observatory_homepage() {
        let html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            docs_path: DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        assert!(html.contains("OJ API"));
        assert!(html.contains("Every judge. One problem space."));
        assert!(html.contains("Problem Intelligence Infrastructure"));
        assert!(html.contains("scene-shell"));
        assert!(html.contains("scene-source-data"));
        assert!(html.contains("data-source=\"leetcode\""));
        assert!(html.contains("Token auth enabled"));
        assert!(html.contains("/api/v1/problems/{source}/{id}"));
        assert!(html.contains("/api/v1/daily"));
        assert!(html.contains("/api/v1/similar"));
        assert!(html.contains("/docs"));
        assert!(html.contains("/docs#tag/problems"));
        assert!(html.contains("/docs#tag/daily"));
        assert!(html.contains("/docs#tag/similar"));
        assert!(html.contains("/docs/mcp"));
        assert!(html.contains("/api/v1/problems/leetcode/1"));
        for route in ["/", "/health", "/api/v1/*", "/status", "/mcp"] {
            assert!(html.contains(route));
        }
        assert!(!html.contains("bento-grid"));
        assert!(!html.contains("Playfair Display"));
        assert!(!html.contains("Dashboard"));
    }

    #[test]
    fn renders_public_homepage_with_public_api_badge_when_auth_disabled() {
        let html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: false,
            version: "0.3.2-test",
            docs: docs_registry(),
            docs_path: DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        assert!(html.contains("Token auth disabled"));
        assert!(html.contains("/health"));
        assert!(html.contains("/status"));
        assert!(html.contains("/mcp"));
        assert!(html.contains("/api/v1/problems/leetcode/1"));
        assert!(html.contains("/static/home.css"));
    }

    #[tokio::test]
    async fn root_route_returns_public_homepage_without_admin_navigation() {
        let state = test_state(true);
        let app = Router::new().merge(public_router()).with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let html = String::from_utf8(body.to_vec()).expect("homepage should be utf-8");
        assert!(html.contains("Every judge. One problem space."));
        assert!(html.contains("One switch. Explicit access rules."));
        assert!(html.contains("Explore the API"));
        assert!(html.contains("MCP for Agents"));
        assert!(!html.contains("Dashboard"));
        assert!(!html.contains("/admin/"));
    }

    #[tokio::test]
    async fn homepage_is_public_while_status_route_remains_protected() {
        let state = test_state(true);
        let token_auth = state.token_auth_enabled.clone();
        let rw_pool = state.rw_pool.clone();

        let app = Router::new()
            .merge(public_router())
            .merge(crate::api::public_router())
            .layer(Extension(AuthRwPool(Arc::new(rw_pool))))
            .layer(Extension(TokenAuthEnabled(token_auth)))
            .with_state(state);

        let home_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(home_response.status(), StatusCode::OK);

        let status_response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(status_response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn docs_registry_matches_phase_one_scope() {
        let docs = docs_registry();

        assert_eq!(docs.homepage_cards.len(), 3);
        assert_eq!(docs.http_route_cards.len(), 11);
        assert_eq!(docs.mcp_transport_cards.len(), 2);
        assert_eq!(docs.mcp_tool_cards.len(), 5);

        let homepage_paths: Vec<_> = docs.homepage_cards.iter().map(|card| card.path).collect();
        assert_eq!(
            homepage_paths,
            vec![
                "/api/v1/problems/{source}/{id}",
                "/api/v1/daily",
                "/api/v1/similar",
            ]
        );

        let supported_sources: BTreeSet<_> = docs.supported_sources.iter().copied().collect();
        let api_sources: BTreeSet<_> = crate::api::problems::VALID_SOURCES
            .iter()
            .copied()
            .collect();
        assert_eq!(supported_sources, api_sources);

        let mcp_tools: Vec<_> = docs.mcp_tool_cards.iter().map(|card| card.name).collect();
        assert_eq!(
            mcp_tools,
            vec![
                "resolve_problem",
                "get_problem",
                "get_daily_challenge",
                "find_similar_problems",
                "get_platform_status",
            ]
        );
    }

    #[test]
    fn docs_base_rewrites_example_origin_at_runtime() {
        let base_template = include_str!("../templates/docs_base.html");
        assert!(base_template.contains("window.location.origin"));
        assert!(base_template.contains("http://127.0.0.1:7856"));
    }

    #[test]
    fn public_pages_load_only_their_owned_assets() {
        let home_html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            docs_path: DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        assert!(home_html.contains("/static/site.css"));
        assert!(home_html.contains("/static/home.css"));
        assert!(home_html.contains("/static/home-scene.js"));

        let mcp_html = McpDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            docs_path: DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("mcp docs template should render");

        assert!(mcp_html.contains("/static/site.css"));
        assert!(mcp_html.contains("/static/mcp.css"));
        assert!(mcp_html.contains("/static/mcp.js"));
        assert!(!mcp_html.contains("three.module.min.js"));
        assert!(!mcp_html.contains("home-scene.js"));
    }

    #[test]
    fn reviewer_regressions_have_bounded_contracts() {
        let home_template = include_str!("../templates/home.html");
        assert!(home_template.contains("href=\"#overview\""));

        let site_css = include_str!("../static/site.css");
        assert!(site_css.contains("@media (prefers-reduced-motion: reduce)"));
        assert!(site_css.contains("scroll-behavior: auto;"));
        assert!(site_css.contains("-webkit-mask-image:"));

        let home_css = include_str!("../static/home.css");
        assert!(home_css.contains("-webkit-mask-image:"));

        let mcp_script = include_str!("../static/mcp.js");
        assert!(mcp_script.contains("window.clearTimeout(button._copyTimeout)"));
        assert!(mcp_script.contains("button._copyTimeout = window.setTimeout"));

        let worker = include_str!("../static/home-scene-worker.js");
        assert!(worker.contains("const sampleWidth = Math.min(100, width)"));
        assert!(worker.contains("const sampleHeight = Math.min(100, height)"));
        assert!(!worker.contains("new Uint8Array(width * height * 4)"));
    }

    #[test]
    fn homepage_scene_has_bounded_accessible_contract() {
        let home_html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            docs_path: DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        assert!(home_html.contains("class=\"scene-shell\" aria-hidden=\"true\""));
        assert!(home_html.contains("class=\"scene-inspector\""));
        assert!(!home_html.contains("class=\"scene-inspector\" aria-live"));
        assert!(home_html.contains("type=\"module\" src=\"/static/home-scene.js\""));
        for source in docs_registry().supported_sources {
            assert!(home_html.contains(&format!("data-source=\"{source}\"")));
        }

        let scene_module = include_str!("../static/home-scene.js");
        for marker in [
            "transferControlToOffscreen",
            "new Worker('/static/home-scene-worker.js', { type: 'module' })",
            "type: 'init'",
            "type: 'resize'",
            "type: 'pointer'",
            "type: 'select'",
            "type: 'rendering'",
            "case 'ready'",
            "case 'frame'",
            "case 'hover'",
            "case 'selection'",
            "case 'failure'",
            "message.nonblank !== true || message.pixelStatus !== 'nonblank'",
            "resetInteractionState();",
            "IntersectionObserver",
            "visibilitychange",
            "prefers-reduced-motion",
            "is-webgl-fallback",
            "problemTitle",
            "canvas.dataset.selectedSource",
            "hero.addEventListener('click'",
            "metadata.accessPath",
            "pointerInside",
            "INTERACTIVE_SELECTOR",
            "closest(INTERACTIVE_SELECTOR)",
            "window.i18n.t",
            "languageChanged",
            "home.hero.inspector_idle",
        ] {
            assert!(
                scene_module.contains(marker),
                "missing scene marker {marker}"
            );
        }
        assert!(!scene_module.contains("three.home.min.js"));
        assert!(!scene_module.contains("new THREE"));

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let worker_module = std::fs::read_to_string(root.join("static/home-scene-worker.js"))
            .expect("homepage scene worker module should be readable");
        for marker in [
            "from '/static/vendor/three.home.min.js'",
            "case 'init'",
            "case 'resize'",
            "case 'pointer'",
            "case 'select'",
            "case 'rendering'",
            "type: 'ready'",
            "type: 'frame'",
            "type: 'hover'",
            "type: 'selection'",
            "type: 'failure'",
            "MAX_DESKTOP_DPR = 1.5",
            "MAX_MOBILE_DPR = 1.25",
            "DESKTOP_NODE_COUNT = 110",
            "MOBILE_NODE_COUNT = 48",
            "requestAnimationFrame",
            "readPixels",
            "FRAME_REPORT_INTERVAL_MS = 250",
            "time - lastFrameReportTime < FRAME_REPORT_INTERVAL_MS",
            "reportFrame(time, frameCount === 1)",
            "return 'nonblank'",
            "return 'blank'",
            "return 'unknown'",
            "pixelStatus !== 'nonblank'",
            "webglcontextlost",
            "postFailureOnce('webgl context lost')",
            "if (!reducedMotion) updatePointerTarget()",
            "if (reducedMotion) return",
            "problemMetadata",
            "raycaster.params.Points.threshold",
            "raycaster.layers.enable(INTERACTION_LAYER)",
            "intersection.index",
            "selectedTarget",
            "MAX_CONTROLLED_PULSES",
            "createControlledPulses",
            "controlledPulses.map((pulse) => pulse.userData.pulseHitTarget)",
            "refreshHoveredTarget",
            "if (!forceStatic && pointerInside) refreshHoveredTarget()",
            "pulseHitTarget",
            "pulseHitTarget.layers.set(INTERACTION_LAYER)",
            "intersectObjects(raycastTargets, false, intersections)",
        ] {
            assert!(
                worker_module.contains(marker),
                "missing scene worker marker {marker}"
            );
        }
        assert_eq!(
            worker_module.matches("type: 'frame'").count(),
            1,
            "frame reporting must stay behind the bounded reportFrame gate"
        );
        assert_eq!(
            worker_module.matches("renderFrame(").count(),
            4,
            "reduced-motion interactions must raycast without rendering a new frame"
        );
        assert!(!worker_module.contains("if (reducedMotion) renderFrame"));

        let mcp_html = McpDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            docs_path: DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("mcp docs template should render");
        assert!(!mcp_html.contains("three.module.min.js"));
        assert!(!mcp_html.contains("home-scene.js"));
    }

    #[test]
    fn homepage_ambient_motion_is_bounded_and_accessible() {
        let home_html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            docs_path: DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        assert_eq!(home_html.matches("data-motion-section").count(), 6);
        assert_eq!(home_html.matches("--motion-order:").count(), 17);

        let home_css = include_str!("../static/home.css");
        for marker in [
            ".home-shell::before",
            ".home-shell::after",
            "@keyframes observatory-field-route",
            "@keyframes observatory-signal-scan",
            "@supports (animation-timeline: view())",
            "animation-timeline: view()",
            "[data-motion-section]",
            "@media (hover: hover) and (pointer: fine)",
            ".source-orbit span:hover",
            ".capability-list li:hover",
            ".endpoint-row:focus-within",
            ".integration-console:focus-within",
            ".auth-matrix tbody tr:hover",
            ".final-command:focus-within",
        ] {
            assert!(
                home_css.contains(marker),
                "missing ambient motion marker {marker}"
            );
        }

        let reduced_motion = home_css
            .split_once("@media (prefers-reduced-motion: reduce) {")
            .map(|(_, block)| block)
            .expect("reduced-motion media block should exist");
        for marker in [
            ".home-shell::before",
            ".home-shell::after",
            "[data-motion-section]",
            ".source-orbit span::after",
            ".capability-list li::after",
            ".endpoint-row::after",
        ] {
            assert!(
                reduced_motion.contains(marker),
                "reduced-motion block must include {marker}"
            );
        }
        assert!(reduced_motion.contains("animation: none;"));
        assert!(reduced_motion.contains("transition: none;"));
    }

    #[test]
    fn homepage_fallback_is_declarative_animated_and_bounded() {
        fn css_block<'a>(source: &'a str, marker: &str) -> Option<&'a str> {
            let marker_start = source.find(marker)?;
            let opening_brace = marker_start + source[marker_start..].find('{')?;
            let mut depth = 0usize;

            for (offset, byte) in source.as_bytes()[opening_brace..].iter().enumerate() {
                match *byte {
                    b'{' => depth += 1,
                    b'}' => {
                        depth = depth.checked_sub(1)?;
                        if depth == 0 {
                            return Some(&source[marker_start..opening_brace + offset + 1]);
                        }
                    }
                    _ => {}
                }
            }

            None
        }

        let home_html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            docs_path: DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        assert!(home_html.contains(
            "data-fallback-scene aria-hidden=\"true\" focusable=\"false\" viewBox=\"0 0 1440 804\" preserveAspectRatio=\"xMidYMid slice\""
        ));
        assert_eq!(home_html.matches("data-fallback-core").count(), 1);
        assert_eq!(home_html.matches("data-fallback-hub").count(), 5);
        assert_eq!(home_html.matches("data-fallback-node").count(), 30);
        assert_eq!(home_html.matches("data-fallback-route").count(), 4);
        assert_eq!(home_html.matches("data-fallback-pulse").count(), 4);
        assert!(!home_html.contains("<animate"));

        let fallback_css = include_str!("../static/home.css");
        for marker in [
            "@keyframes fallback-core-drift",
            "@keyframes fallback-hub-breathe",
            "@keyframes fallback-route-pulse",
            "stroke-dasharray",
            "stroke-dashoffset",
        ] {
            assert!(
                fallback_css.contains(marker),
                "missing fallback CSS marker {marker}"
            );
        }
        assert_eq!(fallback_css.matches("@keyframes fallback-").count(), 3);
        assert!(
            fallback_css.contains(".scene-shell.is-ready .fallback-scene {\n    opacity: 0;\n}")
        );
        assert!(fallback_css
            .contains(".scene-shell.is-webgl-fallback .fallback-scene {\n    opacity: 1;\n}"));
        assert!(fallback_css
            .contains("linear-gradient(120deg, transparent 0 65%, rgba(83, 224, 232, 0.06)"));
        assert!(fallback_css
            .contains("linear-gradient(32deg, transparent 0 72%, rgba(255, 106, 85, 0.05)"));
        for selector in [".fallback-core", ".fallback-hub", ".fallback-pulse"] {
            let rule_start = format!("{selector} {{");
            let animation_rule = fallback_css
                .match_indices(&rule_start)
                .filter_map(|(index, _)| {
                    let rule = &fallback_css[index..];
                    rule.find("\n}").map(|end| &rule[..end])
                })
                .find(|rule| rule.contains("animation:"))
                .unwrap_or_else(|| panic!("missing base animation rule for {selector}"));
            assert!(
                animation_rule.contains("animation-play-state: paused"),
                "base animation must be paused for {selector}"
            );
        }
        assert_eq!(
            fallback_css
                .matches("@media (prefers-reduced-motion: reduce) {")
                .count(),
            1
        );
        let reduced_motion = css_block(fallback_css, "@media (prefers-reduced-motion: reduce)")
            .expect("reduced-motion media block should exist");
        let reduced_animation_rule = css_block(reduced_motion, ".fallback-network,")
            .expect("reduced-motion fallback animation rule should exist");
        for selector in [
            ".fallback-core",
            ".fallback-hub",
            ".fallback-node",
            ".fallback-pulse",
        ] {
            assert!(
                reduced_animation_rule.contains(selector),
                "reduced-motion rule must include {selector}"
            );
        }
        assert!(reduced_animation_rule.contains("animation: none;"));
        assert_eq!(
            fallback_css
                .matches("animation-play-state: running")
                .count(),
            1
        );
        let running_rule = css_block(
            fallback_css,
            ".scene-shell.is-webgl-fallback .fallback-core,",
        )
        .expect("fallback running animation rule should exist");
        for selector in [
            ".scene-shell.is-webgl-fallback .fallback-core",
            ".scene-shell.is-webgl-fallback .fallback-hub",
            ".scene-shell.is-webgl-fallback .fallback-pulse",
        ] {
            assert!(running_rule.contains(selector));
        }
        assert_eq!(
            running_rule
                .matches(".scene-shell.is-webgl-fallback .fallback-")
                .count(),
            3
        );
        assert!(!running_rule.contains(".fallback-node"));
        assert!(running_rule.contains("animation-play-state: running;"));
        let node_rule = css_block(fallback_css, ".fallback-node {\n    opacity: 0.66;")
            .expect("individual fallback node rule should exist");
        assert!(!node_rule.contains("animation:"));
        assert!(!node_rule.contains("animation-delay:"));
        assert!(!node_rule.contains("transform"));
        assert!(!fallback_css.contains("fallback-node-breathe"));

        let scene_module = include_str!("../static/home-scene.js");
        let failure_object = scene_module
            .split("const SCENE_FAILURE = Object.freeze({\n")
            .nth(1)
            .and_then(|javascript| javascript.split("\n});").next())
            .expect("SCENE_FAILURE object should exist");
        let failure_entries: Vec<_> = failure_object.lines().map(str::trim).collect();
        assert_eq!(
            failure_entries,
            vec![
                "WORKER_UNSUPPORTED: 'worker-unsupported',",
                "OFFSCREEN_UNSUPPORTED: 'offscreen-unsupported',",
                "WORKER_LOAD: 'worker-load',",
                "WORKER_MESSAGE: 'worker-message',",
                "CANVAS_TRANSFER: 'canvas-transfer',",
                "INVALID_READY: 'invalid-ready',",
                "WORKER_RUNTIME: 'worker-runtime',",
                "WEBGL_CONTEXT_LOST: 'webgl-context-lost',",
            ],
            "SCENE_FAILURE must contain exactly the eight bounded codes"
        );
        for code in [
            "worker-unsupported",
            "offscreen-unsupported",
            "worker-load",
            "worker-message",
            "canvas-transfer",
            "invalid-ready",
            "worker-runtime",
            "webgl-context-lost",
        ] {
            assert!(
                scene_module.contains(code),
                "missing bounded failure code {code}"
            );
            assert_eq!(
                scene_module.matches(&format!("'{code}'")).count(),
                1,
                "failure code literal must have one canonical owner: {code}"
            );
        }
        for marker in [
            "const SCENE_FAILURE = Object.freeze({",
            "const ALLOWED_SCENE_FAILURES = new Set(Object.values(SCENE_FAILURE))",
            "activateWorkerFallback(reason = SCENE_FAILURE.WORKER_RUNTIME)",
            "activateFallback(shell, canvas, reason)",
            "function handleWorkerMessage(event) {\n        if (workerFailed) return;",
            "const normalizedReason = ALLOWED_SCENE_FAILURES.has(reason)",
            "targetCanvas.dataset.sceneFailure = normalizedReason",
            "delete canvas.dataset.sceneFailure",
            "worker.postMessage(message, transfer || []);\n        } catch {\n            activateWorkerFallback(SCENE_FAILURE.WORKER_MESSAGE);",
            "message.nonblank !== true || message.pixelStatus !== 'nonblank'",
            "activateWorkerFallback(SCENE_FAILURE.INVALID_READY);",
            "message.reason === 'webgl context lost'\n                        ? SCENE_FAILURE.WEBGL_CONTEXT_LOST\n                        : SCENE_FAILURE.WORKER_RUNTIME",
            "if (typeof Worker !== 'function') {\n        activateWorkerFallback(SCENE_FAILURE.WORKER_UNSUPPORTED);",
            "typeof OffscreenCanvas !== 'function' ||\n        typeof canvas.transferControlToOffscreen !== 'function'\n    ) {\n        activateWorkerFallback(SCENE_FAILURE.OFFSCREEN_UNSUPPORTED);",
            "worker = new Worker('/static/home-scene-worker.js', { type: 'module' });\n    } catch {\n        activateWorkerFallback(SCENE_FAILURE.WORKER_LOAD);",
            "addEventListener('error', () => activateWorkerFallback(SCENE_FAILURE.WORKER_LOAD))",
            "addEventListener('messageerror', () =>\n        activateWorkerFallback(SCENE_FAILURE.WORKER_MESSAGE)",
            "offscreenCanvas = canvas.transferControlToOffscreen();\n    } catch {\n        activateWorkerFallback(SCENE_FAILURE.CANVAS_TRANSFER);",
            "if (!shell || !hero) {\n        activateWorkerFallback(SCENE_FAILURE.WORKER_RUNTIME);",
        ] {
            assert!(
                scene_module.contains(marker),
                "missing fallback diagnostic marker {marker}"
            );
        }
        assert_eq!(scene_module.matches("dataset.sceneFailure =").count(), 1);
        assert!(!scene_module.contains("sceneFailure = message"));
        assert!(!scene_module.contains("error.message"));
        assert!(!scene_module.contains("error.stack"));

        let fallback_contract = format!("{home_html}\n{fallback_css}\n{scene_module}");
        for forbidden in [
            "CanvasRenderingContext2D",
            "getContext('2d')",
            "getContext(\"2d\")",
            "requestAnimationFrame",
            "fetch(",
        ] {
            assert!(
                !fallback_contract.contains(forbidden),
                "fallback contract must not contain {forbidden}"
            );
        }
    }

    #[test]
    fn homepage_scene_failure_is_terminal_in_executed_bridge() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let scene_module = root.join("static/home-scene.js");
        let harness = r#"
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

const scenePath = process.argv[1];
const source = fs.readFileSync(scenePath, 'utf8');
const state = { worker: null };

class ClassList {
    constructor() {
        this.values = new Set();
    }

    add(value) {
        this.values.add(value);
    }

    remove(value) {
        this.values.delete(value);
    }

    contains(value) {
        return this.values.has(value);
    }
}

class MockWorker {
    constructor() {
        this.listeners = {};
        this.terminated = false;
        state.worker = this;
    }

    addEventListener(type, listener) {
        this.listeners[type] = listener;
    }

    postMessage() {}

    terminate() {
        this.terminated = true;
    }
}

class MockObserver {
    observe() {}
}

const shell = {
    classList: new ClassList(),
    clientWidth: 1200,
    clientHeight: 700,
};
const hero = { addEventListener() {} };
const inspector = { textContent: 'idle' };
const canvas = {
    dataset: {},
    closest(selector) {
        if (selector === '.scene-shell') return shell;
        if (selector === '.observatory-hero') return hero;
        return null;
    },
    getBoundingClientRect() {
        return { left: 0, top: 0, width: 1200, height: 700 };
    },
    transferControlToOffscreen() {
        return {};
    },
};
const document = {
    hidden: false,
    querySelector(selector) {
        if (selector === '[data-observatory-scene]') return canvas;
        if (selector === '.scene-inspector span:last-child') return inspector;
        return null;
    },
    querySelectorAll(selector) {
        if (selector === '.scene-source-data [data-source]') {
            return [{ dataset: { source: 'leetcode' } }];
        }
        return [];
    },
    addEventListener() {},
};
const window = {
    devicePixelRatio: 1,
    matchMedia() {
        return { matches: false };
    },
};

vm.runInNewContext(source, {
    document,
    window,
    Worker: MockWorker,
    OffscreenCanvas: function OffscreenCanvas() {},
    IntersectionObserver: MockObserver,
    ResizeObserver: MockObserver,
});

assert.ok(state.worker, 'scene bridge should create a worker');
const dispatch = (data) => state.worker.listeners.message({ data });
dispatch({ type: 'failure', reason: 'webgl initialization failed' });

assert.equal(state.worker.terminated, true);
assert.equal(canvas.dataset.sceneStatus, 'fallback');
assert.equal(canvas.dataset.sceneNonblank, 'false');
assert.equal(canvas.dataset.sceneFailure, 'worker-runtime');
assert.equal(shell.classList.contains('is-webgl-fallback'), true);
assert.equal(shell.classList.contains('is-ready'), false);

const terminalDataset = { ...canvas.dataset };
dispatch({ type: 'ready', nonblank: true, pixelStatus: 'nonblank', frameCount: 7 });
dispatch({ type: 'frame', frameCount: 8 });
dispatch({
    type: 'selection',
    identity: 'queued-selection',
    metadata: {
        source: 'leetcode',
        problemTitle: 'Queued',
        algorithm: 'graph',
        similarity: 99,
        accessPath: '/queued',
    },
});

assert.deepEqual(canvas.dataset, terminalDataset);
assert.equal(canvas.dataset.sceneStatus, 'fallback');
assert.equal(canvas.dataset.sceneNonblank, 'false');
assert.equal(canvas.dataset.sceneFailure, 'worker-runtime');
assert.equal(shell.classList.contains('is-ready'), false);
"#;

        let output = match std::process::Command::new("node")
            .arg("-e")
            .arg(harness)
            .arg(&scene_module)
            .output()
        {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("skipping homepage scene lifecycle harness: node is not installed");
                return;
            }
            Err(error) => panic!("failed to execute homepage scene lifecycle harness: {error}"),
        };

        assert!(
            output.status.success(),
            "homepage scene lifecycle harness failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn homepage_scene_vendor_is_self_contained_and_retires_full_distribution() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let worker_module = std::fs::read_to_string(root.join("static/home-scene-worker.js"))
            .expect("homepage scene worker module should be readable");
        assert!(worker_module.contains("from '/static/vendor/three.home.min.js'"));

        let bundle_path = root.join("static/vendor/three.home.min.js");
        let bundle = std::fs::read_to_string(&bundle_path)
            .expect("tree-shaken homepage Three.js bundle should exist");
        for external_reference in [
            "three.core.min.js",
            "three.module.min.js",
            "from\"http",
            "from'http",
            "from\"/",
            "from'/",
        ] {
            assert!(
                !bundle.contains(external_reference),
                "homepage Three.js bundle must be self-contained: {external_reference}"
            );
        }

        for retired in [
            "three.module.min.js",
            "three.core.min.js",
            "three.home.entry.js",
        ] {
            assert!(
                !root.join("static/vendor").join(retired).exists(),
                "retired full Three.js distribution remains: {retired}"
            );
        }
    }

    #[tokio::test]
    async fn scalar_compatibility_path_remains_a_redirect() {
        let state = test_state(false);
        let app = Router::new().merge(public_router()).with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(API_DOCS_PATH)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION),
            Some(&axum::http::HeaderValue::from_static(DOCS_PATH))
        );
    }

    #[test]
    fn renders_api_docs_with_grouped_routes_and_collapsed_details() {
        let html = ApiDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            mcp_docs_path: MCP_DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("api docs template should render");

        assert!(html.contains("HTTP API Reference"));
        assert!(html.contains("0.3.2-test"));
        assert!(html.contains("Problems"));
        assert!(html.contains("Discovery"));
        assert!(html.contains("Service"));
        assert!(html.contains("/api/v1/problems/{source}/{id}"));
        assert!(html.contains("/api/v1/problems/{source}"));
        assert!(html.contains("/api/v1/problems/tags/{source}"));
        assert!(html.contains("/api/v1/problems/difficulties/{source}"));
        assert!(html.contains("/api/v1/resolve/{*query}"));
        assert!(html.contains("/api/v1/daily"));
        assert!(html.contains("/api/v1/similar/{source}/{id}"));
        assert!(html.contains("/api/v1/similar"));
        assert!(html.contains("/status"));
        assert!(html.contains("/health"));
        assert!(html.contains("leetcode"));
        assert!(html.contains("codeforces"));
        assert!(html.contains("atcoder"));
        assert!(html.contains("luogu"));
        assert!(html.contains("spoj"));
        assert_eq!(html.matches("class=\"panel reference-card\"").count(), 11);
        assert!(!html.contains("/admin/"));
    }

    #[test]
    fn renders_mcp_work_surface_with_tools_and_examples() {
        let html = McpDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            docs_path: DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("mcp docs template should render");

        assert!(html.contains("MCP Reference"));
        assert!(html.contains("mcp-layout"));
        assert!(html.contains("mcp-toc"));
        assert!(html.contains("transport-flow"));
        assert!(html.contains("0.3.2-test"));
        assert!(html.contains("/mcp"));
        assert!(html.contains("resolve_problem"));
        assert!(html.contains("get_problem"));
        assert!(html.contains("get_daily_challenge"));
        assert!(html.contains("find_similar_problems"));
        assert!(html.contains("get_platform_status"));
        assert!(html.contains("streamable-http"));
        assert!(html.contains("tools/call"));
        assert!(html.contains("mcpServers.oj-api"));
        assert_eq!(html.matches("class=\"mcp-reference-row").count(), 7);
        assert_eq!(html.matches("class=\"reference-details\"").count(), 7);
        assert_eq!(html.matches("data-copy-target=").count(), 2);
        assert!(!html.contains("class=\"scene-inspector\" aria-live"));
        assert!(html.contains("/static/mcp.js"));
        assert!(!html.contains("panel reference-card"));
        assert!(!html.contains("home-scene.js"));
        assert!(!html.contains("/admin/"));
    }

    #[test]
    fn homepage_auth_matrix_documents_expected_routes() {
        let html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            docs_path: DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        for route in ["/", "/health", "/api/v1/*", "/status", "/mcp"] {
            assert!(html.contains(route));
        }
        assert!(html.contains("Public"));
        assert!(html.contains("Protected"));
    }

    #[test]
    fn docs_examples_match_current_public_route_contract() {
        let docs = docs_registry();
        let tags = docs
            .http_route_cards
            .iter()
            .find(|card| card.fragment_id == "problem-tags")
            .expect("tags card should exist");
        assert_eq!(tags.success_shape, "JSON array of tag strings.");

        let difficulties = docs
            .http_route_cards
            .iter()
            .find(|card| card.fragment_id == "problem-difficulties")
            .expect("difficulties card should exist");
        assert_eq!(
            difficulties.success_shape,
            "JSON array of difficulty strings."
        );
        assert!(difficulties
            .example
            .contains("/api/v1/problems/difficulties/leetcode"));

        let resolve = docs
            .http_route_cards
            .iter()
            .find(|card| card.fragment_id == "resolve-problem")
            .expect("resolve card should exist");
        assert_eq!(
            resolve.success_shape,
            "JSON object with source, id, and problem fields."
        );
        assert!(resolve.example.contains("/api/v1/resolve/leetcode:1"));

        let similar = docs
            .http_route_cards
            .iter()
            .find(|card| card.fragment_id == "similar-search")
            .expect("similar search card should exist");
        assert!(similar.inputs.contains("source."));
        assert!(!similar.inputs.contains("source_filter"));
    }

    #[test]
    fn docs_locale_bundles_cover_public_pages() {
        fn collect_paths(value: &serde_json::Value, prefix: &str, paths: &mut BTreeSet<String>) {
            if let serde_json::Value::Object(map) = value {
                for (key, child) in map {
                    let path = format!("{prefix}/{key}");
                    paths.insert(path.clone());
                    collect_paths(child, &path, paths);
                }
            }
        }

        let locales = [
            ("en", include_str!("../static/i18n/en.json")),
            ("zh-TW", include_str!("../static/i18n/zh-TW.json")),
            ("zh-CN", include_str!("../static/i18n/zh-CN.json")),
        ];

        let parsed_locales: Vec<_> = locales
            .iter()
            .map(|(name, locale)| {
                let parsed: serde_json::Value =
                    serde_json::from_str(locale).expect("locale json should parse");
                (*name, parsed)
            })
            .collect();

        for (name, parsed) in &parsed_locales {
            for key in [
                "/home/nav/overview",
                "/home/nav/api_docs",
                "/home/nav/mcp_docs",
                "/home/hero/category",
                "/home/hero/statement",
                "/home/hero/primary_cta",
                "/home/hero/inspector_access_path",
                "/home/hero/inspector_core_to",
                "/home/hero/inspector_similarity",
                "/home/hero/inspector_illustrative",
                "/home/space/title",
                "/home/capabilities/resolve/title",
                "/home/capabilities/retrieve/title",
                "/home/capabilities/relate/title",
                "/home/capabilities/connect/title",
                "/home/integrations/rest",
                "/home/integrations/mcp",
                "/home/final_cta/title",
                "/docs_api/hero/title",
                "/docs_api/groups/problems",
                "/docs_mcp/hero/title",
                "/docs_mcp/groups/transport",
                "/docs_mcp/groups/tools",
                "/docs_mcp/examples/connection_title",
                "/docs_mcp/examples/request_title",
            ] {
                assert!(
                    parsed.pointer(key).is_some(),
                    "missing locale key {key} in {name}"
                );
            }
        }

        let mut expected_paths = BTreeSet::new();
        for root in ["home", "docs_mcp"] {
            collect_paths(
                &parsed_locales[0].1[root],
                &format!("/{root}"),
                &mut expected_paths,
            );
        }

        for (name, parsed) in parsed_locales.iter().skip(1) {
            let mut actual_paths = BTreeSet::new();
            for root in ["home", "docs_mcp"] {
                collect_paths(&parsed[root], &format!("/{root}"), &mut actual_paths);
            }
            assert_eq!(
                actual_paths, expected_paths,
                "locale key mismatch for {name}"
            );
        }
    }

    #[test]
    fn docs_detail_panels_render_collapsed_by_default() {
        let api_html = ApiDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            mcp_docs_path: MCP_DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("api docs template should render");
        assert_eq!(api_html.matches("class=\"reference-details\"").count(), 11);
        assert!(!api_html.contains("<details class=\"reference-details\" open>"));
        assert_eq!(api_html.matches("<summary>").count(), 11);

        let mcp_html = McpDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            docs_path: DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("mcp docs template should render");
        assert_eq!(mcp_html.matches("class=\"reference-details\"").count(), 7);
        assert!(!mcp_html.contains("<details class=\"reference-details\" open>"));
        assert_eq!(mcp_html.matches("<summary>").count(), 7);
    }

    #[test]
    fn public_templates_do_not_reference_retired_presentation() {
        let public_sources = [
            include_str!("../templates/docs_base.html"),
            include_str!("../templates/home.html"),
            include_str!("../templates/docs_mcp.html"),
            include_str!("../static/site.css"),
            include_str!("../static/home.css"),
            include_str!("../static/mcp.css"),
            include_str!("../static/mcp.js"),
        ];

        for retired in [
            "bento-",
            "Playfair Display",
            "panel reference-card",
            "function openByHash",
            "--home-",
        ] {
            assert!(
                public_sources
                    .iter()
                    .all(|source| !source.contains(retired)),
                "retired presentation reference remains: {retired}"
            );
        }

        let shared_shell = include_str!("../templates/docs_base.html");
        assert!(shared_shell.contains("/static/i18n.js"));
        assert!(shared_shell.contains("block page_styles"));
        assert!(shared_shell.contains("block page_scripts"));
    }

    #[tokio::test]
    async fn docs_routes_are_public_while_mcp_route_stays_protected() {
        let state = test_state(true);
        let token_auth = state.token_auth_enabled.clone();
        let rw_pool = state.rw_pool.clone();
        let mcp_config = state.config.mcp.clone();

        let app = Router::new()
            .merge(public_router())
            .merge(crate::mcp::router(state.clone(), &mcp_config))
            .layer(Extension(AuthRwPool(Arc::new(rw_pool))))
            .layer(Extension(TokenAuthEnabled(token_auth)))
            .with_state(state);

        let api_docs_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(API_DOCS_PATH)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(api_docs_response.status(), StatusCode::PERMANENT_REDIRECT);

        let mcp_docs_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(MCP_DOCS_PATH)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(mcp_docs_response.status(), StatusCode::OK);

        let mcp_response = app
            .oneshot(
                Request::builder()
                    .uri("/mcp")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(mcp_response.status(), StatusCode::UNAUTHORIZED);
    }
}
