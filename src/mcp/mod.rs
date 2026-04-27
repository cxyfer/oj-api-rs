use std::sync::Arc;

use ammonia::clean_text;
use axum::{
    body::to_bytes,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    Router,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorData, Implementation, ServerCapabilities, ServerInfo,
        ToolsCapability,
    },
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ServerHandler,
};
use serde::Deserialize;

use crate::{
    api::{daily, error::ProblemDetail, problems, resolve, similar, status},
    config::McpConfig,
    AppState,
};

const MAX_RESPONSE_BODY: usize = 1_048_576;
const MAX_OUTPUT_BYTES: usize = 102_400;

pub fn router(state: Arc<AppState>, config: &McpConfig) -> Router<Arc<AppState>> {
    let mut server_config = StreamableHttpServerConfig::default();
    server_config = if config.allowed_hosts.is_empty() {
        server_config.disable_allowed_hosts()
    } else {
        server_config.with_allowed_hosts(config.allowed_hosts.iter().cloned())
    };

    let service: StreamableHttpService<OjMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(OjMcpServer::new(state.clone())),
            Arc::new(LocalSessionManager::default()),
            server_config,
        );

    Router::<Arc<AppState>>::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn(crate::auth::bearer_auth))
}

#[derive(Clone)]
struct OjMcpServer {
    state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

impl OjMcpServer {
    fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ResolveProblemParams {
    #[schemars(
        description = "A problem URL, slug, or prefixed ID. Examples: 'https://leetcode.com/problems/two-sum', 'https://codeforces.com/problemset/problem/1/A', 'leetcode/two-sum', 'cf1A', 'P1001'"
    )]
    query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetProblemParams {
    #[schemars(description = "Problem source: leetcode, codeforces, atcoder, luogu, or spoj")]
    source: String,
    #[schemars(
        description = "Problem ID on the platform. Examples: '1' or 'two-sum' (leetcode), '1A' (codeforces), 'abc001_1' (atcoder), 'P1001' (luogu)"
    )]
    id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
enum Domain {
    #[default]
    Com,
    Cn,
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Com => write!(f, "com"),
            Self::Cn => write!(f, "cn"),
        }
    }
}

fn remove_default(schema: &mut schemars::Schema) {
    schema.remove("default");
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
struct DailyParams {
    #[serde(default)]
    #[schemars(
        description = "LeetCode domain: 'com' (default) or 'cn'. Daily challenge switches at 00:00 in the respective domain timezone"
    )]
    domain: Domain,
    #[serde(default)]
    #[schemars(with = "String", transform = remove_default)]
    #[schemars(
        description = "Date in YYYY-MM-DD format. Defaults to today for the selected domain"
    )]
    date: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SimilarParams {
    #[serde(default)]
    #[schemars(with = "String", transform = remove_default)]
    #[schemars(
        description = "Problem source: leetcode, codeforces, atcoder, luogu, or spoj (required for ID-based search)"
    )]
    source: Option<String>,
    #[serde(default)]
    #[schemars(with = "String", transform = remove_default)]
    #[schemars(
        description = "Problem ID on the platform, e.g. '1' (leetcode), '1A' (codeforces), 'abc001_1' (atcoder), 'P1001' (luogu). Required for ID-based search"
    )]
    id: Option<String>,
    #[serde(default)]
    #[schemars(with = "String", transform = remove_default)]
    #[schemars(
        description = "Text query for semantic search (3-2000 chars, takes priority over source+id)"
    )]
    query: Option<String>,
    #[serde(default)]
    #[schemars(with = "u32", range(min = 1, max = 50), transform = remove_default)]
    #[schemars(description = "Maximum results to return (1-50, default: 10)")]
    limit: Option<u32>,
    #[serde(default)]
    #[schemars(
        with = "f32",
        range(min = 0.0, max = 1.0),
        transform = remove_default
    )]
    #[schemars(description = "Minimum similarity threshold (0.0-1.0, default: 0.0)")]
    threshold: Option<f32>,
    #[serde(default)]
    #[schemars(with = "String", transform = remove_default)]
    #[schemars(
        description = "Comma-separated platform filter (e.g. 'leetcode,codeforces,atcoder,luogu')"
    )]
    source_filter: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FetchingResponse {
    retry_after: u64,
}

#[tool_router]
impl OjMcpServer {
    #[tool(
        description = "Preferred way to look up a problem. Accepts a URL, problem slug, or prefixed ID and returns the full problem (title, difficulty, tags, and description). Supports LeetCode, Codeforces, AtCoder, Luogu, and SPOJ. Use this when the input format is uncertain; use get_problem when source and ID are already known."
    )]
    async fn resolve_problem(
        &self,
        params: Parameters<ResolveProblemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let query = params.0.query.trim();
        if query.is_empty() {
            return Ok(domain_error("query must be non-empty"));
        }

        let response = resolve::resolve(
            axum::extract::State(self.state.clone()),
            axum::extract::Path(query.to_string()),
        )
        .await
        .into_response();

        match parse_tool_json_response::<resolve::ResolveResponse>(response).await? {
            ToolJsonResponse::Success(parsed) => {
                let Some(problem) = parsed.problem else {
                    return Ok(domain_error("problem not found"));
                };
                Ok(text_result(format_problem_detail(&problem)))
            }
            ToolJsonResponse::DomainError(result) => Ok(result),
        }
    }

    #[tool(
        description = "Get a specific problem by source and ID. Returns the full problem including title, difficulty, tags, and description. Supports LeetCode, Codeforces, AtCoder, Luogu, and SPOJ. Use resolve_problem instead when the input is a URL or the ID format is uncertain."
    )]
    async fn get_problem(
        &self,
        params: Parameters<GetProblemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let source = params.0.source.trim();
        let id = params.0.id.trim();
        if source.is_empty() || id.is_empty() {
            return Ok(domain_error("source and id must be non-empty"));
        }

        let response = problems::get_problem(
            axum::extract::State(self.state.clone()),
            axum::extract::Path((source.to_string(), id.to_string())),
        )
        .await
        .into_response();

        match parse_tool_json_response::<problems::ProblemDetailResponse>(response).await? {
            ToolJsonResponse::Success(parsed) => Ok(text_result(format_problem_detail(&parsed))),
            ToolJsonResponse::DomainError(result) => Ok(result),
        }
    }

    #[tool(
        description = "Get the LeetCode daily challenge problem. Returns the full problem including title, difficulty, tags, and description. Defaults to today in the selected domain's timezone."
    )]
    async fn get_daily_challenge(
        &self,
        params: Parameters<DailyParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let response = daily::get_daily(
            axum::extract::State(self.state.clone()),
            axum::extract::Query(daily::DailyQuery {
                domain: Some(params.0.domain.to_string()),
                source: None,
                date: params.0.date,
                r#async: Some(false),
            }),
        )
        .await
        .into_response();

        match response.status() {
            StatusCode::OK => {
                let parsed: daily::DailyChallengeResponse = parse_json_body(response).await?;
                Ok(text_result(format_daily_challenge(&parsed)))
            }
            StatusCode::ACCEPTED => {
                let parsed: FetchingResponse = parse_json_body(response).await?;
                Ok(text_result(format!(
                    "The daily challenge is currently being fetched. Please retry after {} seconds.",
                    parsed.retry_after
                )))
            }
            _ => parse_error_result(response).await,
        }
    }

    #[tool(
        description = "Find similar problems by problem ID or free-text query across LeetCode, Codeforces, AtCoder, Luogu, and SPOJ. Returns a ranked list with similarity scores. Provide either a text query, or a source + ID pair."
    )]
    async fn find_similar_problems(
        &self,
        params: Parameters<SimilarParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let limit = params.limit.unwrap_or(10);
        if !(1..=50).contains(&limit) {
            return Ok(domain_error("limit must be between 1 and 50"));
        }

        let threshold = params.threshold.unwrap_or(0.0);
        if !(0.0..=1.0).contains(&threshold) {
            return Ok(domain_error("threshold must be between 0.0 and 1.0"));
        }

        let response = if let Some(query) = params
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            let len = query.chars().count();
            if !(3..=2000).contains(&len) {
                return Ok(domain_error("query must be between 3 and 2000 characters"));
            }

            similar::similar_by_text(
                axum::extract::State(self.state.clone()),
                axum::extract::Query(similar::SimilarByTextQuery {
                    query: Some(query.to_string()),
                    limit: Some(limit),
                    threshold: Some(threshold),
                    source: params.source_filter.clone(),
                }),
            )
            .await
            .into_response()
        } else {
            let source = params.source.as_deref().map(str::trim).unwrap_or("");
            let id = params.id.as_deref().map(str::trim).unwrap_or("");
            if source.is_empty() || id.is_empty() {
                return Ok(domain_error(
                    "either 'query' or both 'source' and 'id' must be provided",
                ));
            }

            similar::similar_by_problem(
                axum::extract::State(self.state.clone()),
                axum::extract::Path((source.to_string(), id.to_string())),
                axum::extract::Query(similar::SimilarByProblemQuery {
                    limit: Some(limit),
                    threshold: Some(threshold),
                    source: params.source_filter.clone(),
                }),
            )
            .await
            .into_response()
        };

        match parse_tool_json_response::<similar::SimilarResponse>(response).await? {
            ToolJsonResponse::Success(parsed) => {
                let query_label = parsed
                    .rewritten_query
                    .clone()
                    .or_else(|| params.query.clone())
                    .unwrap_or_else(|| {
                        format!(
                            "{} {}",
                            params.source.unwrap_or_default(),
                            params.id.unwrap_or_default()
                        )
                        .trim()
                        .to_string()
                    })
                    .trim()
                    .to_string();

                Ok(text_result(format_similar(&parsed, &query_label)))
            }
            ToolJsonResponse::DomainError(result) => Ok(result),
        }
    }

    #[tool(
        description = "Get problem counts and indexing coverage for each platform (LeetCode, Codeforces, AtCoder, Luogu, SPOJ). Returns total problems, missing content count, and un-embedded count per platform."
    )]
    async fn get_platform_status(&self) -> Result<CallToolResult, ErrorData> {
        let response = status::get_status(axum::extract::State(self.state.clone()))
            .await
            .into_response();
        match parse_tool_json_response::<status::StatusResponse>(response).await? {
            ToolJsonResponse::Success(parsed) => Ok(text_result(format_status(&parsed))),
            ToolJsonResponse::DomainError(result) => Ok(result),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OjMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools_with(ToolsCapability {
                    list_changed: Some(false),
                })
                .build(),
        )
        .with_server_info(Implementation::new(
            "oj-api-rs-http-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
    }
}

fn domain_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
}

fn protocol_error(message: impl Into<String>) -> ErrorData {
    ErrorData::internal_error(message.into(), None)
}

fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(truncate_output(text.into()))])
}

enum ToolJsonResponse<T> {
    Success(T),
    DomainError(CallToolResult),
}

async fn parse_tool_json_response<T>(response: Response) -> Result<ToolJsonResponse<T>, ErrorData>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    if status == StatusCode::OK {
        return parse_json_body(response)
            .await
            .map(ToolJsonResponse::Success);
    }

    parse_error_result(response)
        .await
        .map(ToolJsonResponse::DomainError)
}

async fn parse_error_result(response: Response) -> Result<CallToolResult, ErrorData> {
    let status = response.status();
    let body = read_body_string(response).await?;
    let detail: ProblemDetail = serde_json::from_str(&body)
        .map_err(|e| protocol_error(format!("invalid error JSON: {e}")))?;
    Ok(domain_error(format_problem_detail_error(status, &detail)))
}

async fn parse_json_body<T>(response: Response) -> Result<T, ErrorData>
where
    T: serde::de::DeserializeOwned,
{
    let body = read_body_string(response).await?;
    serde_json::from_str(&body).map_err(|e| protocol_error(format!("invalid JSON: {e}")))
}

async fn read_body_string(response: Response) -> Result<String, ErrorData> {
    let bytes = to_bytes(response.into_body(), MAX_RESPONSE_BODY)
        .await
        .map_err(|e| protocol_error(format!("failed to read response body: {e}")))?;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| protocol_error(format!("response body was not valid UTF-8: {e}")))
}

fn format_problem_detail_error(status: StatusCode, detail: &ProblemDetail) -> String {
    format!("[{}] {}: {}", status.as_u16(), detail.title, detail.detail)
}

fn looks_like_html(s: &str) -> bool {
    let trimmed = s.trim();
    if !trimmed.contains('<') || !trimmed.contains('>') {
        return false;
    }
    let mut inside_tag = false;
    let mut saw_alpha_tag_name = false;

    for ch in trimmed.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' if inside_tag => return saw_alpha_tag_name,
            c if inside_tag && c.is_ascii_alphabetic() => saw_alpha_tag_name = true,
            _ => {}
        }
    }

    false
}

fn html_to_markdown(content: &str) -> String {
    if content.trim().is_empty() {
        return "No description available.".into();
    }
    if !looks_like_html(content) {
        return content.to_owned();
    }
    match std::panic::catch_unwind(|| htmd::convert(content)) {
        Ok(Ok(md)) if !md.trim().is_empty() => md,
        _ => clean_text(content),
    }
}

fn preferred_title(title: Option<&str>, title_cn: Option<&str>, fallback: &str) -> String {
    title
        .filter(|value| !value.trim().is_empty())
        .or_else(|| title_cn.filter(|value| !value.trim().is_empty()))
        .unwrap_or(fallback)
        .to_string()
}

fn format_problem_detail(problem: &problems::ProblemDetailResponse) -> String {
    let title = preferred_title(
        problem.title.as_deref(),
        problem.title_cn.as_deref(),
        &problem.slug,
    );
    let difficulty = problem.difficulty.as_deref().unwrap_or("N/A");
    let tags = if problem.tags.is_empty() {
        "N/A".to_string()
    } else {
        problem.tags.join(", ")
    };
    let link = problem.link.as_deref().unwrap_or("N/A");
    let ac_rate = problem
        .ac_rate
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "N/A".to_string());

    let mut info = format!(
        "# {title}\n\n- Source: {source} | ID: {id} | Difficulty: {difficulty}\n- Tags: {tags}\n- Link: {link}\n- AC Rate: {ac_rate}",
        source = problem.source,
        id = problem.id,
    );

    if let Some(rating) = problem.rating {
        info.push_str(&format!("\n- Rating: {:.0}", rating));
    }
    if let Some(contest) = &problem.contest {
        info.push_str(&format!("\n- Contest: {}", contest));
    }
    if let Some(idx) = &problem.problem_index {
        info.push_str(&format!("\n- Index: {}", idx));
    }

    let content = html_to_markdown(
        problem
            .content
            .as_deref()
            .or(problem.content_cn.as_deref())
            .unwrap_or(""),
    );

    let mut output = format!("{}\n\n---\n\n{}", info, content);

    if !problem.similar_questions.is_empty() {
        output.push_str("\n\n---\n\n### Similar Questions\n\n");
        for item in &problem.similar_questions {
            let item_title =
                preferred_title(item.title.as_deref(), item.title_cn.as_deref(), &item.slug);
            output.push_str(&format!(
                "- **{}** ({} {}): {}\n",
                item_title,
                item.source,
                item.id,
                item.link.as_deref().unwrap_or("N/A")
            ));
        }
    }

    output
}

fn format_daily_challenge(problem: &daily::DailyChallengeResponse) -> String {
    let title = preferred_title(
        problem.title.as_deref(),
        problem.title_cn.as_deref(),
        &problem.slug,
    );
    let difficulty = problem.difficulty.as_deref().unwrap_or("N/A");
    let tags = if problem.tags.is_empty() {
        "N/A".to_string()
    } else {
        problem.tags.join(", ")
    };
    let link = problem.link.as_deref().unwrap_or("N/A");
    let ac_rate = problem
        .ac_rate
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "N/A".to_string());

    let mut info = format!(
        "# {title}\n\n- Source: leetcode | ID: {id} | Difficulty: {difficulty}\n- Domain: {domain} | Date: {date}\n- Tags: {tags}\n- Link: {link}\n- AC Rate: {ac_rate}",
        id = problem.id,
        domain = problem.domain,
        date = problem.date,
    );

    if let Some(rating) = problem.rating {
        info.push_str(&format!("\n- Rating: {:.0}", rating));
    }

    let content = html_to_markdown(
        problem
            .content
            .as_deref()
            .or(problem.content_cn.as_deref())
            .unwrap_or(""),
    );

    let mut output = format!("{}\n\n---\n\n{}", info, content);

    if !problem.similar_questions.is_empty() {
        output.push_str("\n\n---\n\n### Similar Questions\n\n");
        for item in &problem.similar_questions {
            let item_title =
                preferred_title(item.title.as_deref(), item.title_cn.as_deref(), &item.slug);
            output.push_str(&format!(
                "- **{}** ({} {}): {}\n",
                item_title,
                item.source,
                item.id,
                item.link.as_deref().unwrap_or("N/A")
            ));
        }
    }

    output
}

fn format_similar(response: &similar::SimilarResponse, query_label: &str) -> String {
    let mut output = format!(
        "# Similar Problems\n\nQuery: {}\n\n| # | Source | ID | Title | Difficulty | Similarity | Link |\n|---|--------|----|-------|------------|------------|------|\n",
        query_label,
    );

    for (index, item) in response.results.iter().enumerate() {
        let title = preferred_title(item.title.as_deref(), None, "N/A");
        let difficulty = item.difficulty.as_deref().unwrap_or("N/A");
        let link = item.link.as_deref().unwrap_or("N/A");
        let similarity = format!("{:.1}%", item.similarity * 100.0);
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            index + 1,
            item.source,
            item.id,
            title,
            difficulty,
            similarity,
            link,
        ));
    }

    output
}

fn format_status(response: &status::StatusResponse) -> String {
    let mut output = format!(
        "# OJ Platform Status (v{})\n\n| Platform | Problems | Missing Content | Not Embedded |\n|----------|----------|-----------------|--------------|\n",
        response.version,
    );

    for platform in &response.platforms {
        output.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            platform.source,
            format_number(platform.total as u64),
            format_number(platform.missing_content as u64),
            format_number(platform.not_embedded as u64),
        ));
    }

    output
}

fn format_number(value: u64) -> String {
    let text = value.to_string();
    let mut result = String::with_capacity(text.len() + text.len() / 3);
    for (index, ch) in text.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

fn truncate_output(text: String) -> String {
    if text.len() <= MAX_OUTPUT_BYTES {
        return text;
    }
    let boundary = text.floor_char_boundary(MAX_OUTPUT_BYTES);
    let mut truncated = text[..boundary].to_owned();
    truncated.push_str("\n\n... (truncated)");
    truncated
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, VecDeque},
        fs,
        sync::{atomic::AtomicBool, Arc},
    };

    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        Extension,
    };
    use serde_json::Value;
    use tokio::sync::{RwLock, Semaphore};
    use tower::ServiceExt;

    use super::*;
    use crate::{auth, config::Config, db, models::Problem};

    fn test_db_path() -> String {
        std::env::temp_dir()
            .join(format!(
                "oj-api-rs-mcp-tests-{}.sqlite",
                uuid::Uuid::new_v4()
            ))
            .to_string_lossy()
            .into_owned()
    }

    fn cleanup_db_files(path: &str) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{path}-wal"));
        let _ = fs::remove_file(format!("{path}-shm"));
    }

    fn test_state(auth_enabled: bool) -> (Arc<AppState>, String, String) {
        db::register_sqlite_vec();
        let mut config = Config::default();
        let path = test_db_path();
        config.database.path = path.clone();
        let rw_pool = db::create_rw_pool(&path, 1, config.database.busy_timeout_ms);
        db::ensure_data_tables(&rw_pool);
        db::ensure_api_tokens_table(&rw_pool);
        db::ensure_app_settings_table(&rw_pool);
        if auth_enabled {
            crate::db::settings::set_setting(&rw_pool, "token_auth_enabled", "1");
        } else {
            crate::db::settings::set_setting(&rw_pool, "token_auth_enabled", "0");
        }
        let ro_pool = db::create_ro_pool(&path, 1, config.database.busy_timeout_ms);

        let state = Arc::new(AppState {
            ro_pool,
            rw_pool: rw_pool.clone(),
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
            token_auth_enabled: Arc::new(AtomicBool::new(auth_enabled)),
            admin_sessions: Arc::new(RwLock::new(HashMap::new())),
            config_path: None,
        });

        let token = crate::db::tokens::create_token(&state.rw_pool, Some("mcp-test"))
            .expect("create token")
            .token;

        let problem = Problem {
            id: "1".to_string(),
            source: "leetcode".to_string(),
            slug: "two-sum".to_string(),
            title: Some("Two Sum".to_string()),
            title_cn: Some("两数之和".to_string()),
            difficulty: Some("Easy".to_string()),
            ac_rate: Some(55.1),
            rating: None,
            contest: None,
            problem_index: None,
            tags: vec!["Array".to_string(), "Hash Table".to_string()],
            link: Some("https://leetcode.com/problems/two-sum/".to_string()),
            category: Some("Algorithms".to_string()),
            paid_only: Some(0),
            content: Some("<p>Given an array of integers...</p>".to_string()),
            content_cn: None,
            similar_questions: Vec::new(),
        };
        crate::db::problems::insert_problem(&state.rw_pool, &problem).expect("insert problem");

        (state, path, token)
    }

    fn test_app(state: Arc<AppState>, auth_enabled: bool) -> Router {
        Router::<Arc<AppState>>::new()
            .merge(router(state.clone(), &state.config.mcp))
            .layer(Extension(auth::AuthRwPool(Arc::new(state.rw_pool.clone()))))
            .layer(Extension(auth::TokenAuthEnabled(Arc::new(
                AtomicBool::new(auth_enabled),
            ))))
            .with_state(state)
    }

    async fn sse_json_value(response: Response) -> Value {
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), MAX_RESPONSE_BODY)
            .await
            .unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let payload = body
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        serde_json::from_str(&payload).unwrap_or_else(|err| {
            panic!("failed to parse SSE payload: {err}; body={body:?}; payload={payload:?}")
        })
    }

    fn initialize_request(token: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("host", "localhost")
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        builder
            .body(Body::from(
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-03-26",
                        "capabilities": {},
                        "clientInfo": {"name": "test", "version": "1.0"}
                    }
                })
                .to_string(),
            ))
            .unwrap()
    }

    fn initialized_notification(session_id: &str, token: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("host", "localhost")
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .header("mcp-session-id", session_id);
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        builder
            .body(Body::from(
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                })
                .to_string(),
            ))
            .unwrap()
    }

    fn session_request(
        session_id: &str,
        id: i64,
        method: &str,
        params: Value,
        token: Option<&str>,
    ) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("host", "localhost")
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .header("mcp-session-id", session_id);
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        builder
            .body(Body::from(
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method,
                    "params": params,
                })
                .to_string(),
            ))
            .unwrap()
    }

    #[tokio::test]
    async fn mcp_requires_bearer_token_when_auth_is_enabled() {
        let (state, path, _) = test_state(true);
        let app = test_app(state, true);

        let response = app.oneshot(initialize_request(None)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn mcp_allows_initialize_without_token_when_auth_is_disabled() {
        let (state, path, _) = test_state(false);
        let app = test_app(state, false);

        let response = app.oneshot(initialize_request(None)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = sse_json_value(response).await;
        assert_eq!(json["id"], 1);
        assert!(json["result"].is_object());

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn mcp_tools_list_and_get_problem_work() {
        let (state, path, token) = test_state(true);
        let app = test_app(state, true);

        let init_response = app
            .clone()
            .oneshot(initialize_request(Some(&token)))
            .await
            .unwrap();
        assert_eq!(init_response.status(), StatusCode::OK);
        let session_id = init_response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .unwrap()
            .to_string();

        let initialized = app
            .clone()
            .oneshot(initialized_notification(&session_id, Some(&token)))
            .await
            .unwrap();
        assert_eq!(initialized.status(), StatusCode::ACCEPTED);

        let tools_response = app
            .clone()
            .oneshot(session_request(
                &session_id,
                2,
                "tools/list",
                serde_json::json!({}),
                Some(&token),
            ))
            .await
            .unwrap();
        let tools_json = sse_json_value(tools_response).await;
        let tool_names: Vec<&str> = tools_json["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert_eq!(tool_names.len(), 5);
        assert!(tool_names.contains(&"get_problem"));
        assert!(tool_names.contains(&"resolve_problem"));
        assert!(tool_names.contains(&"get_daily_challenge"));
        assert!(tool_names.contains(&"find_similar_problems"));
        assert!(tool_names.contains(&"get_platform_status"));

        for tool in tools_json["result"]["tools"].as_array().unwrap() {
            let input_schema = &tool["inputSchema"];
            assert_gemini_compatible_schema(
                input_schema,
                &format!("tool {}", tool["name"].as_str().unwrap()),
            );
        }
        assert_eq!(
            tool_input_schema_snapshot(&tools_json),
            expected_tool_input_schema_snapshot()
        );

        let daily_tool = tools_json["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "get_daily_challenge")
            .unwrap();
        let domain_schema = &daily_tool["inputSchema"]["properties"]["domain"];
        assert_eq!(domain_schema["type"], Value::String("string".to_string()));
        assert_eq!(domain_schema["enum"], serde_json::json!(["com", "cn"]));

        let call_response = app
            .clone()
            .oneshot(session_request(
                &session_id,
                3,
                "tools/call",
                serde_json::json!({
                    "name": "get_problem",
                    "arguments": {"source": "leetcode", "id": "1"}
                }),
                Some(&token),
            ))
            .await
            .unwrap();
        let call_json = sse_json_value(call_response).await;
        assert_eq!(call_json["result"]["isError"], Value::Bool(false));
        let text = call_json["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("# Two Sum"));
        assert!(text.contains("Source: leetcode | ID: 1"));

        cleanup_db_files(&path);
    }

    fn tool_input_schema_snapshot(tools_json: &Value) -> Value {
        let mut snapshot = serde_json::Map::new();
        for tool in tools_json["result"]["tools"].as_array().unwrap() {
            snapshot.insert(
                tool["name"].as_str().unwrap().to_string(),
                tool["inputSchema"].clone(),
            );
        }
        Value::Object(snapshot)
    }

    fn expected_tool_input_schema_snapshot() -> Value {
        serde_json::from_str(
            r#"{
  "resolve_problem": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "ResolveProblemParams",
    "type": "object",
    "properties": {
      "query": {
        "description": "A problem URL, slug, or prefixed ID. Examples: 'https://leetcode.com/problems/two-sum', 'https://codeforces.com/problemset/problem/1/A', 'leetcode/two-sum', 'cf1A', 'P1001'",
        "type": "string"
      }
    },
    "required": ["query"]
  },
  "get_problem": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "GetProblemParams",
    "type": "object",
    "properties": {
      "source": {
        "description": "Problem source: leetcode, codeforces, atcoder, luogu, or spoj",
        "type": "string"
      },
      "id": {
        "description": "Problem ID on the platform. Examples: '1' or 'two-sum' (leetcode), '1A' (codeforces), 'abc001_1' (atcoder), 'P1001' (luogu)",
        "type": "string"
      }
    },
    "required": ["source", "id"]
  },
  "get_daily_challenge": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "DailyParams",
    "type": "object",
    "properties": {
      "domain": {
        "description": "LeetCode domain: 'com' (default) or 'cn'. Daily challenge switches at 00:00 in the respective domain timezone",
        "type": "string",
        "enum": ["com", "cn"]
      },
      "date": {
        "description": "Date in YYYY-MM-DD format. Defaults to today for the selected domain",
        "type": "string"
      }
    }
  },
  "find_similar_problems": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "SimilarParams",
    "type": "object",
    "properties": {
      "source": {
        "description": "Problem source: leetcode, codeforces, atcoder, luogu, or spoj (required for ID-based search)",
        "type": "string"
      },
      "id": {
        "description": "Problem ID on the platform, e.g. '1' (leetcode), '1A' (codeforces), 'abc001_1' (atcoder), 'P1001' (luogu). Required for ID-based search",
        "type": "string"
      },
      "query": {
        "description": "Text query for semantic search (3-2000 chars, takes priority over source+id)",
        "type": "string"
      },
      "limit": {
        "description": "Maximum results to return (1-50, default: 10)",
        "type": "integer",
        "format": "uint32",
        "minimum": 1,
        "maximum": 50
      },
      "threshold": {
        "description": "Minimum similarity threshold (0.0-1.0, default: 0.0)",
        "type": "number",
        "format": "float",
        "minimum": 0.0,
        "maximum": 1.0
      },
      "source_filter": {
        "description": "Comma-separated platform filter (e.g. 'leetcode,codeforces,atcoder,luogu')",
        "type": "string"
      }
    }
  },
  "get_platform_status": {
    "type": "object",
    "properties": {}
  }
}"#,
        )
        .unwrap()
    }

    fn assert_gemini_compatible_schema(schema: &Value, path: &str) {
        let Some(obj) = schema.as_object() else {
            return;
        };

        assert!(
            !obj.contains_key("$ref"),
            "{path} contains unsupported $ref: {schema}"
        );
        assert!(
            !obj.contains_key("anyOf"),
            "{path} contains unsupported anyOf: {schema}"
        );

        if let Some(schema_type) = obj.get("type") {
            assert!(
                schema_type.is_string(),
                "{path} has non-string type: {schema_type}"
            );
        }

        if let Some(properties) = obj.get("properties").and_then(Value::as_object) {
            for (name, subschema) in properties {
                assert_gemini_compatible_schema(subschema, &format!("{path}.{name}"));
            }
        }

        if let Some(items) = obj.get("items") {
            assert_gemini_compatible_schema(items, &format!("{path}[]"));
        }
    }
}
