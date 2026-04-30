use utoipa::openapi::schema::{Ref, Schema};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "OJ API",
        version = "0.3.3",
        description = "REST API for querying competitive programming problems across multiple online judges (LeetCode, AtCoder, Codeforces, Luogu, SPOJ)."
    ),
    tags(
        (name = "Problems", description = "Problem CRUD operations"),
        (name = "Tags", description = "Tag listing by source"),
        (name = "Resolve", description = "Problem resolution by query"),
        (name = "Daily", description = "Daily challenge retrieval"),
        (name = "Similar", description = "Similar problem search"),
        (name = "Status", description = "Platform status"),
        (name = "Health", description = "Health check"),
        (name = "Admin", description = "Administrative operations (requires admin authentication)")
    ),
    paths(
        // Public API
        crate::api::problems::get_problem,
        crate::api::problems::batch_problems,
        crate::api::problems::list_problems,
        crate::api::problems::list_tags,
        crate::api::resolve::resolve,
        crate::api::daily::get_daily,
        crate::api::similar::similar_by_problem,
        crate::api::similar::similar_by_text,
        crate::api::status::get_status,
        crate::health::health_check,
        // Admin API - Problems
        crate::admin::handlers::create_problem,
        crate::admin::handlers::get_problems_list,
        crate::admin::handlers::get_tags_list,
        crate::admin::handlers::get_problem_detail,
        crate::admin::handlers::update_problem,
        crate::admin::handlers::delete_problem,
        // Admin API - Tokens
        crate::admin::handlers::list_tokens,
        crate::admin::handlers::create_token,
        crate::admin::handlers::revoke_token,
        // Admin API - Settings
        crate::admin::handlers::get_token_auth_setting,
        crate::admin::handlers::set_token_auth_setting,
        // Admin API - Crawlers
        crate::admin::handlers::trigger_crawler,
        crate::admin::handlers::cancel_crawler,
        crate::admin::handlers::crawler_status,
        crate::admin::handlers::crawler_output,
        crate::admin::handlers::crawler_progress,
        // Admin API - Embeddings
        crate::admin::handlers::embedding_stats,
        crate::admin::handlers::trigger_embedding,
        crate::admin::handlers::cancel_embedding,
        crate::admin::handlers::embedding_status,
        crate::admin::handlers::embedding_output,
        crate::admin::handlers::embedding_progress,
    ),
    components(schemas(
        // Core domain
        crate::models::Problem,
        crate::models::ProblemSummary,
        crate::models::DailyChallenge,
        crate::models::ApiToken,
        crate::models::CrawlerJob,
        crate::models::CrawlerStatus,
        crate::models::CrawlerTrigger,
        crate::models::JobType,
        crate::models::JobArtifactMetadata,
        crate::models::EmbeddingJob,
        crate::models::CrawlerPhase,
        crate::models::CrawlerProgress,
        crate::models::EmbeddingProgress,
        // API types
        crate::api::error::ProblemDetail,
        crate::api::error::FieldError,
        crate::api::problems::ProblemDetailResponse,
        crate::api::problems::ListMeta,
        crate::api::problems::ListResponse<crate::models::ProblemSummary>,
        crate::api::problems::BatchNotFoundItem,
        crate::api::problems::BatchResponse<crate::models::ProblemSummary>,
        crate::api::problems::BatchResponse<crate::api::problems::ProblemDetailResponse>,
        crate::api::resolve::ResolveResponse,
        crate::api::daily::DailyChallengeResponse,
        crate::api::daily::DailyFetchingResponse,
        crate::api::similar::SimilarResponse,
        crate::api::similar::SimilarResult,
        crate::api::status::StatusResponse,
        crate::db::problems::PlatformStats,
        // Admin request types
        crate::admin::handlers::CreateProblemRequest,
        crate::admin::handlers::CreateTokenRequest,
        crate::admin::handlers::TriggerCrawlerRequest,
        crate::admin::handlers::TriggerEmbeddingRequest,
        crate::admin::handlers::TokenAuthSettingRequest,
        crate::admin::handlers::CrawlerStatusResponse,
        crate::admin::handlers::EmbeddingStatusResponse,
    )),
    modifiers(&SecurityAddon, &BatchResponseAddon)
)]
pub struct ApiDoc;

/// Patches `batch_problems` 200 response to
/// `oneOf[BatchResponse<ProblemSummary>, BatchResponse<ProblemDetailResponse>]`
/// so the spec accurately reflects the `detail` query parameter.
struct BatchResponseAddon;

impl utoipa::Modify for BatchResponseAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::schema::{Array, ObjectBuilder, OneOfBuilder};

        let components = match openapi.components.as_ref() {
            Some(c) => c,
            None => return,
        };

        let has_summary = components
            .schemas
            .contains_key("BatchResponse_ProblemSummary");
        let has_detail = components
            .schemas
            .contains_key("BatchResponse_ProblemDetailResponse");
        if !has_summary || !has_detail {
            return;
        }

        fn make_batch_response(result_ref: Ref) -> Schema {
            Schema::Object(
                ObjectBuilder::new()
                    .property("results", Array::new(result_ref))
                    .required("results")
                    .property(
                        "not_found",
                        Array::new(Ref::from_schema_name("BatchNotFoundItem")),
                    )
                    .required("not_found")
                    .build(),
            )
        }

        let one_of = OneOfBuilder::new()
            .item(make_batch_response(Ref::from_schema_name("ProblemSummary")))
            .item(make_batch_response(Ref::from_schema_name(
                "ProblemDetailResponse",
            )))
            .description(Some(
                "Batch results. Schema varies by `detail` query parameter: \
                 ProblemSummary when false (default), ProblemDetailResponse when true."
                    .to_string(),
            ))
            .build();

        if let Some(path_item) = openapi.paths.paths.get_mut("/api/v1/problems/batch") {
            if let Some(post) = path_item.post.as_mut() {
                if let Some(utoipa::openapi::RefOr::T(response)) =
                    post.responses.responses.get_mut("200")
                {
                    for content in response.content.values_mut() {
                        content.schema =
                            Some(utoipa::openapi::RefOr::T(Schema::OneOf(one_of.clone())));
                    }
                }
            }
        }
    }
}

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
        components.add_security_scheme(
            "admin_secret",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-admin-secret"))),
        );
        components.add_security_scheme(
            "admin_session",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("oj_admin_session"))),
        );
    }
}
