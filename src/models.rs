use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

pub(crate) fn parse_string_array(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .map(|values| {
            values
                .into_iter()
                .filter(|value| !value.trim().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_similar_question_slugs(raw: &str) -> Vec<String> {
    let string_values = parse_string_array(raw);
    if !string_values.is_empty() {
        return string_values;
    }

    serde_json::from_str::<Vec<serde_json::Value>>(raw)
        .map(|values| {
            values
                .into_iter()
                .filter_map(|value| match value {
                    serde_json::Value::String(slug) if !slug.trim().is_empty() => Some(slug),
                    serde_json::Value::Object(object) => object
                        .get("titleSlug")
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| object.get("title_slug").and_then(serde_json::Value::as_str))
                        .or_else(|| object.get("slug").and_then(serde_json::Value::as_str))
                        .filter(|slug| !slug.trim().is_empty())
                        .map(str::to_string),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn deserialize_string_array<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => Ok(parse_string_array(&s)),
        _ => Ok(Vec::new()),
    }
}

fn deserialize_similar_question_slugs<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => Ok(parse_similar_question_slugs(&s)),
        _ => Ok(Vec::new()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Problem {
    pub id: String,
    pub source: String,
    pub slug: String,
    pub title: Option<String>,
    pub title_cn: Option<String>,
    pub difficulty: Option<String>,
    pub ac_rate: Option<f64>,
    pub rating: Option<f64>,
    pub contest: Option<String>,
    pub problem_index: Option<String>,
    #[serde(deserialize_with = "deserialize_string_array", default)]
    pub tags: Vec<String>,
    pub link: Option<String>,
    pub category: Option<String>,
    pub paid_only: Option<i32>,
    pub content: Option<String>,
    pub content_cn: Option<String>,
    #[serde(deserialize_with = "deserialize_similar_question_slugs", default)]
    pub similar_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemRecord {
    pub id: String,
    pub source: String,
    pub slug: String,
    pub title: Option<String>,
    pub title_cn: Option<String>,
    pub difficulty: Option<String>,
    pub ac_rate: Option<f64>,
    pub rating: Option<f64>,
    pub contest: Option<String>,
    pub problem_index: Option<String>,
    #[serde(deserialize_with = "deserialize_string_array", default)]
    pub tags: Vec<String>,
    pub link: Option<String>,
    pub category: Option<String>,
    pub paid_only: Option<i32>,
    pub content: Option<String>,
    pub content_cn: Option<String>,
    #[serde(deserialize_with = "deserialize_similar_question_slugs", default)]
    pub similar_questions: Vec<String>,
}

impl From<ProblemRecord> for Problem {
    fn from(record: ProblemRecord) -> Self {
        Self {
            id: record.id,
            source: record.source,
            slug: record.slug,
            title: record.title,
            title_cn: record.title_cn,
            difficulty: record.difficulty,
            ac_rate: record.ac_rate,
            rating: record.rating,
            contest: record.contest,
            problem_index: record.problem_index,
            tags: record.tags,
            link: record.link,
            category: record.category,
            paid_only: record.paid_only,
            content: record.content,
            content_cn: record.content_cn,
            similar_questions: record.similar_questions,
        }
    }
}

impl From<Problem> for ProblemRecord {
    fn from(problem: Problem) -> Self {
        Self {
            id: problem.id,
            source: problem.source,
            slug: problem.slug,
            title: problem.title,
            title_cn: problem.title_cn,
            difficulty: problem.difficulty,
            ac_rate: problem.ac_rate,
            rating: problem.rating,
            contest: problem.contest,
            problem_index: problem.problem_index,
            tags: problem.tags,
            link: problem.link,
            category: problem.category,
            paid_only: problem.paid_only,
            content: problem.content,
            content_cn: problem.content_cn,
            similar_questions: problem.similar_questions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProblemSummary {
    pub id: String,
    pub source: String,
    pub slug: String,
    pub title: Option<String>,
    pub title_cn: Option<String>,
    pub difficulty: Option<String>,
    pub ac_rate: Option<f64>,
    pub rating: Option<f64>,
    pub contest: Option<String>,
    pub problem_index: Option<String>,
    pub tags: Vec<String>,
    pub link: Option<String>,
}

impl From<ProblemRecord> for ProblemSummary {
    fn from(record: ProblemRecord) -> Self {
        Self {
            id: record.id,
            source: record.source,
            slug: record.slug,
            title: record.title,
            title_cn: record.title_cn,
            difficulty: record.difficulty,
            ac_rate: record.ac_rate,
            rating: record.rating,
            contest: record.contest,
            problem_index: record.problem_index,
            tags: record.tags,
            link: record.link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DailyChallenge {
    pub date: String,
    pub source: String,
    pub problems: Vec<Problem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyChallengeRecord {
    pub date: String,
    pub source: String,
    pub problems: Vec<ProblemRecord>,
}

impl From<DailyChallengeRecord> for DailyChallenge {
    fn from(record: DailyChallengeRecord) -> Self {
        Self {
            date: record.date,
            source: record.source,
            problems: record.problems.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DailyChallenge> for DailyChallengeRecord {
    fn from(daily: DailyChallenge) -> Self {
        Self {
            date: daily.date,
            source: daily.source,
            problems: daily.problems.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeetCodeDomain {
    Com,
    Cn,
}

impl LeetCodeDomain {
    pub fn today(&self) -> String {
        match self {
            Self::Com => Utc::now().format("%Y-%m-%d").to_string(),
            Self::Cn => {
                let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
                Utc::now()
                    .with_timezone(&cst)
                    .format("%Y-%m-%d")
                    .to_string()
            }
        }
    }

    pub fn today_naive(&self) -> chrono::NaiveDate {
        match self {
            Self::Com => Utc::now().date_naive(),
            Self::Cn => {
                let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
                Utc::now().with_timezone(&cst).date_naive()
            }
        }
    }
}

impl fmt::Display for LeetCodeDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Com => write!(f, "com"),
            Self::Cn => write!(f, "cn"),
        }
    }
}

impl FromStr for LeetCodeDomain {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "com" => Ok(Self::Com),
            "cn" => Ok(Self::Cn),
            _ => Err(format!("invalid domain: {}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for LeetCodeDomain {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for LeetCodeDomain {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ApiToken {
    pub token: String,
    pub label: Option<String>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub is_active: i32,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CrawlerJob {
    pub job_id: String,
    pub source: String,
    pub args: Vec<String>,
    pub trigger: CrawlerTrigger,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: CrawlerStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

fn lossy_tail(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let start = bytes.len().saturating_sub(MAX_OUTPUT_BYTES);
    Some(String::from_utf8_lossy(&bytes[start..]).into_owned())
}

impl CrawlerJob {
    pub fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.stdout = lossy_tail(&stdout);
        self.stderr = lossy_tail(&stderr);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveCrawlerPid {
    pub job_id: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerStatus {
    Running,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerTrigger {
    Admin,
    DailyFallback,
}

impl std::fmt::Display for CrawlerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::TimedOut => write!(f, "timed_out"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::fmt::Display for CrawlerTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::DailyFallback => write!(f, "daily_fallback"),
        }
    }
}

pub const MANUAL_CRAWLER_RUNTIME_KEY: &str = "manual";

pub fn manual_crawler_runtime_key() -> &'static str {
    MANUAL_CRAWLER_RUNTIME_KEY
}

pub fn daily_fallback_crawler_runtime_key(source: &str, date: &str) -> String {
    format!("daily_fallback:{source}:{date}")
}

pub const JOB_ARTIFACTS_ROOT: &str = "scripts/logs";
pub const JOB_STDOUT_LOG: &str = "stdout.log";
pub const JOB_STDERR_LOG: &str = "stderr.log";
pub const JOB_PYTHON_LOG: &str = "python.log";
pub const JOB_PROGRESS_FILE: &str = "progress.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    Crawler,
    Embedding,
}

impl JobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Crawler => "crawler",
            Self::Embedding => "embedding",
        }
    }
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "crawler" => Ok(Self::Crawler),
            "embedding" => Ok(Self::Embedding),
            _ => Err(format!("invalid job type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobArtifactPaths {
    pub job_type: JobType,
    pub job_id: String,
    pub root_dir: PathBuf,
    pub job_dir: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub python_log: PathBuf,
    pub progress: PathBuf,
}

fn validate_job_id(job_id: &str) -> Result<(), String> {
    if job_id.is_empty() {
        return Err("job_id must not be empty".to_string());
    }
    let path = Path::new(job_id);
    if path.is_absolute() {
        return Err("job_id must be relative".to_string());
    }
    if path.components().count() != 1 {
        return Err("job_id must not contain path separators".to_string());
    }
    match path.components().next() {
        Some(Component::Normal(_)) => Ok(()),
        _ => Err("job_id must be a normal path segment".to_string()),
    }
}

impl JobArtifactPaths {
    pub fn new(job_type: JobType, job_id: impl AsRef<str>) -> Result<Self, String> {
        Self::with_root(JOB_ARTIFACTS_ROOT, job_type, job_id)
    }

    pub fn with_root(
        root: impl AsRef<Path>,
        job_type: JobType,
        job_id: impl AsRef<str>,
    ) -> Result<Self, String> {
        let root_dir = root.as_ref().to_path_buf();
        let job_id = job_id.as_ref().to_string();
        validate_job_id(&job_id)?;
        let job_dir = root_dir.join(job_type.as_str()).join(&job_id);
        Ok(Self {
            job_type,
            job_id,
            stdout: job_dir.join(JOB_STDOUT_LOG),
            stderr: job_dir.join(JOB_STDERR_LOG),
            python_log: job_dir.join(JOB_PYTHON_LOG),
            progress: job_dir.join(JOB_PROGRESS_FILE),
            root_dir,
            job_dir,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct JobArtifactMetadata {
    pub job_id: String,
    pub job_type: JobType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<CrawlerTrigger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl From<&CrawlerJob> for JobArtifactMetadata {
    fn from(job: &CrawlerJob) -> Self {
        Self {
            job_id: job.job_id.clone(),
            job_type: JobType::Crawler,
            source: Some(job.source.clone()),
            args: job.args.clone(),
            trigger: Some(job.trigger.clone()),
            started_at: Some(job.started_at.clone()),
            finished_at: job.finished_at.clone(),
            updated_at: job
                .finished_at
                .clone()
                .or_else(|| Some(job.started_at.clone())),
        }
    }
}

impl From<&EmbeddingJob> for JobArtifactMetadata {
    fn from(job: &EmbeddingJob) -> Self {
        Self {
            job_id: job.job_id.clone(),
            job_type: JobType::Embedding,
            source: Some(job.source.clone()),
            args: job.args.clone(),
            trigger: None,
            started_at: Some(job.started_at.clone()),
            finished_at: job.finished_at.clone(),
            updated_at: job
                .finished_at
                .clone()
                .or_else(|| Some(job.started_at.clone())),
        }
    }
}

// Per-source argument validation

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    None,
    Date,
    Int,
    Float,
    Str,
    YearMonth,
    Domain,
    Source,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ArgSpec {
    pub flag: &'static str,
    pub arity: u8,
    pub value_type: ValueType,
    pub ui_exposed: bool,
}

pub static LEETCODE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--sync-problemset",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--init",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--full",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--daily",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--date",
        arity: 1,
        value_type: ValueType::Date,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--monthly",
        arity: 2,
        value_type: ValueType::YearMonth,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content-workers",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--domain",
        arity: 1,
        value_type: ValueType::Domain,
        ui_exposed: true,
    },
];

pub static ATCODER_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--problem",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--sync-problemset",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--sync-kenkoooo",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--sync-history",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--fetch-contest",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fetch-all",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--resume",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--no-resume",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--contest",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--status",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--reprocess-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static CODEFORCES_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--problem",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--sync-problemset",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fetch-contest",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fetch-all",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--resume",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--no-resume",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--contest",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--status",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-problems",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--reprocess-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--include-gym",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static LUOGU_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--problem",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--sync-problemset",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--sync",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--status",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--overwrite",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--batch-size",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--training-list",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--source",
        arity: 1,
        value_type: ValueType::Source,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static SPOJ_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--sync-problemset",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--sync-spoj",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--overwrite",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--source",
        arity: 1,
        value_type: ValueType::Source,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--batch-size",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static DIAG_ARGS: &[ArgSpec] = &[ArgSpec {
    flag: "--test",
    arity: 1,
    value_type: ValueType::Str,
    ui_exposed: true,
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlerSource {
    LeetCode,
    AtCoder,
    Codeforces,
    Luogu,
    Spoj,
    Diag,
}

impl CrawlerSource {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "leetcode" => Ok(Self::LeetCode),
            "atcoder" => Ok(Self::AtCoder),
            "codeforces" => Ok(Self::Codeforces),
            "luogu" => Ok(Self::Luogu),
            "spoj" => Ok(Self::Spoj),
            "diag" => Ok(Self::Diag),
            _ => Err(format!("invalid source: {}", s)),
        }
    }

    pub fn script_name(&self) -> &'static str {
        match self {
            Self::LeetCode => "leetcode.py",
            Self::AtCoder => "atcoder.py",
            Self::Codeforces => "codeforces.py",
            Self::Luogu => "luogu.py",
            Self::Spoj => "luogu.py",
            Self::Diag => "diag.py",
        }
    }

    pub fn arg_specs(&self) -> &'static [ArgSpec] {
        match self {
            Self::LeetCode => LEETCODE_ARGS,
            Self::AtCoder => ATCODER_ARGS,
            Self::Codeforces => CODEFORCES_ARGS,
            Self::Luogu => LUOGU_ARGS,
            Self::Spoj => SPOJ_ARGS,
            Self::Diag => DIAG_ARGS,
        }
    }
}

pub fn validate_args(source: &CrawlerSource, raw_args: &[String]) -> Result<Vec<String>, String> {
    let specs = source.arg_specs();
    let mut seen = std::collections::HashSet::new();
    let mut i = 0;

    while i < raw_args.len() {
        let token = &raw_args[i];
        if !token.starts_with("--") {
            return Err(format!("unexpected value without flag: {}", token));
        }

        let spec = specs
            .iter()
            .find(|s| s.flag == token)
            .ok_or_else(|| format!("unknown argument: {}", token))?;

        if !seen.insert(spec.flag) {
            return Err(format!("duplicate argument: {}", token));
        }

        let arity = spec.arity as usize;
        if i + arity >= raw_args.len() {
            return Err(format!("{} requires {} value(s)", token, arity));
        }

        match spec.value_type {
            ValueType::None => {}
            ValueType::Date => {
                let v = &raw_args[i + 1];
                if chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err() {
                    return Err(format!(
                        "{}: invalid date '{}', expected YYYY-MM-DD",
                        token, v
                    ));
                }
            }
            ValueType::Int => {
                let v = &raw_args[i + 1];
                if v.parse::<u64>().is_err() {
                    return Err(format!("{}: invalid integer '{}'", token, v));
                }
            }
            ValueType::Float => {
                let v = &raw_args[i + 1];
                match v.parse::<f64>() {
                    Ok(f) if f.is_finite() && f > 0.0 => {}
                    _ => return Err(format!("{}: invalid positive float '{}'", token, v)),
                }
            }
            ValueType::Str => {
                let v = &raw_args[i + 1];
                if v.is_empty() {
                    return Err(format!("{}: value must not be empty", token));
                }
                if spec.flag == "--data-dir" || spec.flag == "--db-path" {
                    if v.starts_with('/') {
                        return Err(format!("{}: must be a relative path", token));
                    }
                    if v.contains("..") {
                        return Err(format!("{}: must not contain '..'", token));
                    }
                }
            }
            ValueType::Domain => {
                let v = &raw_args[i + 1];
                if v != "com" && v != "cn" {
                    return Err(format!(
                        "{}: invalid domain '{}', expected 'com' or 'cn'",
                        token, v
                    ));
                }
            }
            ValueType::Source => {
                let v = &raw_args[i + 1];
                if v != "luogu" && v != "spoj" {
                    return Err(format!(
                        "{}: invalid source '{}', expected 'luogu' or 'spoj'",
                        token, v
                    ));
                }
            }
            ValueType::YearMonth => {
                let yv = &raw_args[i + 1];
                let mv = &raw_args[i + 2];
                let year: u16 = yv
                    .parse()
                    .map_err(|_| format!("{}: invalid year '{}'", token, yv))?;
                let month: u8 = mv
                    .parse()
                    .map_err(|_| format!("{}: invalid month '{}'", token, mv))?;
                if !(2000..=2100).contains(&year) {
                    return Err(format!("{}: year must be between 2000 and 2100", token));
                }
                if !(1..=12).contains(&month) {
                    return Err(format!("{}: month must be between 1 and 12", token));
                }
            }
        }

        i += 1 + arity;
    }

    let mut result = raw_args.to_vec();

    // SPOJ runs through luogu.py, which defaults to source="luogu". The admin
    // handler does not inject --source, so enforce it here as the security
    // boundary: a SPOJ request must resolve to --source spoj regardless of caller.
    if matches!(source, CrawlerSource::Spoj) {
        match result.iter().position(|a| a == "--source") {
            Some(idx) => {
                if result.get(idx + 1).map(String::as_str) != Some("spoj") {
                    return Err("spoj source requires --source spoj".to_string());
                }
            }
            None => {
                result.push("--source".to_string());
                result.push("spoj".to_string());
            }
        }
    }

    Ok(result)
}

pub struct DailyFallbackEntry {
    pub job_id: String,
    pub status: CrawlerStatus,
    pub started_at: tokio::time::Instant,
    pub cooldown_until: Option<tokio::time::Instant>,
    pub notify: Arc<Notify>,
    pub completed: Arc<std::sync::atomic::AtomicBool>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl DailyFallbackEntry {
    pub fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.stdout = lossy_tail(&stdout);
        self.stderr = lossy_tail(&stderr);
    }
}

// Embedding job model (parallel to CrawlerJob)

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct EmbeddingJob {
    pub job_id: String,
    pub source: String,
    pub args: Vec<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: CrawlerStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerPhase {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl fmt::Display for CrawlerPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
        };
        f.write_str(value)
    }
}

impl From<CrawlerStatus> for CrawlerPhase {
    fn from(value: CrawlerStatus) -> Self {
        match value {
            CrawlerStatus::Running => Self::Running,
            CrawlerStatus::Completed => Self::Completed,
            CrawlerStatus::Failed => Self::Failed,
            CrawlerStatus::TimedOut => Self::TimedOut,
            CrawlerStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<&CrawlerStatus> for CrawlerPhase {
    fn from(value: &CrawlerStatus) -> Self {
        value.clone().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CrawlerProgress {
    pub phase: CrawlerPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JobArtifactMetadata>,
}

impl CrawlerProgress {
    pub fn queued(metadata: JobArtifactMetadata) -> Self {
        Self {
            phase: CrawlerPhase::Queued,
            message: None,
            updated_at: metadata.updated_at.clone(),
            metadata: Some(metadata),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EmbeddingProgress {
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewrite_progress: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_progress: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JobArtifactMetadata>,
}

impl EmbeddingJob {
    pub fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.stdout = lossy_tail(&stdout);
        self.stderr = lossy_tail(&stderr);
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_args, CrawlerSource};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn validate_args_accepts_canonical_crawler_flags() {
        assert!(validate_args(&CrawlerSource::LeetCode, &args(&["--sync-problemset"])).is_ok());
        assert!(validate_args(&CrawlerSource::AtCoder, &args(&["--problem", "abc321_a"])).is_ok());
        assert!(validate_args(
            &CrawlerSource::AtCoder,
            &args(&["--sync-problemset", "--fetch-contest", "--no-resume"])
        )
        .is_ok());
        assert!(validate_args(&CrawlerSource::Codeforces, &args(&["--problem", "1988A"])).is_ok());
        assert!(validate_args(
            &CrawlerSource::Codeforces,
            &args(&[
                "--sync-problemset",
                "--fetch-contest",
                "--no-resume",
                "--include-gym",
            ])
        )
        .is_ok());
        assert!(validate_args(&CrawlerSource::Luogu, &args(&["--problem", "P1083"])).is_ok());
        assert!(validate_args(&CrawlerSource::Luogu, &args(&["--sync-problemset"])).is_ok());
        assert!(validate_args(
            &CrawlerSource::Spoj,
            &args(&["--sync-problemset", "--source", "spoj"])
        )
        .is_ok());
    }

    #[test]
    fn validate_args_accepts_legacy_crawler_aliases() {
        assert!(validate_args(&CrawlerSource::LeetCode, &args(&["--init"])).is_ok());
        assert!(validate_args(
            &CrawlerSource::AtCoder,
            &args(&[
                "--sync-kenkoooo",
                "--sync-history",
                "--fetch-all",
                "--resume"
            ])
        )
        .is_ok());
        assert!(validate_args(
            &CrawlerSource::Codeforces,
            &args(&["--fetch-all", "--resume"])
        )
        .is_ok());
        assert!(validate_args(&CrawlerSource::Luogu, &args(&["--sync"])).is_ok());
        assert!(validate_args(&CrawlerSource::Spoj, &args(&["--sync-spoj"])).is_ok());
    }

    #[test]
    fn validate_args_rejects_unsupported_canonical_flags() {
        for source in [CrawlerSource::LeetCode, CrawlerSource::Spoj] {
            let err = validate_args(&source, &args(&["--problem", "P1083"])).unwrap_err();
            assert_eq!(err, "unknown argument: --problem");
        }
        for source in [
            CrawlerSource::LeetCode,
            CrawlerSource::Luogu,
            CrawlerSource::Spoj,
        ] {
            let err = validate_args(&source, &args(&["--fetch-contest"])).unwrap_err();
            assert_eq!(err, "unknown argument: --fetch-contest");
        }
    }

    #[test]
    fn validate_args_rejects_invalid_crawler_values() {
        let err = validate_args(&CrawlerSource::LeetCode, &args(&["--domain", "tw"])).unwrap_err();
        assert!(err.contains("invalid domain"));

        let err =
            validate_args(&CrawlerSource::Codeforces, &args(&["--contest", "abc"])).unwrap_err();
        assert!(err.contains("invalid integer"));
    }

    #[test]
    fn validate_args_enforces_spoj_source() {
        // Bare SPOJ request gets --source spoj injected so luogu.py does not
        // fall back to its default Luogu sync.
        let out = validate_args(&CrawlerSource::Spoj, &args(&["--sync-problemset"])).expect("ok");
        assert_eq!(out, args(&["--sync-problemset", "--source", "spoj"]));

        // Explicit --source spoj passes through unchanged (no duplicate).
        let out = validate_args(
            &CrawlerSource::Spoj,
            &args(&["--sync-problemset", "--source", "spoj"]),
        )
        .expect("ok");
        assert_eq!(out, args(&["--sync-problemset", "--source", "spoj"]));

        // A SPOJ request that tries to force Luogu source is rejected.
        let err = validate_args(
            &CrawlerSource::Spoj,
            &args(&["--sync-problemset", "--source", "luogu"]),
        )
        .unwrap_err();
        assert_eq!(err, "spoj source requires --source spoj");
    }

    #[test]
    fn validate_args_restricts_source_value() {
        let err = validate_args(
            &CrawlerSource::Luogu,
            &args(&["--sync-problemset", "--source", "../etc"]),
        )
        .unwrap_err();
        assert!(err.contains("invalid source"));

        assert!(validate_args(
            &CrawlerSource::Luogu,
            &args(&["--sync-problemset", "--source", "luogu"])
        )
        .is_ok());
    }

    #[test]
    fn validate_args_rejects_path_traversal_in_paths() {
        let err =
            validate_args(&CrawlerSource::AtCoder, &args(&["--data-dir", "/abs"])).unwrap_err();
        assert!(err.contains("relative path"));

        let err =
            validate_args(&CrawlerSource::AtCoder, &args(&["--db-path", "../x"])).unwrap_err();
        assert!(err.contains("'..'"));
    }
}
