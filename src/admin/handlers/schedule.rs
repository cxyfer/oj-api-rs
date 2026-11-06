use crate::{
    api::error::ProblemDetail,
    crawler_schedule::{self, CrawlerSchedule},
    models::{CrawlerJob, CrawlerTrigger},
    AppState,
};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCrawlerScheduleRequest {
    pub source: String,
    pub schedule: CrawlerSchedule,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CrawlerScheduleView {
    pub source: String,
    pub schedule: CrawlerSchedule,
    pub effective_timezone: String,
    pub next_run_at: Option<String>,
    pub next_runs: Vec<String>,
    pub last_job: Option<CrawlerJob>,
    pub last_event: Option<serde_json::Value>,
}

async fn view(state: &AppState, source: String, schedule: CrawlerSchedule) -> CrawlerScheduleView {
    let effective =
        crawler_schedule::effective_timezone(&schedule, &state.config.crawler.default_timezone)
            .to_string();
    let next_runs: Vec<_> = if schedule.cron.is_empty() {
        vec![]
    } else {
        crawler_schedule::next_runs_after(
            &schedule,
            &state.config.crawler.default_timezone,
            Utc::now(),
            5,
        )
        .unwrap_or_default()
        .iter()
        .map(|d| d.to_rfc3339())
        .collect()
    };
    let mut jobs: Vec<_> = state.crawler_jobs.lock().await.values().cloned().collect();
    jobs.extend(state.crawler_history.lock().await.iter().cloned());
    let last_job = jobs
        .into_iter()
        .filter(|j| j.source == source && j.trigger == CrawlerTrigger::Scheduled)
        .max_by(|a, b| a.started_at.cmp(&b.started_at))
        .map(|mut j| {
            j.stdout = None;
            j.stderr = None;
            j
        });
    let pool = state.rw_pool.clone();
    let event_key = format!("crawler_schedule_event:{source}");
    let last_event =
        tokio::task::spawn_blocking(move || crate::db::settings::get_setting(&pool, &event_key))
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str(&v).ok());
    CrawlerScheduleView {
        source,
        effective_timezone: effective,
        next_run_at: if schedule.enabled {
            next_runs.first().cloned()
        } else {
            None
        },
        schedule,
        next_runs,
        last_job,
        last_event,
    }
}

#[utoipa::path(get, path="/admin/api/crawler-schedules", responses((status=200,body=serde_json::Value),(status=500,body=ProblemDetail)), security(("admin_secret"=[]),("admin_session"=[])),tag="Admin")]
pub async fn get_crawler_schedules(State(state): State<Arc<AppState>>) -> Response {
    let schedules = match crawler_schedule::read_schedules(&state).await {
        Ok(v) => v,
        Err(e) => return ProblemDetail::internal(e).into_response(),
    };
    if let Err(err) = crate::utils::maybe_refresh_retained_job_state(&state).await {
        tracing::warn!("schedule history refresh failed: {err}");
    }
    let mut views = Vec::new();
    for source in crawler_schedule::SOURCES {
        views.push(view(&state, (*source).into(), schedules[*source].clone()).await);
    }
    Json(serde_json::json!({"schedules":views,"default_timezone":state.config.crawler.default_timezone,"timezones":chrono_tz::TZ_VARIANTS.iter().map(|tz|tz.name()).collect::<Vec<_>>()})).into_response()
}

#[utoipa::path(put, path="/admin/api/crawler-schedules",request_body=UpdateCrawlerScheduleRequest,responses((status=200,body=CrawlerScheduleView),(status=400,body=ProblemDetail),(status=500,body=ProblemDetail)),security(("admin_secret"=[]),("admin_session"=[])),tag="Admin")]
pub async fn put_crawler_schedule(
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<UpdateCrawlerScheduleRequest>,
) -> Response {
    if let Err(e) = crawler_schedule::validate_schedule(
        &request.source,
        &mut request.schedule,
        &state.config.crawler.default_timezone,
    ) {
        return ProblemDetail::bad_request(e).into_response();
    }
    let _update = state.crawler_schedule.update.lock().await;
    let mut schedules = match crawler_schedule::read_schedules(&state).await {
        Ok(v) => v,
        Err(e) => return ProblemDetail::internal(e).into_response(),
    };
    schedules.insert(request.source.clone(), request.schedule.clone());
    let pool = state.rw_pool.clone();
    match tokio::task::spawn_blocking(move || crawler_schedule::save_schedules(&pool, &schedules))
        .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return ProblemDetail::internal(e).into_response(),
        Err(e) => return ProblemDetail::internal(e.to_string()).into_response(),
    }
    state.crawler_schedule.changed.notify_one();
    Json(view(&state, request.source, request.schedule).await).into_response()
}

#[utoipa::path(post,path="/admin/api/crawler-schedules/preview",request_body=UpdateCrawlerScheduleRequest,responses((status=200,body=serde_json::Value),(status=400,body=ProblemDetail)),security(("admin_secret"=[]),("admin_session"=[])),tag="Admin")]
pub async fn preview_crawler_schedule(
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<UpdateCrawlerScheduleRequest>,
) -> Response {
    request.schedule.enabled = true;
    if let Err(e) = crawler_schedule::validate_schedule(
        &request.source,
        &mut request.schedule,
        &state.config.crawler.default_timezone,
    ) {
        return ProblemDetail::bad_request(e).into_response();
    }
    match crawler_schedule::next_runs_after(&request.schedule,&state.config.crawler.default_timezone,Utc::now(),5) {
        Ok(next)=>Json(serde_json::json!({"next_runs":next.iter().map(|date|date.to_rfc3339()).collect::<Vec<_>>(),"effective_timezone":crawler_schedule::effective_timezone(&request.schedule,&state.config.crawler.default_timezone)})).into_response(),
        Err(e)=>ProblemDetail::bad_request(e).into_response(),
    }
}
