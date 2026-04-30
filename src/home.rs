use std::sync::atomic::Ordering;
use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::AppState;

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
        docs_href: "/docs/api#problem-detail",
    },
    HomepageCard {
        title: "Daily challenge",
        method: "GET",
        path: "/api/v1/daily",
        summary: "Fetch the current LeetCode daily challenge with the existing fallback behavior.",
        docs_href: "/docs/api#daily-challenge",
    },
    HomepageCard {
        title: "Similar search",
        method: "GET",
        path: "/api/v1/similar",
        summary: "Search related problems by free-text query against the public similarity index.",
        docs_href: "/docs/api#similar-search",
    },
];

const HTTP_ROUTE_CARDS: [HttpRouteCard; 10] = [
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
        path: "/api/v1/tags/{source}",
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
        example: "curl -H 'Authorization: Bearer <token>' http://127.0.0.1:7856/api/v1/tags/leetcode",
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
        method: "GET",
        path: "/api/v1/similar",
        title: "Similar by query",
        title_i18n: "docs_api.cards.similar_by_query.title",
        purpose: "Find similar problems from a free-text query.",
        purpose_i18n: "docs_api.cards.similar_by_query.purpose",
        auth_rule: "Bearer auth follows the existing public API middleware setting.",
        auth_rule_i18n: "docs_api.cards.similar_by_query.auth_rule",
        inputs: "Query params include query, limit, threshold, and source.",
        inputs_i18n: "docs_api.cards.similar_by_query.inputs",
        success_shape: "JSON object with ranked similar-problem results.",
        success_shape_i18n: "docs_api.cards.similar_by_query.success_shape",
        example: "curl -H 'Authorization: Bearer <token>' 'http://127.0.0.1:7856/api/v1/similar?query=graph%20shortest%20path&limit=5'",
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
        rest_capability: "Maps to GET /api/v1/similar and GET /api/v1/similar/{source}/{id}.",
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
    api_docs_path: &'static str,
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
    api_docs_path: &'static str,
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
        api_docs_path: API_DOCS_PATH,
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
        api_docs_path: API_DOCS_PATH,
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
    fn renders_public_homepage_with_protected_api_badge() {
        let html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            api_docs_path: API_DOCS_PATH,
            mcp_docs_path: MCP_DOCS_PATH,
        }
        .render()
        .expect("home template should render");

        assert!(html.contains("Access coding problem data through one unified API."));
        assert!(html.contains("Token auth enabled"));
        assert!(html.contains("/api/v1/problems/{source}/{id}"));
        assert!(html.contains("/api/v1/daily"));
        assert!(html.contains("/api/v1/similar"));
        assert!(html.contains("/docs/api"));
        assert!(html.contains("/docs/mcp"));
        assert!(!html.contains("Dashboard"));
    }

    #[test]
    fn renders_public_homepage_with_public_api_badge_when_auth_disabled() {
        let html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: false,
            version: "0.3.2-test",
            docs: docs_registry(),
            api_docs_path: API_DOCS_PATH,
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
        assert!(html.contains("Access coding problem data through one unified API."));
        assert!(html.contains("Dual-state access rules"));
        assert!(html.contains("HTTP API Reference"));
        assert!(html.contains("MCP Reference"));
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
        assert_eq!(docs.http_route_cards.len(), 10);
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
        assert!(html.contains("/api/v1/tags/{source}"));
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
        assert_eq!(html.matches("class=\"panel reference-card\"").count(), 10);
        assert!(!html.contains("/admin/"));
    }

    #[test]
    fn renders_mcp_docs_with_transport_tools_and_examples() {
        let html = McpDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            api_docs_path: API_DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("mcp docs template should render");

        assert!(html.contains("MCP Reference"));
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
        assert_eq!(html.matches("class=\"panel reference-card\"").count(), 9);
        assert!(!html.contains("/admin/"));
    }

    #[test]
    fn homepage_auth_matrix_documents_expected_routes() {
        let html = HomeTemplate {
            total_problems: 42,
            token_auth_enabled: true,
            version: "0.3.2-test",
            docs: docs_registry(),
            api_docs_path: API_DOCS_PATH,
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
        let locales = [
            include_str!("../static/i18n/en.json"),
            include_str!("../static/i18n/zh-TW.json"),
            include_str!("../static/i18n/zh-CN.json"),
        ];

        for locale in locales {
            let parsed: serde_json::Value =
                serde_json::from_str(locale).expect("locale json should parse");
            for key in [
                "/home/nav/overview",
                "/home/nav/api_docs",
                "/home/nav/mcp_docs",
                "/docs_api/hero/title",
                "/docs_api/groups/problems",
                "/docs_mcp/hero/title",
                "/docs_mcp/groups/transport",
                "/docs_mcp/groups/tools",
                "/docs_mcp/examples/connection_title",
                "/docs_mcp/examples/request_title",
            ] {
                assert!(parsed.pointer(key).is_some(), "missing locale key {key}");
            }
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
        assert_eq!(api_html.matches("class=\"reference-details\"").count(), 10);
        assert!(!api_html.contains("<details class=\"reference-details\" open>"));
        assert_eq!(api_html.matches("<summary>").count(), 10);

        let mcp_html = McpDocsTemplate {
            docs: docs_registry(),
            version: "0.3.2-test",
            home_path: "/",
            api_docs_path: API_DOCS_PATH,
            token_auth_enabled: true,
        }
        .render()
        .expect("mcp docs template should render");
        assert_eq!(mcp_html.matches("class=\"reference-details\"").count(), 7);
        assert!(!mcp_html.contains("<details class=\"reference-details\" open>"));
        assert_eq!(mcp_html.matches("<summary>").count(), 7);
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
